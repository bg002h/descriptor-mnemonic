//! Identity computation per spec §8.

use crate::bitstream::BitWriter;
use crate::encode::{encode_payload, Descriptor};
use crate::error::Error;
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
pub fn compute_md1_encoding_id(d: &Descriptor) -> Result<Md1EncodingId, Error> {
    let (bytes, _bit_len) = encode_payload(d)?;
    let hash = sha256::Hash::hash(&bytes);
    let mut id = [0u8; 16];
    id.copy_from_slice(&hash.to_byte_array()[0..16]);
    Ok(Md1EncodingId(id))
}

/// 128-bit BIP 388 wallet-descriptor-template identifier (spec §8.1, γ-flavor).
///
/// Hashes ONLY the BIP 388 template content: use-site-path-decl bits, tree
/// bits, and the `UseSitePathOverrides` TLV entry bits when present. Excludes
/// the header, origin-path-decl, `Fingerprints` TLV, HRP, and BCH checksum,
/// so it is invariant to origin-path changes (e.g. account index) and to
/// fingerprint additions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WalletDescriptorTemplateId([u8; 16]);

impl WalletDescriptorTemplateId {
    /// Construct from a raw 16-byte array.
    pub fn new(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Borrow the underlying 16-byte identifier.
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

/// Compute the [`WalletDescriptorTemplateId`] for a descriptor by hashing only
/// the BIP 388 template content per spec §8.1.
pub fn compute_wallet_descriptor_template_id(
    d: &Descriptor,
) -> Result<WalletDescriptorTemplateId, Error> {
    let mut w = BitWriter::new();
    // Per spec §8.1: use-site-path-decl bits || tree bits || UseSitePathOverrides TLV bits
    let kiw = d.key_index_width();
    d.use_site_path.write(&mut w)?;
    crate::tree::write_node(&mut w, &d.tree, kiw)?;
    if let Some(overrides) = &d.tlv.use_site_path_overrides {
        // Re-encode the UseSitePathOverrides TLV ENTRY (tag + length + payload).
        let mut sub = BitWriter::new();
        for (idx, path) in overrides {
            sub.write_bits(u64::from(*idx), kiw as usize);
            path.write(&mut sub)?;
        }
        let bit_len = sub.bit_len();
        w.write_bits(u64::from(crate::tlv::TLV_USE_SITE_PATH_OVERRIDES), 5);
        crate::varint::write_varint(&mut w, bit_len as u32)?;
        let payload = sub.into_bytes();
        let mut subr = crate::bitstream::BitReader::new(&payload);
        let mut remaining = bit_len;
        while remaining > 0 {
            let chunk = remaining.min(8);
            let bits = subr.read_bits(chunk)?;
            w.write_bits(bits, chunk);
            remaining -= chunk;
        }
    }
    let bytes = w.into_bytes();
    let hash = sha256::Hash::hash(&bytes);
    let mut id = [0u8; 16];
    id.copy_from_slice(&hash.to_byte_array()[0..16]);
    Ok(WalletDescriptorTemplateId(id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::origin_path::{OriginPath, PathComponent, PathDecl, PathDeclPaths};
    use crate::tag::Tag;
    use crate::tlv::TlvSection;
    use crate::tree::{Body, Node};
    use crate::use_site_path::UseSitePath;

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

    #[test]
    fn wdt_id_invariant_to_origin_path_change() {
        let d1 = bip84_descriptor();
        let mut d2 = bip84_descriptor();
        if let PathDeclPaths::Shared(p) = &mut d2.path_decl.paths {
            p.components[2] = PathComponent { hardened: true, value: 1 };
        }
        let id1 = compute_wallet_descriptor_template_id(&d1).unwrap();
        let id2 = compute_wallet_descriptor_template_id(&d2).unwrap();
        // Same template structure (use-site path, tree) → same WDT-Id
        assert_eq!(id1, id2);
    }

    #[test]
    fn wdt_id_differs_for_different_use_site_paths() {
        let d1 = bip84_descriptor();
        let mut d2 = bip84_descriptor();
        d2.use_site_path = UseSitePath { multipath: None, wildcard_hardened: false };
        let id1 = compute_wallet_descriptor_template_id(&d1).unwrap();
        let id2 = compute_wallet_descriptor_template_id(&d2).unwrap();
        assert_ne!(id1, id2);
    }

    #[test]
    fn wdt_id_invariant_to_fingerprint_addition() {
        let d1 = bip84_descriptor();
        let mut d2 = bip84_descriptor();
        d2.tlv.fingerprints = Some(vec![(0u8, [0xaa, 0xbb, 0xcc, 0xdd])]);
        let id1 = compute_wallet_descriptor_template_id(&d1).unwrap();
        let id2 = compute_wallet_descriptor_template_id(&d2).unwrap();
        // Fingerprints are excluded from WDT-Id hash domain
        assert_eq!(id1, id2);
    }
}
