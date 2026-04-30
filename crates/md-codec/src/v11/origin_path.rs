//! Origin-path-decl block per spec §3.4.
//!
//! Block format:
//!   shared mode (bit 4 = 0): [n: 5 bits, encoded n-1][origin-path-encoding]
//!   divergent mode (bit 4 = 1): [n: 5 bits, encoded n-1][origin-path-encoding × n]
//!
//! origin-path-encoding (explicit-only per D19′):
//!   [depth: 4 bits][component × depth]
//!
//! component:
//!   [hardened: 1 bit][value: LP4-ext varint]

use crate::v11::bitstream::{BitReader, BitWriter};
use crate::v11::error::V11Error;
use crate::v11::varint::{read_varint, write_varint};

/// A single BIP-32 path component (e.g. `84'` or `0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathComponent {
    /// Whether this component is hardened (apostrophe in BIP-32 notation).
    pub hardened: bool,
    /// Index value (u31 effective range, encoded as LP4-ext varint).
    pub value: u32,
}

impl PathComponent {
    /// Encode this component into `w`: 1 hardened bit + LP4-ext varint value.
    pub fn write(&self, w: &mut BitWriter) {
        w.write_bits(u64::from(self.hardened), 1);
        write_varint(w, self.value);
    }

    /// Decode a `PathComponent` from `r`.
    pub fn read(r: &mut BitReader) -> Result<Self, V11Error> {
        let hardened = r.read_bits(1)? != 0;
        let value = read_varint(r)?;
        Ok(Self { hardened, value })
    }
}

/// Maximum number of components in a single origin path (4-bit depth field).
pub const MAX_PATH_COMPONENTS: usize = 15;

/// An explicit BIP-32 origin path (a sequence of `PathComponent`s).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginPath {
    /// Ordered components from root toward leaf.
    pub components: Vec<PathComponent>,
}

impl OriginPath {
    /// Encode the path: 4-bit depth followed by each component.
    pub fn write(&self, w: &mut BitWriter) -> Result<(), V11Error> {
        if self.components.len() > MAX_PATH_COMPONENTS {
            return Err(V11Error::PathDepthExceeded {
                got: self.components.len(),
                max: MAX_PATH_COMPONENTS,
            });
        }
        w.write_bits(self.components.len() as u64, 4);
        for c in &self.components {
            c.write(w);
        }
        Ok(())
    }

    /// Decode an `OriginPath` from `r`.
    pub fn read(r: &mut BitReader) -> Result<Self, V11Error> {
        let depth = r.read_bits(4)? as usize;
        let mut components = Vec::with_capacity(depth);
        for _ in 0..depth {
            components.push(PathComponent::read(r)?);
        }
        Ok(Self { components })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bip84() -> OriginPath {
        // m/84'/0'/0'
        OriginPath {
            components: vec![
                PathComponent { hardened: true, value: 84 },
                PathComponent { hardened: true, value: 0 },
                PathComponent { hardened: true, value: 0 },
            ],
        }
    }

    #[test]
    fn origin_path_round_trip_bip84() {
        let p = bip84();
        let mut w = BitWriter::new();
        p.write(&mut w).unwrap();
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(OriginPath::read(&mut r).unwrap(), p);
    }

    #[test]
    fn origin_path_bit_cost_bip84() {
        // depth(4) + 84' (1+11) + 0' (1+4) + 0' (1+4) = 26 bits
        let p = bip84();
        let mut w = BitWriter::new();
        p.write(&mut w).unwrap();
        assert_eq!(w.bit_len(), 26);
    }

    #[test]
    fn origin_path_rejects_depth_too_large() {
        let p = OriginPath {
            components: (0..16).map(|_| PathComponent { hardened: false, value: 0 }).collect(),
        };
        let mut w = BitWriter::new();
        assert!(matches!(
            p.write(&mut w),
            Err(V11Error::PathDepthExceeded { got: 16, max: 15 })
        ));
    }
}
