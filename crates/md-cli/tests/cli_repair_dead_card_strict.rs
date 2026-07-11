//! P1.3 — `md repair` regression: repair does NOT opt into partial decode
//! (stays strict). Pins two invariants per SPEC/plan:
//!   1. An untouched dead card (`canonical_origin == None`, no explicit
//!      origin) is UN-repairable via `md repair` — exit 2 (unchanged).
//!   2. A BCH correction that resolves to a dead card is PRUNED — exit 2
//!      (never `Ok`/exit 5, i.e. never treated as a successful repair of a
//!      dead card).
//!
//! `md_codec::decode_with_correction` was NOT given a partial variant (see
//! P0/`design/SPEC_pathless_partial_decode.md` — repair stays strict); this
//! file exists purely to pin that non-change against regression, since a
//! future cycle could otherwise be tempted to thread `DecodeOpts` through
//! repair too.

#![allow(missing_docs)]

use assert_cmd::Command;
use std::process::Command as StdCommand;

/// Codex32 alphabet — mirrors `md_codec::chunk::CODEX32_ALPHABET` (module-
/// private) for deterministic single-char corruption. Stable per BIP 173.
const CODEX32_ALPHABET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// Encode a template via `md encode --force-chunked --group-size 0 <T>`
/// (repair requires chunked-form input). Returns the ordered chunk strings.
fn encode_chunked(template: &str) -> Vec<String> {
    let out = StdCommand::new(assert_cmd::cargo::cargo_bin("md"))
        .args(["encode", "--force-chunked", "--group-size", "0", template])
        .output()
        .expect("invoke md encode --force-chunked");
    assert!(
        out.status.success(),
        "md encode --force-chunked {template:?} failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let s = String::from_utf8(out.stdout).expect("stdout utf-8");
    s.lines()
        .filter(|l| l.starts_with("md1"))
        .map(String::from)
        .collect()
}

/// Flip 1 character at `pos` (0-indexed into the data-part, i.e. chars
/// after `md1`) by XORing its 5-bit symbol with `xor_mask & 0x1F`.
fn corrupt_at(chunk: &str, pos: usize, xor_mask: u8) -> String {
    let hrp_len = 3; // "md1"
    let mut chars: Vec<char> = chunk.chars().collect();
    let abs_idx = hrp_len + pos;
    let original_sym = CODEX32_ALPHABET
        .iter()
        .position(|&b| b == chars[abs_idx].to_ascii_lowercase() as u8)
        .expect("char in codex32 alphabet") as u8;
    let new_sym = (original_sym ^ (xor_mask & 0x1F)) & 0x1F;
    chars[abs_idx] = CODEX32_ALPHABET[new_sym as usize] as char;
    chars.iter().collect()
}

/// A dead shape (canonical_origin == None): legacy P2SH sortedmulti, no
/// --path, no explicit per-key origin.
const DEAD_TEMPLATE: &str = "sh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*))";

#[test]
fn untouched_dead_card_is_unrepairable_exit_2() {
    let chunks = encode_chunked(DEAD_TEMPLATE);
    assert_eq!(
        chunks.len(),
        1,
        "small dead template should force-chunk to exactly 1 chunk; got {chunks:?}"
    );
    // Sanity: decode --strict (no repair) on this same untouched chunk
    // rejects with MissingExplicitOrigin via `md decode` too — but that is
    // now a PARTIAL-decode (exit 4) per P1.1. `md repair`'s oracle is a
    // DIFFERENT strict primitive (`decode_with_correction`) that never
    // opts into partial — an untouched-but-VALID dead card has zero BCH
    // corrections to apply, so `decode_with_correction` must still reject
    // it with the strict `MissingExplicitOrigin` (exit 2), never pass it
    // through as "no corrections needed" (exit 0).
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["repair", &chunks[0]])
        .output()
        .expect("invoke md repair");
    assert_eq!(
        out.status.code(),
        Some(2),
        "untouched dead card must stay UN-repairable (exit 2, unchanged); stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.is_empty(),
        "no card output should be emitted on the exit-2 reject path; got {stdout:?}"
    );
}

#[test]
fn correction_resolving_to_dead_card_is_pruned_exit_2_never_ok_or_5() {
    let chunks = encode_chunked(DEAD_TEMPLATE);
    assert_eq!(chunks.len(), 1);
    // A small, correctable (<=4 symbol errors) corruption. If
    // `decode_with_correction` were to opt into partial decode, this would
    // succeed (exit 5, REPAIR_APPLIED) since the corrected bytes decode to
    // the same dead-shape descriptor. Per P1.3 (repair stays strict), the
    // correction candidate is PRUNED (the origin gate rejects it) — exit 2.
    let corrupted = corrupt_at(&chunks[0], 10, 0b10110);
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["repair", &corrupted])
        .output()
        .expect("invoke md repair");
    let code = out.status.code().expect("exited normally");
    assert_ne!(
        code, 0,
        "a correction resolving to a dead card must NOT silently pass as exit 0"
    );
    assert_ne!(
        code, 5,
        "a correction resolving to a dead card must NEVER be reported as REPAIR_APPLIED (exit 5) \
         — repair does not opt into partial-decode (P1.3)"
    );
    assert_eq!(
        code,
        2,
        "expected exit 2 (pruned/atomic-fail, unchanged strict behavior); stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}
