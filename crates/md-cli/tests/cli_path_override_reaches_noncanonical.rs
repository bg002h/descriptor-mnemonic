#![allow(missing_docs)]
//! `--path` on `md address` and `md verify` — R4 of the arbitrary-`tr()`/`wsh()`
//! cycle.
//!
//! THE GAP THIS CLOSES. A non-canonical wrapper (taproot with a script tree, and
//! the miniscript wrappers generally) has no canonical default origin, so the
//! codec refuses it:
//!
//! ```text
//! md: codec error: non-canonical wrapper requires explicit origin for @0,
//!     but none provided
//! ```
//!
//! `md encode` could supply one with `--path`. `md address` and `md verify`
//! could not — so exactly the shapes this feature exists to support could be
//! ENCODED but never had an address derived, or a backup verified, from their
//! template. The flag is the same one, applied through one shared helper
//! (`parse::path::apply_path_override`) rather than a third copy of the rule.

use assert_cmd::Command;
use bitcoin::Network;
use bitcoin::bip32::{DerivationPath, Xpriv, Xpub};
use bitcoin::secp256k1::Secp256k1;
use std::str::FromStr;

const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

/// A taproot template with a script leaf: the shape that requires an explicit
/// origin, and the shape the whole cycle is about.
const TR_WITH_LEAF: &str = "tr(@0/<0;1>/*,pk(@1/<0;1>/*))";
const PATH: &str = "48'/0'/0'/2'";

fn xpub_at(path: &str) -> Xpub {
    let mn = bip39::Mnemonic::parse(ABANDON).unwrap();
    let seed = mn.to_seed("");
    let secp = Secp256k1::new();
    let master = Xpriv::new_master(Network::Bitcoin, &seed).unwrap();
    let dp = DerivationPath::from_str(path).unwrap();
    Xpub::from_priv(&secp, &master.derive_priv(&secp, &dp).unwrap())
}

fn keys() -> (String, String) {
    (
        format!("@0={}", xpub_at("48'/0'/0'/2'")),
        format!("@1={}", xpub_at("48'/0'/1'/2'")),
    )
}

fn md() -> Command {
    Command::cargo_bin("md").unwrap()
}

/// Without `--path` the refusal must SURVIVE. The flag exists to make a shape
/// reachable, not to make the codec lax: a template that genuinely has no
/// canonical origin and was given none is still an error.
#[test]
fn address_without_path_still_refuses_a_noncanonical_wrapper() {
    let (k0, k1) = keys();
    let out = md()
        .args([
            "address",
            "--template",
            TR_WITH_LEAF,
            "--key",
            &k0,
            "--key",
            &k1,
            "--count",
            "1",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success(), "expected a refusal, got success");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("non-canonical wrapper requires explicit origin"),
        "refusal did not name the missing origin: {err}"
    );
}

/// With `--path`, the shape derives — and the addresses are taproot.
#[test]
fn address_with_path_derives_the_noncanonical_wrapper() {
    let (k0, k1) = keys();
    let out = md()
        .args([
            "address",
            "--template",
            TR_WITH_LEAF,
            "--key",
            &k0,
            "--key",
            &k1,
            "--path",
            PATH,
            "--count",
            "2",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let addrs: Vec<&str> = stdout.lines().filter(|l| l.starts_with("bc1p")).collect();
    assert_eq!(addrs.len(), 2, "want 2 taproot addresses, got: {stdout}");
    assert_ne!(addrs[0], addrs[1], "index 0 and 1 derived the same address");
}

/// THE ROUND TRIP, and the reason this test exists rather than just the one
/// above: producing an address is not proving it is the RIGHT address.
///
/// The same template and path are encoded to md1 chunks and the addresses are
/// derived again from the PHRASE — a route through encode, the wire format and
/// decode that shares no code with the `--template` path beyond the codec
/// itself. If `--path` were applied differently on the two sides, these would
/// disagree.
#[test]
fn template_path_and_phrase_path_derive_the_same_addresses() {
    let (k0, k1) = keys();

    let enc = md()
        .args([
            "encode",
            TR_WITH_LEAF,
            "--key",
            &k0,
            "--key",
            &k1,
            "--path",
            PATH,
            "--force-chunked",
        ])
        .output()
        .unwrap();
    assert!(
        enc.status.success(),
        "encode: {}",
        String::from_utf8_lossy(&enc.stderr)
    );
    let chunks: Vec<String> = String::from_utf8_lossy(&enc.stdout)
        .lines()
        .filter(|l| l.starts_with("md1"))
        .map(|l| l.replace(' ', ""))
        .collect();
    assert!(
        chunks.len() > 1,
        "expected a chunked encoding, got {chunks:?}"
    );

    let mut via_phrase = md();
    via_phrase.arg("address");
    for c in &chunks {
        via_phrase.arg(c);
    }
    let phrase_out = via_phrase.args(["--count", "2"]).output().unwrap();
    assert!(
        phrase_out.status.success(),
        "phrase: {}",
        String::from_utf8_lossy(&phrase_out.stderr)
    );

    let tmpl_out = md()
        .args([
            "address",
            "--template",
            TR_WITH_LEAF,
            "--key",
            &k0,
            "--key",
            &k1,
            "--path",
            PATH,
            "--count",
            "2",
        ])
        .output()
        .unwrap();
    assert!(tmpl_out.status.success());

    let addrs = |o: &std::process::Output| -> Vec<String> {
        String::from_utf8_lossy(&o.stdout)
            .lines()
            .filter(|l| l.starts_with("bc1"))
            .map(str::to_string)
            .collect()
    };
    let a = addrs(&phrase_out);
    let b = addrs(&tmpl_out);
    assert_eq!(a.len(), 2, "phrase path produced {a:?}");
    assert_eq!(
        a, b,
        "the two routes disagree:\n  phrase:   {a:?}\n  template: {b:?}"
    );
}

/// `md verify` gets the same flag, and the same two behaviours.
#[test]
fn verify_needs_path_for_a_noncanonical_wrapper_and_passes_with_it() {
    let (k0, k1) = keys();
    let enc = md()
        .args([
            "encode",
            TR_WITH_LEAF,
            "--key",
            &k0,
            "--key",
            &k1,
            "--path",
            PATH,
            "--force-chunked",
        ])
        .output()
        .unwrap();
    assert!(enc.status.success());
    let chunks: Vec<String> = String::from_utf8_lossy(&enc.stdout)
        .lines()
        .filter(|l| l.starts_with("md1"))
        .map(|l| l.replace(' ', ""))
        .collect();

    let run = |path: Option<&str>| -> std::process::Output {
        let mut c = md();
        c.arg("verify");
        for ch in &chunks {
            c.arg(ch);
        }
        c.args(["--template", TR_WITH_LEAF, "--key", &k0, "--key", &k1]);
        if let Some(p) = path {
            c.args(["--path", p]);
        }
        c.output().unwrap()
    };

    let with = run(Some(PATH));
    assert!(
        with.status.success(),
        "verify --path should pass; stderr: {}",
        String::from_utf8_lossy(&with.stderr)
    );

    // Without it the expected template describes a DIFFERENT payload, so the
    // comparison must fail rather than quietly pass on a near-match.
    let without = run(None);
    assert!(
        !without.status.success(),
        "verify without --path passed, so the flag changed nothing: {}",
        String::from_utf8_lossy(&without.stdout)
    );
}
