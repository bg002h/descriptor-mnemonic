//! **W-A / W-B / W-PIN** — SPEC "Acceptance" as executable walks (plan §3 C4
//! item 1; roster row `W-A/B/C, W-PIN`).
//!
//! The pathological vault is a real 11-key wallet that exists in TWO backup
//! forms, both minted by the mnemonic-engrave journey and both committed here
//! as fixtures:
//!
//! * **the SPLIT set** — `fixtures/pathological/backup-strings.txt`, 6 md1
//!   chunks of a KEYLESS policy card + 30 mk1 chunks forming 11 key cards;
//! * **the KEYED card** — `fixtures/pathological/keyed-card.txt`, 22 md1
//!   chunks carrying the same eleven keys in md's `Pubkeys` TLV.
//!
//! ## The three legs, and where each is discharged
//!
//! **(a) — HERE.** `md descriptor <md1…> --from-mk1 <mk1…>` composes the split
//! set, and address 0 of the composition is the journey's own
//! `bc1qkuknuy6dsm0fq44cyyhzqy9wl3ex2n6ed39zxhx867l9wlh4yhlsejms64`.
//!
//! **(b) — HERE.** The 22-string keyed card composes SPEND-EQUAL to (a).
//!
//! **(c) — NOT HERE, and nothing new is asserted for it.** SPEC Acceptance
//! 1(c) (decompose∘compose round-trips ROUND-TRIP-EQUAL through
//! `md encode --key` + `mk encode --keys`) was discharged in full by C3's
//! V-D-RT rows in `tests/cmd_decompose_roundtrip.rs`, by name:
//!
//! * `v_d_rt_fixture_is_the_pinned_wallet_from_the_first_three_keys_txt_records`
//! * `v_d_rt_emissions_still_match_what_mk_consumed`
//! * `v_d_rt_key_lines_are_as_parsed_never_a_depth_zero_reserialisation`
//! * `v_d_rt_the_recorded_mint_commands_are_still_the_ones_decompose_emits`
//! * `v_d_rt_md_encode_key_accepts_the_emissions_and_reproduces_the_keyed_card`
//! * `v_d_rt_md_encode_reproduces_the_policy_card_from_the_live_template`
//! * `v_d_rt_round_trip_equality_through_the_keyed_card`
//! * `v_d_rt_round_trip_equality_through_the_split_set`
//! * `v_d_rt_both_routes_derive_the_same_address_zero_as_the_input`
//! * `v_d_rt_round_trip_equality_fails_when_one_key_is_swapped`
//! * `v_d_rt_round_trip_equality_fails_when_an_origin_is_altered`
//!
//! `walk_c_leg_c_rows_still_exist_in_cmd_decompose_roundtrip` below is the only
//! thing this file adds for (c): it greps that list out of the other file's
//! source, so the pointer above cannot rot into a comment describing tests
//! nobody kept. It re-asserts none of their content. (It earned its keep on
//! 2026-08-30: REVIEW-converter-whole-diff-r1 I5 replaced
//! `v_d_rt_mk_encode_keys_accepted_the_emitted_file` — a row that could not
//! fail — and this grep is what caught the stale pointer.)
//!
//! ## Why the oracle is not `seat::compose::spend_equal`
//!
//! An acceptance that asks the code under test whether it succeeded agrees
//! with itself by construction. Both relations are computed in
//! `tests/common/facts.rs` from a rust-miniscript parse of the emitted
//! descriptor STRING, and address 0 of leg (a)'s composition is derived from
//! that same string by rust-miniscript — not by md — so the pinned address
//! binds the descriptor md printed, not a second run of the engine.
//!
//! ## The reproduction pins (SPEC Acceptance 4), and one correction
//!
//! SPEC Acceptance 4 pins **1,648 characters (1,649 bytes with the trailing
//! newline)** to "the composed KEYED-CARD descriptor", and the keyed card to
//! 22 strings = 21×86 + one 59-char tail. Both are asserted below, exactly.
//!
//! The SPLIT set composes to **1,901** characters, not 1,648 — measured, and
//! not a defect: the split composition carries each slot's
//! `[fingerprint/path]` origin, which the keyed card does not (its TLV holds
//! key material, and the journey minted it without declared origins). Eleven
//! origins × 23 characters = 253, and 1,648 + 253 = 1,901.
//! `walk_a_the_split_composition_is_the_keyed_form_plus_eleven_origins`
//! asserts that arithmetic against the real strings rather than restating it.

#![allow(missing_docs)]

use assert_cmd::Command;
use miniscript::descriptor::{Descriptor, DescriptorPublicKey};
use std::str::FromStr;

#[path = "common/facts.rs"]
mod common_facts;
use common_facts::{facts, spend_equal, spend_equal_report};

/// The journey's own receive address 0 for the pathological vault — the
/// external pin every leg is measured against.
const PINNED_ADDRESS_0: &str = "bc1qkuknuy6dsm0fq44cyyhzqy9wl3ex2n6ed39zxhx867l9wlh4yhlsejms64";

/// SPEC Acceptance 4: the composed KEYED-CARD descriptor.
const KEYED_DESCRIPTOR_CHARS: usize = 1_648;

// ─── fixtures ───────────────────────────────────────────────────────────────

fn fixture_path(rel: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(rel)
}

/// Payload lines of a fixture — `#` comments and blanks are provenance.
fn payload_lines(rel: &str) -> Vec<String> {
    std::fs::read_to_string(fixture_path(rel))
        .unwrap_or_else(|e| panic!("fixture {rel} unreadable: {e}"))
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(String::from)
        .collect()
}

/// The 36-string split set: 6 md1 policy chunks, then 30 mk1 key chunks.
///
/// The shape is asserted HERE rather than only in a row of its own, so a
/// truncated or re-sourced fixture fails every walk that reads it instead of
/// quietly shrinking the input.
fn split_set() -> (Vec<String>, Vec<String>) {
    let lines = payload_lines("pathological/backup-strings.txt");
    let md1: Vec<String> = lines
        .iter()
        .filter(|l| l.starts_with("md1"))
        .cloned()
        .collect();
    let mk1: Vec<String> = lines
        .iter()
        .filter(|l| l.starts_with("mk1"))
        .cloned()
        .collect();
    assert_eq!(
        md1.len() + mk1.len(),
        lines.len(),
        "backup-strings.txt carries a line that is neither md1 nor mk1"
    );
    assert_eq!(md1.len(), 6, "the keyless policy card is 6 md1 chunks");
    assert_eq!(mk1.len(), 30, "the 11 key cards are 30 mk1 chunks");
    (md1, mk1)
}

/// The 22-string keyed card, with the **W-PIN shape asserted before it is
/// handed to any walk** (plan §3 C4 item 1: "a failed shape assertion is a
/// failed extraction, not a skipped test").
///
/// The fixture is DERIVED — `fixtures/pathological/extract-keyed-card.sh`
/// pulls it out of the journey page, where the card is rendered twice (44
/// tokens) — so this is the second of the two shape gates, the first being in
/// the extractor itself.
fn keyed_card() -> Vec<String> {
    let card = payload_lines("pathological/keyed-card.txt");
    assert_eq!(
        card.len(),
        22,
        "W-PIN: the keyed card is 22 strings; the extraction deduped wrong"
    );
    let lens: Vec<usize> = card.iter().map(|s| s.chars().count()).collect();
    assert_eq!(
        &lens[..21],
        &[86usize; 21],
        "W-PIN: the first 21 strings must each be 86 chars"
    );
    assert_eq!(lens[21], 59, "W-PIN: the 22nd string is the 59-char tail");
    assert!(
        card.iter().all(|s| s.starts_with("md1")),
        "the keyed card must be md1 strings"
    );
    card
}

// ─── the binary ─────────────────────────────────────────────────────────────

fn md(args: &[String]) -> (i32, String, String) {
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

/// `md descriptor <md1…> --from-mk1 <mk1…>` — the operator's leg-(a) command.
fn split_args(verb: &str) -> Vec<String> {
    let (md1, mk1) = split_set();
    let mut args = vec![verb.to_string()];
    args.extend(md1);
    for c in mk1 {
        args.push("--from-mk1".to_string());
        args.push(c);
    }
    args
}

/// `md descriptor <22 keyed md1 strings>` — the operator's leg-(b) command.
fn keyed_args(verb: &str) -> Vec<String> {
    let mut args = vec![verb.to_string()];
    args.extend(keyed_card());
    args
}

fn compose(args: Vec<String>) -> (String, String) {
    let (code, stdout, stderr) = md(&args);
    assert_eq!(code, 0, "composition refused:\n{stderr}");
    let first = stdout
        .lines()
        .next()
        .expect("a composition prints its descriptor on stdout")
        .to_string();
    (first, stderr)
}

/// Address 0 (chain 0, index 0) of a descriptor STRING, derived by
/// rust-miniscript in the test — md is not in this loop, so it is an
/// independent oracle on the text md printed.
fn address_zero_of(desc: &str) -> String {
    let d = Descriptor::<DescriptorPublicKey>::from_str(desc)
        .unwrap_or_else(|e| panic!("the composed descriptor must parse: {e}"));
    let receive = d
        .into_single_descriptors()
        .expect("multipath descriptor splits into single-path forms")
        .into_iter()
        .next()
        .expect("chain 0 exists");
    receive
        .derive_at_index(0)
        .expect("index 0 derives")
        .address(bitcoin::Network::Bitcoin)
        .expect("a wsh descriptor has an address")
        .to_string()
}

// ─── W-PIN: the reproduction pins (SPEC Acceptance 4) ───────────────────────

#[test]
fn walk_pin_the_keyed_card_is_22_strings_21x86_plus_a_59_char_tail() {
    // The gate on the DERIVED fixture. `keyed_card()` asserts the shape; this
    // row exists so the assertion is reported as itself rather than only as a
    // walk that happened to read a bad file.
    let card = keyed_card();
    assert_eq!(card.len(), 22);
    assert_eq!(
        card.iter().map(|s| s.chars().count()).sum::<usize>(),
        21 * 86 + 59
    );
}

#[test]
fn walk_pin_the_keyed_composition_is_1648_chars_and_1649_bytes() {
    // SPEC Acceptance 4, measured through the walk rather than transcribed.
    let (code, stdout, stderr) = md(&keyed_args("descriptor"));
    assert_eq!(code, 0, "{stderr}");
    let desc = stdout.lines().next().unwrap();
    assert_eq!(
        desc.chars().count(),
        KEYED_DESCRIPTOR_CHARS,
        "the composed keyed-card descriptor is pinned at 1,648 characters"
    );
    assert_eq!(desc.len(), KEYED_DESCRIPTOR_CHARS, "ASCII: chars == bytes");
    assert_eq!(
        stdout.len(),
        KEYED_DESCRIPTOR_CHARS + 1,
        "1,649 bytes with the trailing newline (String::len is bytes)"
    );
    assert!(stdout.ends_with('\n'));
}

// ─── W-A: the 36-string split set (SPEC Acceptance 1(a)) ────────────────────

#[test]
fn walk_a_the_split_set_composes_and_derives_the_pinned_address_zero() {
    let (desc, stderr) = compose(split_args("descriptor"));

    // The descriptor is COMPLETE: every one of the eleven slots carries a real
    // xpub, and no placeholder survived.
    assert!(
        !desc.contains('@'),
        "an unseated placeholder reached the output: {desc}"
    );
    assert_eq!(
        desc.matches("xpub").count(),
        11,
        "the pathological vault has eleven keys"
    );

    // The pin, derived from the composed TEXT by rust-miniscript.
    assert_eq!(
        address_zero_of(&desc),
        PINNED_ADDRESS_0,
        "the composed descriptor is not the journey's wallet"
    );

    // …and by md itself, through the same card channel the operator used.
    let (code, stdout, err) = md(&split_args("address"));
    assert_eq!(code, 0, "{err}");
    assert_eq!(stdout.lines().next().unwrap(), PINNED_ADDRESS_0);

    // SPEC B2: md volunteers the same address on stderr when composing.
    assert!(
        stderr.contains(PINNED_ADDRESS_0),
        "the B2 address-0 note is missing from stderr:\n{stderr}"
    );
}

#[test]
fn walk_a_the_split_composition_is_the_keyed_form_plus_eleven_origins() {
    // SPEC Acceptance 4 pins 1,648 characters to the KEYED-card composition.
    // The split composition is 1,901, and this row shows the difference is
    // exactly the origin metadata rather than a different wallet: strip the
    // eleven `[fingerprint/path]` brackets and 1,648 characters remain.
    let (split, _) = compose(split_args("descriptor"));
    let (keyed, _) = compose(keyed_args("descriptor"));

    assert_eq!(split.chars().count(), 1_901);
    assert_eq!(keyed.chars().count(), KEYED_DESCRIPTOR_CHARS);

    let origins = facts(&split).origins;
    assert_eq!(origins.len(), 11, "eleven slots");

    let mut stripped = split.clone();
    for origin in origins {
        assert_eq!(origin.chars().count(), 23, "each origin is 23 chars");
        assert!(
            stripped.contains(&origin),
            "origin {origin} is not in the composed descriptor"
        );
        stripped = stripped.replacen(&origin, "", 1);
    }
    assert_eq!(
        stripped.chars().count(),
        KEYED_DESCRIPTOR_CHARS,
        "1,648 + 11 origins x 23 chars = 1,901"
    );
    // Only the BIP-380 checksum still differs, because it is computed over the
    // origin-bearing text. Everything before the `#` is identical.
    let (a, b) = (
        stripped.split('#').next().unwrap(),
        keyed.split('#').next().unwrap(),
    );
    assert_eq!(a, b, "the two forms differ by more than their origins");
    assert_ne!(
        stripped, keyed,
        "the checksums are computed over different text and must differ"
    );
}

// ─── W-B: the 22-string keyed card (SPEC Acceptance 1(b)) ───────────────────

#[test]
fn walk_b_the_keyed_card_composes_spend_equal_to_the_split_set() {
    let (split, _) = compose(split_args("descriptor"));
    let (keyed, _) = compose(keyed_args("descriptor"));

    let (a, b) = (facts(&split), facts(&keyed));
    assert!(
        spend_equal(&a, &b),
        "SPEC Acceptance 1: the two forms are not SPEND-EQUAL\n{}",
        spend_equal_report(&a, &b)
    );
    assert_eq!(a.values.len(), 11, "eleven slots compared, not zero");
}

#[test]
fn walk_b_the_two_forms_are_not_round_trip_equal_because_only_one_declares_origins() {
    // The other half of SPEC Acceptance 1's TWO relations (r3 C2): an
    // origin-including relation fails here by design — the split set's cards
    // declare `[73c5da0a/48'/0'/N'/2']`, the keyed card declares nothing — and
    // that is exactly why spend-equality exists as a separate relation. If
    // this ever started holding, someone changed what a keyed card carries.
    let (split, _) = compose(split_args("descriptor"));
    let (keyed, _) = compose(keyed_args("descriptor"));
    let (a, b) = (facts(&split), facts(&keyed));
    assert!(spend_equal(&a, &b));
    assert_ne!(
        a.origins, b.origins,
        "ROUND-TRIP-EQUALITY must not hold across the two card forms"
    );
    assert!(
        a.origins.iter().all(|o| o.starts_with('[')),
        "every split-set slot declares an origin: {:?}",
        a.origins
    );
    assert!(
        b.origins.iter().all(|o| o == "-"),
        "the keyed card declares none: {:?}",
        b.origins
    );
}

#[test]
fn walk_b_the_keyed_card_derives_the_pinned_address_zero() {
    let (keyed, _) = compose(keyed_args("descriptor"));
    assert_eq!(address_zero_of(&keyed), PINNED_ADDRESS_0);

    let (code, stdout, err) = md(&keyed_args("address"));
    assert_eq!(code, 0, "{err}");
    assert_eq!(stdout.lines().next().unwrap(), PINNED_ADDRESS_0);
}

#[test]
fn walk_b_spend_equality_fails_when_two_slots_exchange_their_keys() {
    // The mutation guard on the relation itself. Without it `spend_equal`
    // could return `true` unconditionally and every row above would still
    // pass.
    //
    // The mutation EXCHANGES slots 0 and 1 rather than overwriting one with
    // the other: an overwrite leaves ten distinct keys where there were
    // eleven, which the STRUCTURE half catches on its own and so proves
    // nothing about the value half. After an exchange the eleven keys are
    // still eleven and still in the same positions of the same script, the
    // placeholder numbering (by first textual appearance) is unchanged, and
    // the origins are all `-` either way — so only the VALUE half can see it.
    // It is a real wallet change: `multi` is order-sensitive, so the swapped
    // form is a different script.
    let (keyed, _) = compose(keyed_args("descriptor"));
    let good = facts(&keyed);
    let xpubs: Vec<String> = {
        let d = Descriptor::<DescriptorPublicKey>::from_str(&keyed).unwrap();
        let rendered = format!("{d:#}");
        let mut v: Vec<String> = Vec::new();
        use miniscript::ForEachKey;
        d.for_each_key(|k| {
            v.push(k.to_string());
            true
        });
        v.sort_by_key(|k| rendered.find(k.as_str()).unwrap_or(usize::MAX));
        v.dedup();
        v
    };
    assert_eq!(xpubs.len(), 11);
    const HOLE: &str = "\u{1}HOLE\u{1}";
    let mutated = keyed
        .replacen(&xpubs[0], HOLE, 1)
        .replacen(&xpubs[1], &xpubs[0], 1)
        .replacen(HOLE, &xpubs[1], 1);
    assert_ne!(mutated, keyed, "the mutation must actually change the text");
    let bad = facts(mutated.split('#').next().unwrap());
    assert_eq!(
        good.structure, bad.structure,
        "the exchange is value-only, by construction"
    );
    assert_eq!(good.origins, bad.origins, "the exchange touches no origin");
    assert_eq!(good.values.len(), bad.values.len(), "still eleven keys");
    assert!(
        !spend_equal(&good, &bad),
        "exchanging two slots' keys must break SPEND-EQUALITY"
    );
}

// ─── (c): the pointer, kept honest ──────────────────────────────────────────

#[test]
fn walk_c_leg_c_rows_still_exist_in_cmd_decompose_roundtrip() {
    // SPEC Acceptance 1(c) is discharged by C3, not re-asserted here (plan §3
    // C4 item 1(c)). What this row protects is the POINTER: a comment naming
    // eleven tests goes stale silently, a grep does not.
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cmd_decompose_roundtrip.rs"),
    )
    .unwrap();
    let named = [
        "v_d_rt_fixture_is_the_pinned_wallet_from_the_first_three_keys_txt_records",
        "v_d_rt_emissions_still_match_what_mk_consumed",
        "v_d_rt_key_lines_are_as_parsed_never_a_depth_zero_reserialisation",
        "v_d_rt_the_recorded_mint_commands_are_still_the_ones_decompose_emits",
        "v_d_rt_md_encode_key_accepts_the_emissions_and_reproduces_the_keyed_card",
        "v_d_rt_md_encode_reproduces_the_policy_card_from_the_live_template",
        "v_d_rt_round_trip_equality_through_the_keyed_card",
        "v_d_rt_round_trip_equality_through_the_split_set",
        "v_d_rt_both_routes_derive_the_same_address_zero_as_the_input",
        "v_d_rt_round_trip_equality_fails_when_one_key_is_swapped",
        "v_d_rt_round_trip_equality_fails_when_an_origin_is_altered",
    ];
    let missing: Vec<&str> = named
        .iter()
        .copied()
        .filter(|n| !src.contains(&format!("fn {n}(")))
        .collect();
    assert!(
        missing.is_empty(),
        "SPEC Acceptance 1(c) rows this file points at no longer exist: {missing:?}"
    );
    assert_eq!(named.len(), 11);
}
