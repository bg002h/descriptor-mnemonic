//! Tree (operator AST) per spec §3.6 + §6.

use crate::v11::bitstream::{BitReader, BitWriter};
use crate::v11::error::V11Error;
use crate::v11::tag::Tag;

/// A node in the operator AST: a tag plus its body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// Operator tag identifying this node's kind.
    pub tag: Tag,
    /// Body fields and/or children, shape determined by `tag`.
    pub body: Body,
}

/// Body shape for a [`Node`], determined by its [`Tag`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Body {
    /// No body fields beyond N child nodes (Class 1 fixed-arity).
    Children(Vec<Node>),
    /// Variable-arity (Multi*, Thresh): k, children. n is implicit (= children.len()).
    Variable {
        /// Threshold `k`.
        k: u8,
        /// Child nodes; `n = children.len()`.
        children: Vec<Node>,
    },
    /// Tr's body: key index, has-tree, optional tap-script-tree root.
    /// The wire bit-width for `key_index` is determined by Descriptor.key_index_width()
    /// (parsed from the path-decl head); not carried in the AST.
    Tr {
        /// Internal-key index into the descriptor's key table.
        key_index: u8,
        /// Optional tap-script-tree root.
        tree: Option<Box<Node>>,
    },
    /// Single key-arg (Pkh, Wpkh, PkK, PkH, multi-family children).
    /// Wire bit-width for `index` is determined by the parent Descriptor's
    /// key_index_width(); not carried in the AST.
    KeyArg {
        /// Key index into the descriptor's key table.
        index: u8,
    },
    /// 256-bit hash literal (Sha256, Hash256).
    Hash256Body([u8; 32]),
    /// 160-bit hash literal (Hash160, Ripemd160, RawPkH).
    Hash160Body([u8; 20]),
    /// 32-bit Bitcoin-native u32 (After, Older).
    Timelock(u32),
    /// No body (False, True).
    Empty,
}

/// Encode a [`Node`] to the bit stream.
///
/// `key_index_width` is the bit width used for key-index fields, derived from
/// the descriptor's path-decl head. Filled in across phases 7-11.
pub fn write_node(_w: &mut BitWriter, _n: &Node, _key_index_width: u8) -> Result<(), V11Error> {
    unimplemented!("filled in across phases 7-11")
}

/// Decode a [`Node`] from the bit stream.
///
/// `key_index_width` is the bit width used for key-index fields, derived from
/// the descriptor's path-decl head. Filled in across phases 7-11.
pub fn read_node(_r: &mut BitReader, _key_index_width: u8) -> Result<Node, V11Error> {
    unimplemented!("filled in across phases 7-11")
}
