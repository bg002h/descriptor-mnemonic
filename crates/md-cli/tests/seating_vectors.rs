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
