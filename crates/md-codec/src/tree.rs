//! Tree (operator AST) per spec §3.6 + §6.

use crate::bitstream::{BitReader, BitWriter};
use crate::error::Error;
use crate::tag::Tag;

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
    /// Variable-arity body for `Tag::Thresh` only (post-v0.30 Phase C).
    /// Encodes `k` + N child Nodes. Multi-family tags use [`Body::MultiKeys`]
    /// per SPEC v0.30 §4.
    Variable {
        /// Threshold `k`.
        k: u8,
        /// Child nodes; `n = children.len()`.
        children: Vec<Node>,
    },
    /// Multi-family body (`Tag::Multi`, `SortedMulti`, `MultiA`,
    /// `SortedMultiA`): k-of-n with raw `kiw`-width key indices, NOT full
    /// child Nodes. Per SPEC v0.30 §4: wire layout is
    /// `tag | (k-1)(5) | (n-1)(5) | n × index(kiw)`.
    MultiKeys {
        /// Threshold `k`.
        k: u8,
        /// Placeholder indices `@i`; `n = indices.len()`. Each entry is
        /// emitted as `kiw` bits.
        indices: Vec<u8>,
    },
    /// Tr's body: key index, has-tree, optional tap-script-tree root.
    /// The wire bit-width for `key_index` is determined by Descriptor.key_index_width()
    /// (parsed from the path-decl head); not carried in the AST.
    ///
    /// **v0.18 NUMS sentinel:** the reserved value `key_index = n` (where `n`
    /// is the descriptor's placeholder count) signals that the implicit
    /// internal key is the BIP-341 NUMS H-point
    /// `50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0`.
    /// Encoders MUST emit `key_index = n` iff the descriptor's `tr()`
    /// internal key is exactly the BIP-341 NUMS H-point. Values `0..n-1`
    /// reference `@i` placeholders. Values `> n` are rejected.
    Tr {
        /// Internal-key index into the descriptor's key table, OR the
        /// reserved sentinel `n` (NUMS H-point — see variant doc-comment).
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
pub fn write_node(w: &mut BitWriter, node: &Node, key_index_width: u8) -> Result<(), Error> {
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
            // Thresh-only post-v0.30 Phase C. Encode k-1 in 5 bits per spec §4.2.
            if !(1..=32).contains(&(*k as u32)) {
                return Err(Error::ThresholdOutOfRange { k: *k });
            }
            if !(1..=32).contains(&(children.len() as u32)) {
                return Err(Error::ChildCountOutOfRange {
                    count: children.len(),
                });
            }
            w.write_bits((*k - 1) as u64, 5);
            w.write_bits((children.len() - 1) as u64, 5);
            for c in children {
                write_node(w, c, key_index_width)?;
            }
        }
        Body::MultiKeys { k, indices } => {
            // Multi-family per SPEC v0.30 §4: k-of-n + raw kiw-width indices.
            if !(1..=32).contains(&(*k as u32)) {
                return Err(Error::ThresholdOutOfRange { k: *k });
            }
            if !(1..=32).contains(&(indices.len() as u32)) {
                return Err(Error::ChildCountOutOfRange {
                    count: indices.len(),
                });
            }
            w.write_bits((*k - 1) as u64, 5);
            w.write_bits((indices.len() - 1) as u64, 5);
            for idx in indices {
                w.write_bits(u64::from(*idx), key_index_width as usize);
            }
        }
        Body::Tr { key_index, tree } => {
            w.write_bits(u64::from(*key_index), key_index_width as usize);
            w.write_bits(u64::from(tree.is_some()), 1);
            if let Some(t) = tree {
                write_node(w, t, key_index_width)?;
            }
        }
        Body::Timelock(v) => {
            w.write_bits(u64::from(*v), 32);
        }
        Body::Hash256Body(h) => {
            for byte in h {
                w.write_bits(u64::from(*byte), 8);
            }
        }
        Body::Hash160Body(h) => {
            for byte in h {
                w.write_bits(u64::from(*byte), 8);
            }
        }
        Body::Empty => {}
    }
    Ok(())
}

/// Hard cap on `read_node` recursion depth. Shared across all recursive tags
/// (`Sh`, `AndV`, `AndOr`, `TapTree`, `Multi`, `Tr`, …) as a generic anti-DOS
/// hardening bound — not a spec-mandated value for non-taproot sites. The
/// value 128 happens to coincide with BIP-341 `TAPROOT_CONTROL_MAX_NODE_COUNT`,
/// but its role here is just "any depth a real miniscript expression could
/// plausibly reach + headroom"; P2WSH script-size limits cap practical
/// miniscript depth at well under 50.
pub const MAX_DECODE_DEPTH: u8 = 128;

/// Decode a [`Node`] from the bit stream.
///
/// `key_index_width` is the bit width used for key-index fields, derived from
/// the descriptor's path-decl head. Filled in across phases 7-11.
///
/// Top-level entry point. Internally threads a recursion-depth counter that
/// errors out at [`MAX_DECODE_DEPTH`] before parsing the next node, so a
/// hostile wire payload nesting recursive tags (`Tag::Sh`, `Tag::AndV`,
/// `Tag::TapTree`, etc.) arbitrarily deep cannot blow the Rust stack.
pub fn read_node(r: &mut BitReader, key_index_width: u8) -> Result<Node, Error> {
    read_node_with_depth(r, key_index_width, 0)
}

/// Inner recursive form of `read_node` that threads `depth`. Public callers
/// should use `read_node` instead, which starts at depth 0. Increments
/// `depth` once per call and errors if it reaches [`MAX_DECODE_DEPTH`].
fn read_node_with_depth(r: &mut BitReader, key_index_width: u8, depth: u8) -> Result<Node, Error> {
    if depth >= MAX_DECODE_DEPTH {
        return Err(Error::DecodeRecursionDepthExceeded {
            depth,
            max: MAX_DECODE_DEPTH,
        });
    }
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
            let child = read_node_with_depth(r, key_index_width, depth + 1)?;
            Body::Children(vec![child])
        }
        Tag::AndV | Tag::AndB | Tag::OrB | Tag::OrC | Tag::OrD | Tag::OrI => {
            let l = read_node_with_depth(r, key_index_width, depth + 1)?;
            let r2 = read_node_with_depth(r, key_index_width, depth + 1)?;
            Body::Children(vec![l, r2])
        }
        Tag::AndOr => {
            let a = read_node_with_depth(r, key_index_width, depth + 1)?;
            let b = read_node_with_depth(r, key_index_width, depth + 1)?;
            let c = read_node_with_depth(r, key_index_width, depth + 1)?;
            Body::Children(vec![a, b, c])
        }
        Tag::TapTree => {
            let l = read_node_with_depth(r, key_index_width, depth + 1)?;
            let r2 = read_node_with_depth(r, key_index_width, depth + 1)?;
            Body::Children(vec![l, r2])
        }
        Tag::Multi | Tag::SortedMulti | Tag::MultiA | Tag::SortedMultiA => {
            let k = (r.read_bits(5)? + 1) as u8;
            let count = (r.read_bits(5)? + 1) as usize;
            if k as usize > count {
                return Err(Error::KGreaterThanN { k, n: count });
            }
            let mut indices = Vec::with_capacity(count);
            for _ in 0..count {
                indices.push(r.read_bits(key_index_width as usize)? as u8);
            }
            Body::MultiKeys { k, indices }
        }
        Tag::Thresh => {
            let k = (r.read_bits(5)? + 1) as u8;
            let count = (r.read_bits(5)? + 1) as usize;
            if k as usize > count {
                return Err(Error::KGreaterThanN { k, n: count });
            }
            let mut children = Vec::with_capacity(count);
            for _ in 0..count {
                children.push(read_node_with_depth(r, key_index_width, depth + 1)?);
            }
            Body::Variable { k, children }
        }
        Tag::Tr => {
            let key_index = r.read_bits(key_index_width as usize)? as u8;
            let has_tree = r.read_bits(1)? != 0;
            let tree = if has_tree {
                Some(Box::new(read_node_with_depth(
                    r,
                    key_index_width,
                    depth + 1,
                )?))
            } else {
                None
            };
            Body::Tr { key_index, tree }
        }
        Tag::After | Tag::Older => {
            let v = r.read_bits(32)? as u32;
            Body::Timelock(v)
        }
        Tag::Sha256 => {
            let mut h = [0u8; 32];
            for byte in &mut h {
                *byte = r.read_bits(8)? as u8;
            }
            Body::Hash256Body(h)
        }
        Tag::Hash160 => {
            let mut h = [0u8; 20];
            for byte in &mut h {
                *byte = r.read_bits(8)? as u8;
            }
            Body::Hash160Body(h)
        }
        Tag::Hash256 => {
            let mut h = [0u8; 32];
            for byte in &mut h {
                *byte = r.read_bits(8)? as u8;
            }
            Body::Hash256Body(h)
        }
        Tag::Ripemd160 | Tag::RawPkH => {
            let mut h = [0u8; 20];
            for byte in &mut h {
                *byte = r.read_bits(8)? as u8;
            }
            Body::Hash160Body(h)
        }
        Tag::False | Tag::True => Body::Empty,
    };
    Ok(Node { tag, body })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitstream::{BitReader, BitWriter};

    #[test]
    #[ignore = "v0.30 Phase A: kiw-related bit-count pin stale; lifted in Phase F (NUMS flag) or H (corpus regen)"]
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
    #[ignore = "v0.30 Phase A: kiw-related bit-count pin stale; lifted in Phase F (NUMS flag) or H (corpus regen)"]
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
            body: Body::MultiKeys {
                k: 2,
                indices: vec![0, 1, 2],
            },
        };
        let mut w = BitWriter::new();
        write_node(&mut w, &n, 2).unwrap();
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(read_node(&mut r, 2).unwrap(), n);
    }

    /// v0.30 Phase C — multi packing bit-cost pin.
    /// `Tag(6-bit) | k-1(5) | n-1(5) | 3×kiw(2 at n=3) = 22 bits` (SPEC §4.2).
    /// Saves 14 bits over v0.x's per-child encoding (which was 36 bits).
    #[test]
    fn sortedmulti_2of3_bit_cost() {
        let n = Node {
            tag: Tag::SortedMulti,
            body: Body::MultiKeys {
                k: 2,
                indices: vec![0, 1, 2],
            },
        };
        let mut w = BitWriter::new();
        write_node(&mut w, &n, 2).unwrap();
        assert_eq!(w.bit_len(), 22);
    }

    /// v0.30 Phase C — `Body::MultiKeys` round-trips under `Tag::Multi`.
    #[test]
    fn multi_keys_body_round_trip() {
        let n = Node {
            tag: Tag::Multi,
            body: Body::MultiKeys {
                k: 2,
                indices: vec![0, 1, 2],
            },
        };
        let mut w = BitWriter::new();
        write_node(&mut w, &n, 2).unwrap();
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(read_node(&mut r, 2).unwrap(), n);
    }

    /// v0.30 Phase C — `Body::MultiKeys` round-trips under `Tag::SortedMultiA`.
    #[test]
    fn sortedmulti_a_indices_round_trip() {
        let n = Node {
            tag: Tag::SortedMultiA,
            body: Body::MultiKeys {
                k: 2,
                indices: vec![0, 1, 2],
            },
        };
        let mut w = BitWriter::new();
        write_node(&mut w, &n, 2).unwrap();
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(read_node(&mut r, 2).unwrap(), n);
    }

    #[test]
    #[ignore = "v0.30 Phase A: NUMS-sentinel-based; lifted in Phase F (is_nums flag replaces sentinel)"]
    fn tr_bip86_no_tree() {
        let n = Node {
            tag: Tag::Tr,
            body: Body::Tr {
                key_index: 0,
                tree: None,
            },
        };
        let mut w = BitWriter::new();
        // Synthetic width-0 unit test of the tree layer. v0.18 Descriptor
        // formula gives width=1 at n=1; width=0 only arises at n=0 (no
        // placeholders). The test exercises the zero-width edge of write_node /
        // read_node directly, not the live n=1 encoding path.
        write_node(&mut w, &n, 0).unwrap();
        // Tr tag (5) + key-arg (0 bits, synthetic width=0) + has-tree=0 (1 bit) = 6 bits
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
                    Node {
                        tag: Tag::PkK,
                        body: Body::KeyArg { index: 0 },
                    },
                    Node {
                        tag: Tag::PkK,
                        body: Body::KeyArg { index: 1 },
                    },
                    Node {
                        tag: Tag::PkK,
                        body: Body::KeyArg { index: 2 },
                    },
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
    fn tr_with_single_leaf() {
        // tr(@0, multi_a(2, @1, @2))
        let n = Node {
            tag: Tag::Tr,
            body: Body::Tr {
                key_index: 0,
                tree: Some(Box::new(Node {
                    tag: Tag::MultiA,
                    body: Body::MultiKeys {
                        k: 2,
                        indices: vec![1, 2],
                    },
                })),
            },
        };
        let mut w = BitWriter::new();
        write_node(&mut w, &n, 2).unwrap();
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(read_node(&mut r, 2).unwrap(), n);
    }

    #[test]
    #[ignore = "v0.30 Phase A: tag-width bit-count pin stale; lifted in Phase H (corpus regen)"]
    fn after_700_000_round_trip() {
        let n = Node {
            tag: Tag::After,
            body: Body::Timelock(700_000),
        };
        let mut w = BitWriter::new();
        write_node(&mut w, &n, 0).unwrap();
        // Tag(5) + u32(32) = 37 bits
        assert_eq!(w.bit_len(), 37);
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(read_node(&mut r, 0).unwrap(), n);
    }

    #[test]
    #[ignore = "v0.30 Phase A: tag-width bit-count pin stale; lifted in Phase H (corpus regen)"]
    fn sha256_round_trip() {
        let h = [0xab; 32];
        let n = Node {
            tag: Tag::Sha256,
            body: Body::Hash256Body(h),
        };
        let mut w = BitWriter::new();
        write_node(&mut w, &n, 0).unwrap();
        // Tag(5) + 256 = 261 bits
        assert_eq!(w.bit_len(), 261);
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(read_node(&mut r, 0).unwrap(), n);
    }

    #[test]
    #[ignore = "v0.30 Phase A: tag-width bit-count pin stale; lifted in Phase H (corpus regen)"]
    fn hash160_round_trip() {
        let h = [0xcd; 20];
        let n = Node {
            tag: Tag::Hash160,
            body: Body::Hash160Body(h),
        };
        let mut w = BitWriter::new();
        write_node(&mut w, &n, 0).unwrap();
        // Tag(5) + 160 = 165 bits
        assert_eq!(w.bit_len(), 165);
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(read_node(&mut r, 0).unwrap(), n);
    }

    #[test]
    #[ignore = "v0.30 Phase A: bit-count pin stale (Hash256 promoted from extension to primary); lifted in Phase H (corpus regen)"]
    fn hash256_extension_round_trip() {
        let h = [0xef; 32];
        let n = Node {
            tag: Tag::Hash256,
            body: Body::Hash256Body(h),
        };
        let mut w = BitWriter::new();
        write_node(&mut w, &n, 0).unwrap();
        // Extension tag = 5+5 = 10 bits, then 256 = 266 total
        assert_eq!(w.bit_len(), 266);
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(read_node(&mut r, 0).unwrap(), n);
    }

    #[test]
    fn ripemd160_round_trip() {
        let h = [0x42; 20];
        let n = Node {
            tag: Tag::Ripemd160,
            body: Body::Hash160Body(h),
        };
        let mut w = BitWriter::new();
        write_node(&mut w, &n, 0).unwrap();
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(read_node(&mut r, 0).unwrap(), n);
    }

    #[test]
    #[ignore = "v0.30 Phase A: bit-count pin stale (False promoted from extension to primary); lifted in Phase H (corpus regen)"]
    fn false_round_trip() {
        let n = Node {
            tag: Tag::False,
            body: Body::Empty,
        };
        let mut w = BitWriter::new();
        write_node(&mut w, &n, 0).unwrap();
        assert_eq!(w.bit_len(), 10); // extension tag = 10 bits, no body
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(read_node(&mut r, 0).unwrap(), n);
    }

    #[test]
    fn true_round_trip() {
        let n = Node {
            tag: Tag::True,
            body: Body::Empty,
        };
        let mut w = BitWriter::new();
        write_node(&mut w, &n, 0).unwrap();
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(read_node(&mut r, 0).unwrap(), n);
    }

    #[test]
    fn older_144_round_trip() {
        let n = Node {
            tag: Tag::Older,
            body: Body::Timelock(144),
        };
        let mut w = BitWriter::new();
        write_node(&mut w, &n, 0).unwrap();
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(read_node(&mut r, 0).unwrap(), n);
    }

    #[test]
    #[ignore = "v0.30 Phase A: NUMS-sentinel-based; lifted in Phase F (is_nums flag replaces sentinel)"]
    fn tr_sentinel_n_1_bare_round_trip() {
        // v0.18: tr(<NUMS>) with no script tree at n=1 (single-placeholder
        // descriptor that ignores @0 by going pure script-path). This is the
        // narrowest sentinel case — width changes from 0 (v0.17 ceil(log2(1)))
        // to 1 (v0.18 ceil(log2(2))). Architect C1 from round 1 — without
        // updating decode.rs's key_index_width formula in lockstep, this
        // case silently desyncs the bitstream.
        let n = Node {
            tag: Tag::Tr,
            body: Body::Tr {
                key_index: 1,
                tree: None,
            },
        };
        let mut w = BitWriter::new();
        // v0.18 width formula at n=1: ceil(log2(2)) = 1.
        write_node(&mut w, &n, 1).unwrap();
        // Tag::Tr (5) + key_index (1) + has_tree (1) = 7 bits (was 11 v0.17).
        assert_eq!(w.bit_len(), 7);
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(read_node(&mut r, 1).unwrap(), n);
    }

    #[test]
    fn tr_sentinel_n_2_and_v_inheritance_round_trip() {
        // v0.18: tr(<NUMS>, and_v(v:pk(@0), pk(@1))) — inheritance pattern via
        // NUMS internal key. n=2 sentinel (key_index = 2). Exercises and_v +
        // verify wrapper inside the script-path branch. Width formula change
        // boundary: n=2 was width=1 (v0.17 ceil(log2(2))) and is now width=2
        // (v0.18 ceil(log2(3))).
        let n = Node {
            tag: Tag::Tr,
            body: Body::Tr {
                key_index: 2,
                tree: Some(Box::new(Node {
                    tag: Tag::AndV,
                    body: Body::Children(vec![
                        Node {
                            tag: Tag::Verify,
                            body: Body::Children(vec![Node {
                                tag: Tag::PkK,
                                body: Body::KeyArg { index: 0 },
                            }]),
                        },
                        Node {
                            tag: Tag::PkK,
                            body: Body::KeyArg { index: 1 },
                        },
                    ]),
                })),
            },
        };
        let mut w = BitWriter::new();
        // v0.18 width at n=2: ceil(log2(3)) = 2 (was 1 in v0.17).
        write_node(&mut w, &n, 2).unwrap();
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(read_node(&mut r, 2).unwrap(), n);
    }

    #[test]
    fn tr_sentinel_n_3_multi_a_2_of_3_round_trip() {
        // v0.18: tr(<NUMS>, multi_a(2, @0, @1, @2)) — the canonical 2-of-3
        // hardware-wallet multisig encoding (the headline use case). n=3
        // sentinel. Width unchanged from v0.17: ceil(log2(3)) and ceil(log2(4))
        // both equal 2. This is the "no-bit-width-delta but still a wire
        // change" boundary: tag bytes shrink (no extension prefix) but width
        // stays the same.
        let n = Node {
            tag: Tag::Tr,
            body: Body::Tr {
                key_index: 3,
                tree: Some(Box::new(Node {
                    tag: Tag::MultiA,
                    body: Body::MultiKeys {
                        k: 2,
                        indices: vec![0, 1, 2],
                    },
                })),
            },
        };
        let mut w = BitWriter::new();
        write_node(&mut w, &n, 2).unwrap();
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(read_node(&mut r, 2).unwrap(), n);
    }

    #[test]
    #[ignore = "v0.30 Phase A: NUMS-sentinel-based; lifted in Phase F (is_nums flag replaces sentinel)"]
    fn tr_sentinel_n_4_bare_round_trip() {
        // v0.18: tr(<NUMS>) at n=4 — boundary where width goes 2→3 between
        // v0.17 (ceil(log2(4)) = 2) and v0.18 (ceil(log2(5)) = 3). Catches
        // off-by-one errors in the upper-boundary ceil(log2(n+1)) edge.
        let n = Node {
            tag: Tag::Tr,
            body: Body::Tr {
                key_index: 4,
                tree: None,
            },
        };
        let mut w = BitWriter::new();
        // v0.18 width at n=4: ceil(log2(5)) = 3 (was 2 in v0.17).
        write_node(&mut w, &n, 3).unwrap();
        // Tag::Tr (5) + key_index (3) + has_tree (1) = 9 bits.
        assert_eq!(w.bit_len(), 9);
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(read_node(&mut r, 3).unwrap(), n);
    }

    /// v0.19 — multi-branch tap tree wire-format round-trip. Closes audit
    /// Concern B (no codec-level tests for `Tag::TapTree` with branching
    /// existed before v0.19; multi-branch was previously walker-rejected
    /// so there was no real input that exercised this wire shape).
    /// `tr(@0, {pk(@1), pk(@2)})` with key_index_width=2.
    /// Bit-length pin: Tag::Tr (5) + key_index (2) + has_tree (1)
    ///                 + Tag::TapTree (5) + 2×(Tag::PkK (5) + key_index (2)) = 27 bits.
    #[test]
    #[ignore = "v0.30 Phase A: tag-width bit-count pin stale; lifted in Phase H (corpus regen)"]
    fn tap_tree_two_leaf_round_trip() {
        let n = Node {
            tag: Tag::Tr,
            body: Body::Tr {
                key_index: 0,
                tree: Some(Box::new(Node {
                    tag: Tag::TapTree,
                    body: Body::Children(vec![
                        Node {
                            tag: Tag::PkK,
                            body: Body::KeyArg { index: 1 },
                        },
                        Node {
                            tag: Tag::PkK,
                            body: Body::KeyArg { index: 2 },
                        },
                    ]),
                })),
            },
        };
        let mut w = BitWriter::new();
        write_node(&mut w, &n, 2).unwrap();
        assert_eq!(w.bit_len(), 27, "2-leaf TapTree wire layout pin");
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(read_node(&mut r, 2).unwrap(), n);
    }

    /// v0.19 — 4-leaf nested multi-branch tap tree:
    /// `tr(@0, {{pk(@1),pk(@2)}, {pk(@3),pk(@4)}})`. Verifies recursion
    /// through `read_node`/`write_node` on nested Tag::TapTree.
    #[test]
    fn tap_tree_nested_four_leaf_round_trip() {
        let mk_branch = |a: u8, b: u8| Node {
            tag: Tag::TapTree,
            body: Body::Children(vec![
                Node {
                    tag: Tag::PkK,
                    body: Body::KeyArg { index: a },
                },
                Node {
                    tag: Tag::PkK,
                    body: Body::KeyArg { index: b },
                },
            ]),
        };
        let n = Node {
            tag: Tag::Tr,
            body: Body::Tr {
                key_index: 0,
                tree: Some(Box::new(Node {
                    tag: Tag::TapTree,
                    body: Body::Children(vec![mk_branch(1, 2), mk_branch(3, 4)]),
                })),
            },
        };
        let mut w = BitWriter::new();
        // n=4 → ceil(log2(5)) = 3.
        write_node(&mut w, &n, 3).unwrap();
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(read_node(&mut r, 3).unwrap(), n);
    }

    /// v0.19 — 3-leaf unbalanced: `tr(@0, {pk(@1), {pk(@2),pk(@3)}})`.
    /// Asymmetric shape — the right child is a TapTree, the left is a
    /// bare PkK leaf. Verifies the wire format doesn't require balanced
    /// trees.
    #[test]
    fn tap_tree_unbalanced_round_trip() {
        let n = Node {
            tag: Tag::Tr,
            body: Body::Tr {
                key_index: 0,
                tree: Some(Box::new(Node {
                    tag: Tag::TapTree,
                    body: Body::Children(vec![
                        Node {
                            tag: Tag::PkK,
                            body: Body::KeyArg { index: 1 },
                        },
                        Node {
                            tag: Tag::TapTree,
                            body: Body::Children(vec![
                                Node {
                                    tag: Tag::PkK,
                                    body: Body::KeyArg { index: 2 },
                                },
                                Node {
                                    tag: Tag::PkK,
                                    body: Body::KeyArg { index: 3 },
                                },
                            ]),
                        },
                    ]),
                })),
            },
        };
        let mut w = BitWriter::new();
        // n=3 → ceil(log2(4)) = 2.
        write_node(&mut w, &n, 2).unwrap();
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(read_node(&mut r, 2).unwrap(), n);
    }

    /// v0.19 hardening — reject deeply-nested TapTree on the decode side.
    /// Encode-side has no cap (input here is a programmatically-constructed
    /// Node tree, not from the walker), but the decode-side cap fires
    /// when the deepest left-child read attempts at depth MAX_DECODE_DEPTH.
    #[test]
    fn read_node_rejects_excessive_taptree_nesting() {
        let mut left = Node {
            tag: Tag::PkK,
            body: Body::KeyArg { index: 0 },
        };
        // 128 TapTree wrappers: deepest leaf ends up at depth 128 on the
        // left chain; cap fires when reading that leaf.
        for _ in 0..128 {
            left = Node {
                tag: Tag::TapTree,
                body: Body::Children(vec![
                    left,
                    Node {
                        tag: Tag::PkK,
                        body: Body::KeyArg { index: 0 },
                    },
                ]),
            };
        }
        let mut w = BitWriter::new();
        write_node(&mut w, &left, 0).unwrap();
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        let err = read_node(&mut r, 0).unwrap_err();
        assert_eq!(
            err,
            Error::DecodeRecursionDepthExceeded {
                depth: 128,
                max: MAX_DECODE_DEPTH,
            }
        );
    }

    /// v0.19 hardening — cap is tag-agnostic; fires for non-taproot
    /// recursive tags (AndV chain) the same way it fires for TapTree.
    #[test]
    fn read_node_rejects_excessive_andv_chain_nesting() {
        let mut left = Node {
            tag: Tag::PkK,
            body: Body::KeyArg { index: 0 },
        };
        // 128 AndV wrappers on the left, with PkK leaves on the right at
        // each level. Deepest left-leaf at depth 128 triggers the cap.
        for _ in 0..128 {
            left = Node {
                tag: Tag::AndV,
                body: Body::Children(vec![
                    left,
                    Node {
                        tag: Tag::PkK,
                        body: Body::KeyArg { index: 0 },
                    },
                ]),
            };
        }
        let mut w = BitWriter::new();
        write_node(&mut w, &left, 0).unwrap();
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        let err = read_node(&mut r, 0).unwrap_err();
        assert_eq!(
            err,
            Error::DecodeRecursionDepthExceeded {
                depth: 128,
                max: MAX_DECODE_DEPTH,
            }
        );
    }

    /// v0.19 hardening — depth exactly at the limit (deepest leaf at
    /// depth 127, one shy of MAX_DECODE_DEPTH) round-trips successfully.
    #[test]
    fn read_node_accepts_max_depth_minus_one() {
        let mut left = Node {
            tag: Tag::PkK,
            body: Body::KeyArg { index: 0 },
        };
        // 127 TapTree wrappers: deepest leaf at depth 127, just under cap.
        for _ in 0..127 {
            left = Node {
                tag: Tag::TapTree,
                body: Body::Children(vec![
                    left,
                    Node {
                        tag: Tag::PkK,
                        body: Body::KeyArg { index: 0 },
                    },
                ]),
            };
        }
        let mut w = BitWriter::new();
        write_node(&mut w, &left, 0).unwrap();
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        let decoded = read_node(&mut r, 0).unwrap();
        assert_eq!(decoded, left);
    }
}
