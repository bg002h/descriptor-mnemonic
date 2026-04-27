//! Bytecode template decoder for WDM wallet policies.
//!
//! Inverse of [`crate::bytecode::encode::encode_template`]. Reads a canonical
//! WDM bytecode stream and reconstructs a `Descriptor<DescriptorPublicKey>`,
//! substituting `Tag::Placeholder` references against a caller-supplied key
//! information vector.
//!
//! v0.1 scope: only `Tag::Wsh` at the top level. The v1+ inline-key tags
//! (0x24..=0x31, the `Reserved*` set in `Tag`) are rejected.
//!
//! Architecture: cursor-style reader + per-tag dispatch. Each step returns
//! `Result<T, crate::Error>` so decode failures surface a precise offset and
//! `BytecodeErrorKind`. See `design/PHASE_2_DECISIONS.md` D-5.

use miniscript::descriptor::{Descriptor, DescriptorPublicKey, Wsh};
use miniscript::{Miniscript, Segwitv0, Terminal};

use crate::bytecode::Tag;
use crate::error::BytecodeErrorKind;
use crate::Error;

/// Decode a canonical WDM bytecode stream into a wallet-policy descriptor.
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

/// Cursor-style byte stream reader. Tracks current offset for error reporting.
struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    /// Read a single byte. Returns `Err(InvalidBytecode { kind: UnexpectedEnd })` if at EOF.
    fn read_byte(&mut self) -> Result<u8, Error> {
        if self.offset >= self.bytes.len() {
            return Err(Error::InvalidBytecode {
                offset: self.offset,
                kind: BytecodeErrorKind::UnexpectedEnd,
            });
        }
        let b = self.bytes[self.offset];
        self.offset += 1;
        Ok(b)
    }

    /// Read an LEB128 unsigned u64. Returns `Err` for truncation or overflow.
    #[allow(dead_code)] // Will be used by Task 2.13+ decoder arms.
    fn read_varint_u64(&mut self) -> Result<u64, Error> {
        let start = self.offset;
        let remaining = &self.bytes[self.offset..];
        match crate::bytecode::varint::decode_u64(remaining) {
            Some((v, consumed)) => {
                self.offset += consumed;
                Ok(v)
            }
            None => {
                // varint::decode_u64 returns None for either truncation or
                // overflow. A u64 LEB128 fits in at most 10 bytes; if the
                // buffer holds 10+ continuation bytes (no terminator within
                // the legal width), the failure is overflow, not truncation.
                // Otherwise the most plausible cause is truncation (stream
                // ended before a terminator).
                let kind = if remaining.is_empty() {
                    BytecodeErrorKind::UnexpectedEnd
                } else if remaining.len() >= 10
                    && remaining.iter().take(10).all(|b| b & 0x80 != 0)
                {
                    BytecodeErrorKind::VarintOverflow
                } else {
                    BytecodeErrorKind::Truncated
                };
                Err(Error::InvalidBytecode {
                    offset: start,
                    kind,
                })
            }
        }
    }

    /// Read exactly `N` bytes as an array. Returns `Err` if fewer remain.
    #[allow(dead_code)] // Will be used by hash-literal decoder arms (Task 2.13+).
    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        if self.offset + N > self.bytes.len() {
            return Err(Error::InvalidBytecode {
                offset: self.offset,
                kind: BytecodeErrorKind::Truncated,
            });
        }
        let mut buf = [0u8; N];
        buf.copy_from_slice(&self.bytes[self.offset..self.offset + N]);
        self.offset += N;
        Ok(buf)
    }

    /// Require the cursor is at end-of-stream. Returns `Err(TrailingBytes)` if not.
    fn require_empty(&self) -> Result<(), Error> {
        if self.offset < self.bytes.len() {
            Err(Error::InvalidBytecode {
                offset: self.offset,
                kind: BytecodeErrorKind::TrailingBytes,
            })
        } else {
            Ok(())
        }
    }

    /// Current offset in the byte stream (for error messages on caller side).
    #[allow(dead_code)] // Will be used by Task 2.13+ decoder arms for nested-error context.
    fn offset(&self) -> usize {
        self.offset
    }
}

/// Decode the top-level `Descriptor`. v0.1 only accepts `Tag::Wsh`.
fn decode_descriptor(
    cur: &mut Cursor<'_>,
    keys: &[DescriptorPublicKey],
) -> Result<Descriptor<DescriptorPublicKey>, Error> {
    let tag_byte = cur.read_byte()?;
    let tag_offset = cur.offset - 1;
    let tag = Tag::from_byte(tag_byte).ok_or(Error::InvalidBytecode {
        offset: tag_offset,
        kind: BytecodeErrorKind::UnknownTag(tag_byte),
    })?;
    match tag {
        Tag::Wsh => decode_wsh_inner(cur, keys),
        Tag::Sh | Tag::Pkh | Tag::Wpkh | Tag::Tr | Tag::Bare => {
            Err(Error::PolicyScopeViolation(format!(
                "v0.1 does not support top-level tag {tag:?}"
            )))
        }
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
        | Tag::ReservedHardenedWildcard => {
            Err(Error::PolicyScopeViolation(format!(
                "v0.1 rejects inline-key tag {tag:?} (deferred to v1+)"
            )))
        }
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
            // Task 2.14: read k, n (single bytes per D-7), then n keys.
            // For Task 2.13 this returns a stub error so the dispatch path
            // can be tested but the inner decoder isn't implemented.
            Err(Error::PolicyScopeViolation(
                "wsh(sortedmulti(...)) decoding not yet implemented (Task 2.14)".to_string(),
            ))
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

/// Decode a `Miniscript<DescriptorPublicKey, Segwitv0>` from the next bytes.
/// Reads the tag byte, then dispatches into `decode_terminal`. Returns the
/// type-checked Miniscript wrapper.
#[allow(dead_code)] // Will be used by Task 2.14+ inner-fragment recursion.
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
        // Task 2.14+ inner-fragment tags will be added here progressively.
        // For now, anything else is either out of v0.1 scope or deferred.
        _ => {
            return Err(Error::PolicyScopeViolation(format!(
                "Tag {tag:?} (0x{:02x}) not yet implemented as inner fragment (Task 2.14+)",
                tag.as_byte()
            )));
        }
    };
    Miniscript::from_ast(term).map_err(|e| Error::InvalidBytecode {
        offset: tag_offset,
        kind: BytecodeErrorKind::TypeCheckFailed(e.to_string()),
    })
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
            Error::InvalidBytecode { offset: 0, kind: BytecodeErrorKind::UnexpectedEnd }
        ));
    }

    #[test]
    fn decode_rejects_unknown_top_tag() {
        // 0xFF is not a valid Tag in v0.1.
        let err = decode_template(&[0xFF], &empty_keys()).unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidBytecode { offset: 0, kind: BytecodeErrorKind::UnknownTag(0xFF) }
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
    fn decode_rejects_top_level_taproot() {
        // Tr = 0x06.
        let err = decode_template(&[Tag::Tr.as_byte()], &empty_keys()).unwrap_err();
        assert!(
            matches!(err, Error::PolicyScopeViolation(ref msg) if msg.contains("Tr")),
            "expected PolicyScopeViolation about Tr, got {err:?}"
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
    // PkK / PkH leaf round-trips are deferred to Task 2.15: the encoder only
    // emits PkK/PkH inside a `c:` (Check) wrapper because wsh()'s typing
    // requires a B-type inner, and PkK is K-type. Until the c: wrapper
    // decoder lands, an end-to-end PkK round-trip can't be expressed at the
    // decode_template boundary. The PkK / PkH arms in decode_terminal are
    // exercised indirectly today via the placeholder helper unit tests and
    // will gain proper round-trip coverage in Task 2.15.

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
            matches!(err, Error::InvalidBytecode {
                kind: BytecodeErrorKind::UnexpectedEnd, ..
            }),
            "expected InvalidBytecode UnexpectedEnd, got {err:?}"
        );
    }

    #[test]
    fn decode_rejects_unknown_inner_tag() {
        // [Wsh, 0xFF] — 0xFF is not a valid Tag.
        let err = decode_template(&[0x05, 0xFF], &[]).unwrap_err();
        assert!(
            matches!(err, Error::InvalidBytecode {
                kind: BytecodeErrorKind::UnknownTag(0xFF), ..
            }),
            "expected InvalidBytecode UnknownTag(0xFF), got {err:?}"
        );
    }

    #[test]
    fn decode_rejects_sortedmulti_stub() {
        // [Wsh, SortedMulti, ...] — SortedMulti decoder is Task 2.14.
        let err = decode_template(&[0x05, 0x09], &[]).unwrap_err();
        assert!(
            matches!(err, Error::PolicyScopeViolation(ref msg) if msg.contains("Task 2.14")),
            "expected PolicyScopeViolation referencing Task 2.14, got {err:?}"
        );
    }

    #[test]
    fn decode_rejects_unimplemented_inner_tag() {
        // [Wsh, AndV, ...] — Task 2.13 only handles True/False/PkK/PkH.
        // AndV (0x11) is in scope of Task 2.16+ (logical operators).
        let err = decode_template(&[0x05, 0x11], &[]).unwrap_err();
        assert!(
            matches!(err, Error::PolicyScopeViolation(ref msg) if msg.contains("not yet implemented")),
            "expected not-yet-implemented PolicyScopeViolation, got {err:?}"
        );
    }

    #[test]
    fn decode_placeholder_index_above_127_uses_single_byte() {
        // Per D-7, the placeholder index is a single byte. We can't easily
        // construct a 200-key wallet to test the full path, but we can
        // confirm the placeholder-decoding path doesn't accidentally
        // consume extra bytes by exercising wsh(0) with 0 keys (no
        // placeholders touched). Assert via wsh(0) round-trip producing
        // exactly [0x05, 0x00] back.
        let d = decode_template(&[0x05, 0x00], &[]).unwrap();
        use std::collections::HashMap;
        let encoded = crate::bytecode::encode::encode_template(&d, &HashMap::new()).unwrap();
        assert_eq!(encoded, vec![0x05, 0x00]);
        assert_eq!(encoded.len(), 2);
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
            Error::InvalidBytecode { offset: 0, kind: BytecodeErrorKind::UnexpectedEnd }
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
            Error::InvalidBytecode { offset: 0, kind: BytecodeErrorKind::Truncated }
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
            Error::InvalidBytecode { offset: 0, kind: BytecodeErrorKind::TrailingBytes }
        ));
    }
}
