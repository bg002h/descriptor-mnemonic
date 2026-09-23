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
        // Plan 1b: every coordinator refuses a keyless path, so compose's
        // none-case stop needs the operator's explicit `--md-only`.
        "--md-only",
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
    // F-551: "needs exactly N ... got M". The old wording ended ", lowercase",
    // which named a second rule the input might already satisfy -- and did, for
    // an uppercase digest of the right length.
    .stderr(predicate::str::contains(
        "sha256 needs exactly 64 hex characters",
    ));
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

/// F-603: `--json`'s `experimental[]` carried the STDERR sentence, which numbers
/// paths from 1, while `slots[].path` in the same object counts from 0. A
/// consumer joining the two got a false statement with no parse error to warn
/// it.
///
/// MEASURED on the object this test builds: `experimental` says "path 2 has no
/// key" and `slots[].path == 2` holds key slots @3 and @4. Both fields are
/// right about their own numbering; the object as a whole is not.
///
/// The fix is ADDITIVE -- `experimental[]` keeps its exact prose so nothing
/// that reads it breaks, and the top-level `"schema"` string stays honest,
/// since docs/json-schema-v1.md bumps the version only on BREAKING changes --
/// this addition, on its own, was not one (a later, unrelated stage 1b change
/// did bump it; see `format/json.rs`'s `SCHEMA` doc comment). This test
/// asserts the trap is still reproducible AND that the new field is free of it;
/// if the prose is ever made 0-based, the first half fails and this test should
/// be rewritten rather than deleted.
///
/// MUTATION: emit `i + 1` in `experimental_json` and the join assertion fails.
#[test]
fn compose_json_experimental_paths_join_the_slot_map() {
    const SHA: &str = "2d711642b726b04401627ca9fbac32f5c8530fb1903cc4db02258717921a4881";
    let out = md()
        .args([
            "compose",
            "--wrapper",
            "wsh",
            "--json",
            "--experimental",
            "--md-only",
            "--path",
            "2of3",
            "--path",
            &format!("keyless,sha256={SHA}"),
            "--path",
            "1of2,older=144",
        ])
        .assert()
        .success();
    let v: serde_json::Value = serde_json::from_slice(&out.get_output().stdout).unwrap();

    let slot_paths: Vec<u64> = v["slots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["path"].as_u64().unwrap())
        .collect();

    // The prose is 1-based and unchanged -- it is what stderr prints.
    assert_eq!(
        v["experimental"][0].as_str().unwrap(),
        "path 2 has no key (bearer access to whoever holds the preimage)"
    );
    // And it still collides: read as a slots[].path value, "2" has key slots.
    assert!(
        slot_paths.contains(&2),
        "the prose/slot-map collision this field exists to route around is gone; \
         re-read F-603 before changing this test (slot paths {slot_paths:?})"
    );

    // The joinable field is 0-based, and the join is TRUE: the key-less path
    // carries no key slots.
    let e = &v["experimental_paths"][0];
    assert_eq!(e["kind"].as_str().unwrap(), "keyless_path");
    let p = e["path"].as_u64().unwrap();
    assert_eq!(p, 1, "experimental_paths[].path must be 0-based");
    assert!(
        !slot_paths.contains(&p),
        "path {p} is reported key-less but holds key slots {slot_paths:?}"
    );

    // Unchanged for the no-experimental case: the new array is empty too.
    md().args([
        "compose",
        "--wrapper",
        "tr",
        "--json",
        "--path",
        "2of2",
        "--path",
        "1of1",
    ])
    .assert()
    .success()
    .stdout(predicates::prelude::predicate::str::contains(
        "\"experimental_paths\": []",
    ));
}
