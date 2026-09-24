//! F-672, CLI level: `md descriptor --network <test network>` renders EVERY
//! key under that network's version bytes, and `md decompose --network <test
//! network> --emit commands` prints an `md encode` recipe that runs.
//!
//! Reproduces engrave `design/agent-reports/e2e-live-site-wallets.md` D-2
//! with the report's own command: `md descriptor --template … --key
//! @i=[fp/48'/1'/k'/3']tpub… --network regtest` printed every leaf as
//! mainnet `xpub6DXu…` beside a `tpub` Liana internal key, and Core 31.1
//! regtest refused it (`Multi: key 'xpub6DXuQW1Q2Jpa1DM9…' is not valid`).
//! The keys are the report's regtest keys (`regtest/keys.txt`: seeds A
//! 73c5da0a, B b8688df1, C 28645006 at m/48'/1'/k'/3').

use assert_cmd::Command;
use bitcoin::NetworkKind;
use bitcoin::bip32::Xpub;
use miniscript::Descriptor;
use miniscript::ForEachKey;
use miniscript::descriptor::DescriptorPublicKey;
use std::str::FromStr;

fn md() -> Command {
    Command::cargo_bin("md").unwrap()
}

const A0: &str = "[73c5da0a/48'/1'/0'/3']tpubDFH9dgzveyD94P86sEzUzWtd2wxFkUoK78rSBqSWyXNuFq46dy4HbPTEZEP4fbSY4L5Vb2LFnm23JeGQppq5SPcPDNuHZU3JQwMSFXLdudh";
const B0: &str = "[b8688df1/48'/1'/0'/3']tpubDEfobrrtptRTd4Qp6K7RtkNC9GZTdyWPwrXBnPEiu9o5gbcFpscfHbhghiNuBBRuq9RfiN3nwNkLs3E2nRwnFmwKq6NPAbVa6btM8iMjsW6";
const C0: &str = "[28645006/48'/1'/0'/3']tpubDEwqCvJxKwKWXeYgVhaoUCFf5QtViLFofqBLuTnjAd5CwmK8NoKbxnkYP2ba1YD9vjT8i5o1iod8RgzKtVaRjTfApksx1Cwt6FCKtTED2V8";
const A1: &str = "[73c5da0a/48'/1'/1'/3']tpubDEYM1BmQ5rp2QyHNpbpDxS4FyPkU8EmSeSaTeAdZkkNVxJUjfi5AwTFkmGU85DnoH4T8Mi73XTuLSAvPyzh5oe3gBvjppL192MR69n9xrnU";
const KEYS: [&str; 4] = [A0, B0, C0, A1];

/// The e2e report's kofn-recovery / Liana-key template (D-2's exact shape).
const KOFN_LIANA: &str = "tr(UNSPENDABLE(liana),{multi_a(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*),and_v(v:pk(@3/<0;1>/*),older(26280))})";
/// Kind 0: the same policy under the NUMS H-point.
const KOFN_NUMS: &str = "tr(50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0,{multi_a(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*),and_v(v:pk(@3/<0;1>/*),older(26280))})";
/// A real internal key (slot @0).
const REAL_IK: &str =
    "tr(@0/<0;1>/*,{multi_a(2,@1/<0;1>/*,@2/<0;1>/*),and_v(v:pk(@3/<0;1>/*),older(26280))})";
/// wsh.
const WSH: &str =
    "wsh(or_d(multi(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*),and_v(v:pkh(@3/<0;1>/*),older(26280))))";

/// D-2's expected regtest output, pinned. Derived OUTSIDE md: the leaves are
/// the report's buggy `xpub6DXu…` strings with their version bytes swapped to
/// `043587CF` (engrave's `e2e-live-site-wallets/regtest/kofn-liana.descriptor.txt`
/// through md-codec's `tests/fixtures/network/derive.py`); the internal key
/// was already `tpub` there. Parent fingerprints stay `00000000` (md1 cannot
/// carry them), so the leaves differ from `regtest/keys.txt` only in that
/// field — the key and chain code are the input's (asserted below).
const D2_EXPECTED: &str = "tr(tpubD6NzVbkrYhZ4Xj2FFF4p6kScAaid4Q3ohdGkAitrmrQhVqhN3U3NETELvixX9h63jqpxTLtcLE2uvrSpXXyg6J6NZVHQ44xijAp9AYkMHEb/<0;1>/*,{multi_a(2,[73c5da0a/48'/1'/0'/3']tpubDDuXvjq5jan2H1XzhCfB9Xn93hswxh267HxrvA9d8ZChMxiMiwjuQXUuwWByCQssdtjUTi6JHtN3mQrw6kZnZ7qEBKyuJjnZoQCSmEvP4sg/<0;1>/*,[b8688df1/48'/1'/0'/3']tpubDDuXvjq5jan2GChxDL12YM5dRiwjb2Vvw5HTE8zk3QJJ9xNwTFbtahk2R7eTumsHeCiHgHsEgb7f2YSncmtZqXBBx1Wc4qaPKnEV7K5xhtJ/<0;1>/*,[28645006/48'/1'/0'/3']tpubDDuXvjq5jan2FTEMh2M4aJQJMaZqzHLB2KmVCZFFHPVj4jM6qhjMpsnYwbnq1PcCv8sugBdA2ivdWo8kpq2tfUYxcigXvoSbWd5BeuhSGZe/<0;1>/*),and_v(v:pk([73c5da0a/48'/1'/1'/3']tpubDDuXvjq5jan2GK3yLymq11WnC6NkTNgDnv4fLQv415FaMTy1kVGyACP3bYxLHANaTZFfZ1ZBebG4DGPKu9At6uoKSf12uy7oEP7fpTHxoJv/<0;1>/*),older(26280))})#cxm3rd99";

fn descriptor(template: &str, network: &str, extra: &[&str]) -> String {
    let mut c = md();
    c.args(["descriptor", "--template", template, "--network", network]);
    for (i, k) in KEYS.iter().enumerate() {
        c.args(["--key", &format!("@{i}={k}")]);
    }
    c.args(extra);
    let out = c.assert().success().get_output().stdout.clone();
    String::from_utf8(out).unwrap().trim().to_string()
}

/// Every extended key in `text` parses, is a test-network key, and carries
/// the chain code and public key of the input key in its slot.
fn assert_all_test_keys(text: &str, what: &str) {
    assert!(!text.contains("xpub"), "{what}: a mainnet xpub in {text}");
    let d = Descriptor::<DescriptorPublicKey>::from_str(text)
        .unwrap_or_else(|e| panic!("{what}: re-parse: {e}\n{text}"));
    let inputs: Vec<Xpub> = KEYS
        .iter()
        .map(|k| k.split(']').nth(1).unwrap().parse().unwrap())
        .collect();
    let mut n = 0;
    d.for_each_key(|k| {
        let (x, has_origin) = match k {
            DescriptorPublicKey::XPub(x) => (x.xkey, x.origin.is_some()),
            DescriptorPublicKey::MultiXPub(x) => (x.xkey, x.origin.is_some()),
            DescriptorPublicKey::Single(_) => return true,
        };
        assert_eq!(x.network, NetworkKind::Test, "{what}: {k}");
        // Every key with an origin is a seated key; only Liana's derived
        // internal key has none.
        if has_origin {
            assert!(
                inputs
                    .iter()
                    .any(|i| i.public_key == x.public_key && i.chain_code == x.chain_code),
                "{what}: {k} is not one of the input keys"
            );
        }
        n += 1;
        true
    });
    assert!(n >= 4, "{what}: only {n} extended keys");
}

/// D-2, the report's own command. Mutation: restore `NetworkKind::Main` for
/// the leaves in md-codec's `assemble_origin_and_xkey` -> red.
#[test]
fn d2_regtest_descriptor_renders_every_leaf_as_tpub() {
    let got = descriptor(KOFN_LIANA, "regtest", &[]);
    assert_eq!(got, D2_EXPECTED);
    assert_all_test_keys(&got, "D-2");
}

/// Kind 0, kind 1, a real internal key, and wsh; every test network; all
/// three spellings.
#[test]
fn every_test_network_renders_every_key_as_tpub() {
    for (name, t) in [
        ("kind0-nums", KOFN_NUMS),
        ("kind1-liana", KOFN_LIANA),
        ("real-internal-key", REAL_IK),
        ("wsh", WSH),
    ] {
        for net in ["testnet", "signet", "regtest"] {
            for extra in [&[][..], &["--chain", "0"][..], &["--chain", "1"][..]] {
                let got = descriptor(t, net, extra);
                assert_all_test_keys(&got, &format!("{name} {net} {extra:?}"));
            }
        }
    }
}

/// The regtest output is now something md itself accepts back as a regtest
/// descriptor: `md decompose --network regtest` recovers the template it was
/// built from, Liana key included. Before F-672 decompose refused it ("key
/// @0 is a mainnet extended key, but --network says testnet").
#[test]
fn regtest_descriptor_decomposes_under_regtest() {
    let got = descriptor(KOFN_LIANA, "regtest", &[]);
    let out = md()
        .args([
            "decompose",
            &got,
            "--network",
            "regtest",
            "--emit",
            "template",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let t = String::from_utf8(out).unwrap();
    assert_eq!(
        t.trim(),
        "tr(UNSPENDABLE(liana),{multi_a(2,@0/48'/1'/0'/3'/<0;1>/*,@1/48'/1'/0'/3'/<0;1>/*,@2/48'/1'/0'/3'/<0;1>/*),and_v(v:pk(@3/48'/1'/1'/3'/<0;1>/*),older(26280))})"
    );
}

/// `md decompose --network regtest --emit commands` prints a route-1 `md
/// encode` line carrying tpub keys; md refuses those under its mainnet
/// default ("expected mainnet xpub version 0488B21E, got 043587CF"), so the
/// line must carry `--network regtest`. Mutation: drop the flag from
/// `cmd/decompose.rs`'s route-1 line -> the executed recipe fails -> red.
#[test]
fn decompose_commands_recipe_runs_on_a_test_network() {
    // The report's own regtest wallet, as md now renders it.
    let desc = descriptor(KOFN_LIANA, "regtest", &[]);
    let out = md()
        .args([
            "decompose",
            &desc,
            "--network",
            "regtest",
            "--emit",
            "commands",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    let route1: String = text
        .lines()
        .skip_while(|l| !l.starts_with("md encode"))
        .take_while(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(route1.contains("--network regtest"), "{route1}");
    // Execute it, as printed, with this build's md.
    let bin = assert_cmd::cargo::cargo_bin("md");
    let script = route1.replacen("md encode", &format!("'{}' encode", bin.display()), 1);
    let st = std::process::Command::new("sh")
        .args(["-c", &script])
        .output()
        .unwrap();
    assert!(
        st.status.success(),
        "{route1}\n{}",
        String::from_utf8_lossy(&st.stderr)
    );
}

/// `md address --network regtest` was never affected (addresses carry no
/// version bytes); pinned so it stays so.
#[test]
fn regtest_addresses_use_the_regtest_hrp() {
    let mut c = md();
    c.args(["address", "--template", KOFN_LIANA, "--network", "regtest"]);
    for (i, k) in KEYS.iter().enumerate() {
        c.args(["--key", &format!("@{i}={k}")]);
    }
    let out = c.assert().success().get_output().stdout.clone();
    let s = String::from_utf8(out).unwrap();
    assert!(s.trim().starts_with("bcrt1p"), "{s}");
}
