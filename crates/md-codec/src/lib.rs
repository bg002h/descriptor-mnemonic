//! # `md-codec`
//!
//! Reference implementation of the **Mnemonic Descriptor (MD)** format —
//! an engravable backup format for [BIP 388 wallet policies][bip388].
//!
//! [bip388]: https://github.com/bitcoin/bips/blob/master/bip-0388.mediawiki
//!
//! v0.11 wire format: bit-aligned payload, sparse per-`@N` TLV overrides,
//! 5-bit header (3-bit version + reserved bit + `divergent_paths` flag),
//! symbol-aligned codex32 wrapping with HRP `"md"`. See
//! `design/SPEC_v0_11_wire_format.md` for the normative spec.

mod bch;

pub mod bitstream;
pub mod canonical_origin;
pub mod canonicalize;
pub mod chunk;
pub mod codex32;
pub mod decode;
pub mod encode;
pub mod error;
pub mod header;
pub mod identity;
pub mod origin_path;
pub mod phrase;
pub mod tag;
pub mod tlv;
pub mod tree;
pub mod use_site_path;
pub mod validate;
pub mod varint;

pub use canonicalize::canonicalize_placeholder_indices;
pub use chunk::{ChunkHeader, derive_chunk_set_id, reassemble, split};
pub use decode::{decode_md1_string, decode_payload};
pub use encode::{Descriptor, encode_md1_string, encode_payload};
pub use error::Error;
pub use header::Header;
pub use identity::{
    Md1EncodingId, WalletDescriptorTemplateId, WalletPolicyId, compute_md1_encoding_id,
    compute_wallet_descriptor_template_id, compute_wallet_policy_id,
};
pub use origin_path::{OriginPath, PathComponent, PathDecl, PathDeclPaths};
pub use phrase::Phrase;
pub use tag::Tag;
pub use tlv::TlvSection;
