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
pub fn write_node(w: &mut BitWriter, node: &Node, key_index_width: u8) -> Result<(), V11Error> {
    node.tag.write(w);
    match &node.body {
        Body::KeyArg { index } => {
            w.write_bits(u64::from(*index), key_index_width as usize);
        }
        Body::Children(children) => {
            for c in children {
                write_node(w, c, key_index_width)?;
            }
        }
        Body::Variable { k, children } => {
            // Encode k-1 in 5 bits per spec §4.2.
            if !(1..=32).contains(&(*k as u32)) {
                return Err(V11Error::ThresholdOutOfRange { k: *k });
            }
            if !(1..=32).contains(&(children.len() as u32)) {
                return Err(V11Error::ChildCountOutOfRange { count: children.len() });
            }
            w.write_bits((*k - 1) as u64, 5);
            w.write_bits((children.len() - 1) as u64, 5);
            for c in children {
                write_node(w, c, key_index_width)?;
            }
        }
        Body::Tr { key_index, tree } => {
            w.write_bits(u64::from(*key_index), key_index_width as usize);
            w.write_bits(u64::from(tree.is_some()), 1);
            if let Some(t) = tree {
                write_node(w, t, key_index_width)?;
            }
        }
        _ => unimplemented!("filled in later phases"),
    }
    Ok(())
}

/// Decode a [`Node`] from the bit stream.
///
/// `key_index_width` is the bit width used for key-index fields, derived from
/// the descriptor's path-decl head. Filled in across phases 7-11.
pub fn read_node(r: &mut BitReader, key_index_width: u8) -> Result<Node, V11Error> {
    let tag = Tag::read(r)?;
    let body = match tag {
        Tag::PkK | Tag::PkH | Tag::Wpkh | Tag::Pkh => {
            let index = r.read_bits(key_index_width as usize)? as u8;
            Body::KeyArg { index }
        }
        Tag::Sh
        | Tag::Wsh
        | Tag::Check
        | Tag::Verify
        | Tag::Swap
        | Tag::Alt
        | Tag::DupIf
        | Tag::NonZero
        | Tag::ZeroNotEqual => {
            let child = read_node(r, key_index_width)?;
            Body::Children(vec![child])
        }
        Tag::AndV | Tag::AndB | Tag::OrB | Tag::OrC | Tag::OrD | Tag::OrI => {
            let l = read_node(r, key_index_width)?;
            let r2 = read_node(r, key_index_width)?;
            Body::Children(vec![l, r2])
        }
        Tag::AndOr => {
            let a = read_node(r, key_index_width)?;
            let b = read_node(r, key_index_width)?;
            let c = read_node(r, key_index_width)?;
            Body::Children(vec![a, b, c])
        }
        Tag::TapTree => {
            let l = read_node(r, key_index_width)?;
            let r2 = read_node(r, key_index_width)?;
            Body::Children(vec![l, r2])
        }
        Tag::Multi | Tag::SortedMulti | Tag::MultiA | Tag::SortedMultiA | Tag::Thresh => {
            let k = (r.read_bits(5)? + 1) as u8;
            let count = (r.read_bits(5)? + 1) as usize;
            if k as usize > count {
                return Err(V11Error::KGreaterThanN { k, n: count });
            }
            let mut children = Vec::with_capacity(count);
            for _ in 0..count {
                children.push(read_node(r, key_index_width)?);
            }
            Body::Variable { k, children }
        }
        Tag::Tr => {
            let key_index = r.read_bits(key_index_width as usize)? as u8;
            let has_tree = r.read_bits(1)? != 0;
            let tree = if has_tree {
                Some(Box::new(read_node(r, key_index_width)?))
            } else {
                None
            };
            Body::Tr { key_index, tree }
        }
        _ => unimplemented!("filled in later phases"),
    };
    Ok(Node { tag, body })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v11::bitstream::{BitReader, BitWriter};

    #[test]
    fn key_arg_n1_zero_bits() {
        // n=1 ⇒ index_width = 0; key-arg emits zero bits
        let n = Node {
            tag: Tag::PkK,
            body: Body::KeyArg { index: 0 },
        };
        let mut w = BitWriter::new();
        write_node(&mut w, &n, 0).unwrap();
        // Tag: 5 bits + key-arg: 0 bits = 5 bits total
        assert_eq!(w.bit_len(), 5);
    }

    #[test]
    fn key_arg_n3_two_bits() {
        let n = Node {
            tag: Tag::PkK,
            body: Body::KeyArg { index: 2 },
        };
        let mut w = BitWriter::new();
        write_node(&mut w, &n, 2).unwrap();
        // Tag: 5 + key-arg: 2 = 7 bits
        assert_eq!(w.bit_len(), 7);
    }

    #[test]
    fn key_arg_round_trip() {
        let n = Node {
            tag: Tag::PkK,
            body: Body::KeyArg { index: 1 },
        };
        let mut w = BitWriter::new();
        write_node(&mut w, &n, 2).unwrap();
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(read_node(&mut r, 2).unwrap(), n);
    }

    #[test]
    fn wrapper_chain_v_c_pk_round_trip() {
        // v:c:pk_k(@0) — three nested wrappers around PkK
        let n = Node {
            tag: Tag::Verify,
            body: Body::Children(vec![Node {
                tag: Tag::Check,
                body: Body::Children(vec![Node {
                    tag: Tag::PkK,
                    body: Body::KeyArg { index: 0 },
                }]),
            }]),
        };
        let mut w = BitWriter::new();
        write_node(&mut w, &n, 2).unwrap();
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(read_node(&mut r, 2).unwrap(), n);
    }

    #[test]
    fn sortedmulti_2of3_round_trip() {
        let n = Node {
            tag: Tag::SortedMulti,
            body: Body::Variable {
                k: 2,
                children: vec![
                    Node { tag: Tag::PkK, body: Body::KeyArg { index: 0 } },
                    Node { tag: Tag::PkK, body: Body::KeyArg { index: 1 } },
                    Node { tag: Tag::PkK, body: Body::KeyArg { index: 2 } },
                ],
            },
        };
        let mut w = BitWriter::new();
        write_node(&mut w, &n, 2).unwrap();
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(read_node(&mut r, 2).unwrap(), n);
    }

    #[test]
    fn sortedmulti_2of3_bit_cost() {
        // Tag(5) + k=2 (5, encoded 1) + n=3 (5, encoded 2) + 3× PkK (5+2 each = 7) = 5+5+5+21 = 36
        let n = Node {
            tag: Tag::SortedMulti,
            body: Body::Variable {
                k: 2,
                children: vec![
                    Node { tag: Tag::PkK, body: Body::KeyArg { index: 0 } },
                    Node { tag: Tag::PkK, body: Body::KeyArg { index: 1 } },
                    Node { tag: Tag::PkK, body: Body::KeyArg { index: 2 } },
                ],
            },
        };
        let mut w = BitWriter::new();
        write_node(&mut w, &n, 2).unwrap();
        assert_eq!(w.bit_len(), 36);
    }

    #[test]
    fn tr_bip86_no_tree() {
        let n = Node {
            tag: Tag::Tr,
            body: Body::Tr { key_index: 0, tree: None },
        };
        let mut w = BitWriter::new();
        write_node(&mut w, &n, 0).unwrap();
        // Tr tag (5) + key-arg (0 bits, n=1) + has-tree=0 (1 bit) = 6 bits
        assert_eq!(w.bit_len(), 6);
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(read_node(&mut r, 0).unwrap(), n);
    }

    #[test]
    fn thresh_2of3_with_pk_children() {
        // thresh(2, pk_k(@0), pk_k(@1), pk_k(@2))
        let n = Node {
            tag: Tag::Thresh,
            body: Body::Variable {
                k: 2,
                children: vec![
                    Node { tag: Tag::PkK, body: Body::KeyArg { index: 0 } },
                    Node { tag: Tag::PkK, body: Body::KeyArg { index: 1 } },
                    Node { tag: Tag::PkK, body: Body::KeyArg { index: 2 } },
                ],
            },
        };
        let mut w = BitWriter::new();
        write_node(&mut w, &n, 2).unwrap();
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(read_node(&mut r, 2).unwrap(), n);
    }
}
