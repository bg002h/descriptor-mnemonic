//! Top-level decoder per spec §13.2.

use crate::v11::bitstream::BitReader;
use crate::v11::encode::Descriptor;
use crate::v11::error::V11Error;
use crate::v11::header::Header;
use crate::v11::origin_path::PathDecl;
use crate::v11::tlv::TlvSection;
use crate::v11::tree::read_node;
use crate::v11::use_site_path::UseSitePath;

/// Decode a Descriptor from the canonical payload bit stream.
/// `bytes` may be zero-padded; `total_bits` is the exact payload bit count.
pub fn decode_payload(bytes: &[u8], total_bits: usize) -> Result<Descriptor, V11Error> {
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

    crate::v11::validate::validate_placeholder_usage(&descriptor.tree, descriptor.n)?;
    if let Some(overrides) = &descriptor.tlv.use_site_path_overrides {
        crate::v11::validate::validate_multipath_consistency(&descriptor.use_site_path, overrides)?;
    }
    if matches!(descriptor.tree.tag, crate::v11::tag::Tag::Tr) {
        if let crate::v11::tree::Body::Tr { tree: Some(t), .. } = &descriptor.tree.body {
            crate::v11::validate::validate_tap_script_tree(t)?;
        }
    }

    Ok(descriptor)
}
