//! F-449 stage 2, Task 1: `md compose --unspendable liana|nums`.
//!
//! The flag selects the taproot internal key's KIND at the one decision site
//! (`md_codec::compose::tr::lower_tr`) when no path supplies a real key.
//! Omitting it must be byte-identical to md-cli 0.18.0, which is asserted
//! against a golden generated from that release, not against this binary.

#![allow(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};
use std::process::Command as StdCommand;

/// Run `md` with `args`, returning `(stdout, stderr, exit code)`. Every call
/// site must bind and check all three — a bare `md(&[...]);` asserts nothing.
///
/// DEFINED HERE, deliberately: md-cli's tests have NO shared helper, and the
/// two that exist have DIFFERENT signatures — `cli_compose.rs:8` is
/// `fn md() -> Command` (assert_cmd), `liana_input_side.rs:33` is this one.
/// Copied verbatim from the latter rather than invented, so one vocabulary
/// covers both Liana test files. Omitting it is an E0425 the build gate
/// caught in this plan's own first draft.
fn md(args: &[&str]) -> (String, String, i32) {
    let out = StdCommand::new(assert_cmd::cargo::cargo_bin("md"))
        .args(args)
        .output()
        .expect("invoke md");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().expect("md exited normally"),
    )
}

/// The regression floor. Composing WITHOUT the flag must not move a byte,
/// for every vendored preset — this is what lets the flag ship without a
/// re-vector of the whole corpus.
#[test]
fn omitting_the_flag_is_byte_identical_to_the_previous_release() {
    // The golden is generated ONCE from md-cli 0.18.0 (pre-flag) by
    // `scripts/gen-compose-golden.sh` and committed. It is an external fact,
    // so a parse bug in THIS binary cannot move both sides of the comparison.
    // Read at RUNTIME, not `include_str!`. A compile-time include makes a
    // missing golden a BUILD error, which invites stubbing the file to get
    // green -- and a stub blessed by this binary is the exact defect the
    // golden exists to prevent. At runtime a missing golden is a loud,
    // specific test failure instead.
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/golden/compose_pre_unspendable.json"
    );
    let raw = std::fs::read_to_string(path).unwrap_or_else(|e| {
        panic!("golden missing ({e}) -- run scripts/gen-compose-golden.sh with the PRE-flag md and commit it: {path}")
    });
    let golden: BTreeMap<String, String> = serde_json::from_str(&raw).expect("golden parses");

    // The value after `--wrapper`, matched as a VALUE rather than by
    // `args.contains("tr")` (R2 N-h) -- a generator that ever emits
    // `--wrapper=tr` would make a contains() guard false for every row and
    // silently retire the explicit-nums comparison below.
    fn value_after<'a>(args: &[&'a str], flag: &str) -> Option<&'a str> {
        if let Some(i) = args.iter().position(|a| *a == flag) {
            return args.get(i + 1).copied();
        }
        let eq = format!("{flag}=");
        args.iter().find_map(|a| a.strip_prefix(eq.as_str()))
    }

    // Coverage by SET, not by length (R1 N-e, R2 M-h). `len() >= 6` cannot
    // tell the correct 14-row golden from a `tr`-only golden of six -- which
    // is exactly the shrunken floor a careless recovery from Task 1b would
    // produce, and it would still report green.
    let mut wrappers: BTreeSet<&str> = BTreeSet::new();
    let mut presets: BTreeSet<&str> = BTreeSet::new();
    for invocation in golden.keys() {
        let args: Vec<&str> = invocation.split(' ').collect();
        wrappers.insert(value_after(&args, "--wrapper").expect("row names a wrapper"));
        let spec = value_after(&args, "--preset").expect("row names a preset");
        presets.insert(spec.split(',').next().unwrap_or(spec));
    }
    assert_eq!(wrappers.len(), 4, "the floor lost a wrapper: {wrappers:?}");
    assert_eq!(presets.len(), 6, "the floor lost a preset: {presets:?}");
    assert_eq!(
        golden.len(),
        14,
        "the floor is the 14 exit-0 cells measured at md-cli 0.18.0, got {}",
        golden.len()
    );

    for (invocation, expected) in &golden {
        let args: Vec<&str> = invocation.split(' ').collect();
        let (out, err, code) = md(&args);
        assert_eq!(code, 0, "{invocation}: {err}");
        assert_eq!(&out, expected, "{invocation} moved against 0.18.0");
        // Explicit `nums` must equal the default -- but ONLY for `tr`.
        // RULING (R2 NEW-I-1): Task 1b refuses `--unspendable` ENTIRELY under
        // wsh/sh/sh-wsh -- the flag, not just the `liana` value -- so
        // appending it to those rows would assert exit 0 on an invocation
        // this stage deliberately makes fail. That refusal is only
        // expressible because Step 7 declares `Option<String>` with NO clap
        // default: OMITTING the flag stays exit 0 everywhere, which is what
        // keeps the eight non-`tr` rows above green.
        if value_after(&args, "--wrapper") == Some("tr") {
            let mut with = args.clone();
            with.extend_from_slice(&["--unspendable", "nums"]);
            let (out2, err2, code2) = md(&with);
            assert_eq!(code2, 0, "{invocation} --unspendable nums: {err2}");
            assert_eq!(out, out2, "{invocation}: explicit nums != default");
        }
    }
}

/// `liana` swaps the internal key's KIND and nothing else: the taptree, the
/// slot numbering and the origins are untouched. Asserted by diffing the two
/// outputs rather than by matching a shape, so any collateral change fails.
#[test]
fn liana_changes_the_internal_key_and_nothing_else() {
    let (nums, nerr, ncode) = md(&[
        "compose",
        "--wrapper",
        "tr",
        "--preset",
        "kofn-recovery,2of3,older=26280",
        "--unspendable",
        "nums",
    ]);
    let (liana, _, code) = md(&[
        "compose",
        "--wrapper",
        "tr",
        "--preset",
        "kofn-recovery,2of3,older=26280",
        "--unspendable",
        "liana",
    ]);
    assert_eq!(ncode, 0, "compose --unspendable nums failed: {nerr}");
    assert_eq!(code, 0, "compose --unspendable liana failed");
    assert!(
        liana.contains("UNSPENDABLE(liana)"),
        "kind 1 not rendered: {liana}"
    );
    assert!(
        !liana.contains("50929b74"),
        "kind 1 must not render NUMS hex: {liana}"
    );
    // Everything after the internal key is identical.
    let tail = |s: &str| {
        s.split_once(',')
            .map(|(_, t)| t.to_string())
            .unwrap_or_default()
    };
    assert_eq!(
        tail(&nums),
        tail(&liana),
        "the flag changed more than the internal key"
    );
}

// ---------------------------------------------------------------------------
// Task 1b: the flag must not be a silent no-op anywhere it cannot apply.
// ---------------------------------------------------------------------------

/// The one preset per wrapper that composes there at exit 0 (R3 N-2: under
/// `sh`/`sh-wsh` only `plain-multisig` does). `tr` uses `kofn-recovery`,
/// which `liana` can actually apply to (Task 2 refuses `plain-multisig`'s
/// `sortedmulti_a` leaf under kind 1).
const WRAPPER_PRESETS: [(&str, &str); 4] = [
    ("tr", "kofn-recovery,2of3,older=26280"),
    ("wsh", "plain-multisig,2of3"),
    ("sh-wsh", "plain-multisig,2of3"),
    ("sh", "plain-multisig,2of3"),
];

/// RULING (R2 NEW-I-1): the refusal is on the FLAG, not on its value.
/// `--unspendable nums --wrapper wsh` is refused too -- a flag that cannot
/// affect the output is refused, not quietly honoured -- and the FLAGLESS
/// invocation must still exit 0 under every wrapper, which is what catches
/// a refusal gated on the wrong thing.
#[test]
fn unspendable_is_refused_under_every_non_tr_wrapper_and_only_there() {
    for (wrapper, preset) in WRAPPER_PRESETS {
        let base = ["compose", "--wrapper", wrapper, "--preset", preset];
        let (out, err, code) = md(&base);
        assert_eq!(code, 0, "flagless --wrapper {wrapper}: {err}");
        assert!(
            !out.is_empty(),
            "flagless --wrapper {wrapper} printed nothing"
        );
        for value in ["nums", "liana"] {
            let mut args = base.to_vec();
            args.extend_from_slice(&["--unspendable", value]);
            let (out, err, code) = md(&args);
            if wrapper == "tr" {
                assert_eq!(code, 0, "--wrapper tr --unspendable {value}: {err}");
                continue;
            }
            assert_eq!(
                code, 1,
                "--wrapper {wrapper} --unspendable {value} must be refused"
            );
            assert!(out.is_empty(), "a refusal printed a template: {out}");
            assert!(
                err.contains(&format!("--wrapper {wrapper}")) && err.contains("taproot"),
                "the refusal must name the wrapper and say why: {err}"
            );
        }
    }
}

/// `--unspendable`'s value is matched exactly. A case variant or prefix is a
/// DIFFERENT wallet guessed for the operator, so it is refused naming both.
#[test]
fn unspendable_value_is_matched_exactly() {
    for bad in ["Liana", "NUMS", "lia", "", "liana "] {
        let (out, err, code) = md(&[
            "compose",
            "--wrapper",
            "tr",
            "--preset",
            "kofn-recovery,2of3,older=26280",
            "--unspendable",
            bad,
        ]);
        assert_eq!(code, 1, "--unspendable {bad:?} must be refused: {out}");
        assert!(out.is_empty(), "refusal printed: {out}");
        assert!(
            err.contains("expected nums or liana"),
            "--unspendable {bad:?}: {err}"
        );
    }
}

/// §4a's compose half (R0 I-4): `internal_key_path: null` alone cannot tell a
/// NUMS composition from a Liana one. `unspendable_kind` uses decode's own
/// vocabulary (`format/json.rs`, `JsonBody::Tr`): `"liana_unspendable"` for
/// wire kind 1, ABSENT for NUMS and for a real key -- read from the COMPOSED
/// descriptor, so it reports what was built, not what was asked for.
#[test]
fn compose_json_carries_the_third_state() {
    let json = |preset: &str, kind: &str| -> serde_json::Value {
        let (out, err, code) = md(&[
            "compose",
            "--wrapper",
            "tr",
            "--preset",
            preset,
            "--json",
            "--unspendable",
            kind,
        ]);
        assert_eq!(code, 0, "{preset} --unspendable {kind}: {err}");
        serde_json::from_str(&out).unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out}"))
    };
    let kofn = "kofn-recovery,2of3,older=26280";
    let liana = json(kofn, "liana");
    assert_eq!(liana["internal_key_path"], serde_json::Value::Null);
    assert_eq!(liana["unspendable_kind"], "liana_unspendable", "{liana}");
    let nums = json(kofn, "nums");
    assert_eq!(nums["internal_key_path"], serde_json::Value::Null);
    assert!(
        nums.get("unspendable_kind").is_none(),
        "NUMS must leave the field ABSENT, as decode does: {nums}"
    );
    // A real key was extracted: liana was asked for and could not apply.
    let real = json("simple-timelocked-inheritance,older=26280", "liana");
    assert_eq!(real["internal_key_path"], 0);
    assert!(
        real.get("unspendable_kind").is_none(),
        "a real internal key has no unspendable kind: {real}"
    );
}
