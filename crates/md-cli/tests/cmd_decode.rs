#![allow(missing_docs)]

use assert_cmd::Command;
use std::process::Command as StdCommand;

fn encode(template: &str) -> String {
    let out = StdCommand::new(assert_cmd::cargo::cargo_bin("md"))
        .args(["encode", template])
        .output()
        .unwrap();
    let s = String::from_utf8(out.stdout).unwrap();
    s.lines().next().unwrap().to_string()
}

/// Insert a comma every 5 chars to simulate a grouped/transcribed card.
/// Comma is the SPEC §3.2 separator md-codec's codex32 layer does NOT already
/// tolerate (it strips whitespace/hyphen via D11), so this genuinely exercises
/// the md-cli intake strip (`strip_md1_inputs`).
fn group5(s: &str) -> String {
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && i % 5 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[test]
fn decode_accepts_grouped_input() {
    // mstring display-grouping (SPEC §3.2): a separator-bearing card re-ingests.
    let template = "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))";
    let phrase = encode(template);
    let grouped = group5(&phrase);
    Command::cargo_bin("md")
        .unwrap()
        .args(["decode", &grouped])
        .assert()
        .success()
        .stdout(predicates::str::contains(template));
}

#[test]
fn decode_round_trips_to_template() {
    let template = "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))";
    let phrase = encode(template);
    let mut cmd = Command::cargo_bin("md").unwrap();
    cmd.args(["decode", &phrase])
        .assert()
        .success()
        .stdout(predicates::str::contains(template));
}

/// Abandon-mnemonic tpub at m/84'/1'/0' (BIP 84 testnet account, depth 3).
const TPUB_FIXTURE: &str = "tpubDC8msFGeGuwnKG9Upg7DM2b4DaRqg3CUZa5g8v2SRQ6K4NSkxUgd7HsL2XVWbVm39yBA4LAxysQAm397zwQSQoQgewGiYZqrA9DsP4zbQ1M";

/// Collect the `md1...` lines from an `md encode --force-chunked` run.
fn encode_chunked(extra_args: &[&str]) -> Vec<String> {
    let mut args = vec!["encode", "--force-chunked"];
    args.extend_from_slice(extra_args);
    let out = StdCommand::new(assert_cmd::cargo::cargo_bin("md"))
        .args(&args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "encode --force-chunked failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .filter(|l| l.starts_with("md1"))
        .map(String::from)
        .collect()
}

/// F-A2: `md decode` must read a chunked single-string card. Before the fix
/// this errored `wire-format version mismatch: got 9`.
#[test]
fn decode_reads_force_chunked_single_string() {
    let template = "wpkh(@0/<0;1>/*)";
    let chunks = encode_chunked(&[template]);
    assert_eq!(
        chunks.len(),
        1,
        "fixture must be a single chunk: {chunks:?}"
    );
    Command::cargo_bin("md")
        .unwrap()
        .args(["decode", &chunks[0]])
        .assert()
        .success()
        .stdout(predicates::str::contains(template));
}

/// F-A2: a genuine multi-chunk set (keyed wallet-policy exceeds 320 bits)
/// still round-trips via multi-arg `md decode`.
#[test]
fn decode_reads_genuine_multi_chunk() {
    let key_arg = format!("@0={TPUB_FIXTURE}");
    let chunks = encode_chunked(&[
        "--network",
        "testnet",
        "--key",
        &key_arg,
        "wpkh(@0/<0;1>/*)",
    ]);
    assert!(chunks.len() >= 2, "expected >=2 chunks, got {chunks:?}");
    let mut args = vec!["decode".to_string()];
    args.extend(chunks.iter().cloned());
    Command::cargo_bin("md")
        .unwrap()
        .args(&args)
        .assert()
        .success()
        .stdout(predicates::str::contains("wpkh(@0/<0;1>/*)"));
}

/// F-A1: an origin-elided `sh(wpkh(...))` card (no `--path`) now round-trips
/// through `md decode` (previously rejected: non-canonical wrapper requires
/// explicit origin for @0).
#[test]
fn sh_wpkh_elided_round_trips_via_cli() {
    let template = "sh(wpkh(@0/<0;1>/*))";
    let phrase = encode(template);
    assert!(phrase.starts_with("md1"));
    Command::cargo_bin("md")
        .unwrap()
        .args(["decode", &phrase])
        .assert()
        .success()
        .stdout(predicates::str::contains(template));
}

#[cfg(feature = "json")]
#[test]
fn decode_json_emits_schema_and_descriptor() {
    let phrase = encode("wpkh(@0/<0;1>/*)");
    Command::cargo_bin("md")
        .unwrap()
        .args(["decode", &phrase, "--json"])
        .assert()
        .success()
        .stdout(predicates::str::contains("\"schema\": \"md-cli/1\""))
        .stdout(predicates::str::contains("\"descriptor\":"));
}
