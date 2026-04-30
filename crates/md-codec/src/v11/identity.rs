//! Identity computation per spec §8.

use crate::v11::encode::{encode_payload, Descriptor};
use crate::v11::error::V11Error;
use bitcoin::hashes::{sha256, Hash};

/// 128-bit canonical identifier for an md1 encoding (spec §8).
///
/// Computed as the first 16 bytes of `SHA-256` over the canonical
/// bit-packed payload bytes produced by [`encode_payload`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Md1EncodingId([u8; 16]);

impl Md1EncodingId {
    /// Construct from a raw 16-byte array.
    pub fn new(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Borrow the underlying 16-byte identifier.
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// Return the 4-byte fingerprint (first 4 bytes of the id).
    pub fn fingerprint(&self) -> [u8; 4] {
        let mut fp = [0u8; 4];
        fp.copy_from_slice(&self.0[0..4]);
        fp
    }
}

/// Compute the [`Md1EncodingId`] for a descriptor by hashing its canonical
/// bit-packed payload encoding (spec §8).
pub fn compute_md1_encoding_id(d: &Descriptor) -> Result<Md1EncodingId, V11Error> {
    let (bytes, _bit_len) = encode_payload(d)?;
    let hash = sha256::Hash::hash(&bytes);
    let mut id = [0u8; 16];
    id.copy_from_slice(&hash.to_byte_array()[0..16]);
    Ok(Md1EncodingId(id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v11::origin_path::{OriginPath, PathComponent, PathDecl, PathDeclPaths};
    use crate::v11::tag::Tag;
    use crate::v11::tlv::TlvSection;
    use crate::v11::tree::{Body, Node};
    use crate::v11::use_site_path::UseSitePath;

    fn bip84_descriptor() -> Descriptor {
        Descriptor {
            n: 1,
            path_decl: PathDecl {
                n: 1,
                paths: PathDeclPaths::Shared(OriginPath {
                    components: vec![
                        PathComponent { hardened: true, value: 84 },
                        PathComponent { hardened: true, value: 0 },
                        PathComponent { hardened: true, value: 0 },
                    ],
                }),
            },
            use_site_path: UseSitePath::standard_multipath(),
            tree: Node {
                tag: Tag::Wpkh,
                body: Body::KeyArg { index: 0 },
            },
            tlv: TlvSection::new_empty(),
        }
    }

    #[test]
    fn md1_encoding_id_deterministic() {
        let d = bip84_descriptor();
        let id1 = compute_md1_encoding_id(&d).unwrap();
        let id2 = compute_md1_encoding_id(&d).unwrap();
        assert_eq!(id1, id2);
    }

    #[test]
    fn md1_encoding_id_differs_for_different_paths() {
        let d1 = bip84_descriptor();
        let mut d2 = bip84_descriptor();
        if let PathDeclPaths::Shared(p) = &mut d2.path_decl.paths {
            p.components[2] = PathComponent { hardened: true, value: 1 };
        }
        let id1 = compute_md1_encoding_id(&d1).unwrap();
        let id2 = compute_md1_encoding_id(&d2).unwrap();
        assert_ne!(id1, id2);
    }
}
