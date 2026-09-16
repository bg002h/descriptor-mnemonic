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

/// F-595: `md decode` prints the template on STDOUT and the origins as a
/// `note:` on STDERR, so the pasteable half is missing exactly what the next
/// verb needs. `md address --template "$(md decode …)"` failed with
/// "non-canonical wrapper requires explicit origin for @0". The restoring
/// operator has plates, not the original policy file -- that is the point of
/// the backup.
///
/// The note now names the flag. This test RUNS what it prescribes rather than
/// matching its text (the F-581 lesson from the same session).
///
/// MUTATION: delete the note -> the first assertion fails. MUTATION: emit it
/// for divergent per-@N origins too -> `--path` takes a single PATH and could
/// not carry them, which is the very defect being fixed.
#[test]
fn decodes_note_names_a_flag_that_actually_derives() {
    let minted = Command::cargo_bin("md")
        .unwrap()
        .args([
            "encode",
            "wsh(and_v(v:sha256(2d711642b726b04401627ca9fbac32f5c8530fb1903cc4db02258717921a4881),pk(@0/<0;1>/*)))",
            "--path", "bip84", "--group-size", "0",
        ])
        .output().unwrap();
    let md1 = String::from_utf8_lossy(&minted.stdout)
        .lines()
        .find(|l| l.starts_with("md1"))
        .expect("no md1")
        .to_string();

    let dec = Command::cargo_bin("md")
        .unwrap()
        .args(["decode", &md1])
        .output()
        .unwrap();
    let tmpl = String::from_utf8_lossy(&dec.stdout).trim().to_string();
    let note = String::from_utf8_lossy(&dec.stderr).to_string();
    assert!(
        note.contains("does NOT carry the origin") && note.contains("--path"),
        "decode does not tell the operator the template is missing the origin:\n{note}"
    );

    // Pull the path OUT OF THE NOTE and use it, so the test breaks if the note
    // ever names a path the card does not carry.
    let path = note
        .lines()
        .find_map(|l| l.split("--path ").nth(1))
        .expect("the note names no --path value")
        .trim()
        .to_string();

    Command::cargo_bin("md")
        .unwrap()
        .args(["address", "--template", &tmpl])
        .args(["--key", "@0=xpub6EddmgK6uMbst7251zES359MJzfz3o2wTJWeefTzSiPJf1FUg6easA2Uk3jUbztBffWS4Dg2oWjhomvU2jJKr4EZukDfxVxb4VJva3jXGAK"])
        .args(["--path", &path, "--count", "1"])
        .assert()
        .success()
        .stdout(predicates::str::contains("bc1q"));
}

/// F-610: `md encode --experimental` shouts "THE PLATE IS BEARER ACCESS" at the
/// person who MINTS. `md decode` is the verb the RESTORER runs — possibly a
/// different person, years later — and it said nothing about a spend path that
/// needs no signature. `cmd/encode.rs` names the gap itself: "the card itself
/// carries no record that a flag was used to create it — the operator's memory
/// and this line are the only trace."
///
/// The key-less path IS visible in the template, to a reader who can read
/// miniscript. The decode side is where the audience least able to do that is
/// standing.
///
/// THE DISPOSITION IS THE TRICK, and the first version of this got it wrong:
/// `Disposition::Warn` is the READ-side disposition and returns Ok for a
/// key-less path, so the check compiled, installed, and printed nothing. The
/// question is "would a MINTING verb refuse this?", so it is asked with
/// `Refuse`.
///
/// MUTATION: switch the predicate back to `Warn` -> the warning silently stops
/// firing and this test fails. That is the exact bug this test exists for.
#[test]
fn decoding_a_bearer_card_warns_the_restorer() {
    const H: &str = "2d711642b726b04401627ca9fbac32f5c8530fb1903cc4db02258717921a4881";

    // A policy with a genuinely key-less arm: refused without --experimental.
    let minted = Command::cargo_bin("md")
        .unwrap()
        .args([
            "encode",
            &format!("wsh(or_d(pk(@0/<0;1>/*),and_v(v:sha256({H}),after(100))))"),
            "--path",
            "bip84",
            "--experimental",
            "--group-size",
            "0",
        ])
        .output()
        .unwrap();
    assert!(minted.status.success(), "mint failed");
    let md1 = String::from_utf8_lossy(&minted.stdout)
        .lines()
        .find(|l| l.starts_with("md1"))
        .expect("no md1")
        .to_string();

    let dec = Command::cargo_bin("md")
        .unwrap()
        .args(["decode", &md1])
        .output()
        .unwrap();
    assert!(dec.status.success(), "a bearer card must still decode");
    let err = String::from_utf8_lossy(&dec.stderr);
    assert!(
        err.contains("needs NO KEY") && err.contains("BEARER ACCESS"),
        "the restorer is told nothing about the key-less path:\n{err}"
    );

    // Control: an ordinary multisig card must NOT carry the warning, or it is
    // noise and will be ignored on the card that matters.
    let plain = Command::cargo_bin("md")
        .unwrap()
        .args([
            "encode",
            "wsh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))",
            "--path",
            "bip48",
            "--group-size",
            "0",
        ])
        .output()
        .unwrap();
    let plain_md1 = String::from_utf8_lossy(&plain.stdout)
        .lines()
        .find(|l| l.starts_with("md1"))
        .expect("no md1")
        .to_string();
    let plain_dec = Command::cargo_bin("md")
        .unwrap()
        .args(["decode", &plain_md1])
        .output()
        .unwrap();
    assert!(
        !String::from_utf8_lossy(&plain_dec.stderr).contains("NO KEY"),
        "an ordinary multisig card is called bearer access"
    );
}
