//! F-227 — `md encode` advisory for a keyless template whose slots cannot be
//! told apart.
//!
//! WHAT THIS IS ABOUT. A keyless md1 template names its slots by origin. When
//! such a template is restored, each `@N` is filled from a gathered mk1 key
//! card, and SeedHammer II's seating rule (`gui/key_card_seating.go`) matches a
//! card to a slot on **the slot's declared origin, plus its fingerprint only
//! when the template declares one** — refusing every state it cannot decide.
//!
//! So a template with two slots carrying the *same declaration* is
//! **unseatable**: every card matches both slots, the device refuses
//! (`errSeatSlotContested`), and the operator discovers it only on attempting a
//! restore — after the plate is cut. `md encode` has every input needed to say
//! so at authoring time and said nothing.
//!
//! Found by building the hashlock-vault journey: six keys, one seed each, all
//! at `m/270028'/0'/0'/0'`. Not exotic — `48'/0'/0'/2'` is *the* standard
//! multisig account path, so every cosigner using it collides by default. Both
//! pathological journeys carry the same shape (11 slots, 4 distinct origins).
//!
//! THE ORACLE IS THE DEVICE'S RULE, NOT A HEURISTIC. Two slots are
//! indistinguishable iff `(fingerprint-when-declared, origin-path)` are equal —
//! the exact predicate `slotMatchesCard` applies. In particular a template that
//! declares fingerprints is fine even with identical paths, and a template with
//! distinct paths is fine with no fingerprints at all.
//!
//! WARN, DO NOT REFUSE. A bare template is legal and a user may deliberately
//! record slot order out of band. The advisory is stderr-only and encode stays
//! exit 0, matching the `--experimental` and pathless advisories.

#![allow(missing_docs)]

use std::process::Command as StdCommand;

/// The standard multisig account path — the collision that matters in practice.
const P: &str = "48'/0'/0'/2'";

fn encode(args: &[&str]) -> (String, String, i32) {
    let out = StdCommand::new(assert_cmd::cargo::cargo_bin("md"))
        .args(args)
        .output()
        .expect("invoke md encode");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().expect("md exited normally"),
    )
}

/// The advisory's distinguishing phrase. Kept in one place so a wording change
/// is a one-line edit rather than a hunt.
const NEEDLE: &str = "cannot be told apart";

#[test]
fn two_slots_at_one_path_with_no_fingerprints_warns() {
    let (stdout, stderr, code) = encode(&[
        "encode",
        "--group-size",
        "0",
        &format!("wsh(multi(2,@0/{P}/<0;1>/*,@1/{P}/<0;1>/*))"),
    ]);
    assert_eq!(code, 0, "advisory must not change the exit code");
    assert!(
        stdout.lines().any(|l| l.starts_with("md1")),
        "the card is still emitted: {stdout}"
    );
    assert!(
        stderr.contains(NEEDLE),
        "expected the unseatable-template advisory, got stderr: {stderr}"
    );
    // It must name WHICH slots, or the operator cannot act on it.
    assert!(
        stderr.contains("@0") && stderr.contains("@1"),
        "the advisory must name the colliding slots: {stderr}"
    );
    // And it must name the remedy.
    assert!(
        stderr.contains("--fingerprint"),
        "the advisory must name the fix: {stderr}"
    );
}

#[test]
fn declaring_fingerprints_silences_it() {
    let (_, stderr, code) = encode(&[
        "encode",
        "--group-size",
        "0",
        "--fingerprint",
        "@0=39ec1b6e",
        "--fingerprint",
        "@1=d02bcedf",
        &format!("wsh(multi(2,@0/{P}/<0;1>/*,@1/{P}/<0;1>/*))"),
    ]);
    assert_eq!(code, 0);
    assert!(
        !stderr.contains(NEEDLE),
        "distinct fingerprints make the slots decidable; no advisory is due: {stderr}"
    );
}

#[test]
fn distinct_origins_need_no_fingerprints() {
    let (_, stderr, code) = encode(&[
        "encode",
        "--group-size",
        "0",
        "wsh(multi(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/1'/2'/<0;1>/*))",
    ]);
    assert_eq!(code, 0);
    assert!(
        !stderr.contains(NEEDLE),
        "the origins already differ; no advisory is due: {stderr}"
    );
}

/// THE PARTIAL CASE, which a naive "are any fingerprints declared?" check gets
/// wrong. Declaring a fingerprint on ONE of two colliding slots leaves them
/// still indistinguishable: `slotMatchesCard` checks the fingerprint only when
/// the slot declares one, so a card matches the undeclared slot regardless of
/// whose key it is — and matches the declared slot too when it happens to be
/// that master's.
#[test]
fn one_fingerprint_of_two_is_still_unseatable() {
    let (_, stderr, code) = encode(&[
        "encode",
        "--group-size",
        "0",
        "--fingerprint",
        "@0=39ec1b6e",
        &format!("wsh(multi(2,@0/{P}/<0;1>/*,@1/{P}/<0;1>/*))"),
    ]);
    assert_eq!(code, 0);
    assert!(
        stderr.contains(NEEDLE),
        "a half-declared template is still unseatable: {stderr}"
    );
}

/// Three-way, and the message must not claim only two.
#[test]
fn three_colliding_slots_are_all_named() {
    let (_, stderr, code) = encode(&[
        "encode",
        "--group-size",
        "0",
        &format!("wsh(multi(2,@0/{P}/<0;1>/*,@1/{P}/<0;1>/*,@2/{P}/<0;1>/*))"),
    ]);
    assert_eq!(code, 0);
    for slot in ["@0", "@1", "@2"] {
        assert!(
            stderr.contains(slot),
            "slot {slot} missing from the advisory: {stderr}"
        );
    }
}

/// A KEYED card carries its own keys, so nothing is ever seated onto it and the
/// advisory would be noise. The colliding paths are identical to the warning
/// case above — only the presence of keys differs.
#[test]
fn a_keyed_card_is_not_warned() {
    const X0: &str = "xpub6E6MpT6SMBy1Ti3KJ9heS5M4vZnunHCZSrhWXa5YTn2psXd7RiWrafZ9aDAuaLQZRoLvJJLSr2SPi4tSoKvp6tdk15kMu7UPRHygVgz7aHz";
    const X1: &str = "xpub6BosfCnifzxcFwrSzQiqu2DBVTshkCXacvNsWGYJVVhhawA7d4R5WSWGFNbi8Aw6ZRc1brxMyWMzG3DSSSSoekkudhUd9yLb6qx39T9nMdj";
    let (stdout, stderr, code) = encode(&[
        "encode",
        "--group-size",
        "0",
        "--key",
        &format!("@0={X0}"),
        "--key",
        &format!("@1={X1}"),
        &format!("wsh(multi(2,@0/{P}/<0;1>/*,@1/{P}/<0;1>/*))"),
    ]);
    assert_eq!(code, 0, "stdout: {stdout}\nstderr: {stderr}");
    assert!(
        !stderr.contains(NEEDLE),
        "a keyed card is never seated onto; the advisory is noise here: {stderr}"
    );
}

/// The advisory must fire on the `--json` branch too. The pathless advisory
/// gained exactly this test after shipping without it, and a machine-readable
/// caller is the one least likely to notice a missing warning.
#[test]
fn json_branch_warns_too() {
    let (_, stderr, code) = encode(&[
        "encode",
        "--group-size",
        "0",
        "--json",
        &format!("wsh(multi(2,@0/{P}/<0;1>/*,@1/{P}/<0;1>/*))"),
    ]);
    assert_eq!(code, 0);
    assert!(
        stderr.contains(NEEDLE),
        "--json must warn on stderr as the text branch does: {stderr}"
    );
}
