//! Fuzz target: `chunk::reassemble` (multi-chunk md1 card set).
//!
//! md phase of the constellation stress-fuzz program (Cycle C).
//!
//! Structured multi-chunk input uses a SENTINEL-BYTE splitter (R0 [M2]):
//! split the fuzz input on `\n` (0x0A, outside the bech32 alphabet) into
//! ≤8 parts (truncate excess). A libFuzzer insert/delete then moves ONE
//! chunk boundary locally instead of re-shearing all of them.
//!
//! Oracles:
//! 1. Never-panic / clean-error (implicit).
//! 2. Decode → re-encode fixed-point: on `Ok(d)`, `split(&d)` then
//!    `reassemble` the parts and assert the Descriptor is equal. A `split`
//!    `Err` on a reassemble-accepted value is a REAL FINDING (R0 [I6]).
#![no_main]

use libfuzzer_sys::fuzz_target;
use md_codec::chunk::{reassemble, split};

/// Maximum chunk count accepted by the splitter; extra parts are dropped.
const MAX_PARTS: usize = 8;

fuzz_target!(|data: &[u8]| {
    // Sentinel split on `\n`; cap at MAX_PARTS by truncation.
    let parts: Vec<std::borrow::Cow<str>> = data
        .split(|&b| b == b'\n')
        .take(MAX_PARTS)
        .map(String::from_utf8_lossy)
        .collect();
    let refs: Vec<&str> = parts.iter().map(|c| c.as_ref()).collect();

    if let Ok(d) = reassemble(&refs) {
        // Re-split the accepted descriptor; Err here = finding.
        let chunks = split(&d).expect("FINDING: reassemble-accepted descriptor failed to split");
        let chunk_refs: Vec<&str> = chunks.iter().map(String::as_str).collect();
        let d2 = reassemble(&chunk_refs).expect("FINDING: re-split chunks failed to reassemble");
        assert_eq!(
            d, d2,
            "FINDING: reassemble/split/reassemble is not a fixed point"
        );
    }
});
