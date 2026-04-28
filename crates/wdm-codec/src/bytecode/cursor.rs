//! Cursor-style byte-stream reader shared across the bytecode codec.
//!
//! `Cursor` tracks the current read position within a borrowed byte slice and
//! provides a small set of typed read primitives — single byte, LEB128 varint,
//! fixed-size array — each returning `Err(Error::InvalidBytecode { offset, kind })`
//! on failure so that all decode errors carry a precise stream offset.
//!
//! The type is `pub(crate)`: it is an internal codec primitive and is not part
//! of the public wdm-codec API surface. It lives here rather than in
//! `decode.rs` because multiple modules need it (path decoding, template
//! decoding, and the path-declaration framing layer in Task 3.5').

use crate::Error;
use crate::error::BytecodeErrorKind;

/// Cursor-style byte stream reader. Tracks current offset for error reporting.
pub(crate) struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    /// Read a single byte. Returns `Err(InvalidBytecode { kind: UnexpectedEnd })` if at EOF.
    pub(crate) fn read_byte(&mut self) -> Result<u8, Error> {
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
    pub(crate) fn read_varint_u64(&mut self) -> Result<u64, Error> {
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
                } else if remaining.len() >= 10 && remaining.iter().take(10).all(|b| b & 0x80 != 0)
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
    pub(crate) fn read_array<const N: usize>(&mut self) -> Result<[u8; N], Error> {
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
    pub(crate) fn require_empty(&self) -> Result<(), Error> {
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
    pub(crate) fn offset(&self) -> usize {
        self.offset
    }

    /// `true` iff the cursor has consumed every byte in the underlying slice.
    pub(crate) fn is_empty(&self) -> bool {
        self.offset >= self.bytes.len()
    }

    /// Read the next byte without advancing the cursor. Returns
    /// `Err(InvalidBytecode { kind: UnexpectedEnd })` at EOF, mirroring
    /// `read_byte`'s contract.
    pub(crate) fn peek_byte(&self) -> Result<u8, Error> {
        if self.offset >= self.bytes.len() {
            return Err(Error::InvalidBytecode {
                offset: self.offset,
                kind: BytecodeErrorKind::UnexpectedEnd,
            });
        }
        Ok(self.bytes[self.offset])
    }
}
