//! Bytecode template decoder for MD wallet policies.
//!
//! Inverse of [`crate::bytecode::encode::encode_template`]. Reads a canonical
//! MD bytecode stream and reconstructs a `Descriptor<DescriptorPublicKey>`,
//! substituting `Tag::Placeholder` references against a caller-supplied key
//! information vector.
//!
//! v0.1 scope: only `Tag::Wsh` at the top level. The v1+ inline-key tags
//! (0x24..=0x31, the `Reserved*` set in `Tag`) are rejected.
//!
//! v0.2 (Phase D) scope: `Tag::Tr` is also accepted at the top level and
//! decodes to `Descriptor::Tr` with at most a single tap-leaf miniscript
//! attached. `Tag::TapTree` (0x08) is reserved for v1+ multi-leaf encoding;
//! its appearance in v0.2 input is rejected as a `PolicyScopeViolation`.
//!
//! Architecture: cursor-style reader + per-tag dispatch. Each step returns
//! `Result<T, crate::Error>` so decode failures surface a precise offset and
//! `BytecodeErrorKind`. See `design/PHASE_2_DECISIONS.md` D-5.

use bitcoin::hashes::Hash;
use miniscript::descriptor::TapTree;
use miniscript::descriptor::{Descriptor, DescriptorPublicKey, Tr, Wsh};
use miniscript::{Miniscript, Segwitv0, Tap, Terminal, Threshold};

use crate::Error;
use crate::bytecode::Tag;
use crate::bytecode::cursor::Cursor;
use crate::error::BytecodeErrorKind;

/// Decode a canonical MD bytecode stream into a wallet-policy descriptor.
///
/// `keys` is the wallet policy's key information vector; the decoder
/// substitutes each `Tag::Placeholder` + LEB128 index with `keys[index]`.
///
/// Returns:
/// - `Err(Error::InvalidBytecode { offset, kind })` if the stream is
///   malformed (empty, truncated, unknown tag, varint overflow).
/// - `Err(Error::PolicyScopeViolation(...))` if the stream uses a
///   v0.1-out-of-scope construct (e.g. an inline-key tag, taproot tag,
///   or a placeholder index ≥ `keys.len()`).
pub fn decode_template(
    bytes: &[u8],
    keys: &[DescriptorPublicKey],
) -> Result<Descriptor<DescriptorPublicKey>, Error> {
    let mut cur = Cursor::new(bytes);
    let descriptor = decode_descriptor(&mut cur, keys)?;
    cur.require_empty()?;
    Ok(descriptor)
}

/// Decode the top-level `Descriptor`. v0.1 only accepts `Tag::Wsh`.
fn decode_descriptor(
    cur: &mut Cursor<'_>,
    keys: &[DescriptorPublicKey],
) -> Result<Descriptor<DescriptorPublicKey>, Error> {
    let tag_byte = cur.read_byte()?;
    let tag_offset = cur.offset() - 1;
    let tag = Tag::from_byte(tag_byte).ok_or(Error::InvalidBytecode {
        offset: tag_offset,
        kind: BytecodeErrorKind::UnknownTag(tag_byte),
    })?;
    match tag {
        Tag::Wsh => decode_wsh_inner(cur, keys),
        Tag::Tr => decode_tr_inner(cur, keys),
        Tag::Sh | Tag::Pkh | Tag::Wpkh | Tag::Bare => Err(Error::PolicyScopeViolation(format!(
            "v0.1 does not support top-level tag {tag:?}"
        ))),
        // Reserved key tags (descriptor-codec inline-key forms unused in v0.1).
        Tag::ReservedOrigin
        | Tag::ReservedNoOrigin
        | Tag::ReservedUncompressedFullKey
        | Tag::ReservedCompressedFullKey
        | Tag::ReservedXOnly
        | Tag::ReservedXPub
        | Tag::ReservedMultiXPub
        | Tag::ReservedUncompressedSinglePriv
        | Tag::ReservedCompressedSinglePriv
        | Tag::ReservedXPriv
        | Tag::ReservedMultiXPriv
        | Tag::ReservedNoWildcard
        | Tag::ReservedUnhardenedWildcard
        | Tag::ReservedHardenedWildcard => Err(Error::PolicyScopeViolation(format!(
            "v0.1 rejects inline-key tag {tag:?} (deferred to v1+)"
        ))),
        // A known fragment tag (e.g. AndV, PkK, True) appearing at the top
        // level — malformed input from a v0.1 perspective. Use
        // PolicyScopeViolation rather than UnknownTag because the byte was
        // recognised; only its position is wrong.
        _ => Err(Error::PolicyScopeViolation(format!(
            "tag {tag:?} (0x{tag_byte:02x}) is not valid at the top level in v0.1"
        ))),
    }
}

/// Decode a `Wsh<DescriptorPublicKey>` inner: either a SortedMulti or a
/// regular miniscript fragment. Wraps the result in `Descriptor::Wsh`.
fn decode_wsh_inner(
    cur: &mut Cursor<'_>,
    keys: &[DescriptorPublicKey],
) -> Result<Descriptor<DescriptorPublicKey>, Error> {
    let inner_tag_byte = cur.read_byte()?;
    let inner_tag_offset = cur.offset() - 1;
    let inner_tag = Tag::from_byte(inner_tag_byte).ok_or(Error::InvalidBytecode {
        offset: inner_tag_offset,
        kind: BytecodeErrorKind::UnknownTag(inner_tag_byte),
    })?;
    match inner_tag {
        Tag::SortedMulti => {
            // D-7: k and n are single bytes (not LEB128).
            let k = cur.read_byte()? as usize;
            let n = cur.read_byte()? as usize;
            let mut pks: Vec<DescriptorPublicKey> = Vec::with_capacity(n);
            for i in 0..n {
                match decode_placeholder(cur, keys) {
                    Ok(pk) => pks.push(pk),
                    Err(Error::InvalidBytecode {
                        kind: BytecodeErrorKind::UnexpectedEnd | BytecodeErrorKind::Truncated,
                        ..
                    }) => {
                        return Err(Error::InvalidBytecode {
                            offset: cur.offset(),
                            kind: BytecodeErrorKind::MissingChildren {
                                expected: n,
                                got: i,
                            },
                        });
                    }
                    Err(other) => return Err(other),
                }
            }
            // miniscript v13: Wsh::new_sortedmulti takes a Threshold<Pk, MAX>.
            // Construct the threshold first, then wrap in Wsh.
            let thresh = Threshold::new(k, pks).map_err(|e| Error::InvalidBytecode {
                offset: inner_tag_offset,
                kind: BytecodeErrorKind::TypeCheckFailed(e.to_string()),
            })?;
            let wsh = Wsh::new_sortedmulti(thresh).map_err(|e| Error::InvalidBytecode {
                offset: inner_tag_offset,
                kind: BytecodeErrorKind::TypeCheckFailed(e.to_string()),
            })?;
            Ok(Descriptor::Wsh(wsh))
        }
        // Anything else — must be a miniscript inner-fragment tag. Pass
        // the tag we already consumed back to decode_terminal so it can
        // dispatch on it without re-reading.
        _ => {
            let inner_ms = decode_terminal(cur, keys, inner_tag, inner_tag_offset)?;
            // Wrap in Wsh::new — this validates that the miniscript fragment
            // satisfies wsh's typing requirements (B-type, etc.).
            let wsh = Wsh::new(inner_ms).map_err(|e| Error::InvalidBytecode {
                offset: inner_tag_offset,
                kind: BytecodeErrorKind::TypeCheckFailed(e.to_string()),
            })?;
            Ok(Descriptor::Wsh(wsh))
        }
    }
}

/// Decode a `Tag::Tr` body. Reads the internal-key placeholder reference,
/// then optionally a single tap-leaf miniscript fragment.
///
/// Per Phase D D-1 / D-3:
/// - Multi-leaf TapTree is reserved for v1+ — `Tag::TapTree` (0x08) is not
///   a valid first byte of the leaf payload in v0.2.
/// - Anything but a top-level `Tag::Tr` is rejected (no nested taproot
///   inside `wsh()` or another `tr()`); this is enforced by the caller
///   chain (`decode_wsh_inner` does not dispatch through here, and a
///   `decode_terminal` mid-tree match on `Tag::Tr` falls through to the
///   default arm with `PolicyScopeViolation`).
fn decode_tr_inner(
    cur: &mut Cursor<'_>,
    keys: &[DescriptorPublicKey],
) -> Result<Descriptor<DescriptorPublicKey>, Error> {
    let internal_key = decode_placeholder(cur, keys)?;
    let tap_tree = if cur.is_empty() {
        // Key-path-only `tr(K)`.
        None
    } else {
        // Peek at the next tag without consuming so we can reject 0x08
        // (multi-leaf TapTree) with a precise message.
        let next_tag_byte = cur.peek_byte()?;
        if next_tag_byte == Tag::TapTree.as_byte() {
            return Err(Error::PolicyScopeViolation(
                "multi-leaf TapTree (Tag::TapTree=0x08) reserved for v1+; v0.2 supports single-leaf only"
                    .to_string(),
            ));
        }
        let leaf = decode_tap_miniscript(cur, keys)?;
        // D-2: enforce per-leaf subset before constructing the TapTree.
        crate::bytecode::encode::validate_tap_leaf_subset(&leaf)?;
        Some(TapTree::leaf(leaf))
    };
    let tr = Tr::new(internal_key, tap_tree).map_err(|e| Error::InvalidBytecode {
        offset: cur.offset(),
        kind: BytecodeErrorKind::TypeCheckFailed(e.to_string()),
    })?;
    Ok(Descriptor::Tr(tr))
}

/// Decode a `Miniscript<DescriptorPublicKey, Segwitv0>` from the next bytes.
/// Reads the tag byte, then dispatches into `decode_terminal`. Returns the
/// type-checked Miniscript wrapper.
fn decode_miniscript(
    cur: &mut Cursor<'_>,
    keys: &[DescriptorPublicKey],
) -> Result<Miniscript<DescriptorPublicKey, Segwitv0>, Error> {
    let tag_byte = cur.read_byte()?;
    let tag_offset = cur.offset() - 1;
    let tag = Tag::from_byte(tag_byte).ok_or(Error::InvalidBytecode {
        offset: tag_offset,
        kind: BytecodeErrorKind::UnknownTag(tag_byte),
    })?;
    decode_terminal(cur, keys, tag, tag_offset)
}

/// Decode a Terminal fragment given its already-consumed tag. The
/// `tag_offset` is the byte position of `tag` in the original stream
/// (used for error reporting if the reconstructed Miniscript fails type-check).
///
/// Per D-8: this dispatcher does NOT use `#[allow(unreachable_patterns)]`.
/// The catch-all is reachable for tags that are valid at other positions
/// (e.g. Tag::Wsh appearing mid-tree) and emits a `PolicyScopeViolation`.
fn decode_terminal(
    cur: &mut Cursor<'_>,
    keys: &[DescriptorPublicKey],
    tag: Tag,
    tag_offset: usize,
) -> Result<Miniscript<DescriptorPublicKey, Segwitv0>, Error> {
    let term: Terminal<DescriptorPublicKey, Segwitv0> = match tag {
        Tag::True => Terminal::True,
        Tag::False => Terminal::False,
        Tag::PkK => {
            let key = decode_placeholder(cur, keys)?;
            Terminal::PkK(key)
        }
        Tag::PkH => {
            let key = decode_placeholder(cur, keys)?;
            Terminal::PkH(key)
        }
        Tag::Multi => {
            // D-7: k and n are single bytes.
            let k = cur.read_byte()? as usize;
            let n = cur.read_byte()? as usize;
            let mut pks: Vec<DescriptorPublicKey> = Vec::with_capacity(n);
            for i in 0..n {
                match decode_placeholder(cur, keys) {
                    Ok(pk) => pks.push(pk),
                    Err(Error::InvalidBytecode {
                        kind: BytecodeErrorKind::UnexpectedEnd | BytecodeErrorKind::Truncated,
                        ..
                    }) => {
                        return Err(Error::InvalidBytecode {
                            offset: cur.offset(),
                            kind: BytecodeErrorKind::MissingChildren {
                                expected: n,
                                got: i,
                            },
                        });
                    }
                    Err(other) => return Err(other),
                }
            }
            let thresh = Threshold::new(k, pks).map_err(|e| Error::InvalidBytecode {
                offset: tag_offset,
                kind: BytecodeErrorKind::TypeCheckFailed(e.to_string()),
            })?;
            Terminal::Multi(thresh)
        }
        Tag::MultiA => {
            // Taproot multi-A. v0.1 rejects Tr at the top level (Task 2.4),
            // so this arm is unreachable through normal flow today. We
            // implement the MultiA wire format here for completeness (encoder
            // Task 2.6 emits it). Enabling taproot in v0.2 will additionally
            // require: (a) a `Tag::Tr` arm in `decode_descriptor`, (b) a
            // `Tag::TapTree` arm in a Tap-context dispatcher, and (c) a
            // separate `decode_terminal` path that returns
            // `Miniscript<_, Tap>` instead of `Miniscript<_, Segwitv0>`.
            // This MultiA body alone is not sufficient.
            let k = cur.read_byte()? as usize;
            let n = cur.read_byte()? as usize;
            let mut pks: Vec<DescriptorPublicKey> = Vec::with_capacity(n);
            for i in 0..n {
                match decode_placeholder(cur, keys) {
                    Ok(pk) => pks.push(pk),
                    Err(Error::InvalidBytecode {
                        kind: BytecodeErrorKind::UnexpectedEnd | BytecodeErrorKind::Truncated,
                        ..
                    }) => {
                        return Err(Error::InvalidBytecode {
                            offset: cur.offset(),
                            kind: BytecodeErrorKind::MissingChildren {
                                expected: n,
                                got: i,
                            },
                        });
                    }
                    Err(other) => return Err(other),
                }
            }
            let thresh = Threshold::new(k, pks).map_err(|e| Error::InvalidBytecode {
                offset: tag_offset,
                kind: BytecodeErrorKind::TypeCheckFailed(e.to_string()),
            })?;
            Terminal::MultiA(thresh)
        }
        Tag::Thresh => {
            // D-7: k and n are single bytes. Each child is a full recursive
            // Miniscript starting with its own tag byte.
            let k = cur.read_byte()? as usize;
            let n = cur.read_byte()? as usize;
            let mut children: Vec<std::sync::Arc<Miniscript<DescriptorPublicKey, Segwitv0>>> =
                Vec::with_capacity(n);
            for i in 0..n {
                match decode_miniscript(cur, keys) {
                    Ok(child) => children.push(std::sync::Arc::new(child)),
                    Err(Error::InvalidBytecode {
                        kind: BytecodeErrorKind::UnexpectedEnd | BytecodeErrorKind::Truncated,
                        ..
                    }) => {
                        return Err(Error::InvalidBytecode {
                            offset: cur.offset(),
                            kind: BytecodeErrorKind::MissingChildren {
                                expected: n,
                                got: i,
                            },
                        });
                    }
                    Err(other) => return Err(other),
                }
            }
            let thresh = Threshold::new(k, children).map_err(|e| Error::InvalidBytecode {
                offset: tag_offset,
                kind: BytecodeErrorKind::TypeCheckFailed(e.to_string()),
            })?;
            Terminal::Thresh(thresh)
        }
        // Single-child wrappers (Task 2.15). Each reads one recursive child
        // via decode_miniscript and wraps it in Arc::new before constructing
        // the Terminal variant.
        Tag::Alt => {
            let child = decode_miniscript(cur, keys)?;
            Terminal::Alt(std::sync::Arc::new(child))
        }
        Tag::Swap => {
            let child = decode_miniscript(cur, keys)?;
            Terminal::Swap(std::sync::Arc::new(child))
        }
        Tag::Check => {
            let child = decode_miniscript(cur, keys)?;
            Terminal::Check(std::sync::Arc::new(child))
        }
        Tag::DupIf => {
            let child = decode_miniscript(cur, keys)?;
            Terminal::DupIf(std::sync::Arc::new(child))
        }
        Tag::Verify => {
            let child = decode_miniscript(cur, keys)?;
            Terminal::Verify(std::sync::Arc::new(child))
        }
        Tag::NonZero => {
            let child = decode_miniscript(cur, keys)?;
            Terminal::NonZero(std::sync::Arc::new(child))
        }
        Tag::ZeroNotEqual => {
            let child = decode_miniscript(cur, keys)?;
            Terminal::ZeroNotEqual(std::sync::Arc::new(child))
        }
        Tag::RawPkH => {
            // 20-byte pubkey-hash literal embedded directly in the fragment
            // (no key info vector lookup). Distinct tag from Hash160 (0x23)
            // even though both are 20-byte payloads — see encoder Task 2.10.
            let bytes = cur.read_array::<20>()?;
            let hash = bitcoin::hashes::hash160::Hash::from_byte_array(bytes);
            Terminal::RawPkH(hash)
        }
        Tag::After => {
            let v = cur.read_varint_u64()?;
            let v32 = u32::try_from(v).map_err(|_| Error::InvalidBytecode {
                offset: tag_offset,
                kind: BytecodeErrorKind::VarintOverflow,
            })?;
            let lock = miniscript::AbsLockTime::from_consensus(v32).map_err(|e| {
                Error::InvalidBytecode {
                    offset: tag_offset,
                    kind: BytecodeErrorKind::TypeCheckFailed(e.to_string()),
                }
            })?;
            Terminal::After(lock)
        }
        Tag::Older => {
            let v = cur.read_varint_u64()?;
            let v32 = u32::try_from(v).map_err(|_| Error::InvalidBytecode {
                offset: tag_offset,
                kind: BytecodeErrorKind::VarintOverflow,
            })?;
            let lock = miniscript::RelLockTime::from_consensus(v32).map_err(|e| {
                Error::InvalidBytecode {
                    offset: tag_offset,
                    kind: BytecodeErrorKind::TypeCheckFailed(e.to_string()),
                }
            })?;
            Terminal::Older(lock)
        }
        Tag::Sha256 => {
            let bytes = cur.read_array::<32>()?;
            Terminal::Sha256(bitcoin::hashes::sha256::Hash::from_byte_array(bytes))
        }
        Tag::Hash256 => {
            // Uses miniscript::hash256::Hash (forward-display newtype around
            // sha256d::Hash) — NOT sha256d::Hash directly. The encoder used the
            // same type. Internal byte order is the same; the newtype only
            // changes display behavior.
            let bytes = cur.read_array::<32>()?;
            Terminal::Hash256(miniscript::hash256::Hash::from_byte_array(bytes))
        }
        Tag::Ripemd160 => {
            let bytes = cur.read_array::<20>()?;
            Terminal::Ripemd160(bitcoin::hashes::ripemd160::Hash::from_byte_array(bytes))
        }
        Tag::Hash160 => {
            let bytes = cur.read_array::<20>()?;
            Terminal::Hash160(bitcoin::hashes::hash160::Hash::from_byte_array(bytes))
        }
        Tag::AndV => {
            let left = decode_miniscript(cur, keys)?;
            let right = decode_miniscript(cur, keys)?;
            Terminal::AndV(std::sync::Arc::new(left), std::sync::Arc::new(right))
        }
        Tag::AndB => {
            let left = decode_miniscript(cur, keys)?;
            let right = decode_miniscript(cur, keys)?;
            Terminal::AndB(std::sync::Arc::new(left), std::sync::Arc::new(right))
        }
        Tag::AndOr => {
            let a = decode_miniscript(cur, keys)?;
            let b = decode_miniscript(cur, keys)?;
            let c = decode_miniscript(cur, keys)?;
            Terminal::AndOr(
                std::sync::Arc::new(a),
                std::sync::Arc::new(b),
                std::sync::Arc::new(c),
            )
        }
        Tag::OrB => {
            let left = decode_miniscript(cur, keys)?;
            let right = decode_miniscript(cur, keys)?;
            Terminal::OrB(std::sync::Arc::new(left), std::sync::Arc::new(right))
        }
        Tag::OrC => {
            let left = decode_miniscript(cur, keys)?;
            let right = decode_miniscript(cur, keys)?;
            Terminal::OrC(std::sync::Arc::new(left), std::sync::Arc::new(right))
        }
        Tag::OrD => {
            let left = decode_miniscript(cur, keys)?;
            let right = decode_miniscript(cur, keys)?;
            Terminal::OrD(std::sync::Arc::new(left), std::sync::Arc::new(right))
        }
        Tag::OrI => {
            let left = decode_miniscript(cur, keys)?;
            let right = decode_miniscript(cur, keys)?;
            Terminal::OrI(std::sync::Arc::new(left), std::sync::Arc::new(right))
        }
        // After Task 2.18 every Segwitv0 Terminal variant is decoded above.
        // The wildcard remains required to compile because Tag is
        // #[non_exhaustive] and its variant set includes top-level-only and
        // reserved tags that must not appear inside an inner fragment.
        // Per D-8, this guard is reachable for tags valid at other positions
        // (e.g. Tag::Wsh appearing mid-tree) and emits a structural
        // PolicyScopeViolation rather than a deferred-milestone stub.
        _ => {
            return Err(Error::PolicyScopeViolation(format!(
                "Tag {tag:?} (0x{:02x}) is not a valid inner-fragment tag",
                tag.as_byte()
            )));
        }
    };
    Miniscript::from_ast(term).map_err(|e| Error::InvalidBytecode {
        offset: tag_offset,
        kind: BytecodeErrorKind::TypeCheckFailed(e.to_string()),
    })
}

// ---------------------------------------------------------------------------
// Tap-context decoder (Phase D)
// ---------------------------------------------------------------------------
//
// Mirrors `decode_miniscript` / `decode_terminal` but returns a
// `Miniscript<DescriptorPublicKey, Tap>` so the result can be wrapped in
// `TapTree::leaf(...)`. The wire format reuses the shared operator tags;
// the per-leaf subset gate (`validate_tap_leaf_subset`) prevents the
// constructor from accepting tap-illegal terminals.

/// Decode a `Miniscript<DescriptorPublicKey, Tap>` from the next bytes.
fn decode_tap_miniscript(
    cur: &mut Cursor<'_>,
    keys: &[DescriptorPublicKey],
) -> Result<Miniscript<DescriptorPublicKey, Tap>, Error> {
    let tag_byte = cur.read_byte()?;
    let tag_offset = cur.offset() - 1;
    let tag = Tag::from_byte(tag_byte).ok_or(Error::InvalidBytecode {
        offset: tag_offset,
        kind: BytecodeErrorKind::UnknownTag(tag_byte),
    })?;
    decode_tap_terminal(cur, keys, tag, tag_offset)
}

/// Decode a single tap-context Terminal given its already-consumed tag.
///
/// The accepted set is intentionally narrower than the Segwitv0 dispatch:
/// only the operators in the BIP §"Taproot tree" Coldcard subset
/// (`pk_k`, `pk_h`, `multi_a`, `or_d`, `and_v`, `older` + safe wrappers
/// `c:` / `v:`) parse cleanly. Out-of-subset tags surface
/// `Error::TapLeafSubsetViolation` immediately rather than being typed-out
/// at `Miniscript::from_ast`, so the diagnostic names the offending
/// operator.
fn decode_tap_terminal(
    cur: &mut Cursor<'_>,
    keys: &[DescriptorPublicKey],
    tag: Tag,
    tag_offset: usize,
) -> Result<Miniscript<DescriptorPublicKey, Tap>, Error> {
    let term: Terminal<DescriptorPublicKey, Tap> = match tag {
        Tag::PkK => {
            let key = decode_placeholder(cur, keys)?;
            Terminal::PkK(key)
        }
        Tag::PkH => {
            let key = decode_placeholder(cur, keys)?;
            Terminal::PkH(key)
        }
        Tag::MultiA => {
            let k = cur.read_byte()? as usize;
            let n = cur.read_byte()? as usize;
            let mut pks: Vec<DescriptorPublicKey> = Vec::with_capacity(n);
            for i in 0..n {
                match decode_placeholder(cur, keys) {
                    Ok(pk) => pks.push(pk),
                    Err(Error::InvalidBytecode {
                        kind: BytecodeErrorKind::UnexpectedEnd | BytecodeErrorKind::Truncated,
                        ..
                    }) => {
                        return Err(Error::InvalidBytecode {
                            offset: cur.offset(),
                            kind: BytecodeErrorKind::MissingChildren {
                                expected: n,
                                got: i,
                            },
                        });
                    }
                    Err(other) => return Err(other),
                }
            }
            let thresh = Threshold::new(k, pks).map_err(|e| Error::InvalidBytecode {
                offset: tag_offset,
                kind: BytecodeErrorKind::TypeCheckFailed(e.to_string()),
            })?;
            Terminal::MultiA(thresh)
        }
        Tag::Older => {
            let v = cur.read_varint_u64()?;
            let v32 = u32::try_from(v).map_err(|_| Error::InvalidBytecode {
                offset: tag_offset,
                kind: BytecodeErrorKind::VarintOverflow,
            })?;
            let lock = miniscript::RelLockTime::from_consensus(v32).map_err(|e| {
                Error::InvalidBytecode {
                    offset: tag_offset,
                    kind: BytecodeErrorKind::TypeCheckFailed(e.to_string()),
                }
            })?;
            Terminal::Older(lock)
        }
        Tag::AndV => {
            let left = decode_tap_miniscript(cur, keys)?;
            let right = decode_tap_miniscript(cur, keys)?;
            Terminal::AndV(std::sync::Arc::new(left), std::sync::Arc::new(right))
        }
        Tag::OrD => {
            let left = decode_tap_miniscript(cur, keys)?;
            let right = decode_tap_miniscript(cur, keys)?;
            Terminal::OrD(std::sync::Arc::new(left), std::sync::Arc::new(right))
        }
        Tag::Check => {
            let child = decode_tap_miniscript(cur, keys)?;
            Terminal::Check(std::sync::Arc::new(child))
        }
        Tag::Verify => {
            let child = decode_tap_miniscript(cur, keys)?;
            Terminal::Verify(std::sync::Arc::new(child))
        }
        // Reject Tag::TapTree (0x08) explicitly with a v0.2-specific
        // message (Phase D D-1).
        Tag::TapTree => {
            return Err(Error::PolicyScopeViolation(
                "multi-leaf TapTree (Tag::TapTree=0x08) reserved for v1+; v0.2 supports single-leaf only"
                    .to_string(),
            ));
        }
        // Anything else is out-of-subset for v0.2 tap leaves. Use the
        // BIP 388 lowercase operator name (matches encode-side
        // `tap_terminal_name`) so encode and decode rejections of the same
        // operator surface identical user-facing diagnostics.
        other => {
            return Err(Error::TapLeafSubsetViolation {
                operator: tag_to_bip388_name(other).to_string(),
            });
        }
    };
    Miniscript::from_ast(term).map_err(|e| Error::InvalidBytecode {
        offset: tag_offset,
        kind: BytecodeErrorKind::TypeCheckFailed(e.to_string()),
    })
}

/// Map a `Tag` to its BIP 388 lowercase operator name for user-facing
/// error messages. Mirrors `bytecode::encode::tap_terminal_name` so
/// encode-side and decode-side rejections of the same out-of-subset
/// operator surface identical diagnostics. Tags that don't correspond
/// to a BIP 388 operator (framing tags, reserved-for-v1+ inline-key
/// tags) fall back to a `<framing:0xNN>` or `<reserved:0xNN>` label.
fn tag_to_bip388_name(tag: Tag) -> &'static str {
    match tag {
        // Top-level descriptor wrappers
        Tag::Sh => "sh",
        Tag::Wsh => "wsh",
        Tag::Pkh => "pkh",
        Tag::Wpkh => "wpkh",
        Tag::Tr => "tr",
        Tag::TapTree => "TapTree",
        Tag::Bare => "bare",
        // Key + multisig
        Tag::PkK => "pk_k",
        Tag::PkH => "pk_h",
        Tag::RawPkH => "raw_pk_h",
        Tag::Multi => "multi",
        Tag::SortedMulti => "sortedmulti",
        Tag::MultiA => "multi_a",
        // Wrapper terminals
        Tag::Alt => "a:",
        Tag::Swap => "s:",
        Tag::Check => "c:",
        Tag::DupIf => "d:",
        Tag::Verify => "v:",
        Tag::NonZero => "j:",
        Tag::ZeroNotEqual => "n:",
        // Logical operators
        Tag::AndV => "and_v",
        Tag::AndB => "and_b",
        Tag::AndOr => "andor",
        Tag::OrB => "or_b",
        Tag::OrC => "or_c",
        Tag::OrD => "or_d",
        Tag::OrI => "or_i",
        Tag::Thresh => "thresh",
        // Constants
        Tag::True => "1",
        Tag::False => "0",
        // Timelocks
        Tag::Older => "older",
        Tag::After => "after",
        // Hashes
        Tag::Sha256 => "sha256",
        Tag::Hash256 => "hash256",
        Tag::Ripemd160 => "ripemd160",
        Tag::Hash160 => "hash160",
        // Reserved-for-v1+ inline-key forms (descriptor-codec compatibility,
        // tracked in FOLLOWUPS as `p2-inline-key-tags`).
        Tag::ReservedOrigin => "<reserved:0x24>",
        Tag::ReservedNoOrigin => "<reserved:0x25>",
        Tag::ReservedUncompressedFullKey => "<reserved:0x26>",
        Tag::ReservedCompressedFullKey => "<reserved:0x27>",
        Tag::ReservedXOnly => "<reserved:0x28>",
        Tag::ReservedXPub => "<reserved:0x29>",
        Tag::ReservedMultiXPub => "<reserved:0x2A>",
        Tag::ReservedUncompressedSinglePriv => "<reserved:0x2B>",
        Tag::ReservedCompressedSinglePriv => "<reserved:0x2C>",
        Tag::ReservedXPriv => "<reserved:0x2D>",
        Tag::ReservedMultiXPriv => "<reserved:0x2E>",
        Tag::ReservedNoWildcard => "<reserved:0x2F>",
        Tag::ReservedUnhardenedWildcard => "<reserved:0x30>",
        Tag::ReservedHardenedWildcard => "<reserved:0x31>",
        // Framing tags (not operators)
        Tag::Placeholder => "<framing:0x32>",
        Tag::SharedPath => "<framing:0x33>",
        Tag::Fingerprints => "<framing:0x35>",
    }
}

/// Read a placeholder reference from the cursor: `Tag::Placeholder` (0x32)
/// followed by a single-byte index per D-7. Look up the index in `keys`
/// and return the corresponding `DescriptorPublicKey`.
fn decode_placeholder(
    cur: &mut Cursor<'_>,
    keys: &[DescriptorPublicKey],
) -> Result<DescriptorPublicKey, Error> {
    let tag_byte = cur.read_byte()?;
    let tag_offset = cur.offset() - 1;
    let tag = Tag::from_byte(tag_byte).ok_or(Error::InvalidBytecode {
        offset: tag_offset,
        kind: BytecodeErrorKind::UnknownTag(tag_byte),
    })?;
    if tag != Tag::Placeholder {
        return Err(Error::PolicyScopeViolation(format!(
            "expected Tag::Placeholder, got {tag:?} at offset {tag_offset}"
        )));
    }
    let index = cur.read_byte()?; // Single byte per D-7.
    keys.get(usize::from(index)).cloned().ok_or_else(|| {
        Error::PolicyScopeViolation(format!(
            "placeholder index {index} out of range (keys.len()={})",
            keys.len()
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_keys() -> Vec<DescriptorPublicKey> {
        Vec::new()
    }

    #[test]
    fn decode_rejects_empty_input() {
        let err = decode_template(&[], &empty_keys()).unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidBytecode {
                offset: 0,
                kind: BytecodeErrorKind::UnexpectedEnd
            }
        ));
    }

    #[test]
    fn decode_rejects_unknown_top_tag() {
        // 0xFF is not a valid Tag in v0.1.
        let err = decode_template(&[0xFF], &empty_keys()).unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidBytecode {
                offset: 0,
                kind: BytecodeErrorKind::UnknownTag(0xFF)
            }
        ));
    }

    #[test]
    fn decode_rejects_top_level_pkh() {
        // Pkh = 0x02. v0.1 doesn't support top-level pkh.
        let err = decode_template(&[Tag::Pkh.as_byte()], &empty_keys()).unwrap_err();
        assert!(
            matches!(err, Error::PolicyScopeViolation(ref msg) if msg.contains("Pkh")),
            "expected PolicyScopeViolation about Pkh, got {err:?}"
        );
    }

    #[test]
    fn decode_top_level_taproot_truncated_internal_key() {
        // Tr = 0x06 with no body — v0.2 accepts Tag::Tr at the top level
        // but the internal-key placeholder reference is missing, so the
        // cursor errors out with UnexpectedEnd while reading the
        // placeholder tag byte.
        let err = decode_template(&[Tag::Tr.as_byte()], &empty_keys()).unwrap_err();
        assert!(
            matches!(
                err,
                Error::InvalidBytecode {
                    kind: BytecodeErrorKind::UnexpectedEnd,
                    ..
                }
            ),
            "expected UnexpectedEnd from truncated tr() body, got {err:?}"
        );
    }

    #[test]
    fn decode_rejects_reserved_inline_key_tag() {
        // 0x24 = ReservedOrigin (a v1+ inline-key form).
        let err = decode_template(&[0x24], &empty_keys()).unwrap_err();
        assert!(
            matches!(err, Error::PolicyScopeViolation(ref msg) if msg.contains("inline-key")),
            "expected PolicyScopeViolation about inline-key, got {err:?}"
        );
    }

    #[test]
    fn decode_rejects_non_top_level_fragment_at_top() {
        // Tag::True is valid but only as a Miniscript fragment, not at top level.
        // The decoder reports PolicyScopeViolation since the byte is recognized;
        // only its placement is wrong.
        let err = decode_template(&[Tag::True.as_byte()], &empty_keys()).unwrap_err();
        assert!(
            matches!(err, Error::PolicyScopeViolation(ref msg) if msg.contains("top level")),
            "expected PolicyScopeViolation about top-level placement, got {err:?}"
        );
    }

    // --- Wsh inner / Terminal leaf round-trips and rejections (Task 2.13) ---
    //
    // PkK / PkH leaf round-trips were originally deferred to Task 2.15
    // because the encoder only emits PkK/PkH inside a `c:` (Check) wrapper
    // (wsh()'s typing requires a B-type inner, and PkK is K-type). Task 2.15
    // landed the c: wrapper decoder, so wsh(pk(K)) and wsh(pkh(K)) now
    // round-trip end-to-end through the parser. See
    // `decode_wsh_pk_round_trip_via_parser` and
    // `decode_wsh_pkh_round_trip_via_parser` below.

    #[test]
    fn decode_wsh_false() {
        // wsh(0) encoded as [Wsh, False] = [0x05, 0x00].
        let d = decode_template(&[0x05, 0x00], &[]).unwrap();
        // Re-encode it via the encoder and check we got the same bytes.
        use std::collections::HashMap;
        let encoded = crate::bytecode::encode::encode_template(&d, &HashMap::new()).unwrap();
        assert_eq!(encoded, vec![0x05, 0x00]);
    }

    #[test]
    fn decode_wsh_true() {
        let d = decode_template(&[0x05, 0x01], &[]).unwrap();
        use std::collections::HashMap;
        let encoded = crate::bytecode::encode::encode_template(&d, &HashMap::new()).unwrap();
        assert_eq!(encoded, vec![0x05, 0x01]);
    }

    #[test]
    fn decode_rejects_truncated_wsh_inner() {
        // [Wsh] alone, no inner tag → cursor reads end-of-stream when
        // looking for the inner tag.
        let err = decode_template(&[0x05], &[]).unwrap_err();
        assert!(
            matches!(
                err,
                Error::InvalidBytecode {
                    kind: BytecodeErrorKind::UnexpectedEnd,
                    ..
                }
            ),
            "expected InvalidBytecode UnexpectedEnd, got {err:?}"
        );
    }

    #[test]
    fn decode_rejects_unknown_inner_tag() {
        // [Wsh, 0xFF] — 0xFF is not a valid Tag.
        let err = decode_template(&[0x05, 0xFF], &[]).unwrap_err();
        assert!(
            matches!(
                err,
                Error::InvalidBytecode {
                    kind: BytecodeErrorKind::UnknownTag(0xFF),
                    ..
                }
            ),
            "expected InvalidBytecode UnknownTag(0xFF), got {err:?}"
        );
    }

    #[test]
    fn decode_rejects_top_level_only_tag_mid_tree() {
        // Tag::Wsh (0x05) is valid at the top level but cannot appear inside
        // an inner fragment. After Task 2.18 every Segwitv0 terminal tag is
        // implemented; this test pins the catch-all behavior for genuinely
        // misplaced tags.
        let bytes = vec![0x05, 0x05]; // [Wsh outer, Wsh inner — invalid mid-tree]
        let err = decode_template(&bytes, &[]).unwrap_err();
        assert!(
            matches!(err, Error::PolicyScopeViolation(ref msg) if msg.contains("not a valid inner-fragment tag")),
            "expected PolicyScopeViolation about invalid inner-fragment tag, got {err:?}"
        );
    }

    // The decoder-side counterpart to encode.rs's
    // `encode_placeholder_index_above_127_uses_single_byte` is intentionally
    // omitted: D-7 made placeholder indices a single byte (0..=255), so the
    // ≥128 case has no special encoding to round-trip. The encoder-side
    // coverage already pins the wire format.

    // --- Multisig family round-trips and rejections (Task 2.14) -----------

    #[test]
    fn decode_wsh_sortedmulti_2_of_3_round_trip() {
        // Build the bytecode by encoding a known wsh(sortedmulti(2, K0, K1, K2)),
        // then decode it back and re-encode to assert byte equality.
        use std::collections::HashMap;
        use std::str::FromStr;

        let k0 = DescriptorPublicKey::from_str(
            "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5",
        )
        .unwrap();
        let k1 = DescriptorPublicKey::from_str(
            "03a34b99f22c790c4e36b2b3c2c35a36db06226e41c692fc82b8b56ac1c540c5bd",
        )
        .unwrap();
        let k2 = DescriptorPublicKey::from_str(
            "0395bcfdb728e8b1f0eda94f0db26d4ee3eebca73d11611ace1c0e4eed1bdc0e8a",
        )
        .unwrap();

        let mut map = HashMap::new();
        map.insert(k0.clone(), 0u8);
        map.insert(k1.clone(), 1u8);
        map.insert(k2.clone(), 2u8);

        let descriptor = miniscript::descriptor::Descriptor::<DescriptorPublicKey>::from_str(
            &format!("wsh(sortedmulti(2,{k0},{k1},{k2}))"),
        )
        .unwrap();
        let bytes = crate::bytecode::encode::encode_template(&descriptor, &map).unwrap();

        // Decode against keys[0..3] = [k0, k1, k2] (in placeholder-index order).
        let keys_vec = vec![k0.clone(), k1.clone(), k2.clone()];
        let decoded = decode_template(&bytes, &keys_vec).unwrap();
        let reencoded = crate::bytecode::encode::encode_template(&decoded, &map).unwrap();
        assert_eq!(
            reencoded, bytes,
            "round-trip should produce identical bytes"
        );
    }

    #[test]
    fn decode_wsh_multi_2_of_3_round_trip() {
        // wsh(multi(...)) goes through WshInner::Ms -> Terminal::Multi.
        use std::collections::HashMap;
        use std::str::FromStr;

        let k0 = DescriptorPublicKey::from_str(
            "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5",
        )
        .unwrap();
        let k1 = DescriptorPublicKey::from_str(
            "03a34b99f22c790c4e36b2b3c2c35a36db06226e41c692fc82b8b56ac1c540c5bd",
        )
        .unwrap();
        let k2 = DescriptorPublicKey::from_str(
            "0395bcfdb728e8b1f0eda94f0db26d4ee3eebca73d11611ace1c0e4eed1bdc0e8a",
        )
        .unwrap();

        let mut map = HashMap::new();
        map.insert(k0.clone(), 0u8);
        map.insert(k1.clone(), 1u8);
        map.insert(k2.clone(), 2u8);

        let descriptor = miniscript::descriptor::Descriptor::<DescriptorPublicKey>::from_str(
            &format!("wsh(multi(2,{k0},{k1},{k2}))"),
        )
        .unwrap();
        let bytes = crate::bytecode::encode::encode_template(&descriptor, &map).unwrap();

        let keys_vec = vec![k0.clone(), k1.clone(), k2.clone()];
        let decoded = decode_template(&bytes, &keys_vec).unwrap();
        let reencoded = crate::bytecode::encode::encode_template(&decoded, &map).unwrap();
        assert_eq!(reencoded, bytes);
    }

    #[test]
    fn decode_thresh_with_constants_round_trip() {
        // The encoder's `encode_terminal_thresh_2_of_3_with_constants` test
        // exercises exactly this shape (k=2, n=3, [False, True, False]) at
        // the Terminal level. The expected bytes were:
        //   [Thresh, 0x02, 0x03, False, True, False] = [0x18, 0x02, 0x03, 0x00, 0x01, 0x00]
        //
        // Drive the decoder with a manually-constructed byte stream that
        // wraps this in Wsh: [Wsh, Thresh, 0x02, 0x03, False, True, False].
        // Note: this byte stream may FAIL to decode because miniscript's type
        // checker rejects thresh(2, 0, 1, 0) under Wsh's B-type requirement
        // for the inner. If so, this test demonstrates the correct error path.
        let bytes: Vec<u8> = vec![0x05, 0x18, 0x02, 0x03, 0x00, 0x01, 0x00];
        let result = decode_template(&bytes, &[]);
        // Either Ok (if miniscript accepts) or Err(InvalidBytecode {
        // kind: TypeCheckFailed }) (if it rejects). Both are acceptable
        // outcomes — what matters is no panic and the decoder consumed
        // all input bytes.
        match result {
            Ok(d) => {
                use std::collections::HashMap;
                let reencoded =
                    crate::bytecode::encode::encode_template(&d, &HashMap::new()).unwrap();
                assert_eq!(reencoded, bytes);
            }
            // miniscript may reject the reconstruction — acceptable; anything else is a bug.
            Err(e) => assert!(
                matches!(
                    e,
                    Error::InvalidBytecode {
                        kind: BytecodeErrorKind::TypeCheckFailed(_),
                        ..
                    }
                ),
                "unexpected error: {e:?}"
            ),
        }
    }

    #[test]
    fn decode_sortedmulti_rejects_placeholder_index_out_of_range() {
        // wsh(sortedmulti(2, K0, K1, K2)) bytes but supply only 1 key in keys[]
        // — index 1 (and 2) will be out of range.
        use std::collections::HashMap;
        use std::str::FromStr;

        let k0 = DescriptorPublicKey::from_str(
            "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5",
        )
        .unwrap();
        let k1 = DescriptorPublicKey::from_str(
            "03a34b99f22c790c4e36b2b3c2c35a36db06226e41c692fc82b8b56ac1c540c5bd",
        )
        .unwrap();
        let k2 = DescriptorPublicKey::from_str(
            "0395bcfdb728e8b1f0eda94f0db26d4ee3eebca73d11611ace1c0e4eed1bdc0e8a",
        )
        .unwrap();

        let mut map = HashMap::new();
        map.insert(k0.clone(), 0u8);
        map.insert(k1.clone(), 1u8);
        map.insert(k2.clone(), 2u8);

        let descriptor = miniscript::descriptor::Descriptor::<DescriptorPublicKey>::from_str(
            &format!("wsh(sortedmulti(2,{k0},{k1},{k2}))"),
        )
        .unwrap();
        let bytes = crate::bytecode::encode::encode_template(&descriptor, &map).unwrap();

        // Decode with only 1 key — placeholder indices 1, 2 are out of range.
        let err = decode_template(&bytes, std::slice::from_ref(&k0)).unwrap_err();
        assert!(
            matches!(err, Error::PolicyScopeViolation(ref msg) if msg.contains("placeholder index")),
            "expected PolicyScopeViolation about placeholder index, got {err:?}"
        );
    }

    #[test]
    fn decode_multi_rejects_truncated_after_k() {
        // [Wsh, Multi, k=2] — truncated before n.
        let bytes = vec![0x05, 0x19, 0x02];
        let err = decode_template(&bytes, &[]).unwrap_err();
        assert!(
            matches!(
                err,
                Error::InvalidBytecode {
                    kind: BytecodeErrorKind::UnexpectedEnd,
                    ..
                }
            ),
            "expected UnexpectedEnd after truncated k, got {err:?}"
        );
    }

    #[test]
    fn decode_sortedmulti_rejects_truncated_after_k() {
        // [Wsh, SortedMulti, k=2] — truncated before n. Mirror of the Multi
        // test above but routed through decode_wsh_inner instead of
        // decode_terminal.
        let bytes = vec![0x05, 0x09, 0x02];
        let err = decode_template(&bytes, &[]).unwrap_err();
        assert!(
            matches!(
                err,
                Error::InvalidBytecode {
                    kind: BytecodeErrorKind::UnexpectedEnd,
                    ..
                }
            ),
            "expected UnexpectedEnd after truncated SortedMulti k, got {err:?}"
        );
    }

    #[test]
    fn decode_multi_rejects_truncated_mid_keys() {
        // [Wsh, Multi, k=2, n=3, Placeholder, 0, Placeholder, 1] — only 2 of
        // the 3 promised keys are present. The first two placeholder lookups
        // succeed; the third loop iteration runs out of bytes. The explicit
        // arity check converts this into MissingChildren { expected: 3, got: 2 }.
        use std::str::FromStr;
        let k0 = DescriptorPublicKey::from_str(
            "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5",
        )
        .unwrap();
        let k1 = DescriptorPublicKey::from_str(
            "03a34b99f22c790c4e36b2b3c2c35a36db06226e41c692fc82b8b56ac1c540c5bd",
        )
        .unwrap();

        let bytes = vec![0x05, 0x19, 0x02, 0x03, 0x32, 0x00, 0x32, 0x01];
        let err = decode_template(&bytes, &[k0, k1]).unwrap_err();
        assert!(
            matches!(
                err,
                Error::InvalidBytecode {
                    kind: BytecodeErrorKind::MissingChildren {
                        expected: 3,
                        got: 2
                    },
                    ..
                }
            ),
            "expected MissingChildren {{ expected: 3, got: 2 }} mid-multisig, got {err:?}"
        );
    }

    // --- Wrappers + RawPkH round-trips (Task 2.15) -------------------------

    #[test]
    fn decode_wsh_pk_round_trip_via_parser() {
        // wsh(pk(K)) parses to Wsh -> Ms -> Check(PkK(K)).
        // With Tag::Check now decoded, this is the first PkK/PkH path that
        // round-trips through the parser end-to-end.
        use std::collections::HashMap;
        use std::str::FromStr;

        let key = DescriptorPublicKey::from_str(
            "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert(key.clone(), 0u8);

        let descriptor = miniscript::descriptor::Descriptor::<DescriptorPublicKey>::from_str(
            &format!("wsh(pk({key}))"),
        )
        .unwrap();
        let bytes = crate::bytecode::encode::encode_template(&descriptor, &map).unwrap();

        let keys_vec = vec![key.clone()];
        let decoded = decode_template(&bytes, &keys_vec).unwrap();
        let reencoded = crate::bytecode::encode::encode_template(&decoded, &map).unwrap();
        assert_eq!(reencoded, bytes);
    }

    #[test]
    fn decode_wsh_pkh_round_trip_via_parser() {
        // wsh(pkh(K)) parses through Check + PkH path. The c: wrapper now
        // decodes (Task 2.15), so PkH end-to-end works.
        use std::collections::HashMap;
        use std::str::FromStr;

        let key = DescriptorPublicKey::from_str(
            "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert(key.clone(), 0u8);

        let descriptor = miniscript::descriptor::Descriptor::<DescriptorPublicKey>::from_str(
            &format!("wsh(pkh({key}))"),
        )
        .unwrap();
        let bytes = crate::bytecode::encode::encode_template(&descriptor, &map).unwrap();

        let keys_vec = vec![key.clone()];
        let decoded = decode_template(&bytes, &keys_vec).unwrap();
        let reencoded = crate::bytecode::encode::encode_template(&decoded, &map).unwrap();
        assert_eq!(reencoded, bytes);
    }

    #[test]
    fn decode_terminal_alt_swap_directly() {
        // Direct construction with True children. Each wrapper produces
        // [tag, True_tag] = [tag, 0x01]. Wsh wrapping handled by manually
        // building the byte stream — wrappers won't always typecheck under
        // wsh() so the whole-descriptor parser path may reject some.
        //
        // The decoder's job is to parse bytes correctly; whether the
        // resulting AST type-checks under wsh()'s B-type requirement is
        // a separate concern handled by miniscript and surfaces as
        // TypeCheckFailed.

        let alt_bytes = vec![0x05, 0x0A, 0x01]; // [Wsh, Alt, True]
        let result = decode_template(&alt_bytes, &[]);
        match result {
            Ok(d) => {
                use std::collections::HashMap;
                let reencoded =
                    crate::bytecode::encode::encode_template(&d, &HashMap::new()).unwrap();
                assert_eq!(reencoded, alt_bytes);
            }
            // miniscript may reject the reconstruction — acceptable; anything else is a bug.
            Err(e) => assert!(
                matches!(
                    e,
                    Error::InvalidBytecode {
                        kind: BytecodeErrorKind::TypeCheckFailed(_),
                        ..
                    }
                ),
                "unexpected error decoding alt: {e:?}"
            ),
        }

        let swap_bytes = vec![0x05, 0x0B, 0x01]; // [Wsh, Swap, True]
        let result = decode_template(&swap_bytes, &[]);
        match result {
            Ok(d) => {
                use std::collections::HashMap;
                let reencoded =
                    crate::bytecode::encode::encode_template(&d, &HashMap::new()).unwrap();
                assert_eq!(reencoded, swap_bytes);
            }
            // miniscript may reject the reconstruction — acceptable; anything else is a bug.
            Err(e) => assert!(
                matches!(
                    e,
                    Error::InvalidBytecode {
                        kind: BytecodeErrorKind::TypeCheckFailed(_),
                        ..
                    }
                ),
                "unexpected error decoding swap: {e:?}"
            ),
        }
    }

    #[test]
    fn decode_terminal_raw_pk_h() {
        // [Wsh, RawPkH, <20 bytes>] — bypasses parser since wsh(raw_pk_h(...))
        // typically isn't a clean parser fixture. Exercise the decoder directly
        // and verify the bytes round-trip via the encoder.
        let mut bytes = vec![0x05, 0x1D]; // Wsh, RawPkH
        let hash_bytes: [u8; 20] = [
            0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE,
            0xFF, 0x00, 0x01, 0x02, 0x03, 0x04,
        ];
        bytes.extend_from_slice(&hash_bytes);

        let result = decode_template(&bytes, &[]);
        match result {
            Ok(d) => {
                use std::collections::HashMap;
                let reencoded =
                    crate::bytecode::encode::encode_template(&d, &HashMap::new()).unwrap();
                assert_eq!(reencoded, bytes);
            }
            // Wsh::new(...) on a bare RawPkH may reject if RawPkH isn't B-typed. Acceptable —
            // the test's job is to verify the decoder consumed the right number of bytes
            // (no panic / no dangling). Anything other than TypeCheckFailed is a bug.
            Err(e) => assert!(
                matches!(
                    e,
                    Error::InvalidBytecode {
                        kind: BytecodeErrorKind::TypeCheckFailed(_),
                        ..
                    }
                ),
                "unexpected error decoding raw_pk_h: {e:?}"
            ),
        }
    }

    #[test]
    fn decode_raw_pk_h_rejects_truncated() {
        // [Wsh, RawPkH, <19 bytes>] — truncated by 1 byte.
        let mut bytes = vec![0x05, 0x1D];
        bytes.extend_from_slice(&[0xAA; 19]);
        let err = decode_template(&bytes, &[]).unwrap_err();
        assert!(
            matches!(
                err,
                Error::InvalidBytecode {
                    kind: BytecodeErrorKind::Truncated,
                    ..
                }
            ),
            "expected Truncated, got {err:?}"
        );
    }

    #[test]
    fn decode_wrapper_rejects_truncated_child() {
        // [Wsh, Check] — wrapper missing its child fragment.
        let bytes = vec![0x05, 0x0C];
        let err = decode_template(&bytes, &[]).unwrap_err();
        assert!(
            matches!(
                err,
                Error::InvalidBytecode {
                    kind: BytecodeErrorKind::UnexpectedEnd,
                    ..
                }
            ),
            "expected UnexpectedEnd, got {err:?}"
        );
    }

    // --- Cursor-level tests (private API but exercised here for coverage) ---

    #[test]
    fn cursor_read_byte_advances_offset() {
        let mut cur = Cursor::new(&[0xAA, 0xBB, 0xCC]);
        assert_eq!(cur.read_byte().unwrap(), 0xAA);
        assert_eq!(cur.offset(), 1);
        assert_eq!(cur.read_byte().unwrap(), 0xBB);
        assert_eq!(cur.offset(), 2);
    }

    #[test]
    fn cursor_read_byte_returns_unexpected_end_at_eof() {
        let mut cur = Cursor::new(&[]);
        let err = cur.read_byte().unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidBytecode {
                offset: 0,
                kind: BytecodeErrorKind::UnexpectedEnd
            }
        ));
    }

    #[test]
    fn cursor_read_varint_u64_basic() {
        // [0x80, 0x01] = 128 in LEB128, consuming 2 bytes.
        let mut cur = Cursor::new(&[0x80, 0x01, 0xCC]);
        assert_eq!(cur.read_varint_u64().unwrap(), 128);
        assert_eq!(cur.offset(), 2);
        // Next byte after the varint should still be readable.
        assert_eq!(cur.read_byte().unwrap(), 0xCC);
    }

    #[test]
    fn cursor_read_array_basic() {
        let mut cur = Cursor::new(&[1, 2, 3, 4, 5]);
        let arr: [u8; 3] = cur.read_array().unwrap();
        assert_eq!(arr, [1, 2, 3]);
        assert_eq!(cur.offset(), 3);
    }

    #[test]
    fn cursor_read_array_truncated() {
        let mut cur = Cursor::new(&[1, 2]);
        let err = cur.read_array::<3>().unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidBytecode {
                offset: 0,
                kind: BytecodeErrorKind::Truncated
            }
        ));
    }

    #[test]
    fn cursor_require_empty_passes_at_eof() {
        let cur = Cursor::new(&[]);
        cur.require_empty().unwrap();
    }

    #[test]
    fn cursor_require_empty_fails_with_trailing_bytes() {
        let mut cur = Cursor::new(&[0x42]);
        let _ = cur.read_byte();
        // After reading the only byte, cursor is at EOF — require_empty passes.
        cur.require_empty().unwrap();

        // Fresh cursor not at EOF: require_empty must fail with TrailingBytes.
        let cur = Cursor::new(&[0x42]);
        let err = cur.require_empty().unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidBytecode {
                offset: 0,
                kind: BytecodeErrorKind::TrailingBytes
            }
        ));
    }

    // --- After/Older timelock round-trips and rejections (Task 2.16) -------

    #[test]
    fn decode_wsh_after_round_trip_via_parser() {
        // wsh(after(1234)) parses to Wsh -> Ms -> After. Round-trip end-to-end.
        use std::collections::HashMap;
        use std::str::FromStr;

        let descriptor =
            miniscript::descriptor::Descriptor::<DescriptorPublicKey>::from_str("wsh(after(1234))")
                .unwrap();
        let bytes = crate::bytecode::encode::encode_template(&descriptor, &HashMap::new()).unwrap();

        let decoded = decode_template(&bytes, &[]).unwrap();
        let reencoded =
            crate::bytecode::encode::encode_template(&decoded, &HashMap::new()).unwrap();
        assert_eq!(reencoded, bytes);
    }

    #[test]
    fn decode_wsh_older_round_trip_via_parser() {
        // wsh(older(4032)) — 4032 blocks (~28 days) is the conventional
        // segwit recovery delay. Tests the rel-locktime path.
        use std::collections::HashMap;
        use std::str::FromStr;

        let descriptor =
            miniscript::descriptor::Descriptor::<DescriptorPublicKey>::from_str("wsh(older(4032))")
                .unwrap();
        let bytes = crate::bytecode::encode::encode_template(&descriptor, &HashMap::new()).unwrap();

        let decoded = decode_template(&bytes, &[]).unwrap();
        let reencoded =
            crate::bytecode::encode::encode_template(&decoded, &HashMap::new()).unwrap();
        assert_eq!(reencoded, bytes);
    }

    #[test]
    fn decode_after_known_vector() {
        // Pin the wire format independently of the encoder.
        // [Wsh, After, varint(1234)] = [0x05, 0x1E, 0xD2, 0x09]
        // 1234 LEB128:
        //   1234 = 0x4D2 = 0b100_1101_0010
        //   low 7: 0b101_0010 = 0x52, with continuation = 0xD2
        //   high 7: 0b000_1001 = 0x09, last
        let bytes = vec![0x05, 0x1E, 0xD2, 0x09];
        let decoded = decode_template(&bytes, &[]).unwrap();
        use std::collections::HashMap;
        let reencoded =
            crate::bytecode::encode::encode_template(&decoded, &HashMap::new()).unwrap();
        assert_eq!(reencoded, bytes);
    }

    #[test]
    fn decode_older_known_vector() {
        // [Wsh, Older, varint(4032)] = [0x05, 0x1F, 0xC0, 0x1F]
        let bytes = vec![0x05, 0x1F, 0xC0, 0x1F];
        let decoded = decode_template(&bytes, &[]).unwrap();
        use std::collections::HashMap;
        let reencoded =
            crate::bytecode::encode::encode_template(&decoded, &HashMap::new()).unwrap();
        assert_eq!(reencoded, bytes);
    }

    #[test]
    fn decode_after_rejects_truncated_varint() {
        // [Wsh, After, 0x80] — continuation bit set, no terminator. The
        // varint reader's heuristic should call this Truncated (fewer than
        // 10 continuation bytes).
        let bytes = vec![0x05, 0x1E, 0x80];
        let err = decode_template(&bytes, &[]).unwrap_err();
        assert!(
            matches!(
                err,
                Error::InvalidBytecode {
                    kind: BytecodeErrorKind::Truncated,
                    ..
                }
            ),
            "expected Truncated, got {err:?}"
        );
    }

    #[test]
    fn decode_after_rejects_overflow_above_u32() {
        // varint encoding a value > u32::MAX.
        // u32::MAX = 0xFFFFFFFF; encode (0xFFFFFFFF + 1) = 0x100000000 = 2^32.
        // LEB128 of 2^32: 5 bytes [0x80, 0x80, 0x80, 0x80, 0x10].
        let mut bytes = vec![0x05, 0x1E];
        bytes.extend_from_slice(&[0x80, 0x80, 0x80, 0x80, 0x10]);
        let err = decode_template(&bytes, &[]).unwrap_err();
        assert!(
            matches!(
                err,
                Error::InvalidBytecode {
                    kind: BytecodeErrorKind::VarintOverflow,
                    ..
                }
            ),
            "expected VarintOverflow, got {err:?}"
        );
    }

    #[test]
    fn decode_after_rejects_zero_value() {
        // miniscript::AbsLockTime::from_consensus(0) returns Err — miniscript
        // (ab)uses locktime 0 as a boolean false in script fragments and
        // forbids it as an explicit value. The decoder maps this to
        // BytecodeErrorKind::TypeCheckFailed.
        // Wire: [Wsh, After, 0x00].
        let bytes = vec![0x05, 0x1E, 0x00];
        let err = decode_template(&bytes, &[]).unwrap_err();
        assert!(
            matches!(
                err,
                Error::InvalidBytecode {
                    kind: BytecodeErrorKind::TypeCheckFailed(_),
                    ..
                }
            ),
            "expected TypeCheckFailed for after(0), got {err:?}"
        );
    }

    #[test]
    fn decode_older_rejects_zero_value() {
        // RelLockTime::from_consensus(0) returns Err for the same miniscript
        // reason as AbsLockTime: locktime 0 is forbidden.
        // Wire: [Wsh, Older, 0x00].
        let bytes = vec![0x05, 0x1F, 0x00];
        let err = decode_template(&bytes, &[]).unwrap_err();
        assert!(
            matches!(
                err,
                Error::InvalidBytecode {
                    kind: BytecodeErrorKind::TypeCheckFailed(_),
                    ..
                }
            ),
            "expected TypeCheckFailed for older(0), got {err:?}"
        );
    }

    // --- Hash-preimage round-trips and rejections (Task 2.17) --------------

    #[test]
    fn decode_wsh_sha256_round_trip_via_parser() {
        // wsh(sha256(<32-byte hex>)) — full pipeline.
        use std::collections::HashMap;
        use std::str::FromStr;

        let hex = "0000000000000000000000000000000000000000000000000000000000000001";
        let descriptor = miniscript::descriptor::Descriptor::<DescriptorPublicKey>::from_str(
            &format!("wsh(sha256({hex}))"),
        )
        .unwrap();
        let bytes = crate::bytecode::encode::encode_template(&descriptor, &HashMap::new()).unwrap();

        let decoded = decode_template(&bytes, &[]).unwrap();
        let reencoded =
            crate::bytecode::encode::encode_template(&decoded, &HashMap::new()).unwrap();
        assert_eq!(reencoded, bytes);
    }

    #[test]
    fn decode_wsh_hash256_round_trip_via_parser() {
        use std::collections::HashMap;
        use std::str::FromStr;

        let hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let descriptor = miniscript::descriptor::Descriptor::<DescriptorPublicKey>::from_str(
            &format!("wsh(hash256({hex}))"),
        )
        .unwrap();
        let bytes = crate::bytecode::encode::encode_template(&descriptor, &HashMap::new()).unwrap();

        let decoded = decode_template(&bytes, &[]).unwrap();
        let reencoded =
            crate::bytecode::encode::encode_template(&decoded, &HashMap::new()).unwrap();
        assert_eq!(reencoded, bytes);
    }

    #[test]
    fn decode_wsh_ripemd160_round_trip_via_parser() {
        use std::collections::HashMap;
        use std::str::FromStr;

        // 20-byte hex literal.
        let hex = "1122334455667788990011223344556677889900";
        let descriptor = miniscript::descriptor::Descriptor::<DescriptorPublicKey>::from_str(
            &format!("wsh(ripemd160({hex}))"),
        )
        .unwrap();
        let bytes = crate::bytecode::encode::encode_template(&descriptor, &HashMap::new()).unwrap();

        let decoded = decode_template(&bytes, &[]).unwrap();
        let reencoded =
            crate::bytecode::encode::encode_template(&decoded, &HashMap::new()).unwrap();
        assert_eq!(reencoded, bytes);
    }

    #[test]
    fn decode_wsh_hash160_round_trip_via_parser() {
        use std::collections::HashMap;
        use std::str::FromStr;

        let hex = "aabbccddeeff00112233445566778899aabbccdd";
        let descriptor = miniscript::descriptor::Descriptor::<DescriptorPublicKey>::from_str(
            &format!("wsh(hash160({hex}))"),
        )
        .unwrap();
        let bytes = crate::bytecode::encode::encode_template(&descriptor, &HashMap::new()).unwrap();

        let decoded = decode_template(&bytes, &[]).unwrap();
        let reencoded =
            crate::bytecode::encode::encode_template(&decoded, &HashMap::new()).unwrap();
        assert_eq!(reencoded, bytes);
    }

    #[test]
    fn decode_sha256_known_vector_with_asymmetric_pattern() {
        // Hand-crafted byte stream to pin the wire format. Asymmetric pattern
        // would expose a byte-order reversal bug if it crept in (parallel to
        // the encoder side test for hash256).
        let hash_bytes: [u8; 32] = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b,
            0x1c, 0x1d, 0x1e, 0x1f,
        ];
        let mut bytes = vec![0x05, 0x20]; // [Wsh, Sha256]
        bytes.extend_from_slice(&hash_bytes);

        let decoded = decode_template(&bytes, &[]).unwrap();
        use std::collections::HashMap;
        let reencoded =
            crate::bytecode::encode::encode_template(&decoded, &HashMap::new()).unwrap();
        assert_eq!(reencoded, bytes);
    }

    #[test]
    fn decode_sha256_rejects_truncated() {
        // [Wsh, Sha256, <31 bytes>] — one byte short.
        let mut bytes = vec![0x05, 0x20];
        bytes.extend_from_slice(&[0xAA; 31]);
        let err = decode_template(&bytes, &[]).unwrap_err();
        assert!(
            matches!(
                err,
                Error::InvalidBytecode {
                    kind: BytecodeErrorKind::Truncated,
                    ..
                }
            ),
            "expected Truncated, got {err:?}"
        );
    }

    #[test]
    fn decode_ripemd160_rejects_truncated() {
        // [Wsh, Ripemd160, <19 bytes>] — one byte short of the 20-byte hash.
        let mut bytes = vec![0x05, 0x22];
        bytes.extend_from_slice(&[0xBB; 19]);
        let err = decode_template(&bytes, &[]).unwrap_err();
        assert!(
            matches!(
                err,
                Error::InvalidBytecode {
                    kind: BytecodeErrorKind::Truncated,
                    ..
                }
            ),
            "expected Truncated for ripemd160, got {err:?}"
        );
    }

    // --- Logical-operator round-trips and rejections (Task 2.18) -----------

    #[test]
    fn decode_wsh_or_d_with_constants_round_trip() {
        // wsh(or_d(0,1)) — or_d takes B-dissat-unit + B; both 0 and 1
        // satisfy the typing rules. Parser-driven round-trip.
        use std::collections::HashMap;
        use std::str::FromStr;

        let descriptor =
            miniscript::descriptor::Descriptor::<DescriptorPublicKey>::from_str("wsh(or_d(0,1))")
                .unwrap();
        let bytes = crate::bytecode::encode::encode_template(&descriptor, &HashMap::new()).unwrap();

        let decoded = decode_template(&bytes, &[]).unwrap();
        let reencoded =
            crate::bytecode::encode::encode_template(&decoded, &HashMap::new()).unwrap();
        assert_eq!(reencoded, bytes);
    }

    #[test]
    fn decode_wsh_or_i_with_constants_round_trip() {
        // wsh(or_i(0,1)) — or_i parses with two arbitrary B-typed children.
        use std::collections::HashMap;
        use std::str::FromStr;

        let descriptor =
            miniscript::descriptor::Descriptor::<DescriptorPublicKey>::from_str("wsh(or_i(0,1))")
                .unwrap();
        let bytes = crate::bytecode::encode::encode_template(&descriptor, &HashMap::new()).unwrap();

        let decoded = decode_template(&bytes, &[]).unwrap();
        let reencoded =
            crate::bytecode::encode::encode_template(&decoded, &HashMap::new()).unwrap();
        assert_eq!(reencoded, bytes);
    }

    #[test]
    fn decode_wsh_andor_with_constants_round_trip() {
        // wsh(andor(0, 1, 0)) — andor takes B-dissat + B + B.
        use std::collections::HashMap;
        use std::str::FromStr;

        let descriptor = miniscript::descriptor::Descriptor::<DescriptorPublicKey>::from_str(
            "wsh(andor(0,1,0))",
        )
        .unwrap();
        let bytes = crate::bytecode::encode::encode_template(&descriptor, &HashMap::new()).unwrap();

        let decoded = decode_template(&bytes, &[]).unwrap();
        let reencoded =
            crate::bytecode::encode::encode_template(&decoded, &HashMap::new()).unwrap();
        assert_eq!(reencoded, bytes);
    }

    #[test]
    fn decode_or_d_known_vector() {
        // [Wsh, OrD, False, True] = [0x05, 0x16, 0x00, 0x01]
        let bytes = vec![0x05, 0x16, 0x00, 0x01];
        let decoded = decode_template(&bytes, &[]).unwrap();
        use std::collections::HashMap;
        let reencoded =
            crate::bytecode::encode::encode_template(&decoded, &HashMap::new()).unwrap();
        assert_eq!(reencoded, bytes);
    }

    #[test]
    fn decode_andor_known_vector() {
        // [Wsh, AndOr, False, True, False] = [0x05, 0x13, 0x00, 0x01, 0x00]
        let bytes = vec![0x05, 0x13, 0x00, 0x01, 0x00];
        let decoded = decode_template(&bytes, &[]).unwrap();
        use std::collections::HashMap;
        let reencoded =
            crate::bytecode::encode::encode_template(&decoded, &HashMap::new()).unwrap();
        assert_eq!(reencoded, bytes);
    }

    #[test]
    fn decode_logical_op_rejects_truncated_after_first_child() {
        // [Wsh, AndV, True] — only one child present; AndV needs two.
        let bytes = vec![0x05, 0x11, 0x01];
        let err = decode_template(&bytes, &[]).unwrap_err();
        // After reading the True child, the decoder tries to read the second
        // child and the cursor is at EOF. The next read_byte call returns
        // UnexpectedEnd.
        assert!(
            matches!(
                err,
                Error::InvalidBytecode {
                    kind: BytecodeErrorKind::UnexpectedEnd,
                    ..
                }
            ),
            "expected UnexpectedEnd, got {err:?}"
        );
    }

    #[test]
    fn decode_andor_rejects_truncated_after_two_children() {
        // [Wsh, AndOr, False, True] — AndOr expects 3 children, only 2 present.
        let bytes = vec![0x05, 0x13, 0x00, 0x01];
        let err = decode_template(&bytes, &[]).unwrap_err();
        assert!(
            matches!(
                err,
                Error::InvalidBytecode {
                    kind: BytecodeErrorKind::UnexpectedEnd,
                    ..
                }
            ),
            "expected UnexpectedEnd, got {err:?}"
        );
    }
}
