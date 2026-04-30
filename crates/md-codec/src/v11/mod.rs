//! md-codec v0.11 wire format implementation.
//!
//! See `design/SPEC_v0_11_wire_format.md` for the normative spec.
//!
//! This module is an in-progress parallel implementation alongside the
//! existing v0.x byte-aligned codec. v0.11 is bit-aligned per spec D7.

pub mod error;
pub mod bitstream;
pub mod varint;
pub mod header;
pub mod origin_path;
pub mod use_site_path;
pub mod tag;
pub mod tree;
pub mod tlv;
pub mod validate;
pub mod encode;
pub mod decode;
pub mod identity;
