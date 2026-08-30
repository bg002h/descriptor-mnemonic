#![allow(missing_docs)]
//! End-to-end vector rows for P2's CLI surface (plan §3 C2 step 7).
//!
//! The engine's rules are pinned as unit rows inside `src/seat/`, where a
//! fixture can be built and a decoded value inspected. What lives HERE is
//! everything that is only true of the COMMAND: that `--from-mk1` reaches
//! the engine at all on both verbs, that stdout stays the machine contract
//! while every note goes to stderr, that the flags refuse the combinations
//! they must, and that the keyless-phrases message now points somewhere a
//! user can act on.

use assert_cmd::Command;
use std::io::Write;

const PATHOLOGICAL: &str = include_str!("fixtures/pathological/backup-strings.txt");
const V_USP: &str = include_str!("fixtures/seating/v-usp.txt");
const V_MIX: &str = include_str!("fixtures/seating/v-mix.txt");
const V_B1_WARN: &str = include_str!("fixtures/seating/v-b1-warn.txt");

fn lines(text: &str, hrp: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| l.starts_with(hrp))
        .map(str::to_string)
        .collect()
}

fn md1(text: &str) -> Vec<String> {
    lines(text, "md1")
}
fn mk1(text: &str) -> Vec<String> {
    lines(text, "mk1")
}

fn md() -> Command {
    Command::cargo_bin("md").unwrap()
}

/// `md <verb> <policy md1...> --from-mk1 <each mk1> [extra...]`
fn seat_cmd(verb: &str, text: &str, cards: &[String], extra: &[&str]) -> Command {
    let mut c = md();
    c.arg(verb);
    for p in md1(text) {
        c.arg(p);
    }
    for s in cards {
        c.args(["--from-mk1", s]);
    }
    c.args(extra);
    c
}

fn out_of(o: &std::process::Output) -> String {
    String::from_utf8_lossy(&o.stdout).to_string()
}
fn err_of(o: &std::process::Output) -> String {
    String::from_utf8_lossy(&o.stderr).to_string()
}

// ─── V-DUP — the end-to-end must-SEAT row ───────────────────────────────

#[test]
fn v_dup_the_full_split_set_supplied_twice_over_still_seats() {
    // SPEC A3(a): "A full card string set supplied twice over ships as a
    // must-SEAT row." Both halves are doubled -- the policy card's md1
    // phrases AND every mk1 chunk -- because a drawer scan repeats whatever
    // it repeats.
    let mut doubled_cards = mk1(PATHOLOGICAL);
    doubled_cards.extend(mk1(PATHOLOGICAL));
    assert_eq!(doubled_cards.len(), 60);

    let mut c = md();
    c.arg("descriptor");
    for p in md1(PATHOLOGICAL) {
        c.arg(p.clone());
    }
    for p in md1(PATHOLOGICAL) {
        c.arg(p);
    }
    for s in &doubled_cards {
        c.args(["--from-mk1", s]);
    }
    let doubled = c.output().unwrap();
    assert!(doubled.status.success(), "{}", err_of(&doubled));

    let once = seat_cmd("descriptor", PATHOLOGICAL, &mk1(PATHOLOGICAL), &[])
        .output()
        .unwrap();
    assert!(once.status.success(), "{}", err_of(&once));
    assert_eq!(
        out_of(&doubled),
        out_of(&once),
        "a doubled scan is the same wallet, byte for byte"
    );
}

// ─── the composed wallet, on both verbs ─────────────────────────────────

#[test]
fn v_ord_descriptor_and_address_seat_the_same_wallet() {
    // Two commands, one engine. If they ever seated differently, one of
    // them would be deriving addresses for a wallet the other never emits.
    let d = seat_cmd("descriptor", PATHOLOGICAL, &mk1(PATHOLOGICAL), &[])
        .output()
        .unwrap();
    assert!(d.status.success(), "{}", err_of(&d));
    let descriptor = out_of(&d);
    assert!(descriptor.starts_with("wsh(or_i("), "{descriptor}");

    let a = seat_cmd("address", PATHOLOGICAL, &mk1(PATHOLOGICAL), &[])
        .output()
        .unwrap();
    assert!(a.status.success(), "{}", err_of(&a));
    assert_eq!(
        out_of(&a).trim(),
        "bc1qkuknuy6dsm0fq44cyyhzqy9wl3ex2n6ed39zxhx867l9wlh4yhlsejms64",
        "SPEC acceptance 1's `bc1qkuknuy6...`"
    );
}

#[test]
fn v_ord_stdout_carries_the_descriptor_and_nothing_else() {
    // stdout is the machine contract a coordinator pastes; a note in it
    // would corrupt exactly the consumer the descriptor exists for.
    let o = seat_cmd("descriptor", PATHOLOGICAL, &mk1(PATHOLOGICAL), &[])
        .output()
        .unwrap();
    let stdout = out_of(&o);
    assert_eq!(stdout.lines().count(), 1, "one line: {stdout}");
    assert!(!stdout.contains("note:"));
    assert!(!stdout.contains("warning:"));

    let stderr = err_of(&o);
    assert!(stderr.contains("composed wallet id ced22709"), "{stderr}");
    assert!(stderr.contains("SHAPE-CONFIRMED"), "{stderr}");
    assert!(
        stderr.contains(
            "note: address 0 (chain 0, index 0) is \
             bc1qkuknuy6dsm0fq44cyyhzqy9wl3ex2n6ed39zxhx867l9wlh4yhlsejms64 — compare \
             against your wallet software before trusting."
        ),
        "B2's residue-surfacing note, verbatim: {stderr}"
    );
}

#[test]
fn v_b1_warn_reaches_stderr_without_failing_the_command() {
    let o = seat_cmd("descriptor", V_B1_WARN, &mk1(V_B1_WARN), &[])
        .output()
        .unwrap();
    assert!(o.status.success(), "a stub warning is never a refusal");
    assert!(err_of(&o).contains("verify address 0 before trusting"));
    assert!(out_of(&o).starts_with("wsh(sortedmulti(2,"));
}

// ─── V-AMB / V-SEAT-OK, through the CLI ─────────────────────────────────

#[test]
fn v_amb_the_ambiguity_refusal_reaches_the_operator_with_exit_1() {
    let o = seat_cmd("descriptor", V_USP, &mk1(V_USP), &[])
        .output()
        .unwrap();
    assert_eq!(
        o.status.code(),
        Some(1),
        "a content refusal, not a usage one"
    );
    let e = err_of(&o);
    assert!(e.contains("md: seating refused:"), "{e}");
    assert!(e.contains("2 complete candidate assignments"), "{e}");
    assert!(e.contains("--seat '@i=<chunk-set-id>'"), "{e}");
    assert!(e.contains("re-mint the POLICY card"), "{e}");
    assert!(out_of(&o).is_empty(), "nothing on stdout when refusing");
}

#[test]
fn v_seat_ok_resolves_that_refusal_from_the_command_line() {
    // Take an id straight out of the refusal text, the way an operator
    // would, and feed it back.
    let refused = seat_cmd("descriptor", V_USP, &mk1(V_USP), &[])
        .output()
        .unwrap();
    let e = err_of(&refused);
    let id = e
        .lines()
        .find_map(|l| l.trim().strip_prefix("card "))
        .and_then(|s| s.split(' ').next())
        .expect("the refusal lists the cards that fit more than one slot")
        .to_string();
    assert_eq!(id.len(), 5, "a full five-hex-digit id: {id}");

    let seated = seat_cmd(
        "descriptor",
        V_USP,
        &mk1(V_USP),
        &["--seat", &format!("@0={id}")],
    )
    .output()
    .unwrap();
    assert!(seated.status.success(), "{}", err_of(&seated));
    assert!(out_of(&seated).starts_with("wsh(sortedmulti(2,"));
}

#[test]
fn v_seat_bad_a_contradicting_seat_refuses_from_the_command_line() {
    let cards = mk1(V_MIX);
    // @0 declares fingerprint 73c5da0a. Find the id of the card that does
    // NOT carry it by asking the engine which one it seated at @1... which
    // it will not tell us directly, so drive it the other way: try BOTH ids
    // and require exactly one to be refused.
    let ids: Vec<String> = {
        let o = seat_cmd("descriptor", V_MIX, &cards, &["--seat", "@0=00000"])
            .output()
            .unwrap();
        let e = err_of(&o);
        e.split("are: ")
            .nth(1)
            .expect("the unknown-id refusal lists what was supplied")
            .trim_end_matches(".\n")
            .split(", ")
            .map(|s| s.trim().trim_end_matches('.').to_string())
            .collect()
    };
    assert_eq!(ids.len(), 2, "{ids:?}");
    let outcomes: Vec<bool> = ids
        .iter()
        .map(|id| {
            seat_cmd(
                "descriptor",
                V_MIX,
                &cards,
                &["--seat", &format!("@0={id}")],
            )
            .output()
            .unwrap()
            .status
            .success()
        })
        .collect();
    assert_eq!(
        outcomes.iter().filter(|ok| **ok).count(),
        1,
        "exactly one of the two cards may sit in @0: {outcomes:?}"
    );
}

#[test]
fn v_seat_unk_an_unknown_id_refuses_by_name() {
    let o = seat_cmd("descriptor", V_USP, &mk1(V_USP), &["--seat", "@0=abcde"])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(
        err_of(&o).contains("no supplied card has chunk-set id abcde"),
        "{}",
        err_of(&o)
    );
}

#[test]
fn v_seat_bad_seat_without_cards_says_what_it_needs() {
    let mut c = md();
    c.arg("descriptor");
    for p in md1(V_USP) {
        c.arg(p);
    }
    c.args(["--seat", "@0=abcde"]);
    let o = c.output().unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(
        err_of(&o).contains("it needs --from-mk1/--from-mk1-file cards to choose among"),
        "{}",
        err_of(&o)
    );
}

// ─── V-MSG-KEYLESS — the r1-M8 self-contradicting message dies ──────────

#[test]
fn v_msg_keyless_the_refusal_now_points_at_from_mk1() {
    // Before P2 this message prescribed `--key @i=XPUB`, which its OWN
    // constraint rejects: --key requires --template, and --template
    // conflicts with phrases. So the instruction could not be followed.
    let mut c = md();
    c.arg("descriptor");
    for p in md1(PATHOLOGICAL) {
        c.arg(p);
    }
    let o = c.output().unwrap();
    assert!(!o.status.success());
    let e = err_of(&o);
    assert!(e.contains("this card is a keyless TEMPLATE"), "{e}");
    assert!(e.contains("--from-mk1 <STRING>"), "{e}");
    assert!(e.contains("--from-mk1-file <FILE>"), "{e}");

    // And the instruction WORKS as written: the same phrases plus
    // --from-mk1 succeed. Without this half the row would pass on a
    // message that merely named a different unusable flag.
    let fixed = seat_cmd("descriptor", PATHOLOGICAL, &mk1(PATHOLOGICAL), &[])
        .output()
        .unwrap();
    assert!(fixed.status.success(), "{}", err_of(&fixed));
}

#[test]
fn v_msg_keyless_address_says_the_same_thing() {
    let mut c = md();
    c.arg("address");
    for p in md1(PATHOLOGICAL) {
        c.arg(p);
    }
    let o = c.output().unwrap();
    assert!(!o.status.success());
    assert!(err_of(&o).contains("--from-mk1 <STRING>"), "{}", err_of(&o));
}

// ─── the file channel ───────────────────────────────────────────────────

#[test]
fn v_dup_from_mk1_file_and_from_mk1_feed_one_list() {
    // Both channels together, with the SAME cards in each: the input
    // pipeline dedupes, so overlapping channels are harmless by
    // construction rather than by a rule in the flag layer.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cards.txt");
    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(f, "# provenance lines are skipped").unwrap();
    writeln!(f).unwrap();
    for s in mk1(PATHOLOGICAL) {
        writeln!(f, "{s}").unwrap();
    }
    drop(f);

    let both = seat_cmd(
        "descriptor",
        PATHOLOGICAL,
        &mk1(PATHOLOGICAL),
        &["--from-mk1-file", path.to_str().unwrap()],
    )
    .output()
    .unwrap();
    assert!(both.status.success(), "{}", err_of(&both));

    let mut file_only = md();
    file_only.arg("descriptor");
    for p in md1(PATHOLOGICAL) {
        file_only.arg(p);
    }
    file_only.args(["--from-mk1-file", path.to_str().unwrap()]);
    let file_only = file_only.output().unwrap();
    assert!(file_only.status.success(), "{}", err_of(&file_only));
    assert_eq!(out_of(&both), out_of(&file_only));
}

#[test]
fn v_dup_from_mk1_file_refuses_a_line_it_cannot_read() {
    // Blank lines and `#` comments are skipped; anything else is refused
    // rather than silently dropped -- a truncated line is exactly the input
    // a restore must not quietly ignore.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cards.txt");
    let mut f = std::fs::File::create(&path).unwrap();
    for s in mk1(PATHOLOGICAL) {
        writeln!(f, "{s}").unwrap();
    }
    writeln!(f, "md1thisisnotakeycard").unwrap();
    drop(f);

    let mut c = md();
    c.arg("descriptor");
    for p in md1(PATHOLOGICAL) {
        c.arg(p);
    }
    c.args(["--from-mk1-file", path.to_str().unwrap()]);
    let o = c.output().unwrap();
    assert_eq!(o.status.code(), Some(1));
    let e = err_of(&o);
    assert!(e.contains("is not an mk1 string"), "{e}");
    assert!(e.contains("line 31"), "names the line number: {e}");
}

// ─── flag-surface refusals ──────────────────────────────────────────────

#[test]
fn v_msg_keyless_from_mk1_conflicts_with_template() {
    // --template rebuilds a policy from text; --from-mk1 seats cards into a
    // card. Accepting both would leave the origin declarations coming from
    // two places at once.
    for verb in ["descriptor", "address"] {
        let o = md()
            .args([
                verb,
                "--template",
                "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))",
                "--from-mk1",
                "mk1qq",
            ])
            .output()
            .unwrap();
        assert_eq!(o.status.code(), Some(2), "{verb}: {}", err_of(&o));
        assert!(err_of(&o).contains("cannot be used with"), "{}", err_of(&o));
    }
}

#[test]
fn v_msg_keyless_a_keyed_card_with_from_mk1_says_there_is_nothing_to_seat() {
    // The keyed pathological card is a wallet policy already. Seating cards
    // into it is a category error, and saying so beats composing something.
    let keyed = include_str!("fixtures/seating/v-spendeq-keyed.txt");
    let o = seat_cmd("descriptor", keyed, &mk1(PATHOLOGICAL), &[])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    let e = err_of(&o);
    assert!(e.contains("already carry their keys"), "{e}");
    assert!(e.contains("Drop --from-mk1"), "{e}");
}

// ─── every PHASE A refusal actually REACHES the command ─────────────────
//
// The engine's refusals are pinned as unit rows against the function that
// builds each one. That proves the MESSAGE and proves nothing about whether
// `seat::run` ever gets there — and it measurably can hide a defect: the
// first draft of V-LEFTOVER's fixture reused a pathological xpub, so the
// engine refused at A3's pairwise-distinct check and never reached A4, while
// the unit row (which calls A4 directly) went on passing. This table drives
// each fixture end to end and requires the refusal the roster names.

const V_R5M1: &str = include_str!("fixtures/seating/v-r5m1.txt");
const V_DOOR: &str = include_str!("fixtures/seating/v-door.txt");
const V_IMPOSS: &str = include_str!("fixtures/seating/v-imposs.txt");
const V_BOUND_REF: &str = include_str!("fixtures/seating/v-bound-ref.txt");
const V_BOUND_SEAT: &str = include_str!("fixtures/seating/v-bound-seat.txt");
const V_CAP: &str = include_str!("fixtures/seating/v-cap.txt");
const V_FPFREE_CARD: &str = include_str!("fixtures/seating/v-fpfree-card.txt");
const V_R2_ORD: &str = include_str!("fixtures/seating/v-r2-ord.txt");
const V_R4_IK: &str = include_str!("fixtures/seating/v-r4-ik.txt");
const V_GRP: &str = include_str!("fixtures/seating/v-grp.txt");
const V_UNFILLED: &str = include_str!("fixtures/seating/v-unfilled.txt");
const V_LEFTOVER: &str = include_str!("fixtures/seating/v-leftover.txt");
const V_CE1: &str = include_str!("fixtures/seating/v-ce1.txt");

fn refusal_of(text: &str, extra_cards: &[String]) -> String {
    let mut cards = mk1(text);
    cards.extend_from_slice(extra_cards);
    let o = seat_cmd("descriptor", text, &cards, &[]).output().unwrap();
    assert_eq!(
        o.status.code(),
        Some(1),
        "this fixture must refuse; stderr was: {}",
        err_of(&o)
    );
    assert!(out_of(&o).is_empty(), "nothing on stdout when refusing");
    err_of(&o)
}

#[test]
fn v_r5m1_reaches_the_command() {
    let e = refusal_of(V_R5M1, &[]);
    assert!(
        e.contains("same placeholder at more than one position"),
        "{e}"
    );
    assert!(e.contains("forbidden by BIP 388"), "{e}");
    assert!(!e.to_lowercase().contains("invalid"), "{e}");
}

#[test]
fn v_door_reaches_the_command() {
    let e = refusal_of(V_DOOR, &[]);
    assert!(e.contains("declare the IDENTICAL origin"), "{e}");
    // Review r1 M1: pin the named slots and the ground, not just the class.
    assert!(e.contains("@0 [73c5da0a/48'/0'/0'/2']"), "{e}");
    assert!(e.contains("@1 [73c5da0a/48'/0'/0'/2']"), "{e}");
    assert!(e.contains("forbidden by BIP 388"), "{e}");
}

#[test]
fn v_imposs_reaches_the_command() {
    let e = refusal_of(V_IMPOSS, &[]);
    assert!(e.contains("yet carry DIFFERENT xpubs"), "{e}");
}

#[test]
fn v_bound_ref_reaches_the_command() {
    let e = refusal_of(V_BOUND_REF, &[]);
    assert!(e.contains("carry the SAME extended public key"), "{e}");
    assert!(e.contains("BIP 388"), "{e}");
}

#[test]
fn v_cap_reaches_the_command() {
    let e = refusal_of(V_CAP, &[]);
    assert!(
        e.contains("more than 720 complete candidate assignments"),
        "{e}"
    );
}

#[test]
fn v_fpfree_card_reaches_the_command() {
    let e = refusal_of(V_FPFREE_CARD, &[]);
    assert!(e.contains("1 slot(s) unfilled"), "{e}");
    assert!(e.contains("@0 [73c5da0a/48'/0'/0'/2']"), "{e}");
}

#[test]
fn v_r2_ord_reaches_the_command() {
    let e = refusal_of(V_R2_ORD, &[]);
    assert!(e.contains("24 complete candidate assignments"), "{e}");
}

#[test]
fn v_r4_ik_reaches_the_command() {
    let e = refusal_of(V_R4_IK, &[]);
    assert!(e.contains("120 complete candidate assignments"), "{e}");
}

#[test]
fn v_grp_reaches_the_command() {
    let e = refusal_of(V_GRP, &[]);
    assert!(e.contains("do NOT all compose to the same wallet"), "{e}");
    // Review r1 M1: pin the measured matching count, a full chunk-set id,
    // and both remedies at the CLI level, matching the sibling rows' depth.
    assert!(e.contains("120 complete candidate assignments"), "{e}");
    assert!(e.contains("card 34128 (stub 5b48af35)"), "{e}");
    assert!(e.contains("re-mint the POLICY card"), "{e}");
    assert!(e.contains("--seat '@i=<chunk-set-id>'"), "{e}");
}

#[test]
fn v_unfilled_reaches_the_command_naming_the_slot() {
    let e = refusal_of(V_UNFILLED, &[]);
    assert!(e.contains("1 slot(s) unfilled"), "{e}");
    assert!(e.contains("0 card(s) left over"), "{e}");
    assert!(e.contains("11 slots, 10 cards supplied"), "{e}");
    assert!(e.contains("@3 [73c5da0a/48'/0'/3'/2']"), "{e}");
}

#[test]
fn v_leftover_reaches_the_command_naming_the_card() {
    // The pathological set PLUS one extra card. The extra card's xpub is a
    // depth-5 child, deliberately not one of the eleven, so the engine
    // reaches A4 rather than stopping at A3's pairwise-distinct check.
    let e = refusal_of(PATHOLOGICAL, &mk1(V_LEFTOVER));
    assert!(e.contains("0 slot(s) unfilled"), "{e}");
    assert!(e.contains("1 card(s) left over"), "{e}");
    assert!(e.contains("11 slots, 12 cards supplied"), "{e}");
    assert!(e.contains("which wallet do these belong to"), "{e}");
    assert!(
        e.contains("48'/0'/9'/2'/0"),
        "names its declared origin: {e}"
    );
}

// ─── and the must-SEAT fixtures reach it too ───────────────────────────

#[test]
fn v_bound_seat_reaches_the_command_and_seats() {
    let o = seat_cmd("descriptor", V_BOUND_SEAT, &mk1(V_BOUND_SEAT), &[])
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", err_of(&o));
    assert!(out_of(&o).starts_with("wsh(sortedmulti(2,"));
}

#[test]
fn v_mix_reaches_the_command_and_seats() {
    let o = seat_cmd("descriptor", V_MIX, &mk1(V_MIX), &[])
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", err_of(&o));
    assert!(out_of(&o).starts_with("wsh(multi(2,"));
}

#[test]
fn v_ce1_reaches_the_command_and_seats() {
    let o = seat_cmd("descriptor", V_CE1, &mk1(V_CE1), &[])
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", err_of(&o));
    assert!(out_of(&o).starts_with("wsh(multi(2,"));
}

#[test]
fn v_collide_reaches_the_command() {
    // Two cards pinned to one chunk-set id, offered against any policy: the
    // input pipeline merges them and reassembly refuses, before the engine
    // sees a card at all.
    let collide: Vec<String> = mk1(include_str!("fixtures/seating/v-collide.txt"));
    let mut cards = mk1(V_USP);
    cards.extend(collide);
    let o = seat_cmd("descriptor", V_USP, &cards, &[]).output().unwrap();
    assert_eq!(o.status.code(), Some(1));
    let e = err_of(&o);
    assert!(e.contains("chunk-set 12345"), "{e}");
    assert!(e.contains("do not reassemble"), "{e}");
}
