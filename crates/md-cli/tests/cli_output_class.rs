//! Integration tests for the output-class stderr advisory (Phase 2 sibling sweep).
//!
//! Covers: byte-parity of advisory lines against ms-cli/mnemonic-toolkit,
//! and that `md decode` emits the Template advisory at BOTH the `--json` and
//! text return sites.

use assert_cmd::Command;
use std::process::Command as StdCommand;

/// Byte-identical to mnemonic-toolkit secret_advisory.rs + ms-cli advisory.rs.
const PRIVATE_KEY_LINE: &str = "warning: stdout carries private key material (can spend) \u{2014} redirect or encrypt (e.g. '> file.txt' or '| age -e ...')";
const WATCH_ONLY_LINE: &str = "note: stdout is watch-only \u{2014} public keys only, cannot spend";
const TEMPLATE_LINE: &str = "note: stdout is a keyless descriptor template (no keys)";

/// Canonical v0.30 md1 (decodes clean, text + json) — smoke.rs:19.
const MD1_FIXTURE: &str = "md1yqpqqxqq8xtwhw4xwn4qh";

/// Codex32 alphabet — mirrors md_codec::chunk::CODEX32_ALPHABET (module-private).
/// Needed for the repair test's corrupt_at helper.
const CODEX32_ALPHABET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

#[test]
fn byte_parity_advisory_lines() {
    assert_eq!(
        PRIVATE_KEY_LINE,
        "warning: stdout carries private key material (can spend) \u{2014} redirect or encrypt (e.g. '> file.txt' or '| age -e ...')"
    );
    assert_eq!(
        WATCH_ONLY_LINE,
        "note: stdout is watch-only \u{2014} public keys only, cannot spend"
    );
    assert_eq!(
        TEMPLATE_LINE,
        "note: stdout is a keyless descriptor template (no keys)"
    );
}

#[test]
fn decode_text_emits_template_advisory() {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["decode", MD1_FIXTURE])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains(TEMPLATE_LINE)
    );
}

#[test]
fn decode_json_emits_template_advisory() {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["decode", "--json", MD1_FIXTURE])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains(TEMPLATE_LINE),
        "json branch missed the advisory"
    );
}

// ──────────────────────────────────────────────────────────────────────────
// encode
// ──────────────────────────────────────────────────────────────────────────

#[test]
fn encode_text_emits_template() {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["encode", "wpkh(@0/<0;1>/*)"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "encode exited non-zero; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains(TEMPLATE_LINE),
        "encode text: expected TEMPLATE_LINE on stderr"
    );
}

#[test]
fn encode_json_emits_template() {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["encode", "--json", "wpkh(@0/<0;1>/*)"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "encode --json exited non-zero; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains(TEMPLATE_LINE),
        "encode json: expected TEMPLATE_LINE on stderr"
    );
}

// L19 (cycle-9): `md encode --key …` carries watch-only (public) key material
// in the Pubkeys TLV → the advisory must be WatchOnly, NOT the keyless-template
// label. Keyless `md encode <template>` (above) stays Template.

#[test]
fn encode_text_keyed_emits_watch_only() {
    let xpub = account_xpub("m/84'/0'/0'");
    let key_arg = format!("@0={xpub}");
    // A keyed wpkh exceeds the 80-symbol single-string cap → --force-chunked.
    let out = Command::cargo_bin("md")
        .unwrap()
        .args([
            "encode",
            "--force-chunked",
            "--key",
            &key_arg,
            "wpkh(@0/<0;1>/*)",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "encode keyed exited non-zero; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains(WATCH_ONLY_LINE),
        "encode text keyed: expected WATCH_ONLY_LINE; got: {stderr}"
    );
    assert!(
        !stderr.contains(TEMPLATE_LINE),
        "encode text keyed: must NOT emit the keyless-template line; got: {stderr}"
    );
}

#[test]
fn encode_json_keyed_emits_watch_only() {
    let xpub = account_xpub("m/84'/0'/0'");
    let key_arg = format!("@0={xpub}");
    let out = Command::cargo_bin("md")
        .unwrap()
        .args([
            "encode",
            "--json",
            "--force-chunked",
            "--key",
            &key_arg,
            "wpkh(@0/<0;1>/*)",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "encode --json keyed exited non-zero; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains(WATCH_ONLY_LINE),
        "encode json keyed: expected WATCH_ONLY_LINE; got: {stderr}"
    );
    assert!(
        !stderr.contains(TEMPLATE_LINE),
        "encode json keyed: must NOT emit the keyless-template line; got: {stderr}"
    );
}

// ──────────────────────────────────────────────────────────────────────────
// inspect
// ──────────────────────────────────────────────────────────────────────────

#[test]
fn inspect_text_emits_template() {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["inspect", MD1_FIXTURE])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "inspect exited non-zero; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains(TEMPLATE_LINE),
        "inspect text: expected TEMPLATE_LINE on stderr"
    );
}

#[test]
fn inspect_json_emits_template() {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["inspect", "--json", MD1_FIXTURE])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "inspect --json exited non-zero; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains(TEMPLATE_LINE),
        "inspect json: expected TEMPLATE_LINE on stderr"
    );
}

// ──────────────────────────────────────────────────────────────────────────
// bytecode
// ──────────────────────────────────────────────────────────────────────────

#[test]
fn bytecode_text_emits_template() {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["bytecode", MD1_FIXTURE])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "bytecode exited non-zero; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains(TEMPLATE_LINE),
        "bytecode text: expected TEMPLATE_LINE on stderr"
    );
}

#[test]
fn bytecode_json_emits_template() {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["bytecode", "--json", MD1_FIXTURE])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "bytecode --json exited non-zero; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains(TEMPLATE_LINE),
        "bytecode json: expected TEMPLATE_LINE on stderr"
    );
}

// ──────────────────────────────────────────────────────────────────────────
// repair
// ──────────────────────────────────────────────────────────────────────────

/// Encode a template with --force-chunked (required by decode_with_correction).
/// Returns the chunk strings (strip the leading `chunk-set-id:` line).
fn encode_chunked_for_repair(template: &str) -> Vec<String> {
    let out = StdCommand::new(assert_cmd::cargo::cargo_bin("md"))
        .args(["encode", "--force-chunked", template])
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
/// Result is guaranteed parseable but BCH-invalid.
fn corrupt_at_for_repair(chunk: &str, pos: usize, xor_mask: u8) -> String {
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

#[test]
fn repair_emits_template() {
    let chunks = encode_chunked_for_repair("wpkh(@0/<0;1>/*)");
    assert_eq!(
        chunks.len(),
        1,
        "single-chunk fixture must produce exactly 1 chunk; got {chunks:?}"
    );
    let valid = &chunks[0];
    // Corrupt position 10 (past the chunk-header region) — same idiom as cli_repair.rs.
    let corrupted = corrupt_at_for_repair(valid, 10, 0b10110);

    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["repair", &corrupted])
        .output()
        .expect("invoke md repair");
    assert_eq!(
        out.status.code(),
        Some(5),
        "expected exit 5 (REPAIR_APPLIED); stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains(TEMPLATE_LINE),
        "repair: expected TEMPLATE_LINE on stderr"
    );
}

/// Encode a KEYED (wallet-policy) template with --force-chunked. Returns the
/// chunk strings. The Pubkeys TLV is non-empty → `is_wallet_policy()` is true.
fn encode_keyed_chunked_for_repair(template: &str, key_arg: &str) -> Vec<String> {
    let out = StdCommand::new(assert_cmd::cargo::cargo_bin("md"))
        .args(["encode", "--force-chunked", "--key", key_arg, template])
        .output()
        .expect("invoke md encode --force-chunked --key");
    assert!(
        out.status.success(),
        "md encode --force-chunked --key {template:?} failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let s = String::from_utf8(out.stdout).expect("stdout utf-8");
    s.lines()
        .filter(|l| l.starts_with("md1"))
        .map(String::from)
        .collect()
}

#[test]
fn repair_keyed_emits_watch_only() {
    // L4 (cycle-9): a watch-only (keyed) md1 repaired by `md repair` must label
    // WatchOnly, not the keyless-template line.
    let xpub = account_xpub("m/84'/0'/0'");
    let key_arg = format!("@0={xpub}");
    let chunks = encode_keyed_chunked_for_repair("wpkh(@0/<0;1>/*)", &key_arg);
    assert!(!chunks.is_empty(), "expected >=1 chunk; got {chunks:?}");
    // Corrupt position 10 of the FIRST chunk; remaining chunks stay valid → the
    // call is atomic and applies a correction → exit 5, advisory emitted.
    let mut corrupted = chunks.clone();
    corrupted[0] = corrupt_at_for_repair(&chunks[0], 10, 0b10110);

    let mut args = vec!["repair".to_string()];
    args.extend(corrupted.iter().cloned());
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(&args)
        .output()
        .expect("invoke md repair");
    assert_eq!(
        out.status.code(),
        Some(5),
        "expected exit 5 (REPAIR_APPLIED); stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains(WATCH_ONLY_LINE),
        "repair keyed: expected WATCH_ONLY_LINE; got: {stderr}"
    );
    assert!(
        !stderr.contains(TEMPLATE_LINE),
        "repair keyed: must NOT emit the keyless-template line; got: {stderr}"
    );
}

// ──────────────────────────────────────────────────────────────────────────
// address
// ──────────────────────────────────────────────────────────────────────────

/// Derive the BIP-84 account xpub for the ABANDON mnemonic at `path` on mainnet.
/// Byte-identical copy of `cmd_address.rs::account_xpub` (cannot import across
/// integration-test files without a shared helper crate).
fn account_xpub(path: &str) -> String {
    use bitcoin::Network;
    use bitcoin::bip32::{DerivationPath, Xpriv, Xpub};
    use bitcoin::secp256k1::Secp256k1;
    use std::str::FromStr;
    const ABANDON: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let mn = bip39::Mnemonic::parse(ABANDON).unwrap();
    let seed = mn.to_seed("");
    let secp = Secp256k1::new();
    let master = Xpriv::new_master(Network::Bitcoin, &seed).unwrap();
    let dp = DerivationPath::from_str(path).unwrap();
    let xpriv = master.derive_priv(&secp, &dp).unwrap();
    Xpub::from_priv(&secp, &xpriv).to_string()
}

#[test]
fn address_text_emits_watch_only() {
    let xpub = account_xpub("m/84'/0'/0'");
    let key_arg = format!("@0={xpub}");
    let out = Command::cargo_bin("md")
        .unwrap()
        .args([
            "address",
            "--template",
            "wpkh(@0/<0;1>/*)",
            "--key",
            &key_arg,
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "address exited non-zero; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains(WATCH_ONLY_LINE),
        "address text: expected WATCH_ONLY_LINE on stderr (not template)"
    );
}

#[test]
fn address_json_emits_watch_only() {
    let xpub = account_xpub("m/84'/0'/0'");
    let key_arg = format!("@0={xpub}");
    let out = Command::cargo_bin("md")
        .unwrap()
        .args([
            "address",
            "--json",
            "--template",
            "wpkh(@0/<0;1>/*)",
            "--key",
            &key_arg,
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "address --json exited non-zero; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains(WATCH_ONLY_LINE),
        "address json: expected WATCH_ONLY_LINE on stderr (not template)"
    );
}

// ──────────────────────────────────────────────────────────────────────────
// compile (feature-gated)
// ──────────────────────────────────────────────────────────────────────────

#[cfg(feature = "cli-compiler")]
#[test]
fn compile_emits_template() {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["compile", "pk(@0)", "--context", "segwitv0"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "compile exited non-zero; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains(TEMPLATE_LINE),
        "compile text: expected TEMPLATE_LINE on stderr"
    );
}

#[cfg(feature = "cli-compiler")]
#[test]
fn compile_json_emits_template() {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["compile", "--json", "pk(@0)", "--context", "segwitv0"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "compile --json exited non-zero; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains(TEMPLATE_LINE),
        "compile json: expected TEMPLATE_LINE on stderr"
    );
}

/// Error path: input exceeds BCH correction capacity → repair exits non-zero, no advisory.
/// The load-bearing assertion is `assert_no_advisory`; exit 2 is the observed reliable code
/// for "exceeds the BCH correction capacity of t=4 substitution errors; uncorrectable".
#[test]
fn repair_error_path_emits_no_advisory() {
    // Corrupt 10+ symbols of MD1_FIXTURE (same 24-char length, valid codex32
    // alphabet) — well beyond BCH capacity (t=4 correction), so repair
    // exits 2 ("exceeds the BCH correction capacity of t=4 ...") with no advisory.
    let irreparably_corrupt = "md1zqzqqzqqzztzhzzzznzzz";
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["repair", irreparably_corrupt])
        .output()
        .expect("invoke md repair");
    assert_eq!(
        out.status.code(),
        Some(2),
        "expected exit 2 (irrecoverable); stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_no_advisory(&String::from_utf8(out.stderr).unwrap());
}

// ──────────────────────────────────────────────────────────────────────────
// inert subcommands — must NOT emit any advisory line
// ──────────────────────────────────────────────────────────────────────────

fn assert_no_advisory(stderr: &str) {
    for line in [PRIVATE_KEY_LINE, WATCH_ONLY_LINE, TEMPLATE_LINE] {
        assert!(
            !stderr.contains(line),
            "inert command emitted an advisory: {stderr}"
        );
    }
}

#[test]
fn verify_emits_no_advisory() {
    // verify a known md1 against its template — exits 0 (OK), inert output.
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["verify", MD1_FIXTURE, "--template", "wpkh(@0/<0;1>/*)"])
        .output()
        .unwrap();
    assert_no_advisory(&String::from_utf8(out.stderr).unwrap());
}

#[test]
fn vectors_emits_no_advisory() {
    // `md vectors` regenerates test-vector files; pass a tempdir to avoid cwd pollution.
    let dir = tempfile::tempdir().unwrap();
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["vectors", "--out", dir.path().to_str().unwrap()])
        .output()
        .unwrap();
    assert_no_advisory(&String::from_utf8(out.stderr).unwrap());
}

#[cfg(feature = "json")]
#[test]
fn gui_schema_emits_no_advisory() {
    let out = Command::cargo_bin("md")
        .unwrap()
        .arg("gui-schema")
        .output()
        .unwrap();
    assert_no_advisory(&String::from_utf8(out.stderr).unwrap());
}
