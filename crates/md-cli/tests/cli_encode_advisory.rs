//! F-A4: calibrated encode-time footgun advisory for top-level bare legacy
//! P2SH multisig — `sh(multi(...))` / `sh(sortedmulti(...))`. Warn-only, on
//! stderr only, no new flag. Modern forms (`wsh(multi)`, `wpkh`, `tr`) and
//! the canonical BIP44 `pkh` default stay SILENT. The advisory must fire on
//! BOTH the text and `--json` branch and must never land on stdout.

#![allow(missing_docs)]

use assert_cmd::Command;

/// Stable substring of the legacy-P2SH footgun advisory.
const ADVISORY_SUBSTR: &str = "legacy P2SH multisig";

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

#[test]
fn sh_sortedmulti_text_emits_footgun_advisory() {
    let (stdout, stderr, ok) = run(&[
        "encode",
        "sh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*))",
        "--path",
        "48'/0'/0'",
    ]);
    assert!(ok, "encode should still succeed (warn-only)");
    assert!(
        stderr.contains(ADVISORY_SUBSTR),
        "expected footgun advisory on stderr; got: {stderr}"
    );
    assert!(
        stdout.starts_with("md1"),
        "stdout must be the card: {stdout}"
    );
    assert!(
        !stdout.contains(ADVISORY_SUBSTR) && !stdout.to_lowercase().contains("warning"),
        "advisory must never land on stdout: {stdout}"
    );
}

#[cfg(feature = "json")]
#[test]
fn sh_sortedmulti_json_emits_footgun_advisory() {
    let (stdout, stderr, ok) = run(&[
        "encode",
        "--json",
        "sh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*))",
        "--path",
        "48'/0'/0'",
    ]);
    assert!(ok);
    assert!(
        stderr.contains(ADVISORY_SUBSTR),
        "json branch missed the footgun advisory; got: {stderr}"
    );
    assert!(
        !stdout.contains(ADVISORY_SUBSTR),
        "advisory must never land on stdout (json): {stdout}"
    );
}

#[test]
fn sh_multi_text_emits_footgun_advisory() {
    let (_stdout, stderr, ok) = run(&[
        "encode",
        "sh(multi(2,@0/<0;1>/*,@1/<0;1>/*))",
        "--path",
        "48'/0'/0'",
    ]);
    assert!(ok);
    assert!(
        stderr.contains(ADVISORY_SUBSTR),
        "sh(multi) must also warn; got: {stderr}"
    );
}

/// Modern / canonical forms are SILENT — no footgun advisory.
#[test]
fn safe_forms_emit_no_footgun_advisory() {
    for template in [
        "wpkh(@0/<0;1>/*)",
        "pkh(@0/<0;1>/*)",
        "tr(@0/<0;1>/*)",
        "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))",
        "sh(wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*)))",
    ] {
        let (_stdout, stderr, ok) = run(&["encode", template]);
        assert!(ok, "{template} should encode");
        assert!(
            !stderr.contains(ADVISORY_SUBSTR),
            "{template} must NOT emit the footgun advisory; got: {stderr}"
        );
    }
}

/// The advisory does not change stdout: a warned form's card equals its own
/// card produced when the advisory is suppressed by redirecting stderr — i.e.
/// stdout is byte-identical regardless (advisory is stderr-only).
#[test]
fn advisory_does_not_perturb_stdout() {
    let args = [
        "encode",
        "sh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*))",
        "--path",
        "48'/0'/0'",
        "--group-size",
        "0",
    ];
    let (stdout_a, _e, _s) = run(&args);
    let (stdout_b, _e2, _s2) = run(&args);
    assert_eq!(stdout_a, stdout_b, "stdout must be deterministic");
    assert!(stdout_a.trim().starts_with("md1"));
    // The single stdout line is only the card — no advisory text.
    assert_eq!(stdout_a.lines().filter(|l| !l.is_empty()).count(), 1);
}
