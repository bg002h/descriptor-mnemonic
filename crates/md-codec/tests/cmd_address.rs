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

fn account_xpub_testnet(path: &str) -> Xpub { account_xpub(path, Network::Testnet) }

/// Independently derive the BIP 84 single-sig address using rust-bitcoin's
/// own bip32 + Address builders. Used to pin testnet (and any non-published
/// mainnet) golden vectors against a trusted secondary path.
fn expected_wpkh_address(account_xpub: &Xpub, chain: u32, index: u32, network: Network) -> String {
    use bitcoin::Address;
    use bitcoin::bip32::ChildNumber;
    use bitcoin::CompressedPublicKey;
    let secp = Secp256k1::new();
    let leaf = account_xpub
        .derive_pub(&secp, &[
            ChildNumber::Normal { index: chain },
            ChildNumber::Normal { index },
        ]).unwrap();
    let cpk = CompressedPublicKey(leaf.public_key);
    Address::p2wpkh(&cpk, network).to_string()
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

#[test]
fn address_mainnet_wpkh_receive_0_and_1() {
    let xpub = account_xpub("m/84'/0'/0'", Network::Bitcoin);
    let key_arg = format!("@0={xpub}");
    let out = Command::cargo_bin("md").unwrap()
        .args(["address", "--template", "wpkh(@0/<0;1>/*)", "--key", &key_arg, "--count", "2"])
        .output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "expected 2 addresses, got {}: {stdout}", lines.len());
    assert_eq!(lines[0], "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu");
    assert_eq!(lines[1], "bc1qnjg0jd8228aq7egyzacy8cys3knf9xvrerkf9g");
}

#[test]
fn address_mainnet_wpkh_first_change() {
    let xpub = account_xpub("m/84'/0'/0'", Network::Bitcoin);
    let key_arg = format!("@0={xpub}");
    Command::cargo_bin("md").unwrap()
        .args(["address", "--template", "wpkh(@0/<0;1>/*)", "--key", &key_arg, "--change"])
        .assert().success()
        .stdout(predicates::str::contains("bc1q8c6fshw2dlwun7ekn9qwf37cu2rn755upcp6el"));
}

#[test]
fn address_testnet_wpkh_receive_0_via_secondary_path() {
    let xpub = account_xpub_testnet("m/84'/1'/0'");
    let expected = expected_wpkh_address(&xpub, 0, 0, Network::Testnet);
    assert!(expected.starts_with("tb1q"), "expected tb1q... testnet address, got {expected}");
    let key_arg = format!("@0={xpub}");
    Command::cargo_bin("md").unwrap()
        .args(["address", "--template", "wpkh(@0/<0;1>/*)", "--key", &key_arg, "--network", "testnet"])
        .assert().success()
        .stdout(predicates::str::contains(&expected));
}

#[test]
fn address_count_max_succeeds() {
    let xpub = account_xpub("m/84'/0'/0'", Network::Bitcoin);
    let key_arg = format!("@0={xpub}");
    let out = Command::cargo_bin("md").unwrap()
        .args(["address", "--template", "wpkh(@0/<0;1>/*)", "--key", &key_arg, "--count", "1000"])
        .output().unwrap();
    assert!(out.status.success());
    let n = String::from_utf8(out.stdout).unwrap().lines().count();
    assert_eq!(n, 1000);
}

#[test]
fn address_count_over_max_clap_rejects() {
    Command::cargo_bin("md").unwrap()
        .args(["address", "--template", "wpkh(@0/<0;1>/*)", "--key", "@0=ignored", "--count", "1001"])
        .assert().code(2);
}

#[test]
fn address_chain_out_of_range_returns_1() {
    let xpub = account_xpub("m/84'/0'/0'", Network::Bitcoin);
    let key_arg = format!("@0={xpub}");
    Command::cargo_bin("md").unwrap()
        .args(["address", "--template", "wpkh(@0/<0;1>/*)", "--key", &key_arg, "--chain", "5"])
        .assert().code(1)
        .stderr(predicates::str::contains("out of range"));
}

#[test]
fn address_change_and_chain_together_rejected() {
    Command::cargo_bin("md").unwrap()
        .args(["address", "--template", "wpkh(@0/<0;1>/*)", "--key", "@0=ignored",
               "--change", "--chain", "1"])
        .assert().code(2);
}

#[test]
fn address_mainnet_wsh_multi_2of2_receive_0() {
    use bitcoin::Address;
    use bitcoin::bip32::ChildNumber;
    use bitcoin::CompressedPublicKey;
    let xpub = account_xpub("m/48'/0'/0'/2'", Network::Bitcoin);
    let key_arg = format!("@0={xpub}");
    let key_arg_b = format!("@1={xpub}");

    // Independently derive the expected wsh-multi address.
    let secp = Secp256k1::new();
    let leaf = xpub.derive_pub(&secp, &[
        ChildNumber::Normal { index: 0 },
        ChildNumber::Normal { index: 0 },
    ]).unwrap();
    let cpk = CompressedPublicKey(leaf.public_key);
    let pk = bitcoin::PublicKey::new(cpk.0);
    let script = bitcoin::blockdata::script::Builder::new()
        .push_int(2)
        .push_key(&pk).push_key(&pk)
        .push_int(2)
        .push_opcode(bitcoin::opcodes::all::OP_CHECKMULTISIG)
        .into_script();
    let expected = Address::p2wsh(&script, Network::Bitcoin).to_string();

    Command::cargo_bin("md").unwrap()
        .args(["address", "--template", "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))",
               "--key", &key_arg, "--key", &key_arg_b])
        .assert().success()
        .stdout(predicates::str::contains(&expected));
}
