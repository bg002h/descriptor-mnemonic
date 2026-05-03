#![allow(missing_docs)]

use assert_cmd::Command;
use bitcoin::Network;
use bitcoin::bip32::{DerivationPath, Xpriv, Xpub};
use bitcoin::secp256k1::Secp256k1;
use std::str::FromStr;

const ABANDON: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

fn account_xpub(path: &str, network: Network) -> Xpub {
    let mn = bip39::Mnemonic::parse(ABANDON).unwrap();
    let seed = mn.to_seed("");
    let secp = Secp256k1::new();
    let master = Xpriv::new_master(network, &seed).unwrap();
    let dp = DerivationPath::from_str(path).unwrap();
    let xpriv = master.derive_priv(&secp, &dp).unwrap();
    Xpub::from_priv(&secp, &xpriv)
}

fn encode_template_with_key(template: &str, key_arg: &str) -> String {
    let out = Command::cargo_bin("md").unwrap()
        .args(["encode", template, "--key", key_arg])
        .output().unwrap();
    assert!(out.status.success(), "encode failed: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout).unwrap().lines().next().unwrap().to_string()
}

#[test]
fn address_template_mode_emits_bip84_receive_0() {
    // BIP 84 vector: abandon mnemonic at m/84'/0'/0'/0/0 → bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu
    let xpub = account_xpub("m/84'/0'/0'", Network::Bitcoin);
    let key_arg = format!("@0={xpub}");
    Command::cargo_bin("md").unwrap()
        .args(["address", "--template", "wpkh(@0/<0;1>/*)", "--key", &key_arg])
        .assert().success()
        .stdout(predicates::str::contains("bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu"));
}

#[test]
fn address_phrase_mode_round_trips_through_encode() {
    let xpub = account_xpub("m/84'/0'/0'", Network::Bitcoin);
    let key_arg = format!("@0={xpub}");
    let phrase = encode_template_with_key("wpkh(@0/<0;1>/*)", &key_arg);
    Command::cargo_bin("md").unwrap()
        .args(["address", &phrase])
        .assert().success()
        .stdout(predicates::str::contains("bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu"));
}

#[test]
fn address_template_without_key_exits_2_with_helpful_message() {
    Command::cargo_bin("md").unwrap()
        .args(["address", "--template", "wpkh(@0/<0;1>/*)"])
        .assert().code(2)
        .stderr(predicates::str::contains("--key @i=<XPUB> required"));
}

#[test]
fn address_phrase_template_only_exits_2_with_wallet_policy_message() {
    let phrase = Command::cargo_bin("md").unwrap()
        .args(["encode", "wpkh(@0/<0;1>/*)"])
        .output().unwrap();
    let phrase = String::from_utf8(phrase.stdout).unwrap().lines().next().unwrap().to_string();
    Command::cargo_bin("md").unwrap()
        .args(["address", &phrase])
        .assert().code(2)
        .stderr(predicates::str::contains("requires wallet-policy mode"));
}

#[test]
fn address_no_input_exits_2() {
    Command::cargo_bin("md").unwrap()
        .args(["address"])
        .assert().code(2);
}
