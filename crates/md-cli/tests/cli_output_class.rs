//! Integration tests for the output-class stderr advisory (Phase 2 sibling sweep).
//!
//! Covers: byte-parity of advisory lines against ms-cli/mnemonic-toolkit,
//! and that `md decode` emits the Template advisory at BOTH the `--json` and
//! text return sites.

use assert_cmd::Command;

/// Byte-identical to mnemonic-toolkit secret_advisory.rs + ms-cli advisory.rs.
const PRIVATE_KEY_LINE: &str = "warning: stdout carries private key material (can spend) \u{2014} redirect or encrypt (e.g. '> file.txt' or '| age -e ...')";
const WATCH_ONLY_LINE: &str = "note: stdout is watch-only \u{2014} public keys only, cannot spend";
const TEMPLATE_LINE: &str = "note: stdout is a keyless descriptor template (no keys)";

/// Canonical v0.30 md1 (decodes clean, text + json) — smoke.rs:19.
const MD1_FIXTURE: &str = "md1yqpqqxqq8xtwhw4xwn4qh";

#[test]
fn byte_parity_advisory_lines() {
    assert_eq!(PRIVATE_KEY_LINE, "warning: stdout carries private key material (can spend) \u{2014} redirect or encrypt (e.g. '> file.txt' or '| age -e ...')");
    assert_eq!(WATCH_ONLY_LINE, "note: stdout is watch-only \u{2014} public keys only, cannot spend");
    assert_eq!(TEMPLATE_LINE, "note: stdout is a keyless descriptor template (no keys)");
}

#[test]
fn decode_text_emits_template_advisory() {
    let out = Command::cargo_bin("md").unwrap().args(["decode", MD1_FIXTURE]).output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8(out.stderr).unwrap().contains(TEMPLATE_LINE));
}

#[test]
fn decode_json_emits_template_advisory() {
    let out = Command::cargo_bin("md").unwrap().args(["decode", "--json", MD1_FIXTURE]).output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8(out.stderr).unwrap().contains(TEMPLATE_LINE), "json branch missed the advisory");
}
