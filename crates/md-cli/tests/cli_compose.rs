//! `md compose` (SPEC_wallet_policy_composer.md §10 item 1): fixed lowering
//! from a path DSL to a BIP-388 template; round-trips through `md encode` and
//! `md decode`; refuses per §4e; gates EXPERIMENTAL shapes.

use assert_cmd::Command;
use predicates::prelude::*;

fn md() -> Command {
    Command::cargo_bin("md").expect("md binary")
}

#[test]
fn compose_two_path_wsh_prints_the_fixed_template() {
    md().args(["compose", "--wrapper", "wsh", "--path", "2of3", "--path", "1of1,older=26280"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "wsh(or_d(multi(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/1'/2'/<0;1>/*,@2/48'/0'/2'/2'/<0;1>/*),and_v(v:pkh(@3/48'/0'/3'/2'/<0;1>/*),older(26280))))",
        ));
}

#[test]
fn compose_output_round_trips_through_encode_and_decode() {
    let out = md()
        .args([
            "compose",
            "--wrapper",
            "tr",
            "--path",
            "2of3",
            "--path",
            "1of1,older=26280",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let with_origins = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .to_string();
    assert!(
        with_origins.contains("@0/48'/0'/0'/3'/<0;1>/*"),
        "{with_origins}"
    );
    // `md decode` prints the renderer's origin-less text (F-219); get that form from --json.
    let js = md()
        .args([
            "compose",
            "--json",
            "--wrapper",
            "tr",
            "--path",
            "2of3",
            "--path",
            "1of1,older=26280",
        ])
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&js.stdout).unwrap();
    let template = v["template"].as_str().unwrap().to_string();
    assert_eq!(v["template_with_origins"].as_str().unwrap(), with_origins);
    let enc = md().args(["encode", &with_origins]).output().unwrap();
    assert!(
        enc.status.success(),
        "{}",
        String::from_utf8_lossy(&enc.stderr)
    );
    let chunks: Vec<String> = String::from_utf8(enc.stdout)
        .unwrap()
        .lines()
        .filter(|l| l.starts_with("md1") && !l.contains(' '))
        .map(str::to_string)
        .collect();
    assert!(!chunks.is_empty());
    let mut dec = md();
    dec.arg("decode");
    for c in &chunks {
        dec.arg(c);
    }
    dec.assert()
        .success()
        .stdout(predicate::str::starts_with(template));
}

#[test]
fn compose_refuses_a_keyless_path_without_experimental_and_admits_it_with() {
    let h = "a8".repeat(32);
    md().args([
        "compose",
        "--wrapper",
        "wsh",
        "--path",
        "2of3",
        "--path",
        &format!("keyless,sha256={h},after=1383520"),
    ])
    .assert()
    .failure()
    .code(1)
    .stderr(predicate::str::contains("--experimental"));
    md().args([
        "compose",
        "--wrapper",
        "wsh",
        "--experimental",
        "--path",
        "2of3",
        "--path",
        &format!("keyless,sha256={h},after=1383520"),
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains(format!(
        "and_v(v:sha256({h}),after(1383520))"
    )))
    .stderr(predicate::str::contains("EXPERIMENTAL"));
}

#[test]
fn compose_refuses_structural_defects_with_the_spec_wording() {
    md().args([
        "compose",
        "--wrapper",
        "tr",
        "--path",
        "2of3",
        "--path",
        "keyless,sha256=00",
        "--experimental",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("sha256 needs 64 hex characters"));
    md().args(["compose", "--wrapper", "sh", "--path", "1of1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "legacy wrappers hold one plain sorted multisig only",
        ));
    md().args(["compose", "--wrapper", "wsh", "--path", "1of1,older=65536"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("older in blocks needs 1..=65535"));
    md().args([
        "compose",
        "--wrapper",
        "wsh",
        "--path",
        "1of1,older=4194305",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("older in blocks needs 1..=65535"));
    // A Unix time typed without its suffix is refused WITH the suffix named.
    md().args([
        "compose",
        "--wrapper",
        "wsh",
        "--path",
        "1of1,after=1893456000",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("after=1893456000t"));
}

#[test]
fn compose_says_when_unsorted_had_no_effect() {
    md().args([
        "compose",
        "--wrapper",
        "wsh",
        "--path",
        "2of3,unsorted",
        "--path",
        "1of1,older=10",
    ])
    .assert()
    .success()
    .stderr(predicate::str::contains("`unsorted` has no effect here"))
    .stderr(predicate::str::contains("EXPERIMENTAL").not());
}

#[test]
fn compose_json_names_slots_internal_key_and_experimental() {
    md().args([
        "compose",
        "--wrapper",
        "tr",
        "--json",
        "--path",
        "2of2,older=100",
        "--path",
        "1of1",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("\"internal_key_path\": 1"))
    .stdout(predicate::str::contains(
        "\"template_with_origins\": \"tr(@0/48'/0'/0'/3'/<0;1>/*,",
    ))
    .stdout(predicate::str::contains("\"index\": 0"))
    .stdout(predicate::str::contains("\"experimental\": []"));
}
