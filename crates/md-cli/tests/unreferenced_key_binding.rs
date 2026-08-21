#![allow(missing_docs)]
//! F-213: a `--key`/`--fingerprint` bound to a placeholder the template never
//! uses must be refused AT ENCODE TIME.
//!
//! Before this, `md encode` accepted `--key @2=<xpub>` on a two-placeholder
//! template and minted a card whose Pubkeys TLV carried three entries against
//! `n = 2`. The card looked ordinary; the SeedHammer fork refused to read it.
//!
//! The failure that matters is the ORDER of events: mistake, plausible card,
//! engraving, and only then discovery — with the irreversible step in the
//! middle. A stray `@2` for `@1` is an ordinary typo.

use assert_cmd::Command;
use bitcoin::Network;
use bitcoin::bip32::{DerivationPath, Xpriv, Xpub};
use bitcoin::secp256k1::Secp256k1;
use std::str::FromStr;

const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const TWO_KEY: &str = "wsh(or_b(pk(@0/<0;1>/*),s:pk(@1/<0;1>/*)))";
const PATH: &str = "48'/0'/0'/2'";

fn xpub_at(path: &str) -> Xpub {
    let mn = bip39::Mnemonic::parse(ABANDON).unwrap();
    let seed = mn.to_seed("");
    let secp = Secp256k1::new();
    let master = Xpriv::new_master(Network::Bitcoin, &seed).unwrap();
    Xpub::from_priv(
        &secp,
        &master
            .derive_priv(&secp, &DerivationPath::from_str(path).unwrap())
            .unwrap(),
    )
}

fn md() -> Command {
    Command::cargo_bin("md").unwrap()
}

#[test]
fn a_key_for_an_unreferenced_placeholder_is_refused() {
    let k0 = format!("@0={}", xpub_at("48'/0'/0'/2'"));
    let k1 = format!("@1={}", xpub_at("48'/0'/1'/2'"));
    let k2 = format!("@2={}", xpub_at("48'/0'/2'/2'"));
    let out = md()
        .args([
            "encode",
            TWO_KEY,
            "--key",
            &k0,
            "--key",
            &k1,
            "--key",
            &k2,
            "--path",
            PATH,
            "--force-chunked",
        ])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "a card was minted with a key bound to nothing:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let err = String::from_utf8_lossy(&out.stderr);
    // The message must name the offending slot AND the ones that exist —
    // "invalid" alone leaves the operator guessing which of their keys is wrong.
    assert!(
        err.contains("@2"),
        "the refusal does not name the stray slot: {err}"
    );
    assert!(
        err.contains("@0") && err.contains("@1"),
        "the refusal does not say which placeholders the template uses: {err}"
    );
}

#[test]
fn a_fingerprint_for_an_unreferenced_placeholder_is_refused() {
    let k0 = format!("@0={}", xpub_at("48'/0'/0'/2'"));
    let k1 = format!("@1={}", xpub_at("48'/0'/1'/2'"));
    let out = md()
        .args([
            "encode",
            TWO_KEY,
            "--key",
            &k0,
            "--key",
            &k1,
            "--fingerprint",
            "@5=73c5da0a",
            "--path",
            PATH,
            "--force-chunked",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success(), "a stray --fingerprint was accepted");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("@5"),
        "the refusal does not name the stray fingerprint slot"
    );
}

/// The narrowing must not catch legitimate use. A template that references every
/// placeholder it is given still encodes.
#[test]
fn the_correct_binding_still_encodes() {
    let k0 = format!("@0={}", xpub_at("48'/0'/0'/2'"));
    let k1 = format!("@1={}", xpub_at("48'/0'/1'/2'"));
    let out = md()
        .args([
            "encode",
            TWO_KEY,
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
        out.status.success(),
        "a correct invocation was refused: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let chunks = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| l.starts_with("md1"))
        .count();
    assert!(chunks > 0, "no card emitted");
}

/// A template may SKIP an index, so the check is on the placeholder set and not
/// on a count. `@0` and `@2` present means `@1` is the stray one.
#[test]
fn the_check_is_the_placeholder_set_not_a_count() {
    let k0 = format!("@0={}", xpub_at("48'/0'/0'/2'"));
    let k1 = format!("@1={}", xpub_at("48'/0'/1'/2'"));
    // Two placeholders, so a count-based check would accept two keys blindly.
    let out = md()
        .args([
            "encode",
            "wsh(or_b(pk(@0/<0;1>/*),s:pk(@3/<0;1>/*)))",
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
        !out.status.success(),
        "two keys were accepted for @0/@3 by counting rather than matching"
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("@1"),
        "the refusal should name @1 as the unreferenced slot"
    );
}
