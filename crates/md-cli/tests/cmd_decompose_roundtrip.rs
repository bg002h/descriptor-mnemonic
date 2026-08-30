//! **V-D-RT** — the acceptance-grade decompose round trip (plan §4; SPEC
//! Acceptance 1(c)).
//!
//! > `md decompose` of the PINNED depth-consistent fixture (r2 M5: the wallet
//! > built from the first three lines of the pathological `keys.txt`) emits a
//! > template + key file that `md encode --key` + `mk encode --keys` accept and
//! > that re-compose ROUND-TRIP-EQUAL.
//!
//! ## The two equality relations, asserted as two halves (SPEC Acceptance 1)
//!
//! **SPEND-EQUALITY** — canonicalised template structures equal AND per-slot
//! xpub VALUES and use-site paths equal; origin metadata EXCLUDED.
//! **ROUND-TRIP-EQUALITY** — spend-equality AND origin metadata preserved
//! exactly. Both halves are asserted separately here, so a regression in
//! either is reported as itself.
//!
//! "xpub VALUE" is the chain code and the public point, NOT the 111-character
//! serialisation. That distinction is the whole of r1 C3: md's `Pubkeys` TLV
//! carries 65 bytes (chain code ‖ compressed point) and md-codec reconstructs
//! a DEPTH-0 xpub from them, so a re-composed descriptor renders
//! `xpub661MyMwAqRbc…` where the input had `xpub6DkFAXWQ2dHxq…` — measured
//! 2026-08-30. Same spending key, same wallet, different string. An assertion
//! on the strings would fail on a wallet that is provably identical, which is
//! why the spec's relation is over values.
//!
//! ## Why mk runs in the GENERATOR and not in this test
//!
//! `mk` is a sibling repo's binary. This repo's CI runs
//! `cargo test --workspace --all-targets` and never builds it, so a test that
//! shelled out to mk would either red CI or skip — and a skipped gate prints
//! ok. `tests/fixtures/decompose/generate.sh` runs the REAL `mk encode --keys`
//! (its measured exit code is recorded in the fixture header) over the key
//! file `md decompose --emit keys` produced, and this test asserts
//! byte-for-byte that today's emission is still that file. So mk's acceptance
//! is a measurement, and any drift in what decompose emits fails here rather
//! than silently invalidating it. Same pattern as `tests/fixtures/seating/`.

#![allow(missing_docs)]

use assert_cmd::Command;
use miniscript::descriptor::DescriptorPublicKey;
use std::str::FromStr;

// ─── fixture access ─────────────────────────────────────────────────────────

fn fixture_path(rel: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(rel)
}

/// Read one `# @@ <name>` section of `v-d-rt.txt`.
fn section(name: &str) -> Vec<String> {
    let text = std::fs::read_to_string(fixture_path("decompose/v-d-rt.txt")).unwrap();
    let mut out = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        if let Some(h) = line.strip_prefix("# @@ ") {
            inside = h.trim() == name;
            continue;
        }
        if inside && !line.trim().is_empty() && !line.starts_with('#') {
            out.push(line.to_string());
        }
    }
    assert!(!out.is_empty(), "section `{name}` is empty or missing");
    out
}

fn one(name: &str) -> String {
    let v = section(name);
    assert_eq!(v.len(), 1, "section `{name}` must hold one line");
    v.into_iter().next().unwrap()
}

/// The first three records of the pathological `keys.txt` — the fixture's
/// provenance, read from the file rather than transcribed.
fn pinned_key_records() -> Vec<String> {
    std::fs::read_to_string(fixture_path("pathological/keys.txt"))
        .unwrap()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .take(3)
        .map(String::from)
        .collect()
}

fn md(args: &[&str]) -> (i32, String, String) {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(args)
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

/// TODAY's emissions, from the real binary. Every round-trip row below mints
/// from THESE, not from the committed fixture's copy: a row that re-minted the
/// fixture's own template would keep passing while the walker drifted, and the
/// only thing catching it would be
/// `v_d_rt_emissions_still_match_what_mk_consumed`. Mutation-checked
/// 2026-08-30: dropping the origin-path segment from `build_template` fails
/// 8 of the 30 `v_d_*` tests — 6 of them in this file — where before the
/// change it failed 3, only one of which was here.
fn live_emissions() -> (String, Vec<String>) {
    let input = one("input-descriptor");
    let (c1, template, e1) = md(&["decompose", &input, "--emit", "template"]);
    assert_eq!(c1, 0, "{e1}");
    let (c2, keys, e2) = md(&["decompose", &input, "--emit", "keys"]);
    assert_eq!(c2, 0, "{e2}");
    (
        template.trim().to_string(),
        keys.lines()
            .filter(|l| !l.trim().is_empty())
            .map(String::from)
            .collect(),
    )
}

/// `md encode` the KEYED card from the live emissions — decompose's route 1.
fn mint_keyed_card(template: &str, records: &[String]) -> Vec<String> {
    let mut args = vec!["encode".to_string(), template.to_string()];
    for (i, r) in records.iter().enumerate() {
        args.push("--key".into());
        args.push(format!("@{i}={}", r.split(']').next_back().unwrap()));
        if let Some(fp) = r.strip_prefix('[').and_then(|x| x.get(..8)) {
            args.push("--fingerprint".into());
            args.push(format!("@{i}={fp}"));
        }
    }
    args.push("--group-size".into());
    args.push("0".into());
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let (code, stdout, err) = md(&refs);
    assert_eq!(code, 0, "`md encode --key` refused the emissions: {err}");
    stdout
        .lines()
        .filter(|l| l.starts_with("md1"))
        .map(String::from)
        .collect()
}

/// `md encode` the KEYLESS policy card from the live emissions — route 2's
/// first step. The fingerprints are load-bearing: without them the split
/// re-composition renders bare xpubs and every origin is lost (measured).
fn mint_policy_card(template: &str, records: &[String]) -> Vec<String> {
    let mut args = vec!["encode".to_string(), template.to_string()];
    for (i, r) in records.iter().enumerate() {
        if let Some(fp) = r.strip_prefix('[').and_then(|x| x.get(..8)) {
            args.push("--fingerprint".into());
            args.push(format!("@{i}={fp}"));
        }
    }
    args.push("--group-size".into());
    args.push("0".into());
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let (code, stdout, err) = md(&refs);
    assert_eq!(code, 0, "`md encode` refused the keyless template: {err}");
    stdout
        .lines()
        .filter(|l| l.starts_with("md1"))
        .map(String::from)
        .collect()
}

// ─── the two relations ────────────────────────────────────────────────────
//
// `Facts` / `facts` / `spend_equal` moved to `tests/common/facts.rs` in C4 so
// the acceptance walks (`tests/acceptance_walks.rs`) grade SPEC Acceptance 1
// against the same relation this file does, rather than a second copy of it.
// The relation is still computed from rust-miniscript in the TEST, with no
// call into `src/decompose` or `src/seat`.

#[path = "common/facts.rs"]
mod common_facts;
use common_facts::{facts, spend_equal};

/// Assert ROUND-TRIP-EQUALITY, both halves separately.
fn assert_round_trip_equal(input: &str, recomposed: &str, route: &str) {
    let (a, b) = (facts(input), facts(recomposed));
    assert!(
        spend_equal(&a, &b),
        "[{route}] SPEND-EQUALITY failed\n  structure: {:?}\n         vs: {:?}\n  \
         values: {:?}\n      vs: {:?}\n  use-sites: {:?}\n         vs: {:?}",
        a.structure,
        b.structure,
        a.values,
        b.values,
        a.use_sites,
        b.use_sites
    );
    assert_eq!(
        a.origins, b.origins,
        "[{route}] ORIGIN metadata was not preserved exactly"
    );
    // The re-serialised keys are DIFFERENT strings and that is expected
    // (r1 C3). Asserting it keeps the relation honest: if md ever started
    // carrying depth on the wire, this line would fail and someone would have
    // to decide deliberately rather than tighten the relation by accident.
    assert_ne!(
        input, recomposed,
        "[{route}] the strings matched exactly — md's payload is supposed to \
         reconstruct a DEPTH-0 xpub (r1 C3); if that changed, revisit this test"
    );
}

// ─── provenance ─────────────────────────────────────────────────────────────

#[test]
fn v_d_rt_fixture_is_the_pinned_wallet_from_the_first_three_keys_txt_records() {
    let k = pinned_key_records();
    assert_eq!(k.len(), 3, "keys.txt must supply three records");
    let expected = format!(
        "wsh(sortedmulti(2,{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*))",
        k[0], k[1], k[2]
    );
    assert_eq!(
        one("input-descriptor"),
        expected,
        "the committed fixture is not the wallet SPEC Acceptance 1(c) pins"
    );
}

// ─── emission ───────────────────────────────────────────────────────────────

#[test]
fn v_d_rt_emissions_still_match_what_mk_consumed() {
    // This is the binding that makes the generated fixture mean something: mk
    // accepted THAT key file, and decompose must still emit it.
    let input = one("input-descriptor");
    let (c1, template, e1) = md(&["decompose", &input, "--emit", "template"]);
    assert_eq!(c1, 0, "{e1}");
    assert_eq!(template.trim(), one("template"));

    let (c2, keys, e2) = md(&["decompose", &input, "--emit", "keys"]);
    assert_eq!(c2, 0, "{e2}");
    let emitted: Vec<&str> = keys.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(emitted, section("keys"));

    let (c3, desc, e3) = md(&["decompose", &input, "--emit", "descriptor"]);
    assert_eq!(c3, 0, "{e3}");
    assert_eq!(desc.trim(), one("canonical-descriptor"));
}

#[test]
fn v_d_rt_key_lines_are_as_parsed_never_a_depth_zero_reserialisation() {
    // SPEC P3 "Key emission is round-trip-grade (r1 C3)". The emitted records
    // must be the input's own, byte for byte — mk's depth-consistency check
    // refuses a depth-0 re-serialisation, measured.
    let input = one("input-descriptor");
    let (code, keys, err) = md(&["decompose", &input, "--emit", "keys"]);
    assert_eq!(code, 0, "{err}");
    let emitted: Vec<String> = keys
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(String::from)
        .collect();
    assert_eq!(
        emitted,
        pinned_key_records(),
        "the key lines must be the keys.txt records verbatim"
    );
    for rec in &emitted {
        let k = DescriptorPublicKey::from_str(rec).unwrap();
        let (origin, xkey) = match k {
            DescriptorPublicKey::XPub(x) => (x.origin.unwrap(), x.xkey),
            _ => panic!("a bare record parses as XPub"),
        };
        assert_eq!(xkey.depth, 4, "depth was flattened in {rec}");
        // mk's own encoder-side invariant, restated independently of mk: the
        // xpub's depth is the origin path's component count, and its child
        // number is the path's terminal component. This is WHY mk accepted.
        let comps: &[bitcoin::bip32::ChildNumber] = origin.1.as_ref();
        assert_eq!(usize::from(xkey.depth), comps.len(), "{rec}");
        assert_eq!(xkey.child_number, *comps.last().unwrap(), "{rec}");
    }
}

/// The commands the fixture header RECORDS as having been run must still be
/// the commands `md decompose --emit commands` prints.
///
/// REVIEW-converter-whole-diff-r1 I5 replaced what stood here. The old
/// `v_d_rt_mk_encode_keys_accepted_the_emitted_file` asserted three things:
/// that the `mk1-cards` lines start with `mk1`, and that the fixture's own
/// COMMENT HEADER contains two literal substrings. The last two grep a file
/// for text that file was written with, so they passed whether or not `mk`
/// accepted anything — a PASS for a property the test never exercised — and
/// one of them announced "the fixture must record mk's measured exit code"
/// while looking for provenance prose.
///
/// What actually establishes SPEC Acceptance 1(c) is three other things, none
/// of which needs mk in the test process:
///
///  1. `generate.sh` RUNS the real `mk encode --keys` and aborts unless it
///     exits 0 (`[ "$R2" -eq 0 ] || exit 1`), so the fixture cannot be written
///     from a failed mk;
///  2. `v_d_rt_emissions_still_match_what_mk_consumed` asserts decompose still
///     emits, byte for byte, the key file mk consumed;
///  3. `v_d_rt_round_trip_equality_through_the_split_set` seats the mk1 cards
///     mk minted and asserts round-trip equality with the input descriptor.
///
/// The gap none of those covers is the one this row now closes: the header's
/// RECORDED commands are a claim about a reproduction path, and a claim in a
/// fixture header has to be a measurement or it is decoration. If decompose's
/// `--emit commands` output drifts, the recorded commands become a stale
/// record of a run nobody can repeat — and nothing else would notice, because
/// every other row consumes the fixture's SECTIONS rather than its header.
#[test]
fn v_d_rt_the_recorded_mint_commands_are_still_the_ones_decompose_emits() {
    let text = std::fs::read_to_string(fixture_path("decompose/v-d-rt.txt")).unwrap();
    let recorded = |start: &str, end: Option<&str>| -> Vec<String> {
        let mut out = Vec::new();
        let mut inside = false;
        for line in text.lines() {
            if line.trim_end() == start {
                inside = true;
                continue;
            }
            if !inside {
                continue;
            }
            match end {
                Some(e) if line.trim_end() == e => break,
                _ => {}
            }
            if line.trim().is_empty() {
                break;
            }
            match line.strip_prefix("#   ") {
                Some(cmd) => out.push(cmd.to_string()),
                None => break,
            }
        }
        out
    };
    let rec1 = recorded("# Route 1 (keyed card):", Some("# Route 2 (split set):"));
    let rec2 = recorded("# Route 2 (split set):", None);
    assert!(
        !rec1.is_empty() && !rec2.is_empty(),
        "header records no routes"
    );

    // The SAME extraction generate.sh performs on `--emit commands`, in Rust:
    //   route 1: awk '/^md encode /{f=1} f&&NF==0{exit} f'
    //   route 2: awk 'BEGIN{n=0} /^md encode /{n++} n==2{print}'
    let (code, commands, err) = md(&["decompose", &one("input-descriptor"), "--emit", "commands"]);
    assert_eq!(code, 0, "{err}");
    let mut live1: Vec<String> = Vec::new();
    let mut live2: Vec<String> = Vec::new();
    let mut n = 0usize;
    let mut in1 = false;
    for line in commands.lines() {
        if line.starts_with("md encode ") {
            n += 1;
            if n == 1 {
                in1 = true;
            }
        }
        if in1 {
            if line.trim().is_empty() {
                in1 = false;
            } else {
                live1.push(line.to_string());
            }
        }
        if n >= 2 {
            live2.push(line.to_string());
        }
    }
    assert_eq!(
        rec1, live1,
        "route 1 drifted: the fixture header records a command decompose no \
         longer emits, so the recorded run cannot be repeated"
    );
    assert_eq!(
        rec2, live2,
        "route 2 drifted: the fixture header records a command decompose no \
         longer emits, so the recorded mk run cannot be repeated"
    );
    // The recorded route 2 is the one that consumed the key file, and it must
    // still name mk's file-driven flag pair — the whole point of 1(c).
    assert!(
        rec2.iter()
            .any(|l| l.contains("mk encode --keys") && l.contains("--from-md1-set")),
        "route 2 no longer runs `mk encode --keys … --from-md1-set`: {rec2:?}"
    );
}

// ─── the round trip, both routes ────────────────────────────────────────────

#[test]
fn v_d_rt_md_encode_key_accepts_the_emissions_and_reproduces_the_keyed_card() {
    let (template, records) = live_emissions();
    assert_eq!(
        mint_keyed_card(&template, &records),
        section("keyed-card"),
        "the keyed card drifted from the one the generator minted"
    );
}

#[test]
fn v_d_rt_md_encode_reproduces_the_policy_card_from_the_live_template() {
    let (template, records) = live_emissions();
    assert_eq!(
        mint_policy_card(&template, &records),
        section("policy-card"),
        "the keyless policy card drifted from the one the generator minted"
    );
}

#[test]
fn v_d_rt_round_trip_equality_through_the_keyed_card() {
    let (template, records) = live_emissions();
    let mut args = vec!["descriptor".to_string()];
    args.extend(mint_keyed_card(&template, &records));
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let (code, stdout, err) = md(&refs);
    assert_eq!(code, 0, "{err}");
    let recomposed = stdout.lines().next().unwrap().to_string();
    assert_round_trip_equal(&one("input-descriptor"), &recomposed, "keyed card");
}

#[test]
fn v_d_rt_round_trip_equality_through_the_split_set() {
    // The SPLIT route: a keyless policy card minted from TODAY's template,
    // seated with the mk1 cards the real `mk encode --keys` minted from
    // decompose's own key file (fixture; see the module doc for why mk runs in
    // the generator).
    let (template, records) = live_emissions();
    let mut args = vec!["descriptor".to_string()];
    args.extend(mint_policy_card(&template, &records));
    for c in section("mk1-cards") {
        args.push("--from-mk1".to_string());
        args.push(c);
    }
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let (code, stdout, err) = md(&refs);
    assert_eq!(code, 0, "{err}");
    let recomposed = stdout.lines().next().unwrap().to_string();
    assert_round_trip_equal(&one("input-descriptor"), &recomposed, "split set");
}

#[test]
fn v_d_rt_both_routes_derive_the_same_address_zero_as_the_input() {
    // SPEC B2's oracle, and an independent cross-check on `Facts`: a relation
    // computed from parsed structure could agree while the wallets differ, so
    // the address is derived by md itself, three ways.
    let want = address_of_template_and_keys();
    for (route, args) in [
        ("keyed card", {
            let (t, r) = live_emissions();
            let mut a = vec!["address".to_string()];
            a.extend(mint_keyed_card(&t, &r));
            a
        }),
        ("split set", {
            let (t, r) = live_emissions();
            let mut a = vec!["address".to_string()];
            a.extend(mint_policy_card(&t, &r));
            for c in section("mk1-cards") {
                a.push("--from-mk1".to_string());
                a.push(c);
            }
            a
        }),
    ] {
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let (code, stdout, err) = md(&refs);
        assert_eq!(code, 0, "[{route}] {err}");
        assert_eq!(
            stdout.lines().next().unwrap(),
            want,
            "[{route}] address 0 differs from the input wallet's"
        );
    }
}

/// Address 0 of the INPUT wallet, derived by md from the decompose emissions
/// through the `--template` + `--key` channel (no card in the loop).
fn address_of_template_and_keys() -> String {
    let (template, records) = live_emissions();
    let xpubs: Vec<String> = records
        .iter()
        .map(|r| r.split(']').next_back().unwrap().to_string())
        .collect();
    let args: Vec<String> = vec![
        "address".into(),
        "--template".into(),
        template,
        "--key".into(),
        format!("@0={}", xpubs[0]),
        "--key".into(),
        format!("@1={}", xpubs[1]),
        "--key".into(),
        format!("@2={}", xpubs[2]),
    ];
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let (code, stdout, err) = md(&refs);
    assert_eq!(code, 0, "{err}");
    stdout.lines().next().unwrap().to_string()
}

// ─── the negative halves ────────────────────────────────────────────────────

#[test]
fn v_d_rt_round_trip_equality_fails_when_one_key_is_swapped() {
    // Without this the relation could return "equal" unconditionally and every
    // row above would still pass. Swap slot 0's key for slot 1's: the
    // structure and origins still match, so only the VALUE half can catch it.
    let k = pinned_key_records();
    let x1 = k[1].split(']').next_back().unwrap();
    let mutated = format!(
        "wsh(sortedmulti(2,[73c5da0a/48'/0'/0'/2']{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*))",
        x1, k[1], k[2]
    );
    let a = facts(&one("input-descriptor"));
    let b = facts(&mutated);
    assert_eq!(a.structure, b.structure, "the mutation is value-only");
    assert_eq!(a.origins, b.origins, "the mutation is value-only");
    assert!(
        !spend_equal(&a, &b),
        "a swapped key must break spend-equality"
    );
}

#[test]
fn v_d_rt_round_trip_equality_fails_when_an_origin_is_altered() {
    // The ORIGIN half, on its own: same keys, same structure, one origin path
    // changed. Spend-equality must still hold (origins are excluded from it) —
    // and round-trip-equality must not.
    let k = pinned_key_records();
    let x0 = k[0].split(']').next_back().unwrap();
    let mutated = format!(
        "wsh(sortedmulti(2,[73c5da0a/48'/0'/9'/2']{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*))",
        x0, k[1], k[2]
    );
    let a = facts(&one("input-descriptor"));
    let b = facts(&mutated);
    assert!(
        spend_equal(&a, &b),
        "origin metadata is EXCLUDED from spend-equality (SPEC Acceptance 1)"
    );
    assert_ne!(
        a.origins, b.origins,
        "an altered origin must break round-trip-equality"
    );
}
