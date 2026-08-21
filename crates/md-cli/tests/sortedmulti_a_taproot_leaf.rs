#![allow(missing_docs)]
//! `sortedmulti_a` at a taproot leaf — R5, the first work item of Stage 3.
//!
//! THE GAP THIS CLOSES. `md encode` accepted `tr(@0,sortedmulti_a(...))` (that
//! half started working when PR #915 was ported) while `md address` refused it,
//! so the CLI ADMITTED A CARD IT COULD NOT VERIFY. With "derived addresses +
//! wallet id" as the project's proof model, an engraved card whose address
//! cannot be derived is a backup nobody can check with our own tools.
//!
//! THE POSITION IS THE WHOLE RULE. BIP-386 places `sortedmulti_a()` in BIP-387's
//! category — a sibling of the Miniscript fragments, not a member — so it is
//! admissible as a taproot LEAF and forbidden as a sub-expression. Both halves
//! are asserted here; a fix that only added the capability would have quietly
//! widened admission, and rust-miniscript itself would now accept the nested
//! form (its parser has no depth guard for these).

use assert_cmd::Command;
use bitcoin::Network;
use bitcoin::bip32::{DerivationPath, Xpriv, Xpub};
use bitcoin::secp256k1::Secp256k1;
use std::str::FromStr;

const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const PATH: &str = "48'/0'/0'/2'";

fn xpub_at(path: &str) -> Xpub {
    let mn = bip39::Mnemonic::parse(ABANDON).unwrap();
    let seed = mn.to_seed("");
    let secp = Secp256k1::new();
    let master = Xpriv::new_master(Network::Bitcoin, &seed).unwrap();
    let dp = DerivationPath::from_str(path).unwrap();
    Xpub::from_priv(&secp, &master.derive_priv(&secp, &dp).unwrap())
}

fn md() -> Command {
    Command::cargo_bin("md").unwrap()
}

/// First receive address for a template, or None if the CLI refused.
fn address_of(template: &str) -> Option<String> {
    let k0 = format!("@0={}", xpub_at("48'/0'/0'/2'"));
    let k1 = format!("@1={}", xpub_at("48'/0'/1'/2'"));
    let out = md()
        .args([
            "address",
            "--template",
            template,
            "--key",
            &k0,
            "--key",
            &k1,
            "--path",
            PATH,
            "--count",
            "1",
        ])
        .output()
        .unwrap();
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find(|l| l.starts_with("bc1p"))
        .map(str::to_string)
}

/// THE SEMANTIC PROOF, and the reason this test is not just "it returns an
/// address": `sortedmulti_a` SORTS its keys, so the same two keys in either
/// order must give the SAME address — while `multi_a` must not.
///
/// The third assertion is what rules out a sort that merely reverses or
/// no-ops: the sorted form must equal `multi_a` with the keys already in their
/// sorted order, which pins WHICH order the sort produces.
#[test]
fn sortedmulti_a_is_order_invariant_and_multi_a_is_not() {
    let s01 = address_of("tr(@0/<0;1>/*,sortedmulti_a(2,@0/<0;1>/*,@1/<0;1>/*))")
        .expect("sortedmulti_a at a taproot leaf must derive");
    let s10 = address_of("tr(@0/<0;1>/*,sortedmulti_a(2,@1/<0;1>/*,@0/<0;1>/*))")
        .expect("sortedmulti_a at a taproot leaf must derive");
    let m01 =
        address_of("tr(@0/<0;1>/*,multi_a(2,@0/<0;1>/*,@1/<0;1>/*))").expect("multi_a must derive");
    let m10 =
        address_of("tr(@0/<0;1>/*,multi_a(2,@1/<0;1>/*,@0/<0;1>/*))").expect("multi_a must derive");

    assert_eq!(
        s01, s10,
        "sortedmulti_a is NOT order-invariant — the keys were not sorted"
    );
    assert_ne!(
        m01, m10,
        "multi_a became order-invariant — it must preserve the written order, \
         or the two fragments are indistinguishable"
    );
    assert!(
        s01 == m01 || s01 == m10,
        "the sorted address matches NEITHER multi_a ordering, so the sort \
         produced an order that is not one of the permutations:\n  sorted: {s01}\n  \
         multi_a(0,1): {m01}\n  multi_a(1,0): {m10}"
    );
}

/// The other half of the rule: nested is still refused, by the standard.
#[test]
fn sortedmulti_a_nested_in_a_fragment_is_refused() {
    let k0 = format!("@0={}", xpub_at("48'/0'/0'/2'"));
    let k1 = format!("@1={}", xpub_at("48'/0'/1'/2'"));
    let out = md()
        .args([
            "address",
            "--template",
            "tr(@0/<0;1>/*,and_v(v:sortedmulti_a(2,@0/<0;1>/*,@1/<0;1>/*),older(144)))",
            "--key",
            &k0,
            "--key",
            &k1,
            "--path",
            PATH,
            "--count",
            "1",
        ])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "a NESTED sortedmulti_a derived an address; admission was widened"
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("valid only as a taproot leaf"),
        "the refusal does not name the positional rule: {err}"
    );
}

/// Encode and derive must agree about what is admissible. The gap R5 closed was
/// exactly a disagreement between them.
#[test]
fn encode_and_derive_admit_the_same_shape() {
    let k0 = format!("@0={}", xpub_at("48'/0'/0'/2'"));
    let k1 = format!("@1={}", xpub_at("48'/0'/1'/2'"));
    let tmpl = "tr(@0/<0;1>/*,sortedmulti_a(2,@0/<0;1>/*,@1/<0;1>/*))";

    let enc = md()
        .args([
            "encode",
            tmpl,
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
        "encode refused a shape derive accepts: {}",
        String::from_utf8_lossy(&enc.stderr)
    );
    let chunks: Vec<String> = String::from_utf8_lossy(&enc.stdout)
        .lines()
        .filter(|l| l.starts_with("md1"))
        .map(|l| l.replace(' ', ""))
        .collect();
    assert!(!chunks.is_empty(), "encode produced no card");

    // And the CARD derives the same address as the template did — the wire
    // round-trip, not just the in-memory descriptor.
    let mut cmd = md();
    cmd.arg("address");
    for c in &chunks {
        cmd.arg(c);
    }
    let out = cmd.args(["--count", "1"]).output().unwrap();
    assert!(
        out.status.success(),
        "the encoded card could not be derived from: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let from_card = String::from_utf8_lossy(&out.stdout)
        .lines()
        .find(|l| l.starts_with("bc1p"))
        .map(str::to_string)
        .expect("no address from the card");
    assert_eq!(
        Some(from_card),
        address_of(tmpl),
        "the card and the template derive DIFFERENT addresses"
    );
}
