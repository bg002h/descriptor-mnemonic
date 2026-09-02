//! `md compose --preset` (F-453, SPEC_wallet_policy_composer.md §4d, C2): the
//! six archetypes as one-tap presets, mutually exclusive with `--path`, still
//! honouring `--wrapper`/`--experimental`/`--json`. Grammar:
//! `--preset <name>[,<k>of<n>]*[,<param>=<value>]*`.

use assert_cmd::Command;
use predicates::prelude::*;

fn md() -> Command {
    Command::cargo_bin("md").expect("md binary")
}

#[test]
fn preset_plain_multisig_matches_the_equivalent_path_list() {
    let preset = md()
        .args([
            "compose",
            "--wrapper",
            "wsh",
            "--preset",
            "plain-multisig,2of3",
        ])
        .output()
        .unwrap();
    let path = md()
        .args(["compose", "--wrapper", "wsh", "--path", "2of3"])
        .output()
        .unwrap();
    assert!(preset.status.success());
    assert_eq!(preset.stdout, path.stdout);
}

#[test]
fn preset_kofn_recovery_matches_the_equivalent_path_list_under_tr() {
    let preset = md()
        .args([
            "compose",
            "--wrapper",
            "tr",
            "--preset",
            "kofn-recovery,2of3,older=26280",
        ])
        .output()
        .unwrap();
    let path = md()
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
    assert!(preset.status.success());
    assert_eq!(preset.stdout, path.stdout);
}

#[test]
fn preset_tiered_recovery_and_decaying_multisig_and_hashlock_gated_compose() {
    md().args([
        "compose",
        "--wrapper",
        "wsh",
        "--preset",
        "tiered-recovery,2of2,1of2,older=26280",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains(
        "wsh(or_d(multi(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/1'/2'/<0;1>/*),and_v(v:multi(1,@2/48'/0'/2'/2'/<0;1>/*,@3/48'/0'/3'/2'/<0;1>/*),older(26280))))",
    ));
    md().args([
        "compose",
        "--wrapper",
        "wsh",
        "--preset",
        "decaying-multisig,2of2,1of1,older1=13140,older2=26280,after=1000000",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains(
        "wsh(or_i(and_v(v:multi(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/1'/2'/<0;1>/*),older(13140)),or_i(and_v(v:pkh(@2/48'/0'/2'/2'/<0;1>/*),older(26280)),and_v(v:pkh(@3/48'/0'/3'/2'/<0;1>/*),after(1000000)))))",
    ));
    let h = "a8".repeat(32);
    md().args([
        "compose",
        "--wrapper",
        "wsh",
        "--preset",
        &format!("hashlock-gated,sha256={h},older=26280"),
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains(format!(
        "sha256({h})),and_v(v:pkh(@1/48'/0'/1'/2'/<0;1>/*),older(26280))"
    )));
}

#[test]
fn preset_and_path_are_mutually_exclusive() {
    md().args([
        "compose",
        "--wrapper",
        "wsh",
        "--path",
        "2of3",
        "--preset",
        "plain-multisig,2of3",
    ])
    .assert()
    .failure()
    .code(2)
    .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn compose_refuses_when_neither_path_nor_preset_given() {
    md().args(["compose", "--wrapper", "wsh"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn preset_refuses_an_unknown_name() {
    md().args(["compose", "--wrapper", "wsh", "--preset", "frobnicate,2of3"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "expected one of plain-multisig, simple-timelocked-inheritance, kofn-recovery, tiered-recovery, hashlock-gated, decaying-multisig",
        ));
}

#[test]
fn preset_refuses_a_missing_parameter() {
    md().args([
        "compose",
        "--wrapper",
        "wsh",
        "--preset",
        "kofn-recovery,2of3",
    ])
    .assert()
    .failure()
    .code(1)
    .stderr(predicate::str::contains(
        "preset kofn-recovery needs older=<n>",
    ));
}

#[test]
fn preset_refuses_an_extra_parameter() {
    md().args([
        "compose",
        "--wrapper",
        "wsh",
        "--preset",
        "plain-multisig,2of3,older=10",
    ])
    .assert()
    .failure()
    .code(1)
    .stderr(predicate::str::contains(
        "preset plain-multisig admits no older= parameter",
    ));
    md().args([
        "compose",
        "--wrapper",
        "wsh",
        "--preset",
        "plain-multisig,2of3,1of1",
    ])
    .assert()
    .failure()
    .code(1)
    .stderr(predicate::str::contains(
        "preset plain-multisig needs exactly 1 <k>of<n> parameter, got 2",
    ));
}

#[test]
fn preset_propagates_a_parameter_the_constructor_rejects() {
    // kofn_recovery's own `blocks()` guard, not the CLI's --path pre-check:
    // exercises the SAME ComposeError::LockOutOfRange a hand-built --path list
    // would hit, propagated verbatim.
    md().args([
        "compose",
        "--wrapper",
        "wsh",
        "--preset",
        "kofn-recovery,2of3,older=70000",
    ])
    .assert()
    .failure()
    .code(1)
    .stderr(predicate::str::contains(
        "path 2: older in blocks needs 1..=65535",
    ));
}

#[test]
fn preset_every_non_plain_archetype_refuses_under_both_legacy_wrappers_spec_4d_shape() {
    // SPEC §4d: "under sh/sh(wsh) only the plain k-of-n preset is offered."
    // No CLI special-case is needed: every non-plain archetype's PathList
    // fails `validate`'s legacy-wrapper-shape check the same way a hand-built
    // --path list with the same shape would. R0 fidelity M-2: this was tested
    // for one of the ten (archetype, wrapper) pairs; all ten now run.
    let non_plain: [(&str, &str); 5] = [
        ("simple-timelocked-inheritance", "older=100"),
        ("kofn-recovery", "2of3,older=100"),
        ("tiered-recovery", "2of2,1of2,older=100"),
        (
            "hashlock-gated",
            "sha256=a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8,older=100",
        ),
        (
            "decaying-multisig",
            "2of2,1of1,older1=100,older2=200,after=1000",
        ),
    ];
    for wrapper in ["sh", "sh-wsh"] {
        for (name, params) in &non_plain {
            md().args(["compose", "--wrapper", wrapper, "--preset", &format!("{name},{params}")])
                .assert()
                .failure()
                .code(1)
                .stderr(predicate::str::contains(
                    "legacy wrappers hold one plain sorted multisig only (n >= 2, no lock, no hash); use wsh or tr",
                ));
        }
        md().args([
            "compose",
            "--wrapper",
            wrapper,
            "--preset",
            "plain-multisig,2of3",
        ])
        .assert()
        .success();
    }
}

#[test]
fn preset_decaying_multisig_propagates_preset_shape_refusals() {
    md().args([
        "compose",
        "--wrapper",
        "wsh",
        "--preset",
        "decaying-multisig,2of2,2of3,older1=2000,older2=1000,after=100",
    ])
    .assert()
    .failure()
    .code(1)
    .stderr(predicate::str::contains(
        "preset: decaying tiers must unlock progressively later (the second older must exceed the first)",
    ));
    md().args([
        "compose",
        "--wrapper",
        "wsh",
        "--preset",
        "decaying-multisig,1of2,2of3,older1=1000,older2=2000,after=100",
    ])
    .assert()
    .failure()
    .code(1)
    .stderr(predicate::str::contains(
        "preset: a decaying multisig decays: the recovery threshold cannot exceed the primary threshold",
    ));
}

#[test]
fn preset_unknown_name_wins_over_a_malformed_token() {
    // R0 fidelity I-3: `parse_preset` checks the NAME before parsing any
    // token, so an unknown name is reported even when a token is ALSO
    // malformed -- not "`2/3` is not <k>of<n>" for a name that was never a
    // preset in the first place.
    md().args(["compose", "--wrapper", "wsh", "--preset", "multisig,2/3"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "expected one of plain-multisig, simple-timelocked-inheritance, kofn-recovery, tiered-recovery, hashlock-gated, decaying-multisig",
        ));
}

#[test]
fn preset_refuses_a_duplicate_named_parameter() {
    md().args([
        "compose",
        "--wrapper",
        "wsh",
        "--preset",
        "kofn-recovery,2of3,older=100,older=200",
    ])
    .assert()
    .failure()
    .code(1)
    .stderr(predicate::str::contains(
        "preset kofn-recovery: `older=` given twice",
    ));
}

#[test]
fn preset_refuses_a_malformed_kofn_token() {
    md().args([
        "compose",
        "--wrapper",
        "wsh",
        "--preset",
        "plain-multisig,2/3",
    ])
    .assert()
    .failure()
    .code(1)
    .stderr(predicate::str::contains(
        "preset plain-multisig: `2/3` is not <k>of<n>",
    ));
}

#[test]
fn preset_refuses_a_kofn_magnitude_that_does_not_fit_a_small_number() {
    // R0 fidelity I-3: this is a DIFFERENT failure class from BadThreshold --
    // 300 does not fit u8 at all, so there is no (k, n) pair yet to name in
    // BadThreshold's "1 <= k <= n <= 9" wording. A value that DOES fit u8 but
    // violates that band (e.g. 2of15) already reaches the real constructor and
    // surfaces BadThreshold's own text verbatim (asserted below).
    md().args([
        "compose",
        "--wrapper",
        "wsh",
        "--preset",
        "plain-multisig,300of3",
    ])
    .assert()
    .failure()
    .code(1)
    .stderr(predicate::str::contains(
        "preset plain-multisig: k `300` is not a small number",
    ));
    md().args([
        "compose",
        "--wrapper",
        "wsh",
        "--preset",
        "plain-multisig,2of300",
    ])
    .assert()
    .failure()
    .code(1)
    .stderr(predicate::str::contains(
        "preset plain-multisig: n `300` is not a small number",
    ));
    md().args([
        "compose",
        "--wrapper",
        "wsh",
        "--preset",
        "plain-multisig,2of15",
    ])
    .assert()
    .failure()
    .code(1)
    .stderr(predicate::str::contains(
        "2-of-15 is not admitted (1 <= k <= n <= 9)",
    ));
}

#[test]
fn preset_refuses_a_non_numeric_named_value() {
    md().args([
        "compose",
        "--wrapper",
        "wsh",
        "--preset",
        "kofn-recovery,2of3,older=soon",
    ])
    .assert()
    .failure()
    .code(1)
    .stderr(predicate::str::contains(
        "preset kofn-recovery older: `soon` is not a number in 0..=4294967295",
    ));
}

#[test]
fn preset_refuses_a_missing_or_malformed_sha256() {
    md().args([
        "compose",
        "--wrapper",
        "wsh",
        "--preset",
        "hashlock-gated,older=1",
    ])
    .assert()
    .failure()
    .code(1)
    .stderr(predicate::str::contains(
        "preset hashlock-gated needs sha256=<64 hex>",
    ));
    md().args([
        "compose",
        "--wrapper",
        "wsh",
        "--preset",
        "hashlock-gated,sha256=ab,older=1",
    ])
    .assert()
    .failure()
    .code(1)
    .stderr(predicate::str::contains(
        "sha256 needs 64 hex characters, lowercase",
    ));
}

#[test]
fn preset_decaying_multisig_after_in_the_time_band_names_path_as_the_remedy() {
    // R0 fidelity M-3: decaying-multisig's `after` always builds a HEIGHT lock
    // (`presets::decaying_multisig` never emits `AfterTime`) and the preset
    // grammar has no `t` suffix to ask for a time lock -- unlike --path's
    // `after=<T>t`. A Unix-time-sized value therefore names --path, the only
    // way to express it, rather than a bare band refusal with no remedy.
    md().args([
        "compose",
        "--wrapper",
        "wsh",
        "--preset",
        "decaying-multisig,2of2,1of1,older1=100,older2=200,after=1893456000",
    ])
    .assert()
    .failure()
    .code(1)
    .stderr(predicate::str::contains(
        "after=1893456000 reads as a block height and is above the height band (1..=499999999); presets cannot express a Unix time -- use --path with `after=1893456000t` instead",
    ));
}

#[test]
fn preset_never_needs_experimental() {
    // Every presets::* key set is `sorted: true` and every archetype is keyed
    // (never a bare hash-only path), so `composed.experimental` is always
    // empty for a preset -- --experimental is accepted but never required.
    for args in [
        vec![
            "compose",
            "--wrapper",
            "wsh",
            "--preset",
            "plain-multisig,2of3",
        ],
        vec![
            "compose",
            "--wrapper",
            "tr",
            "--preset",
            "kofn-recovery,2of3,older=26280",
        ],
    ] {
        md().args(&args)
            .assert()
            .success()
            .stderr(predicate::str::contains("EXPERIMENTAL").not());
    }
}

#[cfg(feature = "json")]
#[test]
fn preset_json_names_the_preset_and_its_resolved_parameters() {
    let out = md()
        .args([
            "compose",
            "--wrapper",
            "tr",
            "--json",
            "--preset",
            "kofn-recovery,2of3,older=26280",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["preset"]["name"], "kofn-recovery");
    assert_eq!(v["preset"]["params"]["k"], 2);
    assert_eq!(v["preset"]["params"]["n"], 3);
    assert_eq!(v["preset"]["params"]["older_blocks"], 26280);
}

#[cfg(feature = "json")]
#[test]
fn path_json_names_no_preset() {
    let out = md()
        .args(["compose", "--wrapper", "wsh", "--json", "--path", "2of3"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v["preset"].is_null());
}
