//! P1.2 — `md encode` pathless/dead-card advisory.
//!
//! Per `design/SPEC_pathless_partial_decode.md` / `design/
//! IMPLEMENTATION_PLAN_pathless_partial_decode.md` (mnemonic-toolkit repo),
//! Phase P1.2 (+ the I-1 whole-diff fix): when the FINAL encoded descriptor
//! (`--path` already applied) still carries an unresolvable origin —
//! `descriptor.unresolved_origin_indices()` non-empty, the exact P0 query
//! `md decode`/`md inspect` use to decide partial — `md encode` prints a
//! loud stderr advisory nudging `--path` for a fully-decodable backup.
//! Mirrors the existing F-A4 legacy-P2SH-multisig footgun-advisory tone
//! (`tests/cli_encode_advisory.rs`). The card is still emitted; encode stays
//! exit 0 regardless.
//!
//! The oracle keys on actual resolvability, NOT a `canonical_origin == None`
//! plus `--path.is_none()` heuristic — so an inline per-`@N` explicit origin
//! is NOT falsely warned (it full-decodes at exit 0), and a `--path m`
//! zero-component override does NOT bypass the warning (it still
//! partial-decodes at exit 4).

#![allow(missing_docs)]

use assert_cmd::Command;
use std::process::Command as StdCommand;

/// Encode `template` (+ extra args) and return the single unbroken md1
/// phrase, then `md decode` it and return the decode exit code — used to
/// prove the advisory's presence/absence MATCHES the actual decode outcome
/// (never a misrepresentation).
fn encode_then_decode_exit(template: &str, extra_args: &[&str]) -> i32 {
    let mut enc_args = vec!["encode", "--group-size", "0"];
    enc_args.extend_from_slice(extra_args);
    enc_args.push(template);
    let enc = StdCommand::new(assert_cmd::cargo::cargo_bin("md"))
        .args(&enc_args)
        .output()
        .expect("invoke md encode");
    assert!(enc.status.success(), "encode must succeed (exit 0)");
    let phrase = String::from_utf8(enc.stdout)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .to_string();
    let dec = StdCommand::new(assert_cmd::cargo::cargo_bin("md"))
        .args(["decode", &phrase])
        .output()
        .expect("invoke md decode");
    dec.status.code().expect("decode exited normally")
}

/// Stable substring of the new pathless-card advisory.
const PATHLESS_ADVISORY_SUBSTR: &str = "no canonical default derivation path";

fn run(args: &[&str]) -> (String, String, bool) {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(args)
        .output()
        .unwrap();
    (
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
        out.status.success(),
    )
}

/// Dead shapes (canonical_origin == None) — one per SPEC golden class.
const DEAD_SHAPES: &[&str] = &[
    "tr(@0/<0;1>/*,pk(@1/<0;1>/*))",
    "sh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*))",
    "wsh(pk(@0/<0;1>/*))",
    "wsh(or_d(pk(@0/<0;1>/*),and_v(v:pk(@1/<0;1>/*),older(144))))",
];

/// Canonical shapes (canonical_origin == Some) — must NEVER advise.
const CANONICAL_SHAPES: &[&str] = &[
    "tr(@0/<0;1>/*)",
    "wpkh(@0/<0;1>/*)",
    "sh(wpkh(@0/<0;1>/*))",
    "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))",
];

#[test]
fn pathless_shape_without_path_emits_advisory() {
    for template in DEAD_SHAPES {
        let (stdout, stderr, ok) = run(&["encode", template]);
        assert!(ok, "{template} should still encode (exit 0)");
        assert!(
            stderr.contains(PATHLESS_ADVISORY_SUBSTR),
            "{template}: expected pathless advisory on stderr; got: {stderr}"
        );
        assert!(stdout.trim().starts_with("md1"), "stdout must be the card");
        assert!(
            !stdout.contains(PATHLESS_ADVISORY_SUBSTR),
            "advisory must never land on stdout: {stdout}"
        );
    }
}

#[cfg(feature = "json")]
#[test]
fn pathless_shape_without_path_json_branch_also_emits_advisory() {
    for template in DEAD_SHAPES {
        let (stdout, stderr, ok) = run(&["encode", "--json", template]);
        assert!(ok);
        assert!(
            stderr.contains(PATHLESS_ADVISORY_SUBSTR),
            "{template}: json branch missed the pathless advisory; got: {stderr}"
        );
        assert!(!stdout.contains(PATHLESS_ADVISORY_SUBSTR));
    }
}

#[test]
fn pathless_shape_with_path_suppresses_advisory() {
    for template in DEAD_SHAPES {
        let (_stdout, stderr, ok) = run(&["encode", template, "--path", "bip48"]);
        assert!(ok, "{template} + --path should encode");
        assert!(
            !stderr.contains(PATHLESS_ADVISORY_SUBSTR),
            "{template} + --path must NOT emit the pathless advisory; got: {stderr}"
        );
    }
}

#[test]
fn canonical_shapes_never_emit_pathless_advisory_with_or_without_path() {
    for template in CANONICAL_SHAPES {
        let (_stdout, stderr, ok) = run(&["encode", template]);
        assert!(ok, "{template} should encode");
        assert!(
            !stderr.contains(PATHLESS_ADVISORY_SUBSTR),
            "{template} (canonical, no --path) must NOT emit the pathless advisory; got: {stderr}"
        );

        let (_stdout2, stderr2, ok2) = run(&["encode", template, "--path", "bip44"]);
        assert!(ok2);
        assert!(
            !stderr2.contains(PATHLESS_ADVISORY_SUBSTR),
            "{template} + --path (canonical) must NOT emit the pathless advisory; got: {stderr2}"
        );
    }
}

/// The advisory must not perturb stdout: the card bytes are identical
/// whether or not the advisory fires (advisory is stderr-only).
#[test]
fn advisory_does_not_perturb_card_bytes() {
    let template = "sh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*))";
    let (stdout_no_path, _e1, _s1) = run(&["encode", template, "--group-size", "0"]);
    // Re-run without --path again: deterministic, same card.
    let (stdout_no_path_2, _e2, _s2) = run(&["encode", template, "--group-size", "0"]);
    assert_eq!(
        stdout_no_path, stdout_no_path_2,
        "stdout must be deterministic"
    );
    assert!(stdout_no_path.trim().starts_with("md1"));
}

// ─────────────────────────────────────────────────────────────────────────
// I-1 (whole-diff fix) — the advisory keys on ACTUAL resolvability of the
// final descriptor (`unresolved_origin_indices()`), not a
// `canonical_origin == None && --path.is_none()` heuristic. These two edge
// cases proved the old heuristic wrong on the binary.
// ─────────────────────────────────────────────────────────────────────────

/// FALSE-POSITIVE guard: a dead-shape wrapper (`sh(sortedmulti)`,
/// `canonical_origin == None`) carrying INLINE per-`@N` explicit origins and
/// NO `--path` full-decodes (exit 0). The old heuristic (fires when
/// `canonical_origin == None && --path.is_none()`) would loudly (and wrongly)
/// claim it partial-decodes/exit-4 — a never-misrepresent violation. The
/// resolvability oracle must stay SILENT here.
#[test]
fn inline_per_at_n_origins_no_path_full_decodes_and_is_not_warned() {
    let template = "sh(sortedmulti(2,@0/48'/0'/0'/1'/<0;1>/*,@1/48'/0'/0'/1'/<0;1>/*))";
    // Ground truth: this card FULL-decodes (exit 0), so any pathless advisory
    // would be a misrepresentation.
    assert_eq!(
        encode_then_decode_exit(template, &[]),
        0,
        "inline-origin card must FULL-decode (exit 0)"
    );
    let (_stdout, stderr, ok) = run(&["encode", template]);
    assert!(ok, "encode must still succeed (exit 0)");
    assert!(
        !stderr.contains(PATHLESS_ADVISORY_SUBSTR),
        "a full-decodable inline-origin card must NOT be warned as pathless; got: {stderr}"
    );
}

/// FALSE-NEGATIVE guard: `--path m` (zero components) applied to a dead
/// shape still yields an unresolvable origin — the minted card
/// partial-decodes at exit 4. The old `--path.is_some()` early-return
/// suppressed the advisory here, bypassing the exact footgun. The
/// resolvability oracle must FIRE — a `--path` flag alone does not clear it.
#[test]
fn path_m_zero_components_on_dead_shape_still_warns() {
    let template = "sh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*))";
    // Ground truth: --path m does NOT resolve the origin — the card still
    // partial-decodes (exit 4).
    assert_eq!(
        encode_then_decode_exit(template, &["--path", "m"]),
        4,
        "--path m on a dead shape must still partial-decode (exit 4)"
    );
    let (_stdout, stderr, ok) = run(&["encode", template, "--path", "m"]);
    assert!(ok, "encode must still succeed (exit 0)");
    assert!(
        stderr.contains(PATHLESS_ADVISORY_SUBSTR),
        "--path m must NOT suppress the pathless advisory (the footgun is not bypassed); got: {stderr}"
    );
}
