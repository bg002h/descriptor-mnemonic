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

use miniscript::descriptor::{Descriptor, DescriptorPublicKey};

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
                // varint::decode_u64 returns None for both truncation and
                // overflow; classify as VarintOverflow when the remaining
                // bytes are non-empty (would have parsed if buffer continued
                // and value fit), Truncated otherwise.
                let kind = if remaining.is_empty() {
                    BytecodeErrorKind::UnexpectedEnd
                } else if remaining.iter().all(|b| b & 0x80 != 0) {
                    BytecodeErrorKind::Truncated
                } else {
                    BytecodeErrorKind::VarintOverflow
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
    _keys: &[DescriptorPublicKey],
) -> Result<Descriptor<DescriptorPublicKey>, Error> {
    let tag_byte = cur.read_byte()?;
    let tag_offset = cur.offset - 1;
    let tag = Tag::from_byte(tag_byte).ok_or(Error::InvalidBytecode {
        offset: tag_offset,
        kind: BytecodeErrorKind::UnknownTag(tag_byte),
    })?;
    match tag {
        Tag::Wsh => {
            // Task 2.13+: read WshInner from the remaining bytes and wrap in
            // Descriptor::Wsh(Wsh::new(...)). For now, return a stub error so
            // tests can verify the dispatch path reaches here.
            Err(Error::PolicyScopeViolation(
                "wsh() inner decoding not yet implemented (Task 2.13+)".to_string(),
            ))
        }
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
        // Anything else (a non-top-level fragment) at the top level is malformed.
        _ => Err(Error::InvalidBytecode {
            offset: tag_offset,
            kind: BytecodeErrorKind::UnknownTag(tag_byte),
        }),
    }
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
    fn decode_wsh_skeleton_returns_inner_not_implemented_error() {
        // Wsh = 0x05. Top-level dispatch reaches the wsh arm, which returns
        // a stub PolicyScopeViolation pointing to Task 2.13+. This test will
        // be deleted once Task 2.13 fills in the inner walker.
        let err = decode_template(&[Tag::Wsh.as_byte()], &empty_keys()).unwrap_err();
        assert!(
            matches!(err, Error::PolicyScopeViolation(ref msg) if msg.contains("Task 2.13")),
            "expected PolicyScopeViolation referencing Task 2.13, got {err:?}"
        );
    }

    #[test]
    fn decode_rejects_non_top_level_fragment_at_top() {
        // Tag::True is valid but only as a Miniscript fragment, not at top level.
        let err = decode_template(&[Tag::True.as_byte()], &empty_keys()).unwrap_err();
        assert!(
            matches!(err, Error::InvalidBytecode { kind: BytecodeErrorKind::UnknownTag(_), .. }),
            "expected InvalidBytecode UnknownTag for non-top-level tag, got {err:?}"
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
