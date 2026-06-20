#![allow(missing_docs)]

use assert_cmd::Command;
use bitcoin::Network;
use bitcoin::bip32::{DerivationPath, Xpriv, Xpub};
use bitcoin::secp256k1::Secp256k1;
use std::str::FromStr;

const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

fn account_xpub(path: &str, network: Network) -> Xpub {
    let mn = bip39::Mnemonic::parse(ABANDON).unwrap();
    let seed = mn.to_seed("");
    let secp = Secp256k1::new();
    let master = Xpriv::new_master(network, &seed).unwrap();
    let dp = DerivationPath::from_str(path).unwrap();
    let xpriv = master.derive_priv(&secp, &dp).unwrap();
    Xpub::from_priv(&secp, &xpriv)
}

fn account_xpub_testnet(path: &str) -> Xpub {
    account_xpub(path, Network::Testnet)
}

/// Independently derive the BIP 84 single-sig address using rust-bitcoin's
/// own bip32 + Address builders. Used to pin testnet (and any non-published
/// mainnet) golden vectors against a trusted secondary path.
fn expected_wpkh_address(account_xpub: &Xpub, chain: u32, index: u32, network: Network) -> String {
    use bitcoin::Address;
    use bitcoin::CompressedPublicKey;
    use bitcoin::bip32::ChildNumber;
    let secp = Secp256k1::new();
    let leaf = account_xpub
        .derive_pub(
            &secp,
            &[
                ChildNumber::Normal { index: chain },
                ChildNumber::Normal { index },
            ],
        )
        .unwrap();
    let cpk = CompressedPublicKey(leaf.public_key);
    Address::p2wpkh(&cpk, network).to_string()
}

fn encode_template_with_key(template: &str, key_arg: &str) -> String {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["encode", template, "--key", key_arg])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "encode failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .to_string()
}

#[test]
fn address_template_mode_emits_bip84_receive_0() {
    // BIP 84 vector: abandon mnemonic at m/84'/0'/0'/0/0 → bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu
    let xpub = account_xpub("m/84'/0'/0'", Network::Bitcoin);
    let key_arg = format!("@0={xpub}");
    Command::cargo_bin("md")
        .unwrap()
        .args([
            "address",
            "--template",
            "wpkh(@0/<0;1>/*)",
            "--key",
            &key_arg,
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu",
        ));
}

/// Insert a comma every 5 chars to simulate a grouped/transcribed card.
/// Comma is the SPEC §3.2 separator md-codec's codex32 layer does NOT already
/// tolerate (it strips whitespace/hyphen via D11), so this genuinely exercises
/// the md-cli intake strip (applied inside `build_descriptor`).
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
fn address_accepts_grouped_phrase() {
    // mstring display-grouping (SPEC §3.2): a separator-bearing card re-ingests
    // through `build_descriptor`'s strip.
    let xpub = account_xpub("m/84'/0'/0'", Network::Bitcoin);
    let key_arg = format!("@0={xpub}");
    let phrase = encode_template_with_key("wpkh(@0/<0;1>/*)", &key_arg);
    let grouped = group5(&phrase);
    Command::cargo_bin("md")
        .unwrap()
        .args(["address", &grouped])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu",
        ));
}

#[test]
fn address_phrase_mode_round_trips_through_encode() {
    let xpub = account_xpub("m/84'/0'/0'", Network::Bitcoin);
    let key_arg = format!("@0={xpub}");
    let phrase = encode_template_with_key("wpkh(@0/<0;1>/*)", &key_arg);
    Command::cargo_bin("md")
        .unwrap()
        .args(["address", &phrase])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu",
        ));
}

#[test]
fn address_template_without_key_exits_2_with_helpful_message() {
    Command::cargo_bin("md")
        .unwrap()
        .args(["address", "--template", "wpkh(@0/<0;1>/*)"])
        .assert()
        .code(2)
        .stderr(predicates::str::contains("--key @i=<XPUB> required"));
}

#[test]
fn address_phrase_template_only_exits_2_with_wallet_policy_message() {
    let phrase = Command::cargo_bin("md")
        .unwrap()
        .args(["encode", "wpkh(@0/<0;1>/*)"])
        .output()
        .unwrap();
    let phrase = String::from_utf8(phrase.stdout)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .to_string();
    Command::cargo_bin("md")
        .unwrap()
        .args(["address", &phrase])
        .assert()
        .code(2)
        .stderr(predicates::str::contains("requires wallet-policy mode"));
}

#[test]
fn address_no_input_exits_2() {
    Command::cargo_bin("md")
        .unwrap()
        .args(["address"])
        .assert()
        .code(2);
}

#[test]
fn address_mainnet_wpkh_receive_0_and_1() {
    let xpub = account_xpub("m/84'/0'/0'", Network::Bitcoin);
    let key_arg = format!("@0={xpub}");
    let out = Command::cargo_bin("md")
        .unwrap()
        .args([
            "address",
            "--template",
            "wpkh(@0/<0;1>/*)",
            "--key",
            &key_arg,
            "--count",
            "2",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines.len(),
        2,
        "expected 2 addresses, got {}: {stdout}",
        lines.len()
    );
    assert_eq!(lines[0], "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu");
    assert_eq!(lines[1], "bc1qnjg0jd8228aq7egyzacy8cys3knf9xvrerkf9g");
}

#[test]
fn address_mainnet_wpkh_first_change() {
    let xpub = account_xpub("m/84'/0'/0'", Network::Bitcoin);
    let key_arg = format!("@0={xpub}");
    Command::cargo_bin("md")
        .unwrap()
        .args([
            "address",
            "--template",
            "wpkh(@0/<0;1>/*)",
            "--key",
            &key_arg,
            "--change",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "bc1q8c6fshw2dlwun7ekn9qwf37cu2rn755upcp6el",
        ));
}

#[test]
fn address_testnet_wpkh_receive_0_via_secondary_path() {
    let xpub = account_xpub_testnet("m/84'/1'/0'");
    let expected = expected_wpkh_address(&xpub, 0, 0, Network::Testnet);
    assert!(
        expected.starts_with("tb1q"),
        "expected tb1q... testnet address, got {expected}"
    );
    let key_arg = format!("@0={xpub}");
    Command::cargo_bin("md")
        .unwrap()
        .args([
            "address",
            "--template",
            "wpkh(@0/<0;1>/*)",
            "--key",
            &key_arg,
            "--network",
            "testnet",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains(&expected));
}

#[test]
fn address_count_max_succeeds() {
    let xpub = account_xpub("m/84'/0'/0'", Network::Bitcoin);
    let key_arg = format!("@0={xpub}");
    let out = Command::cargo_bin("md")
        .unwrap()
        .args([
            "address",
            "--template",
            "wpkh(@0/<0;1>/*)",
            "--key",
            &key_arg,
            "--count",
            "1000",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let n = String::from_utf8(out.stdout).unwrap().lines().count();
    assert_eq!(n, 1000);
}

#[test]
fn address_count_over_max_clap_rejects() {
    Command::cargo_bin("md")
        .unwrap()
        .args([
            "address",
            "--template",
            "wpkh(@0/<0;1>/*)",
            "--key",
            "@0=ignored",
            "--count",
            "1001",
        ])
        .assert()
        .code(2);
}

#[test]
fn address_chain_out_of_range_returns_1() {
    let xpub = account_xpub("m/84'/0'/0'", Network::Bitcoin);
    let key_arg = format!("@0={xpub}");
    Command::cargo_bin("md")
        .unwrap()
        .args([
            "address",
            "--template",
            "wpkh(@0/<0;1>/*)",
            "--key",
            &key_arg,
            "--chain",
            "5",
        ])
        .assert()
        .code(1)
        .stderr(predicates::str::contains("out of range"));
}

#[test]
fn address_change_and_chain_together_rejected() {
    Command::cargo_bin("md")
        .unwrap()
        .args([
            "address",
            "--template",
            "wpkh(@0/<0;1>/*)",
            "--key",
            "@0=ignored",
            "--change",
            "--chain",
            "1",
        ])
        .assert()
        .code(2);
}

#[test]
fn address_mainnet_wsh_multi_2of2_receive_0() {
    use bitcoin::Address;
    use bitcoin::CompressedPublicKey;
    use bitcoin::bip32::ChildNumber;
    let xpub = account_xpub("m/48'/0'/0'/2'", Network::Bitcoin);
    let key_arg = format!("@0={xpub}");
    let key_arg_b = format!("@1={xpub}");

    // Independently derive the expected wsh-multi address.
    let secp = Secp256k1::new();
    let leaf = xpub
        .derive_pub(
            &secp,
            &[
                ChildNumber::Normal { index: 0 },
                ChildNumber::Normal { index: 0 },
            ],
        )
        .unwrap();
    let cpk = CompressedPublicKey(leaf.public_key);
    let pk = bitcoin::PublicKey::new(cpk.0);
    let script = bitcoin::blockdata::script::Builder::new()
        .push_int(2)
        .push_key(&pk)
        .push_key(&pk)
        .push_int(2)
        .push_opcode(bitcoin::opcodes::all::OP_CHECKMULTISIG)
        .into_script();
    let expected = Address::p2wsh(&script, Network::Bitcoin).to_string();

    Command::cargo_bin("md")
        .unwrap()
        .args([
            "address",
            "--template",
            "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))",
            "--key",
            &key_arg,
            "--key",
            &key_arg_b,
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains(&expected));
}

/// FUNDS-SAFETY regression (`restore-md1-per-key-use-site-and-hardened-wildcard`):
/// `md address` on a DIVERGENT-suffix multisig card — `@1` overrides the
/// shared `<0;1>` baseline with `<2;3>` — must yield the CORRECT per-cosigner
/// address. Pre-fix md-codec collapsed every key onto the baseline chain and
/// `md address` SILENTLY returned the WRONG address (`@1` at `[0,0]` instead
/// of its own `[2,0]`). The golden is computed OUTSIDE md-codec: each leaf
/// pubkey via rust-bitcoin `Xpub::derive_pub` at the cosigner's OWN alt, then
/// the `multi(2,…)` witnessScript / P2WSH address assembled by hand.
#[test]
fn address_divergent_use_site_override_yields_correct_address() {
    use bitcoin::Address;
    use bitcoin::bip32::ChildNumber;

    // Two DISTINCT cosigner xpubs (different BIP-48 accounts).
    let xpub0 = account_xpub("m/48'/0'/0'/2'", Network::Bitcoin);
    let xpub1 = account_xpub("m/48'/0'/1'/2'", Network::Bitcoin);
    let key0 = format!("@0={xpub0}");
    let key1 = format!("@1={xpub1}");

    // INDEPENDENT golden: @0 at its baseline alt[0]=0 → [0,0]; @1 at its OWN
    // override alt[0]=2 → [2,0]. multi(2,…) is UNSORTED → template order.
    let secp = Secp256k1::new();
    let leaf = |xp: &Xpub, first: u32, second: u32| {
        xp.derive_pub(
            &secp,
            &[
                ChildNumber::Normal { index: first },
                ChildNumber::Normal { index: second },
            ],
        )
        .unwrap()
        .public_key
    };
    let pk0 = bitcoin::PublicKey::new(leaf(&xpub0, 0, 0));
    let pk1 = bitcoin::PublicKey::new(leaf(&xpub1, 2, 0)); // <2;3>/0 = [2,0]
    let script = bitcoin::blockdata::script::Builder::new()
        .push_int(2)
        .push_key(&pk0)
        .push_key(&pk1)
        .push_int(2)
        .push_opcode(bitcoin::opcodes::all::OP_CHECKMULTISIG)
        .into_script();
    let expected = Address::p2wsh(&script, Network::Bitcoin).to_string();

    // Sanity: the baseline-collapse (BUG) address differs from the golden, so
    // a vacuous pass is impossible.
    let pk1_wrong = bitcoin::PublicKey::new(leaf(&xpub1, 0, 0));
    let wrong_script = bitcoin::blockdata::script::Builder::new()
        .push_int(2)
        .push_key(&pk0)
        .push_key(&pk1_wrong)
        .push_int(2)
        .push_opcode(bitcoin::opcodes::all::OP_CHECKMULTISIG)
        .into_script();
    let wrong = Address::p2wsh(&wrong_script, Network::Bitcoin).to_string();
    assert_ne!(
        expected, wrong,
        "fixture sanity: golden != baseline-collapse"
    );

    let out = Command::cargo_bin("md")
        .unwrap()
        .args([
            "address",
            "--template",
            "wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))",
            "--key",
            &key0,
            "--key",
            &key1,
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "md address failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    let got = stdout.lines().next().unwrap_or("").trim();
    assert_eq!(
        got, expected,
        "md address must derive @1 at its own <2;3>/0, not the baseline; got {got}"
    );
    assert_ne!(
        got, wrong,
        "md address must NOT silently return the baseline-collapse address"
    );
}

/// A hardened wildcard (`/*h`) card → `md address` exits LOUDLY (exit 1,
/// `hardened public-key derivation`), never silently. xpub-only restore
/// cannot derive a hardened child (BIP 32).
#[test]
fn address_hardened_wildcard_card_refuses_loudly() {
    let xpub = account_xpub("m/84'/0'/0'", Network::Bitcoin);
    let key_arg = format!("@0={xpub}");
    Command::cargo_bin("md")
        .unwrap()
        .args([
            "address",
            "--template",
            "wpkh(@0/<0;1>/*h)",
            "--key",
            &key_arg,
        ])
        .assert()
        .code(1)
        .stderr(predicates::str::contains("hardened public-key derivation"));
}

/// A per-cosigner override that carries a hardened wildcard (`@1/<2;3>/*h`,
/// baseline clean) → `md address` STILL refuses loudly via the shared
/// `has_hardened_use_site` predicate (the override is inspected, not just the
/// baseline). Pre-fix the override-hardened case slipped to a generic
/// `AddressDerivationFailed`.
#[test]
fn address_override_hardened_wildcard_card_refuses_loudly() {
    let xpub0 = account_xpub("m/48'/0'/0'/2'", Network::Bitcoin);
    let xpub1 = account_xpub("m/48'/0'/1'/2'", Network::Bitcoin);
    let key0 = format!("@0={xpub0}");
    let key1 = format!("@1={xpub1}");
    Command::cargo_bin("md")
        .unwrap()
        .args([
            "address",
            "--template",
            "wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*h))",
            "--key",
            &key0,
            "--key",
            &key1,
        ])
        .assert()
        .code(1)
        .stderr(predicates::str::contains("hardened public-key derivation"));
}
