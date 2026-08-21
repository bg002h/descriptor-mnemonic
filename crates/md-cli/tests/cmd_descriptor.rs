#![allow(missing_docs)]
//! `md descriptor` — the concrete output descriptor.
//!
//! THE GATE IS THE CORPUS. Every keyed conformance vector stores, per chain, the
//! canonical descriptor string the primary produced — checksum and all. This
//! asserts the command reproduces it byte for byte, for every vector and every
//! chain. That is a stronger check than any hand-written expectation: the corpus
//! is what the Go port is measured against, so a command that agreed with a test
//! but not with the corpus would be agreeing with nobody.

use assert_cmd::Command;
use std::path::PathBuf;

fn vectors_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("md-codec/tests/vectors")
}

fn md() -> Command {
    Command::cargo_bin("md").unwrap()
}

/// The md1 chunk strings for a vector, one per line.
fn chunks(name: &str) -> Vec<String> {
    let p = vectors_dir().join(format!("{name}.phrase.txt"));
    std::fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
        .lines()
        .map(str::trim)
        // A chunked vector's phrase file opens with a `chunk-set-id: 0x…`
        // header line. Passing it as a card fails with "string does not start
        // with HRP md1", which reads like a corrupt fixture rather than like a
        // header nobody filtered.
        .filter(|l| l.starts_with("md1"))
        .map(str::to_owned)
        .collect()
}

fn conformance(name: &str) -> serde_json::Value {
    let p = vectors_dir().join(format!("{name}.conformance.json"));
    serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap()
}

fn keyed_vector_names() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(vectors_dir())
        .unwrap()
        .filter_map(|e| {
            let n = e.ok()?.file_name().to_string_lossy().into_owned();
            let n = n.strip_suffix(".conformance.json")?;
            n.starts_with("keyed_").then(|| n.to_owned())
        })
        .collect();
    v.sort();
    v
}

#[test]
fn every_keyed_vector_renders_its_stored_descriptor() {
    let names = keyed_vector_names();
    assert!(
        !names.is_empty(),
        "no keyed vectors found — this gate is checking NOTHING"
    );
    let mut checked = 0;
    for name in &names {
        let rec = conformance(name);
        let chains = rec["chains"].as_object().expect("chains");
        for (chain, body) in chains {
            let want = body["descriptor"].as_str().expect("descriptor");
            let out = md()
                .args(["descriptor", "--chain", chain])
                .args(chunks(name))
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "{name} chain {chain}: md descriptor failed: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            let got = String::from_utf8_lossy(&out.stdout).trim().to_owned();
            assert_eq!(
                got, want,
                "{name} chain {chain}: descriptor differs from the corpus"
            );
            checked += 1;
        }
    }
    assert!(
        checked >= names.len(),
        "checked {checked} descriptors across {} vectors — fewer than one each",
        names.len()
    );
}

/// Multipath is the DEFAULT because it is the form a coordinator wants: one
/// descriptor carrying `<0;1>` rather than two a human has to keep in step.
#[test]
fn the_default_is_multipath_and_differs_from_both_single_paths() {
    let name = "keyed_tr_depth2";
    let run = |extra: &[&str]| {
        let out = md()
            .arg("descriptor")
            .args(extra)
            .args(chunks(name))
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_owned()
    };
    let multi = run(&[]);
    let recv = run(&["--chain", "0"]);
    let chg = run(&["--change"]);

    assert!(
        multi.contains("<0;1>"),
        "the default is not multipath: {multi}"
    );
    assert!(
        !recv.contains("<0;1>"),
        "--chain 0 should collapse the group: {recv}"
    );
    assert_ne!(recv, chg, "receive and change render the same string");
    assert_ne!(multi, recv);

    // A BIP-380 checksum on each, and three DIFFERENT ones — the checksum
    // covers the string, so identical checksums across three different
    // descriptors would mean it is not being computed over what is printed.
    let sums: Vec<&str> = [&multi, &recv, &chg]
        .iter()
        .map(|s| s.rsplit_once('#').expect("no checksum").1)
        .collect();
    assert_eq!(sums.len(), 3);
    assert!(
        sums.iter().all(|s| s.len() == 8),
        "checksums are not 8 chars: {sums:?}"
    );
    assert!(
        sums[0] != sums[1] && sums[1] != sums[2] && sums[0] != sums[2],
        "three different descriptors share a checksum: {sums:?}"
    );
}

/// `--change` is sugar for `--chain 1`, and must not drift from it.
#[test]
fn change_is_exactly_chain_one() {
    let name = "keyed_wsh_thresh";
    let run = |extra: &[&str]| {
        let out = md()
            .arg("descriptor")
            .args(extra)
            .args(chunks(name))
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).trim().to_owned()
    };
    assert_eq!(run(&["--change"]), run(&["--chain", "1"]));
}

/// A KEYLESS template has no concrete form, and the refusal must say so rather
/// than rendering something that looks like a wallet and is not one.
#[test]
fn a_keyless_template_is_refused_and_pointed_at_decode() {
    let out = md()
        .args([
            "descriptor",
            "--template",
            "wsh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))",
        ])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "a keyless template produced a descriptor"
    );
    let err = String::from_utf8_lossy(&out.stderr);
    // clap refuses first (--template requires --key); either refusal is correct,
    // but it must not print a descriptor.
    assert!(
        !String::from_utf8_lossy(&out.stdout).contains("wsh("),
        "a descriptor was printed for a keyless template"
    );
    assert!(!err.is_empty(), "refused with no message at all");
}

/// The same requirement, reached through a CARD rather than through clap — and
/// this is the one that actually exercises the check.
///
/// The card is MINTED HERE rather than taken from a fixture. The obvious
/// candidate, `tr_keyonly`, is a keyless vector on an OLDER wire version, so it
/// fails at decode and never reaches `is_wallet_policy` at all: deleting the
/// refusal entirely left the whole suite green. A test whose subject is never
/// reached is decoration, and a fixture is exactly how that happens quietly.
#[test]
fn a_keyless_card_is_refused_by_the_command_not_by_clap() {
    const TEMPLATE: &str = "tr(@0/<0;1>/*,pk(@1/<0;1>/*))";
    let enc = md()
        .args([
            "encode",
            TEMPLATE,
            "--path",
            "48'/0'/0'/2'",
            "--force-chunked",
        ])
        .output()
        .unwrap();
    assert!(
        enc.status.success(),
        "could not mint the keyless card: {}",
        String::from_utf8_lossy(&enc.stderr)
    );
    let card: Vec<String> = String::from_utf8_lossy(&enc.stdout)
        .lines()
        .filter(|l| l.starts_with("md1"))
        .map(str::to_owned)
        .collect();
    assert!(!card.is_empty(), "encode printed no md1 strings");

    // It must DECODE — otherwise this test is the `tr_keyonly` trap again.
    let dec = md().arg("decode").args(&card).output().unwrap();
    assert!(
        dec.status.success(),
        "the minted card does not decode, so `descriptor` would refuse for the wrong reason: {}",
        String::from_utf8_lossy(&dec.stderr)
    );
    assert!(
        String::from_utf8_lossy(&dec.stdout).contains("@0"),
        "the minted card is not a keyless template"
    );

    let out = md().arg("descriptor").args(&card).output().unwrap();
    assert!(
        !out.status.success(),
        "a keyless card produced a descriptor"
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("TEMPLATE") || err.contains("template"),
        "the refusal does not say the card is a template: {err}"
    );
    assert!(
        err.contains("md decode"),
        "the refusal does not point at the command that CAN show this card: {err}"
    );
    assert!(
        !String::from_utf8_lossy(&out.stdout).contains('#'),
        "a checksummed descriptor was printed for a keyless card"
    );
}
