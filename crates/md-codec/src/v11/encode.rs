//! Top-level encoder per spec §13.3.

use crate::v11::bitstream::BitWriter;
use crate::v11::error::V11Error;
use crate::v11::header::Header;
use crate::v11::origin_path::{PathDecl, PathDeclPaths};
use crate::v11::tlv::TlvSection;
use crate::v11::tree::{write_node, Body, Node};
use crate::v11::use_site_path::UseSitePath;

/// Top-level descriptor parsed/built from a v0.11 wire payload.
///
/// Each field corresponds to a spec section: Header (§3.2), origin
/// `PathDecl` (§3.3), use-site `UseSitePath` (§3.4), descriptor `tree`
/// (§3.5–3.6), and trailing `tlv` section (§3.7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Descriptor {
    /// Number of placeholders (1-indexed key universe size).
    pub n: u8,
    /// Origin path declaration (single or per-`@N` divergent).
    pub path_decl: PathDecl,
    /// Use-site (post-key) path applied to every key by default.
    pub use_site_path: UseSitePath,
    /// Descriptor tree root node.
    pub tree: Node,
    /// Trailing TLV section (overrides, fingerprints, etc.).
    pub tlv: TlvSection,
}

impl Descriptor {
    /// Bit width for placeholder-index encoding: ⌈log₂(n)⌉ for n ≥ 2; 0 for n = 1.
    pub fn key_index_width(&self) -> u8 {
        if self.n <= 1 {
            0
        } else {
            (32 - (self.n as u32 - 1).leading_zeros()) as u8
        }
    }
}

/// Encode a [`Descriptor`] into the canonical payload bit stream and return
/// `(bytes, total_bit_count)`. The bytes are zero-padded; `total_bit_count`
/// is the exact unpadded length needed for round-trip decoding (see §3.7's
/// "TLV section ends when codex32 total-length is exhausted" rule).
pub fn encode_payload(d: &Descriptor) -> Result<(Vec<u8>, usize), V11Error> {
    crate::v11::validate::validate_placeholder_usage(&d.tree, d.n)?;
    if let Some(overrides) = &d.tlv.use_site_path_overrides {
        crate::v11::validate::validate_multipath_consistency(&d.use_site_path, overrides)?;
    }
    if matches!(d.tree.tag, crate::v11::tag::Tag::Tr) {
        if let Body::Tr { tree: Some(t), .. } = &d.tree.body {
            crate::v11::validate::validate_tap_script_tree(t)?;
        }
    }

    let mut w = BitWriter::new();
    let header = Header {
        version: 0,
        divergent_paths: matches!(d.path_decl.paths, PathDeclPaths::Divergent(_)),
    };
    header.write(&mut w);
    d.path_decl.write(&mut w)?;
    d.use_site_path.write(&mut w)?;
    let kiw = d.key_index_width();
    write_node(&mut w, &d.tree, kiw)?;
    d.tlv.write(&mut w, kiw)?;
    let total_bits = w.bit_len();
    Ok((w.into_bytes(), total_bits))
}

/// Render a codex32 string with optional N-char hyphen grouping for
/// transcription aid. Per spec §10.2, every 4-5 chars optionally separated by
/// `-` for human readability. `group_size = 0` returns the input unchanged
/// (no grouping).
pub fn render_codex32_grouped(s: &str, group_size: usize) -> String {
    if group_size == 0 {
        return s.to_string();
    }
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && i % group_size == 0 {
            out.push('-');
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod render_tests {
    use super::*;

    #[test]
    fn render_groups_at_4() {
        assert_eq!(
            render_codex32_grouped("md1qpz9r4cy7", 4),
            "md1q-pz9r-4cy7"
        );
    }

    #[test]
    fn render_zero_group_size_no_grouping() {
        assert_eq!(
            render_codex32_grouped("md1qpz9r4cy7", 0),
            "md1qpz9r4cy7"
        );
    }
}
