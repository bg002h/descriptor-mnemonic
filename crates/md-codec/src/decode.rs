//! Top-level decoder per spec §13.2.

use crate::bitstream::BitReader;
use crate::encode::Descriptor;
use crate::error::Error;
use crate::header::Header;
use crate::origin_path::PathDecl;
use crate::tlv::TlvSection;
use crate::tree::read_node;
use crate::use_site_path::UseSitePath;

/// Decode a Descriptor from the canonical payload bit stream.
/// `bytes` may be zero-padded; `total_bits` is the exact payload bit count.
pub fn decode_payload(bytes: &[u8], total_bits: usize) -> Result<Descriptor, Error> {
    let mut r = BitReader::with_bit_limit(bytes, total_bits);

    let header = Header::read(&mut r)?;
    let path_decl = PathDecl::read(&mut r, header.divergent_paths)?;
    let use_site_path = UseSitePath::read(&mut r)?;
    let key_index_width = if path_decl.n <= 1 { 0 }
        else { (32 - (path_decl.n as u32 - 1).leading_zeros()) as u8 };
    let tree = read_node(&mut r, key_index_width)?;
    let tlv = TlvSection::read(&mut r, key_index_width, path_decl.n)?;

    let descriptor = Descriptor {
        n: path_decl.n,
        path_decl,
        use_site_path,
        tree,
        tlv,
    };

    crate::validate::validate_placeholder_usage(&descriptor.tree, descriptor.n)?;
    if let Some(overrides) = &descriptor.tlv.use_site_path_overrides {
        crate::validate::validate_multipath_consistency(&descriptor.use_site_path, overrides)?;
    }
    if matches!(descriptor.tree.tag, crate::tag::Tag::Tr) {
        if let crate::tree::Body::Tr { tree: Some(t), .. } = &descriptor.tree.body {
            crate::validate::validate_tap_script_tree(t)?;
        }
    }
    // Spec v0.13 §6.3 + §6.4: enforce explicit-origin and xpub-validity
    // after the v0.11 ordering / multipath / taptree checks. Order matters:
    // ordering must run first so subsequent checks see canonical indices.
    crate::validate::validate_explicit_origin_required(&descriptor)?;
    crate::validate::validate_xpub_bytes(&descriptor)?;

    Ok(descriptor)
}

/// Decode a Descriptor from a complete codex32 md1 string.
///
/// Uses the symbol-aligned bit count returned by `unwrap_string` (5 × symbol_count),
/// which is exact at the codex32 layer with ≤4 bits of trailing zero-padding —
/// well within the v11 decoder's TLV-rollback tolerance.
pub fn decode_md1_string(s: &str) -> Result<Descriptor, Error> {
    let (bytes, symbol_aligned_bit_count) = crate::codex32::unwrap_string(s)?;
    decode_payload(&bytes, symbol_aligned_bit_count)
}
