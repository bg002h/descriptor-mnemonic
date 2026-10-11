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
//! Oracle: decode → re-encode fixed-point. On `Ok(d)`,
//! `encode_payload_unadmitted(&d)` computes its OWN canonical total_bits
//! (≠ the fuzz-chosen clamped value), and re-decoding with that must
//! reproduce the same Descriptor value. A re-encode `Err` on a
//! decode-accepted value is a REAL FINDING (R0 [I6]).
//!
//! Re-encode is UNADMITTED: decode deliberately skips mint policy (F-217,
//! F-218, SPEC §6 kind-1 rows) so existing cards stay readable, so a decoded
//! card may be one `encode_payload` refuses to mint — not a finding. Seed
//! `corpus/md1_decode_payload/unadmitted_nested_unspendable.payload` is the
//! 2026-10 smoke crash (`UnspendableNotRootTr`) that pinned this. Where mint
//! does admit, its bytes must equal the unadmitted ones.
#![no_main]

use libfuzzer_sys::fuzz_target;
use md_codec::{decode_payload, encode_payload, encode_payload_unadmitted};

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }
    let candidate = u16::from_le_bytes([data[0], data[1]]) as usize;
    let remainder = &data[2..];
    // CLAMP — see module docs. Never exceeds the data budget.
    let total_bits = candidate.min(remainder.len() * 8);

    if let Ok(d) = decode_payload(remainder, total_bits) {
        // Re-encode recomputes its own canonical bit length.
        let (bytes2, tb2) = encode_payload_unadmitted(&d)
            .expect("FINDING: decode-accepted descriptor failed to re-encode");
        if let Ok(minted) = encode_payload(&d) {
            assert_eq!(
                minted,
                (bytes2.clone(), tb2),
                "FINDING: admitted and unadmitted encodings differ"
            );
        }
        let d2 =
            decode_payload(&bytes2, tb2).expect("FINDING: re-encoded payload failed to decode");
        assert_eq!(
            d, d2,
            "FINDING: decode/re-encode/decode is not a fixed point"
        );
    }
});
