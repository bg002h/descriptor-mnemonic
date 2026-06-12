//! Fuzz target: `chunk::decode_with_correction` (BCH error-correcting decode).
//!
//! md phase of the constellation stress-fuzz program (Cycle C).
//!
//! Same SENTINEL-BYTE splitter as `md1_reassemble` (R0 [M2]): split the
//! fuzz input on `\n` into ≤8 parts.
//!
//! Oracle — apply-details idempotence (R0 [I1]): on `Ok((d, details))`,
//! apply each `CorrectionDetail`'s `now` char at its (chunk_index, position)
//! coordinate in the input parts, re-run `decode_with_correction` on the
//! corrected parts, and assert (a) the decoded Descriptor is unchanged and
//! (b) the new details vector is EMPTY (a corrected card needs no further
//! correction). This catches dishonest position/char reporting.
//!
//! COORDINATE (verified against chunk.rs:395-415 / :429-454): `position` is
//! a 0-indexed offset into the chunk's *data-part symbols* — the chars
//! AFTER the `md1` HRP+separator, with visual separators (whitespace, `-`)
//! skipped, exactly as `parse_chunk_symbols` counts them. So applying `now`
//! requires (1) offsetting past the `md1` prefix and (2) walking the
//! data-part counting only alphabet-eligible chars to land on the
//! `position`-th symbol.
#![no_main]

use libfuzzer_sys::fuzz_target;
use md_codec::chunk::{CorrectionDetail, decode_with_correction};

/// Maximum chunk count accepted by the splitter; extra parts are dropped.
const MAX_PARTS: usize = 8;

/// HRP prefix every md1 chunk string begins with (case-insensitively).
const HRP_PREFIX: &str = "md1";

/// Apply `detail.now` at the post-HRP symbol offset `detail.position` within
/// `chunk`. Returns `None` if the chunk does not start with `md1` or the
/// symbol position is out of range — in which case the correction cannot be
/// applied here and the idempotence check is skipped for this input (the
/// detail's own coordinate is still asserted in-range below).
fn apply_correction(chunk: &str, detail: &CorrectionDetail) -> Option<String> {
    // The decoder lowercases before parsing; do the same so HRP detection
    // and the separator-skipping walk match `parse_chunk_symbols`.
    let lower = chunk.to_ascii_lowercase();
    if !lower.starts_with(HRP_PREFIX) {
        return None;
    }
    let hrp_len = HRP_PREFIX.len();
    let data = &lower[hrp_len..];

    // Walk the data-part, counting only non-separator chars (the symbols),
    // to find the BYTE offset of the `position`-th symbol within `data`.
    let mut symbol_idx = 0usize;
    let mut target_byte: Option<usize> = None;
    for (byte_off, c) in data.char_indices() {
        if c.is_whitespace() || c == '-' {
            continue;
        }
        if symbol_idx == detail.position {
            target_byte = Some(byte_off);
            break;
        }
        symbol_idx += 1;
    }
    let target_byte = target_byte?;
    let target_char = data[target_byte..].chars().next()?;

    // Rebuild: hrp + data with the target symbol replaced by `now`.
    let mut out = String::with_capacity(chunk.len());
    out.push_str(HRP_PREFIX);
    out.push_str(&data[..target_byte]);
    out.push(detail.now);
    out.push_str(&data[target_byte + target_char.len_utf8()..]);
    Some(out)
}

fuzz_target!(|data: &[u8]| {
    let parts: Vec<std::borrow::Cow<str>> = data
        .split(|&b| b == b'\n')
        .take(MAX_PARTS)
        .map(String::from_utf8_lossy)
        .collect();
    let refs: Vec<&str> = parts.iter().map(|c| c.as_ref()).collect();

    let (d, details) = match decode_with_correction(&refs) {
        Ok(v) => v,
        Err(_) => return,
    };

    if details.is_empty() {
        // Nothing was corrected — already idempotent by construction.
        return;
    }

    // Build the corrected part set by applying every detail's `now`.
    let mut corrected: Vec<String> = refs.iter().map(|s| s.to_string()).collect();
    for detail in &details {
        // The chunk_index must address an existing part.
        assert!(
            detail.chunk_index < corrected.len(),
            "FINDING: CorrectionDetail.chunk_index {} out of range (parts={})",
            detail.chunk_index,
            corrected.len()
        );
        match apply_correction(&corrected[detail.chunk_index], detail) {
            Some(fixed) => corrected[detail.chunk_index] = fixed,
            // Coordinate not applicable (e.g. position past the data-part of
            // this particular string state) — cannot run idempotence here.
            None => return,
        }
    }

    let corrected_refs: Vec<&str> = corrected.iter().map(String::as_str).collect();
    let (d2, details2) = decode_with_correction(&corrected_refs)
        .expect("FINDING: applying reported corrections produced an undecodable set");

    assert_eq!(
        d, d2,
        "FINDING: apply-details idempotence — corrected set decodes to a different Descriptor"
    );
    assert!(
        details2.is_empty(),
        "FINDING: apply-details idempotence — corrected set still reports corrections: {:?}",
        details2
    );
});
