#![allow(missing_docs)]

use assert_cmd::Command;
use std::process::Command as StdCommand;

fn encode(template: &str) -> String {
    let out = StdCommand::new(assert_cmd::cargo::cargo_bin("md"))
        .args(["encode", template])
        .output()
        .unwrap();
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .to_string()
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
fn verify_accepts_grouped_input() {
    // mstring display-grouping (SPEC §3.2): a separator-bearing card re-ingests.
    let template = "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))";
    let phrase = encode(template);
    let grouped = group5(&phrase);
    Command::cargo_bin("md")
        .unwrap()
        .args(["verify", &grouped, "--template", template])
        .assert()
        .code(0)
        .stdout(predicates::str::contains("OK"));
}

#[test]
fn verify_match_returns_0() {
    let template = "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))";
    let phrase = encode(template);
    Command::cargo_bin("md")
        .unwrap()
        .args(["verify", &phrase, "--template", template])
        .assert()
        .code(0)
        .stdout(predicates::str::contains("OK"));
}

#[test]
fn verify_mismatch_returns_1() {
    let template = "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))";
    let phrase = encode(template);
    let wrong = "wpkh(@0/<0;1>/*)";
    Command::cargo_bin("md")
        .unwrap()
        .args(["verify", &phrase, "--template", wrong])
        .assert()
        .code(1)
        .stderr(predicates::str::contains("MISMATCH"));
}

/// F-582: `md verify` MISMATCHed a CORRECT plate set and named two numbers and
/// no cause. The most common cause is a flag the operator did not repeat:
/// fingerprints are part of the payload, so verifying a plate minted with
/// `--fingerprint` against a template without it mismatches on size.
///
/// The failure mode is the reaction, not the message. An operator who makes
/// verification pass by re-minting WITHOUT `--fingerprint` gets a set that two
/// gates bless and a third had already condemned as unseatable -- so the hint
/// points at the template side and explicitly says not to re-mint.
///
/// MUTATION: drop the hint -> the first assertion fails. MUTATION: emit it
/// unconditionally -> the WITH-fingerprint control still passes (it exits OK
/// and prints no mismatch), so the size-direction assertion is what pins it.
#[test]
fn a_mismatch_caused_by_missing_fingerprints_says_so() {
    const TMPL: &str = "wsh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))";
    const K: [&str; 3] = [
        "xpub6EddmgK6uMbst7251zES359MJzfz3o2wTJWeefTzSiPJf1FUg6easA2Uk3jUbztBffWS4Dg2oWjhomvU2jJKr4EZukDfxVxb4VJva3jXGAK",
        "xpub6EPimu5ztRjy6dfECvb9AQNAadNHN8qumZYQhvdPWh9U9xJ9XMi328aXAFMGHwc11wZWrmdwsHUWJn5in6BHywGrbJx5ZLUWe8xhNPRf5kL",
        "xpub6EVA2mZiCBtGYbGM5eEs42k8tso4QekUog2t8U14UDyjUYpKByLYcXB8t3yVAvapQymRSK2MDM84cspZa5udQEEUSVGo28RJJt47eeC846C",
    ];
    const FP: [&str; 3] = ["@0=aabbccdd", "@1=11223344", "@2=55667788"];

    let dir = tempfile::tempdir().unwrap();
    let plate = dir.path().join("fp.md1");

    // Mint WITH fingerprints.
    let minted = Command::cargo_bin("md")
        .unwrap()
        .args(["encode", TMPL, "--group-size", "0"])
        .args(["--key", &format!("@0={}", K[0])])
        .args(["--key", &format!("@1={}", K[1])])
        .args(["--key", &format!("@2={}", K[2])])
        .args(["--fingerprint", FP[0]])
        .args(["--fingerprint", FP[1]])
        .args(["--fingerprint", FP[2]])
        .output()
        .unwrap();
    assert!(minted.status.success(), "mint failed");
    let md1: String = String::from_utf8_lossy(&minted.stdout)
        .lines()
        .filter(|l| l.starts_with("md1"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!md1.is_empty(), "no md1 minted");
    std::fs::write(&plate, &md1).unwrap();

    // Verify WITHOUT them: mismatches, and must explain why.
    let bare = Command::cargo_bin("md")
        .unwrap()
        .args(["verify", "--template", TMPL])
        .args(["--key", &format!("@0={}", K[0])])
        .args(["--key", &format!("@1={}", K[1])])
        .args(["--key", &format!("@2={}", K[2])])
        .args(["--in", &plate.display().to_string()])
        .output()
        .unwrap();
    assert!(
        !bare.status.success(),
        "a fingerprinted plate verified without them"
    );
    let err = String::from_utf8_lossy(&bare.stderr);
    assert!(
        err.contains("--fingerprint"),
        "the mismatch never names the flag that explains it:\n{err}"
    );
    assert!(
        err.contains("Do not re-mint"),
        "the mismatch does not warn against the dangerous fix:\n{err}"
    );

    // Control: with the same fingerprints it verifies, so the plate was right
    // all along and the hint described a real cause.
    Command::cargo_bin("md")
        .unwrap()
        .args(["verify", "--template", TMPL])
        .args(["--key", &format!("@0={}", K[0])])
        .args(["--key", &format!("@1={}", K[1])])
        .args(["--key", &format!("@2={}", K[2])])
        .args(["--fingerprint", FP[0]])
        .args(["--fingerprint", FP[1]])
        .args(["--fingerprint", FP[2]])
        .args(["--in", &plate.display().to_string()])
        .assert()
        .success();
}

/// F-606: when only the CONTENT differs, the MISMATCH message offered two
/// IDENTICAL numbers as its evidence —
/// `expected 1447-bit payload, got 1447-bit (181 vs 181 bytes)` — at exactly
/// the moment the operator is standing over a plate and a descriptor wanting to
/// know which field drifted. The verdict was right; the evidence was vacuous.
///
/// The first differing byte is the cheapest true thing available and localises
/// the drift without decoding either side again.
///
/// MUTATION: delete the equal-size branch -> the offset assertion fails while
/// the MISMATCH verdict still holds, which is the point: the verdict was never
/// what was wrong.
#[test]
fn a_same_size_mismatch_names_the_first_differing_byte() {
    const H1: &str = "2d711642b726b04401627ca9fbac32f5c8530fb1903cc4db02258717921a4881";
    const H2: &str = "252f10c83610ebca1a059c0bae8255eba2f95be4d1d7bcfa89d7248a82d9f111";

    let dir = tempfile::tempdir().unwrap();
    let plate = dir.path().join("v1.md1");
    let minted = Command::cargo_bin("md")
        .unwrap()
        .args([
            "encode",
            &format!("wsh(and_v(v:sha256({H1}),pk(@0/<0;1>/*)))"),
            "--path",
            "bip84",
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
    std::fs::write(&plate, &md1).unwrap();

    // Same policy shape, ONE digest changed: identical payload size.
    let out = Command::cargo_bin("md")
        .unwrap()
        .args([
            "verify",
            "--template",
            &format!("wsh(and_v(v:sha256({H2}),pk(@0/<0;1>/*)))"),
            "--path",
            "bip84",
            "--in",
            &plate.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(!out.status.success(), "a changed digest must MISMATCH");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("Same SIZE, different CONTENT"),
        "the message still offers two identical numbers as evidence:\n{err}"
    );
    assert!(
        err.contains("First byte that differs: offset"),
        "the message does not localise the drift:\n{err}"
    );

    // Control: the card verified against its OWN template is silent OK, so the
    // branch cannot be firing unconditionally.
    Command::cargo_bin("md")
        .unwrap()
        .args([
            "verify",
            "--template",
            &format!("wsh(and_v(v:sha256({H1}),pk(@0/<0;1>/*)))"),
            "--path",
            "bip84",
            "--in",
            &plate.display().to_string(),
        ])
        .assert()
        .success();
}

/// F-639 (F-449 stage 2): `md verify` compares two serialisations and mints
/// nothing, so mint-time admission policy must not decide whether an engraved
/// card can be CHECKED. MEASURED at 25acb33c: this card decodes (exit 0) but
/// `md verify` against its OWN template refused with the §6 mint error
/// ("wire kind 1 ... with a sortedmulti_a leaf is refused", exit 1).
///
/// The card is a kind-1 `tr()` with a `sortedmulti_a` leaf -- SPEC §6 row 1,
/// refused at mint -- serialised by `md_codec::encode_payload_unadmitted`
/// (re-derive it with md-codec's ignored test `print_the_kind_1_card` in
/// `tests/mint_policy_does_not_reach_decode.rs`). A card like this exists
/// wherever a writer did not apply the rule: another implementation, a Go
/// port, a hand-crafted plate.
const KIND1_SORTEDMULTI_A_CARD: &str = "md1gzfdsssjuqqcreyygvauszq6hnnx9up";

#[test]
fn verify_checks_a_mint_refused_card_instead_of_refusing_it() {
    let tpl = |k: u8| {
        format!(
            "tr(UNSPENDABLE(liana),sortedmulti_a({k},@0/48'/0'/0'/3'/<0;1>/*,\
             @1/48'/0'/0'/3'/<0;1>/*,@2/48'/0'/0'/3'/<0;1>/*))"
        )
    };
    let run = |t: &str| {
        let out = StdCommand::new(assert_cmd::cargo::cargo_bin("md"))
            .args(["verify", KIND1_SORTEDMULTI_A_CARD, "--template", t])
            .output()
            .unwrap();
        (
            out.status.code().unwrap(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    };
    // Its own template: a MATCH, not a mint refusal.
    let (code, err) = run(&tpl(2));
    assert_eq!(code, 0, "verify must compare, not apply mint policy: {err}");
    assert!(!err.contains("is refused"), "{err}");
    // A different template: still a MISMATCH -- verify still verifies.
    let (code, err) = run(&tpl(1));
    assert_ne!(code, 0, "a different template must not verify: {err}");
    assert!(
        !err.contains("is refused"),
        "a mismatch, not a mint refusal: {err}"
    );
}
