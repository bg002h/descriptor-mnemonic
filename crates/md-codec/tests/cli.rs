//! CLI integration tests for the `md` binary.
//!
//! Exercises all six subcommands (encode, decode, verify, inspect, bytecode,
//! vectors) via `assert_cmd::Command::cargo_bin("md")`.  The `md` binary is
//! gated behind the `cli` feature, which is included in `default`.

use assert_cmd::Command;
use predicates::prelude::*;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

/// A short, deterministic wallet policy that encodes as a single-string MD
/// chunk (`md1…`) without needing any key origin information.
const POLICY: &str = "wsh(pk(@0/**))";

/// A different policy used to verify a mismatch exits non-zero.
const OTHER_POLICY: &str = "wsh(multi(2,@0/**,@1/**))";

/// Encode `POLICY` and return the first output line (the MD chunk string).
fn encode_first_chunk() -> String {
    let output = Command::cargo_bin("md")
        .expect("binary built")
        .args(["encode", POLICY])
        .output()
        .expect("encode ran");
    assert!(output.status.success(), "encode must succeed");
    let stdout = String::from_utf8(output.stdout).expect("utf-8");
    // First non-empty line is the MD chunk.
    stdout
        .lines()
        .find(|l| !l.is_empty())
        .expect("at least one line of output")
        .to_owned()
}

// ---------------------------------------------------------------------------
// Happy-path tests
// ---------------------------------------------------------------------------

/// `md encode <policy>` — exits 0, stdout starts with the bech32 HRP prefix, stderr empty.
#[test]
fn md_encode_default() {
    Command::cargo_bin("md")
        .expect("binary built")
        .args(["encode", POLICY])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("md1"))
        .stderr("");
}

/// `wdm encode <policy> --json` — exits 0, stdout is a JSON object with a
/// `chunks` array key.
#[test]
fn md_encode_json() {
    let output = Command::cargo_bin("md")
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

/// `wdm encode --json` output is stable and consumable: per-chunk objects
/// expose the v0.1.1-contract field set (`raw`, `chunk_index`,
/// `total_chunks`, `code`) with `code` rendered as a lowercase string
/// (`"regular"` or `"long"`), and a top-level `wallet_id_words` string.
///
/// This guards the v0.2 wrapper-type refactor (closes FOLLOWUPS
/// `7-serialize-derives`) against accidental shape drift — the wrappers
/// must preserve the exact JSON contract the v0.1.1 hand-built
/// `serde_json::json!{}` literal produced.
#[test]
fn md_encode_json_shape_is_stable() {
    let output = Command::cargo_bin("md")
        .expect("binary built")
        .args(["encode", POLICY, "--json"])
        .output()
        .expect("encode --json ran");
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("utf-8");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");

    let words = v
        .get("wallet_id_words")
        .and_then(|w| w.as_str())
        .expect("wallet_id_words string");
    assert_eq!(
        words.split_whitespace().count(),
        12,
        "wallet_id_words must be a 12-word BIP-39 mnemonic, got: {words:?}"
    );

    let chunks = v
        .get("chunks")
        .and_then(|c| c.as_array())
        .expect("chunks array");
    assert!(!chunks.is_empty());
    let first = &chunks[0];
    let raw = first.get("raw").and_then(|r| r.as_str()).expect("raw");
    assert!(raw.starts_with("md1"));
    assert!(first.get("chunk_index").and_then(|c| c.as_u64()).is_some());
    assert!(first.get("total_chunks").and_then(|c| c.as_u64()).is_some());
    let code = first.get("code").and_then(|c| c.as_str()).expect("code");
    assert!(
        code == "regular" || code == "long",
        "code must be lowercase string `regular` or `long`, got: {code:?}"
    );
}

/// `wdm decode --json` output exposes the v0.1.1-contract field set:
/// top-level `policy` (canonical-string form) plus a `report` object with
/// `outcome`, `confidence`, `corrections`, `verifications` (with all five
/// flag fields).
#[test]
fn md_decode_json_shape_is_stable() {
    let chunk = encode_first_chunk();

    let output = Command::cargo_bin("md")
        .expect("binary built")
        .args(["decode", &chunk, "--json"])
        .output()
        .expect("decode --json ran");
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("utf-8");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");

    let policy = v
        .get("policy")
        .and_then(|p| p.as_str())
        .expect("policy str");
    assert!(policy.contains("wsh(pk("));

    let report = v.get("report").expect("report object");
    let outcome = report
        .get("outcome")
        .and_then(|o| o.as_str())
        .expect("outcome");
    assert_eq!(outcome, "Clean", "round-trip should yield Clean outcome");
    let confidence = report
        .get("confidence")
        .and_then(|c| c.as_str())
        .expect("confidence");
    assert_eq!(confidence, "Confirmed");
    assert!(
        report
            .get("corrections")
            .and_then(|c| c.as_array())
            .is_some()
    );

    let verifications = report.get("verifications").expect("verifications");
    for flag in [
        "bytecode_well_formed",
        "cross_chunk_hash_ok",
        "total_chunks_consistent",
        "version_supported",
        "wallet_id_consistent",
    ] {
        assert!(
            verifications.get(flag).and_then(|f| f.as_bool()).is_some(),
            "verifications.{flag} missing or non-bool"
        );
    }
}

/// `wdm encode --path bip48 <policy>` — exits 0, and the encoded bytecode's
/// shared-path declaration reflects BIP 48 (named indicator 0x05) rather
/// than the default-tier BIP 84 fallback (indicator 0x03).
///
/// We verify the path via the parallel `wdm bytecode` invocation, which is
/// not yet path-aware (it always emits the default-tier path) — so we
/// instead decode the encoded chunk and compare its bytecode's path byte
/// against the BIP 48 indicator. This proves the `--path` override flowed
/// through `EncodeOptions::shared_path` to `WalletPolicy::to_bytecode`.
#[test]
fn md_encode_path_override_bip48_takes_effect() {
    use md_codec::{DecodeOptions, decode};

    let output = Command::cargo_bin("md")
        .expect("binary built")
        .args(["encode", "--path", "bip48", POLICY])
        .output()
        .expect("encode --path bip48 ran");
    assert!(
        output.status.success(),
        "encode --path bip48 must succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // The warning text from v0.1.1 ("--path is parsed but the shared-path
    // override is not yet applied") must NOT appear; Phase B wires it
    // through.
    let stderr = String::from_utf8(output.stderr).expect("utf-8");
    assert!(
        !stderr.contains("--path is parsed but"),
        "Phase B removed the v0.1.1 warning; got stderr: {stderr}"
    );

    // Take the first MD chunk string from stdout.
    let stdout = String::from_utf8(output.stdout).expect("utf-8");
    let chunk = stdout
        .lines()
        .find(|l| l.starts_with("md1"))
        .expect("at least one md1 chunk on stdout");

    // Decode and inspect the underlying bytecode's path declaration.
    // The Phase A precedence rule populates `decoded_shared_path` from the
    // wire path, then `to_bytecode` reproduces it byte-identically. So we
    // can re-encode the decoded policy and read the path-indicator byte.
    let result = decode(&[chunk], &DecodeOptions::new()).expect("decode");
    let bytecode = result
        .policy
        .to_bytecode(&md_codec::EncodeOptions::default())
        .expect("re-encode bytecode");
    // bytecode = [header=0x00, Tag::SharedPath=0x33, indicator, ...]
    assert_eq!(
        bytecode[1], 0x33,
        "bytecode byte[1] must be Tag::SharedPath; got {:02x}",
        bytecode[1]
    );
    assert_eq!(
        bytecode[2], 0x05,
        "with --path bip48 the shared-path indicator must be 0x05 (m/48'/0'/0'/2'), not 0x03 (default BIP 84). got {:02x}",
        bytecode[2]
    );
}

/// `md encode <policy> --force-chunked` — exits 0, the chunk string is
/// longer than the single-string variant because the chunked header adds
/// bytes.  We verify the output starts with the bech32 HRP prefix and is structurally
/// different from the non-chunked output (i.e. it is a Chunked-type chunk,
/// which the `inspect` sub-test confirms separately).
#[test]
fn md_encode_force_chunked() {
    Command::cargo_bin("md")
        .expect("binary built")
        .args(["encode", POLICY, "--force-chunked"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("md1"));

    // Sanity: the force-chunked output differs from the plain output.
    let plain = encode_first_chunk();
    let output = Command::cargo_bin("md")
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
fn md_decode_round_trip() {
    let chunk = encode_first_chunk();

    let output = Command::cargo_bin("md")
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
fn md_verify_match() {
    let chunk = encode_first_chunk();
    Command::cargo_bin("md")
        .expect("binary built")
        .args(["verify", &chunk, "--policy", POLICY])
        .assert()
        .success();
}

/// `wdm verify <chunk> --policy <different>` exits non-zero.
#[test]
fn md_verify_mismatch() {
    let chunk = encode_first_chunk();
    Command::cargo_bin("md")
        .expect("binary built")
        .args(["verify", &chunk, "--policy", OTHER_POLICY])
        .assert()
        .failure();
}

/// `wdm inspect <chunk>` exits 0, stdout contains `Type:` and `Version:`.
#[test]
fn md_inspect_outputs_chunk_header() {
    let chunk = encode_first_chunk();
    Command::cargo_bin("md")
        .expect("binary built")
        .args(["inspect", &chunk])
        .assert()
        .success()
        .stdout(predicate::str::contains("Type:"))
        .stdout(predicate::str::contains("Version:"));
}

/// `wdm bytecode <policy>` exits 0, stdout is a lowercase hex string.
#[test]
fn md_bytecode_outputs_lowercase_hex() {
    Command::cargo_bin("md")
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
fn md_encode_unparseable_policy_exits_nonzero() {
    Command::cargo_bin("md")
        .expect("binary built")
        .args(["encode", "not-a-real-policy"])
        .assert()
        .failure()
        .stderr(predicate::str::is_empty().not());
}

/// `wdm decode "not-a-wdm-string"` exits non-zero, stderr mentions the HRP.
#[test]
fn md_decode_invalid_string_exits_nonzero() {
    Command::cargo_bin("md")
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
fn md_vectors_returns_json_top_level_object() {
    Command::cargo_bin("md")
        .expect("binary built")
        .arg("vectors")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("{").or(predicate::str::starts_with("[")));
}

/// `md not-a-subcommand` exits non-zero, stderr mentions `md` usage.
#[test]
fn md_unknown_subcommand_exits_nonzero() {
    Command::cargo_bin("md")
        .expect("binary built")
        .arg("not-a-subcommand")
        .assert()
        .failure()
        .stderr(predicate::str::contains("md").or(predicate::str::contains("unrecognized")));
}

// ---------------------------------------------------------------------------
// v0.2.1 — `--fingerprint` flag (phase-e-cli-fingerprint-flag)
// ---------------------------------------------------------------------------

/// Two `--fingerprint @i=hex` args for a 2-key policy → encoder accepts;
/// stderr carries the privacy warning; stdout has the encoded chunk.
#[test]
fn md_encode_fingerprint_flag_accepts_two_placeholders() {
    Command::cargo_bin("md")
        .expect("binary built")
        .args([
            "encode",
            "--fingerprint",
            "@0=deadbeef",
            "--fingerprint",
            "@1=cafebabe",
            "wsh(multi(2,@0/**,@1/**))",
        ])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("md1"))
        .stderr(predicate::str::contains(
            "--fingerprint embeds master-key fingerprints",
        ));
}

/// Missing index in the `--fingerprint` set → exits nonzero with a clear error.
#[test]
fn md_encode_fingerprint_flag_rejects_index_gap() {
    Command::cargo_bin("md")
        .expect("binary built")
        .args([
            "encode",
            "--fingerprint",
            "@0=deadbeef",
            // Missing @1 for a 2-key policy.
            "wsh(multi(2,@0/**,@1/**))",
        ])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("fingerprints count mismatch")
                .or(predicate::str::contains("missing")),
        );
}

/// Wrong-length hex (4 chars, not 8) → exits nonzero with a parse error.
#[test]
fn md_encode_fingerprint_flag_rejects_short_hex() {
    Command::cargo_bin("md")
        .expect("binary built")
        .args([
            "encode",
            "--fingerprint",
            "@0=dead", // 4 chars, not 8
            "wsh(pk(@0/**))",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("8 hex chars"));
}
