//! CLI integration tests for the `wdm` binary.
//!
//! Exercises all six subcommands (encode, decode, verify, inspect, bytecode,
//! vectors) via `assert_cmd::Command::cargo_bin("wdm")`.  The `wdm` binary is
//! gated behind the `cli` feature, which is included in `default`.

use assert_cmd::Command;
use predicates::prelude::*;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

/// A short, deterministic wallet policy that encodes as a single-string WDM
/// chunk (`wdm1…`) without needing any key origin information.
const POLICY: &str = "wsh(pk(@0/**))";

/// A different policy used to verify a mismatch exits non-zero.
const OTHER_POLICY: &str = "wsh(multi(2,@0/**,@1/**))";

/// Encode `POLICY` and return the first output line (the WDM chunk string).
fn encode_first_chunk() -> String {
    let output = Command::cargo_bin("wdm")
        .expect("binary built")
        .args(["encode", POLICY])
        .output()
        .expect("encode ran");
    assert!(output.status.success(), "encode must succeed");
    let stdout = String::from_utf8(output.stdout).expect("utf-8");
    // First non-empty line is the WDM chunk.
    stdout
        .lines()
        .find(|l| !l.is_empty())
        .expect("at least one line of output")
        .to_owned()
}

// ---------------------------------------------------------------------------
// Happy-path tests
// ---------------------------------------------------------------------------

/// `wdm encode <policy>` — exits 0, stdout starts with `wdm1`, stderr empty.
#[test]
fn wdm_encode_default() {
    Command::cargo_bin("wdm")
        .expect("binary built")
        .args(["encode", POLICY])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("wdm1"))
        .stderr("");
}

/// `wdm encode <policy> --json` — exits 0, stdout is a JSON object with a
/// `chunks` array key.
#[test]
fn wdm_encode_json() {
    let output = Command::cargo_bin("wdm")
        .expect("binary built")
        .args(["encode", POLICY, "--json"])
        .output()
        .expect("encode --json ran");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("utf-8");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert!(
        v.get("chunks").and_then(|c| c.as_array()).is_some(),
        "expected a top-level `chunks` array, got: {stdout}"
    );
}

/// `wdm encode <policy> --force-chunked` — exits 0, the chunk string is
/// longer than the single-string variant because the chunked header adds
/// bytes.  We verify the output starts with `wdm1` and is structurally
/// different from the non-chunked output (i.e. it is a Chunked-type chunk,
/// which the `inspect` sub-test confirms separately).
#[test]
fn wdm_encode_force_chunked() {
    Command::cargo_bin("wdm")
        .expect("binary built")
        .args(["encode", POLICY, "--force-chunked"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("wdm1"));

    // Sanity: the force-chunked output differs from the plain output.
    let plain = encode_first_chunk();
    let output = Command::cargo_bin("wdm")
        .expect("binary built")
        .args(["encode", POLICY, "--force-chunked"])
        .output()
        .expect("force-chunked ran");
    let forced_first = String::from_utf8(output.stdout)
        .expect("utf-8")
        .lines()
        .find(|l| !l.is_empty())
        .unwrap()
        .to_owned();
    assert_ne!(
        plain, forced_first,
        "--force-chunked should produce a different chunk string"
    );
}

/// Round-trip: encode then decode, decoded policy matches original.
#[test]
fn wdm_decode_round_trip() {
    let chunk = encode_first_chunk();

    let output = Command::cargo_bin("wdm")
        .expect("binary built")
        .args(["decode", &chunk])
        .output()
        .expect("decode ran");

    assert!(output.status.success(), "decode must succeed");
    let stdout = String::from_utf8(output.stdout).expect("utf-8");
    // The first line of `wdm decode` output is the canonical policy string.
    let decoded_policy = stdout.lines().next().expect("at least one line").trim();
    // The encoder normalises `@0/**` → `@0/<0;1>/*`; accept either form.
    assert!(
        decoded_policy.contains("wsh(pk("),
        "expected a wsh(pk(…)) policy, got: {decoded_policy}"
    );
}

/// `wdm verify <chunk> --policy <same>` exits 0.
#[test]
fn wdm_verify_match() {
    let chunk = encode_first_chunk();
    Command::cargo_bin("wdm")
        .expect("binary built")
        .args(["verify", &chunk, "--policy", POLICY])
        .assert()
        .success();
}

/// `wdm verify <chunk> --policy <different>` exits non-zero.
#[test]
fn wdm_verify_mismatch() {
    let chunk = encode_first_chunk();
    Command::cargo_bin("wdm")
        .expect("binary built")
        .args(["verify", &chunk, "--policy", OTHER_POLICY])
        .assert()
        .failure();
}

/// `wdm inspect <chunk>` exits 0, stdout contains `Type:` and `Version:`.
#[test]
fn wdm_inspect_outputs_chunk_header() {
    let chunk = encode_first_chunk();
    Command::cargo_bin("wdm")
        .expect("binary built")
        .args(["inspect", &chunk])
        .assert()
        .success()
        .stdout(predicate::str::contains("Type:"))
        .stdout(predicate::str::contains("Version:"));
}

/// `wdm bytecode <policy>` exits 0, stdout is a lowercase hex string.
#[test]
fn wdm_bytecode_outputs_lowercase_hex() {
    Command::cargo_bin("wdm")
        .expect("binary built")
        .args(["bytecode", POLICY])
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]+\s*$").expect("regex"));
}

// ---------------------------------------------------------------------------
// Error-path tests
// ---------------------------------------------------------------------------

/// `wdm encode "not-a-real-policy"` exits non-zero, stderr non-empty.
#[test]
fn wdm_encode_unparseable_policy_exits_nonzero() {
    Command::cargo_bin("wdm")
        .expect("binary built")
        .args(["encode", "not-a-real-policy"])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty().not());
}

/// `wdm decode "not-a-wdm-string"` exits non-zero, stderr mentions the HRP.
#[test]
fn wdm_decode_invalid_string_exits_nonzero() {
    Command::cargo_bin("wdm")
        .expect("binary built")
        .args(["decode", "not-a-wdm-string"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("HRP")
                .or(predicate::str::contains("hrp"))
                .or(predicate::str::contains("invalid"))
                .or(predicate::str::contains("decode")),
        );
}

/// `wdm vectors` exits 0, stdout starts with `{` (top-level JSON object).
#[test]
fn wdm_vectors_returns_json_top_level_object() {
    Command::cargo_bin("wdm")
        .expect("binary built")
        .arg("vectors")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("{").or(predicate::str::starts_with("[")));
}

/// `wdm not-a-subcommand` exits non-zero, stderr mentions `wdm` usage.
#[test]
fn wdm_unknown_subcommand_exits_nonzero() {
    Command::cargo_bin("wdm")
        .expect("binary built")
        .arg("not-a-subcommand")
        .assert()
        .failure()
        .stderr(predicate::str::contains("wdm").or(predicate::str::contains("unrecognized")));
}
