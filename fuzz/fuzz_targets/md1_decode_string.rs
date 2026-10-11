//! Fuzz target: `decode_md1_string` (single whole-input string).
//!
//! md phase of the constellation stress-fuzz program (Cycle C).
//!
//! Oracles:
//! 1. Never-panic / clean-error (implicit: any panic/abort = libFuzzer
//!    failure).
//! 2. Decode → re-encode fixed-point: on `Ok(d)`, re-encode UNADMITTED
//!    (`wrap_payload` over `encode_payload_unadmitted`) then
//!    `decode_md1_string` again and assert the Descriptor is equal.
//!    A re-encode `Err` on a decode-accepted value is a REAL FINDING — the
//!    decode/encode-asymmetry class the charter targets — so it panics
//!    in-target rather than being swallowed (R0 finding [I6]). Mint policy
//!    is not consulted: decode skips it by design, so a card mint would
//!    refuse is not a finding (see `md1_decode_payload.rs`). Where
//!    `encode_md1_string` does admit, it must produce the same string.
#![no_main]

use libfuzzer_sys::fuzz_target;
use md_codec::codex32::wrap_payload;
use md_codec::{decode_md1_string, encode_md1_string, encode_payload_unadmitted};

fuzz_target!(|data: &[u8]| {
    // md1 strings are ASCII; U+FFFD collapse from lossy conversion just
    // wastes a sliver of input space (R0 [M7]).
    let s = String::from_utf8_lossy(data);

    if let Ok(d) = decode_md1_string(&s) {
        // Re-encode the decode-accepted descriptor. Err here = finding.
        let reencoded = encode_payload_unadmitted(&d)
            .and_then(|(bytes, bits)| wrap_payload(&bytes, bits))
            .expect("FINDING: decode-accepted descriptor failed to re-encode");
        if let Ok(minted) = encode_md1_string(&d) {
            assert_eq!(
                minted, reencoded,
                "FINDING: admitted and unadmitted encodings differ"
            );
        }
        // Fixed-point: re-decoding the canonical re-encode must reproduce
        // the same Descriptor value.
        let d2 =
            decode_md1_string(&reencoded).expect("FINDING: re-encoded md1 string failed to decode");
        assert_eq!(
            d, d2,
            "FINDING: decode/re-encode/decode is not a fixed point"
        );
    }
});
