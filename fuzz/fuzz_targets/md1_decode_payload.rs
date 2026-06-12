//! Fuzz target: `decode_payload` (raw bit-stream payload decoder).
//!
//! md phase of the constellation stress-fuzz program (Cycle C).
//!
//! Input layout: first 2 bytes = a little-endian u16 `total_bits` CANDIDATE;
//! the remainder is the payload byte slice.
//!
//! CLAMP (load-bearing — R0 [I3-residual]): `total_bits` is clamped to the
//! data budget `remainder.len()*8`. cargo-fuzz builds release-WITH-debug-
//! assertions, and `BitReader::with_bit_limit` carries
//! `debug_assert!(bit_limit <= bytes.len()*8)` (bitstream.rs:114) — an
//! UNCLAMPED prefix `> len*8` aborts vacuously on ~the first exec. There is
//! no `>len*8` "validation" path to exercise via the raw entry point.
//! Clamping still fuzzes every partial-byte trailing-bit count `0..=len*8`
//! (the blind spot P3 leaves by pinning `total_bits = len*8`); genuine
//! short reads return `Err(BitStreamTruncated)` — a clean error, not a
//! finding.
//!
//! Oracle: decode → re-encode fixed-point. On `Ok(d)`, `encode_payload(&d)`
//! computes its OWN canonical total_bits (≠ the fuzz-chosen clamped value),
//! and re-decoding with that must reproduce the same Descriptor value. A
//! re-encode `Err` on a decode-accepted value is a REAL FINDING (R0 [I6]).
#![no_main]

use libfuzzer_sys::fuzz_target;
use md_codec::{decode_payload, encode_payload};

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }
    let candidate = u16::from_le_bytes([data[0], data[1]]) as usize;
    let remainder = &data[2..];
    // CLAMP — see module docs. Never exceeds the data budget.
    let total_bits = candidate.min(remainder.len() * 8);

    if let Ok(d) = decode_payload(remainder, total_bits) {
        // encode_payload recomputes its own canonical bit length.
        let (bytes2, tb2) =
            encode_payload(&d).expect("FINDING: decode-accepted descriptor failed to re-encode");
        let d2 =
            decode_payload(&bytes2, tb2).expect("FINDING: re-encoded payload failed to decode");
        assert_eq!(
            d, d2,
            "FINDING: decode/re-encode/decode is not a fixed point"
        );
    }
});
