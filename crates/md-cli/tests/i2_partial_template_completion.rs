#![allow(missing_docs)]
//! **A PARTIAL template completed on the host keeps the host-seated slot's
//! origin** — composer fable review r0, lens 3, I-2, case C13.
//!
//! ## What was wrong
//!
//! A composer template minted with an UNSEATED slot (SPEC §8p) declares that
//! slot's origin but no fingerprint, because the key was not known yet. The
//! host seating engine filled only `tlv.pubkeys` and inherited the
//! fingerprint-free declaration as a choice. It was not a choice: nobody
//! chose, the key simply was not there.
//!
//! The consequence is not cosmetic. `to_miniscript.rs` drops the WHOLE origin
//! when the fingerprint is absent (`e.fingerprint.map(…)`), so the completed
//! wallet rendered its third key as a bare xpub with no `[fp/path]` at all —
//! a BIP-380 key no coordinator and no HWI can attribute to a signer — and
//! its wallet id differed from the id the device mints for the same three
//! keys seated on-device. SPEC §5 l.218: *"EVERY slot declares an origin
//! (§4f) and, when seated, the master fingerprint of the seated key."*
//!
//! ## What is pinned here
//!
//! The two facts that were false, stated as the report states them:
//!
//! - the completed descriptor carries `[fp/path]` on the host-seated key, and
//!   the emitted card's `tlv.fingerprints` has an entry for every slot;
//! - the completed card's wallet id EQUALS the id of the same policy composed
//!   FULLY SEATED with the same three keys — built here, not quoted.
//!
//! The rule that does NOT change — a declared fingerprint is never
//! overwritten — is pinned in `src/seat/compose.rs`'s unit tests, where the
//! assignment that would overwrite one can actually be constructed (A2
//! refuses it before `compose` ever sees it, so no CLI input reaches that
//! code path).
//!
//! MUTATION: drop the fill in `seat::compose::compose` -> the first test's
//! `[fp/path]` assertion fails and the id equality in the second fails with
//! two different ids.

use assert_cmd::Command;

const V_PARTIAL_C13: &str = include_str!("fixtures/seating/v-partial-c13.txt");

/// The fixture's own mint command, from its provenance header. Asserted
/// against the file by `the_fixture_header_still_records_this_mint`, so a
/// regenerated fixture cannot leave the control below building a different
/// wallet than the one the fixture seats.
const C13_TPL: &str =
    "wsh(sortedmulti(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/0'/2'/<0;1>/*,@2/48'/0'/1'/2'/<0;1>/*))";

/// `(xpub, declared fingerprint)` for @0, @1 and @2, in slot order — KEY 1,
/// KEY 5 and KEY 10 of `fixtures/pathological/keys.txt`. @2 is the slot the
/// TEMPLATE leaves unseated; the fully-seated control declares it.
const SLOTS: [(&str, &str); 3] = [
    (
        "xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf",
        "73c5da0a",
    ),
    (
        "xpub6FQya7zGhR92kacYsNnjreouvnHJMpXYsUXnW6NJJAJRCKsa26TzDy4LdnGhEurr3d6y1J8PJ7EEMKQp74XTqYvmGJNogYXSKDszYHtF8mX",
        "b8688df1",
    ),
    (
        "xpub6F6gx8ZP9R3R3eYsU2PeS5EPJ4jN7Wbt9uwyHNXLoaJxQjNT92FGAfCNDjUDRhCHwzjfgDuqAZ7Gk9SugPRMa6A8PnzLVvnyEKBW9jHRGRp",
        "28645006",
    ),
];

/// The fingerprint of the key the HOST seats into the unseated slot.
const HOST_FP: &str = "28645006";

fn md() -> Command {
    Command::cargo_bin("md").unwrap()
}

fn lines(text: &str, hrp: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| l.starts_with(hrp))
        .map(str::to_string)
        .collect()
}

/// `md <verb> <policy md1…> --from-mk1 <each mk1> [extra…]`.
fn seat_cmd(verb: &str, extra: &[&str]) -> Command {
    let mut c = md();
    c.arg(verb);
    for p in lines(V_PARTIAL_C13, "md1") {
        c.arg(p);
    }
    for k in lines(V_PARTIAL_C13, "mk1") {
        c.arg("--from-mk1").arg(k);
    }
    for e in extra {
        c.arg(e);
    }
    c
}

fn stdout_of(mut c: Command) -> String {
    let out = c.output().unwrap();
    assert!(
        out.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn the_fixture_header_still_records_this_mint() {
    // The control's template is hard-coded above so the test can build the
    // fully-seated wallet without a second fixture. If the fixture is ever
    // regenerated from a different template, that constant goes stale
    // silently and the id comparison starts comparing two wallets nobody
    // built together.
    assert!(
        V_PARTIAL_C13.contains(C13_TPL),
        "the fixture no longer records the template this test builds against"
    );
    for (_, fp) in SLOTS.iter().take(2) {
        assert!(
            V_PARTIAL_C13.contains(&format!("--fingerprint @0={fp}"))
                || V_PARTIAL_C13.contains(&format!("--fingerprint @1={fp}")),
            "the fixture no longer declares {fp}"
        );
    }
    assert!(
        V_PARTIAL_C13.contains(&format!("--origin-fingerprint {HOST_FP}")),
        "the fixture no longer mints the host completion card at {HOST_FP}"
    );
}

#[test]
fn the_completed_descriptor_carries_an_origin_on_the_host_seated_key() {
    let desc = stdout_of(seat_cmd("descriptor", &[]));
    // Every slot, not just the two the template declared.
    for (xpub_hint, fp) in SLOTS {
        let short = &xpub_hint[..8];
        assert!(
            desc.contains(&format!("[{fp}/")),
            "no origin for {fp} (key {short}…) in:\n{desc}"
        );
    }
    // The shape of the defect, stated directly: a bare `,xpub` with no `[`
    // before it is a key nobody can attribute.
    assert!(
        !desc.contains(",xpub"),
        "a key is rendered with no origin at all:\n{desc}"
    );
}

#[cfg(feature = "json")]
#[test]
fn the_completed_card_declares_a_fingerprint_for_every_slot() {
    let emitted = stdout_of(seat_cmd(
        "descriptor",
        &["--emit", "md1", "--group-size", "0"],
    ));
    let card: Vec<String> = lines(&emitted, "md1");
    assert!(!card.is_empty(), "no card emitted");

    let mut c = md();
    c.arg("inspect").arg("--json");
    for chunk in &card {
        c.arg(chunk);
    }
    let json: serde_json::Value = serde_json::from_str(&stdout_of(c)).unwrap();
    let fps = json["descriptor"]["tlv"]["fingerprints"]
        .as_array()
        .expect("fingerprints")
        .iter()
        .map(|e| (e[0].as_u64().unwrap(), e[1].as_str().unwrap().to_string()))
        .collect::<Vec<_>>();
    assert_eq!(
        fps,
        vec![
            (0, SLOTS[0].1.to_string()),
            (1, SLOTS[1].1.to_string()),
            (2, SLOTS[2].1.to_string()),
        ],
        "the completed card does not declare all three fingerprints"
    );
}

#[cfg(feature = "json")]
#[test]
fn the_completed_wallet_id_equals_a_full_seating_of_the_same_three_keys() {
    // THE point of I-2. Two routes to one wallet must mint one id, or the
    // operator's restore does not match the device's own record of it.
    let policy_id = |chunks: &[String]| -> String {
        let mut c = md();
        c.arg("inspect").arg("--json");
        for chunk in chunks {
            c.arg(chunk);
        }
        let json: serde_json::Value = serde_json::from_str(&stdout_of(c)).unwrap();
        json["wallet_policy_id"]["hex"]
            .as_str()
            .unwrap()
            .to_string()
    };

    // Route A: the partial template, completed on the host from cards.
    let completed = lines(
        &stdout_of(seat_cmd(
            "descriptor",
            &["--emit", "md1", "--group-size", "0"],
        )),
        "md1",
    );

    // Route B: the same policy minted FULLY SEATED, every slot declared.
    let mut c = md();
    c.arg("encode").arg(C13_TPL).args(["--group-size", "0"]);
    for (i, (xpub, fp)) in SLOTS.iter().enumerate() {
        c.arg("--key").arg(format!("@{i}={xpub}"));
        c.arg("--fingerprint").arg(format!("@{i}={fp}"));
    }
    let full = lines(&stdout_of(c), "md1");

    assert_eq!(
        policy_id(&completed),
        policy_id(&full),
        "a host-completed partial template mints a different wallet than the \
         same three keys seated in full"
    );
}
