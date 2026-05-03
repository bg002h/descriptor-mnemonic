#![allow(missing_docs)]
#![cfg(feature = "json")]

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

#[test]
fn snapshot_wpkh_mainnet_receive_0_to_2() {
    let xpub = account_xpub("m/84'/0'/0'", Network::Bitcoin);
    let key = format!("@0={xpub}");
    let out = Command::cargo_bin("md").unwrap()
        .args(["address", "--template", "wpkh(@0/<0;1>/*)", "--key", &key, "--count", "3", "--json"])
        .output().unwrap();
    let body = String::from_utf8(out.stdout).unwrap();
    insta::assert_snapshot!("wpkh_mainnet_receive_0_to_2", body);
}

#[test]
fn snapshot_wpkh_mainnet_change_0() {
    let xpub = account_xpub("m/84'/0'/0'", Network::Bitcoin);
    let key = format!("@0={xpub}");
    let out = Command::cargo_bin("md").unwrap()
        .args(["address", "--template", "wpkh(@0/<0;1>/*)", "--key", &key, "--change", "--json"])
        .output().unwrap();
    let body = String::from_utf8(out.stdout).unwrap();
    insta::assert_snapshot!("wpkh_mainnet_change_0", body);
}

#[test]
fn snapshot_wpkh_testnet_receive_0() {
    let xpub = account_xpub("m/84'/1'/0'", Network::Testnet);
    let key = format!("@0={xpub}");
    let out = Command::cargo_bin("md").unwrap()
        .args(["address", "--template", "wpkh(@0/<0;1>/*)", "--key", &key, "--network", "testnet", "--json"])
        .output().unwrap();
    let body = String::from_utf8(out.stdout).unwrap();
    insta::assert_snapshot!("wpkh_testnet_receive_0", body);
}
