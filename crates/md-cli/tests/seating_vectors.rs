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
const V_D_RT_KEYED: &str = include_str!("fixtures/decompose/v-d-rt.txt");
const V_COLLIDE: &str = include_str!("fixtures/seating/v-collide.txt");
const V_AP_ROW1_E2E: &str = include_str!("fixtures/seating/v-ap-row1-e2e.txt");

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

/// REVIEW-converter-whole-diff-r1 I2 — the double scan is CASE-variant.
///
/// mk1 strings are bech32, so UPPERCASE is the canonical QR form and md's own
/// mk1 decoder accepts it everywhere: an all-uppercase card set seats to the
/// identical descriptor. Step 1 of P2's normative pipeline normalised
/// WHITESPACE only, so one card scanned twice -- once lowercase, once
/// uppercase -- survived dedupe as two strings, merged into one group at step
/// 2 and blew up at step 3 with a message that BLAMED THE WRONG THING: the
/// pre-P3 wrapper reported the two survivors as though they were two
/// DIFFERENT key cards pinned to one chunk-set id, with a fix that meant
/// re-engraving the survivor whose stamped id it named.
///
/// An operator who follows that re-engraves a plate to fix a problem that
/// does not exist. SPEC A3(a) promises "an accidental double-scan is made
/// harmless BY ORDER OF OPERATIONS": it was not.
#[test]
fn v_dup_a_case_variant_double_scan_still_seats() {
    let cards = mk1(PATHOLOGICAL);
    assert_eq!(cards.len(), 30, "fixture: the full pathological card set");
    let mut with_variant = cards.clone();
    // ONE card re-scanned in the canonical uppercase QR form.
    with_variant.push(cards[0].to_uppercase());

    let variant = seat_cmd("descriptor", PATHOLOGICAL, &with_variant, &[])
        .output()
        .unwrap();
    assert!(
        variant.status.success(),
        "a case-variant re-scan of one card was refused: {}",
        err_of(&variant)
    );

    let once = seat_cmd("descriptor", PATHOLOGICAL, &cards, &[])
        .output()
        .unwrap();
    assert!(once.status.success(), "{}", err_of(&once));
    assert_eq!(
        out_of(&variant),
        out_of(&once),
        "a case-variant re-scan is the same wallet, byte for byte"
    );
}

/// The control that keeps the row above honest: an uppercase mk1 string is
/// something md's decoder really does accept, so the dedupe is normalising a
/// real equivalence rather than papering over a decode failure.
#[test]
fn v_dup_an_all_uppercase_card_set_seats_identically() {
    let cards: Vec<String> = mk1(PATHOLOGICAL).iter().map(|s| s.to_uppercase()).collect();
    let upper = seat_cmd("descriptor", PATHOLOGICAL, &cards, &[])
        .output()
        .unwrap();
    assert!(upper.status.success(), "{}", err_of(&upper));
    let lower = seat_cmd("descriptor", PATHOLOGICAL, &mk1(PATHOLOGICAL), &[])
        .output()
        .unwrap();
    assert_eq!(out_of(&upper), out_of(&lower));
}

// ─── V-SFLAG (REVIEW-converter-whole-diff-r1 I4) ────────────────────────
//
// `--key`, `--fingerprint` and `--path` are TEMPLATE-row flags: the seating
// branch builds a `seat::SeatingRequest { phrases, from_mk1, seats, network,
// cmd }`, which does not carry them at all. Measured at 9d0c30dc, all three
// were accepted and silently discarded on a SUCCESSFUL composition -- the same
// descriptor, the same checksum `#9uzthz8n`, exit 0, not a word. `--key` is
// funds-relevant material.
//
// The declared refusal did not fire. `requires = "template"` only triggers
// when the whole `<PHRASES|--template>` group is absent, so it is inert
// whenever phrases are supplied -- the "a refusal that does not refuse" class.
// It bit hardest on exactly the wallets that need it: v-ce1 declares
// fingerprint-free origins, so the composed descriptor carries NO origin
// metadata, and the obvious operator response ("add the origins I know") was
// accepted and did nothing.

/// `<verb> <policy> --from-mk1 … <flag> <value>` must refuse, naming both
/// sides of the conflict, and compose nothing.
fn assert_seating_flag_conflict(verb: &str, flag: &str, value: &str, card_flag: &str) {
    let cards = mk1(V_CE1);
    let mut c = md();
    c.arg(verb);
    for p in md1(V_CE1) {
        c.arg(p);
    }
    if card_flag == "--from-mk1" {
        for s in &cards {
            c.args(["--from-mk1", s]);
        }
    } else {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        for s in &cards {
            writeln!(f, "{s}").unwrap();
        }
        let path = f.into_temp_path();
        c.args(["--from-mk1-file", path.to_str().unwrap()]);
        // Keep the file alive for the duration of the run.
        let out = c.args([flag, value]).output().unwrap();
        assert_conflict(&out, verb, flag, card_flag);
        return;
    }
    let out = c.args([flag, value]).output().unwrap();
    assert_conflict(&out, verb, flag, card_flag);
}

fn assert_conflict(out: &std::process::Output, verb: &str, flag: &str, card_flag: &str) {
    let stdout = out_of(out);
    assert!(
        !out.status.success(),
        "md {verb} accepted {flag} on the seating route and composed anyway: {stdout}"
    );
    assert!(
        !stdout.contains("wsh(") && !stdout.contains("bc1"),
        "md {verb} composed a wallet while ignoring {flag}: {stdout}"
    );
    let err = err_of(out);
    assert!(
        err.contains(flag) && err.contains(card_flag),
        "the refusal must name BOTH sides of the conflict ({flag} and \
         {card_flag}); got: {err}"
    );
}

#[test]
fn v_sflag_key_on_the_seating_route_refuses_on_both_verbs() {
    let xpub = "xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf";
    for verb in ["descriptor", "address"] {
        assert_seating_flag_conflict(verb, "--key", &format!("@0={xpub}"), "--from-mk1");
    }
}

#[test]
fn v_sflag_fingerprint_on_the_seating_route_refuses_on_both_verbs() {
    for verb in ["descriptor", "address"] {
        assert_seating_flag_conflict(verb, "--fingerprint", "@0=73c5da0a", "--from-mk1");
    }
}

#[test]
fn v_sflag_path_on_the_seating_route_refuses_on_both_verbs() {
    for verb in ["descriptor", "address"] {
        assert_seating_flag_conflict(verb, "--path", "48'/0'/0'/2'", "--from-mk1");
    }
}

/// The file channel is the same route, so it must refuse the same way --
/// otherwise the guard is one spelling away from being bypassed.
#[test]
fn v_sflag_the_file_channel_refuses_identically() {
    assert_seating_flag_conflict("descriptor", "--path", "48'/0'/0'/2'", "--from-mk1-file");
}

/// The `# @@ keyed-card` section of the v-d-rt fixture -- its md1 lines, and
/// NOT the `# @@ policy-card` line that shares the file. Keyed cards carry the
/// `md1f0v5` prefix; the keyless policy card is `md15`.
fn v_d_rt_keyed_card() -> Vec<String> {
    lines(V_D_RT_KEYED, "md1f0v5")
}

/// THE PRE-EXISTING HALF, closed by the same declaration. A KEYED md1 card
/// needs no `--key` either, and measured at 9d0c30dc
/// `md descriptor <keyed card> --key @0=X` composed byte-identically to
/// `md descriptor <keyed card>` -- funds-relevant material discarded on a
/// successful composition, exit 0, no word. It is the same "a refusal that
/// does not refuse" root the review named, one route over.
#[test]
fn v_sflag_the_phrase_route_refuses_the_template_flags_too() {
    let keyed = v_d_rt_keyed_card();
    assert_eq!(keyed.len(), 6, "fixture: the v-d-rt keyed card");
    for (flag, value) in [
        (
            "--key",
            "@0=xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf",
        ),
        ("--fingerprint", "@0=73c5da0a"),
        ("--path", "84'/0'/0'"),
    ] {
        for verb in ["descriptor", "address"] {
            let mut c = md();
            c.arg(verb);
            for p in &keyed {
                c.arg(p);
            }
            let out = c.args([flag, value]).output().unwrap();
            let stdout = out_of(&out);
            assert!(
                !out.status.success(),
                "md {verb} discarded {flag} and composed anyway: {stdout}"
            );
            let err = err_of(&out);
            assert!(err.contains(flag), "the refusal must name {flag}: {err}");
        }
    }
}

/// CONTROL for the row above: the keyed card composes on its own.
#[test]
fn v_sflag_the_phrase_route_itself_is_unaffected() {
    let mut c = md();
    c.arg("descriptor");
    for p in v_d_rt_keyed_card() {
        c.arg(p);
    }
    let out = c.output().unwrap();
    assert!(out.status.success(), "{}", err_of(&out));
    assert!(out_of(&out).contains("wsh(sortedmulti(2,"));
}

/// CONTROL: without the T-row flags the same invocation still seats. Without
/// this the rows above would pass if `--from-mk1` had simply stopped working.
#[test]
fn v_sflag_the_seating_route_itself_is_unaffected() {
    let out = seat_cmd("descriptor", V_CE1, &mk1(V_CE1), &[])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", err_of(&out));
    assert!(out_of(&out).contains("wsh(multi(2,"));
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
        // M1 (whole-diff review): the "are:" list now renders each card's
        // FULL label (`DecodedCard::label`, e.g. "12345 (stub abcdef01)"),
        // not a bare id -- V_MIX has no collision here, so the label's
        // leading token IS still a valid `--seat @i=<id>` value; take just
        // that token.
        e.split("are: ")
            .nth(1)
            .expect("the unknown-id refusal lists what was supplied")
            .trim_end_matches(".\n")
            .split(", ")
            .map(|s| {
                s.trim()
                    .trim_end_matches('.')
                    .split(' ')
                    .next()
                    .expect("each entry has a leading id token")
                    .to_string()
            })
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
const V_BOUND_REF_PATHS: &str = include_str!("fixtures/seating/v-bound-ref-paths.txt");
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

/// The RENDERED line, from the `md:` prefix onward (Acceptance 4) — this
/// diagnostic was REWRITTEN by plan P3 step 3b, when the door check became
/// an invocation of the shared N1 classifier, so it re-earns its row.
///
/// It is the R-N1a message verbatim, and that is the deliverable: one
/// wallet draws one sentence whether it arrives as a template, as a keyed
/// card, or as the keyless policy card of a seating request. Before the
/// unification this line began "md: seating refused: this policy uses the
/// same placeholder at more than one position — @1 (2 positions), @2 (2
/// positions)." and was a second implementation of the same predicate.
#[test]
fn v_r5m1_reaches_the_command() {
    let e = refusal_of(V_R5M1, &[]);
    let lines: Vec<&str> = e.lines().filter(|l| l.starts_with("md: ")).collect();
    assert_eq!(lines.len(), 1, "expected exactly one rendered line:\n{e}");
    assert_eq!(
        lines[0],
        "md: unsupported: @1 appears at 2 use sites in this template with the same path \
         expression, so ONE key would fill every one of them. That is forbidden by BIP 388's \
         disjointness rule (\"if two KEY are KP/<M;N>/* and KP/<P;Q>/* for the same key \
         placeholder KP, then the sets {M, N} and {P, Q} must be disjoint\"), whose \
         forbidden-example list names sh(multi(1,@0/**,@0/**)) — \"Repeated keys with the same \
         path expression\". md declines to mint or compose this shape: give each distinct key \
         its own placeholder."
    );
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

/// The different-PATHS sibling reaches the command too, so the engine's
/// path-blindness is pinned end to end and not only at the function.
#[test]
fn v_bound_ref_paths_reaches_the_command() {
    let e = refusal_of(V_BOUND_REF_PATHS, &[]);
    assert!(e.contains("carry the SAME extended public key"), "{e}");
    assert!(e.contains("BIP 388"), "{e}");
    assert!(!e.to_lowercase().contains("invalid"), "{e}");
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

// ─── SPEC row 1 (canonical-collision) at the command level ─────────────
//
// REWRITTEN from the pre-P1 `v_collide_reaches_the_command` (plan §12
// churn note): auto-partition now SEATS a clean same-id collision instead
// of refusing at reassembly, so this row moved from arm 1's message to the
// seat+AP1-note outcome. `v-collide.txt` itself now demonstrates the
// mixed-totals seat (unit-level, `seat::input`) and the surplus-variant-b
// leftover (below) instead.
//
// Uses `v-ap-row1-e2e.txt`, NOT P0's `v-ap-canonical.txt` — flagged
// deviation (P1 finding, not an engine defect): `v-ap-canonical.txt` and
// its committed control BOTH refuse unconditionally via
// `satisfy::check_no_impossible_card_pair`, because P0 minted both of that
// pair's cards with `--origin-fingerprint <KEY N's fp>` and every key in
// `keys.txt` shares ONE fingerprint (73c5da0a) — so the pair declares an
// IDENTICAL (fingerprint, path) with different xpubs, which that
// PRE-EXISTING, unrelated check refuses regardless of P1. Measured
// directly against both files; see `v-ap-row1-e2e.txt`'s own header for
// the full trace. The auto-partition ENGINE ITSELF correctly seats
// `v-ap-canonical.txt`'s pair — proven independently at the engine-unit
// level (`seat::partition::tests::canonical_pair_seats_two_cards_v_equals_k`)
// and the decode_cards-unit level (`seat::input::tests::row1_canonical_collision_two_cards_seat_with_ap1_note`,
// on an equivalent inline fixture) — this is purely a full-pipeline
// fixture-authoring defect in a DIFFERENT, orthogonal check.
#[test]
fn row1_canonical_collision_reaches_the_command_byte_identical_to_the_unpinned_control() {
    let pinned = seat_cmd("descriptor", V_AP_ROW1_E2E, &mk1(V_AP_ROW1_E2E), &[])
        .output()
        .unwrap();
    assert!(pinned.status.success(), "{}", err_of(&pinned));
    let control = seat_cmd("descriptor", V_BOUND_SEAT, &mk1(V_BOUND_SEAT), &[])
        .output()
        .unwrap();
    assert!(control.status.success(), "{}", err_of(&control));

    // stdout (the descriptor) byte-identical.
    assert_eq!(out_of(&pinned), out_of(&control));

    // Address byte-identical too.
    let pinned_addr = seat_cmd("address", V_AP_ROW1_E2E, &mk1(V_AP_ROW1_E2E), &[])
        .output()
        .unwrap();
    let control_addr = seat_cmd("address", V_BOUND_SEAT, &mk1(V_BOUND_SEAT), &[])
        .output()
        .unwrap();
    assert!(pinned_addr.status.success(), "{}", err_of(&pinned_addr));
    assert!(control_addr.status.success(), "{}", err_of(&control_addr));
    assert_eq!(out_of(&pinned_addr), out_of(&control_addr));

    // WalletPolicyId (the "composed wallet id" B1 note) byte-identical.
    let pinned_err = err_of(&pinned);
    let control_err = err_of(&control);
    let wallet_id_line = |s: &str| {
        s.lines()
            .find(|l| l.starts_with("note: composed wallet id"))
            .unwrap()
            .to_string()
    };
    assert_eq!(wallet_id_line(&pinned_err), wallet_id_line(&control_err));

    // The AP1 note, then the group's two R2 warnings, IN ORDER — the
    // pinned side's own stderr; the control has neither (nothing pinned).
    let lines: Vec<&str> = pinned_err.lines().collect();
    let note_pos = lines
        .iter()
        .position(|l| l.starts_with("note: these "))
        .expect("AP1 note present");
    let warn_positions: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.starts_with("warning: this key card's stamped chunk-set id"))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(
        warn_positions.len(),
        2,
        "two R2 mismatch warnings: {lines:?}"
    );
    assert!(
        warn_positions.iter().all(|&w| note_pos < w),
        "AP1 note must precede both R2 warnings: {lines:?}"
    );
    assert!(
        !control_err.contains("note: these "),
        "control has no AP1 note (no collision)"
    );
}

// ─── I3 (whole-diff review): arm 1/2/3 command-level coverage, restored ──
//
// Deviation 7 repurposed the old `v_collide_reaches_the_command` (which
// asserted arm 1's message reaching the CLI) into the row above, which now
// exercises the SEAT+note outcome instead. After that rewrite, zero
// command-level rows drove any of the R5 classifier's three refusal arms.
// These three are the NAMED inheritors of that lost arm-1/2/3 coverage
// (spec row 12: "every retired assertion's inheriting row is named").

/// A throwaway, deliberately minimal 1-slot policy -- same string as
/// `V_AP_SURPLUS_B_POLICY` below, duplicated here (not reordered) because
/// these three rows only need SOME valid keyless policy to get past `md`'s
/// "no md1 policy phrase" refusal; none of arm 1/2/3 ever reaches policy
/// matching.
const V_ARM_THROWAWAY_POLICY: &str = "md1yqfdss5n9gqpsg5n2ysa4r774vcg";

/// Arm 1 (merged, fail-closed via SPEC §2's engine reporting NoPartition):
/// `v-ap-incomplete.txt`'s one complete 2-chunk class + one 3-chunk class
/// missing a piece classifies `Failure::Merged` (mismatched declared
/// totals), the auto-partition engine cannot admit the incomplete class,
/// and the WHOLE group refuses via arm 1's `merged_refusal` -- unit-pinned
/// at `seat::input::tests::row7b_incomplete_class_set_refuses_the_whole_group_via_arm_1`;
/// this is that message reaching the real CLI, piece-count evidence AND
/// the `mk inspect` id-check (W15(d)'s named remedy) both included.
#[test]
fn arm1_merged_v_ap_incomplete_reaches_the_command() {
    let cards: Vec<String> = mk1(include_str!("fixtures/seating/v-ap-incomplete.txt"));
    assert_eq!(cards.len(), 4);
    let o = seat_cmd("descriptor", V_ARM_THROWAWAY_POLICY, &cards, &[])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(out_of(&o).is_empty(), "nothing on stdout when refusing");
    let e = err_of(&o);
    assert!(e.contains("chunk-set a1006"), "{e}");
    assert!(
        e.contains("declares piece 1 of 2") && e.contains("declares piece 1 of 3"),
        "the piece-count evidence W15(a) requires: {e}"
    );
    assert!(e.contains("piece order does not matter"), "{e}");
    assert!(
        e.contains("`mk inspect`"),
        "W15(d)'s named id-check remedy, pinned nowhere at command level before I3: {e}"
    );
    assert!(
        e.contains("re-mint (re-encoding without --chunk-set-id)"),
        "{e}"
    );
}

/// Arm 2 (incomplete): one 2-chunk card, only its first chunk supplied --
/// received 1 < declared 2, no duplicates. Same literal as
/// `seat::input::tests::r5_incomplete_one_of_two_chunks_classifies_as_incomplete`
/// (chunk-set 33333), reaching the real CLI.
#[test]
fn arm2_incomplete_reaches_the_command() {
    let card = "mk1qpxvenpqqsq4kj90xdeutks2q5zg3vs7rnefw94m5rru59s2su80aw2q4wgdpapgfl4pkhsdyytkwl5z8lphut2hvvpp5drdl5w8ame3clux"
        .to_string();
    let o = seat_cmd("descriptor", V_ARM_THROWAWAY_POLICY, &[card], &[])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(out_of(&o).is_empty(), "nothing on stdout when refusing");
    let e = err_of(&o);
    assert!(e.contains("chunk-set 33333"), "{e}");
    assert!(e.contains("should be 2"), "{e}");
    assert!(e.contains("you supplied 1"), "{e}");
    assert!(e.contains("scan the missing piece(s)"), "{e}");
    assert!(
        !e.contains("piece order does not matter"),
        "must NOT be arm 1's message: {e}"
    );
    assert!(!e.contains("error:"), "no codec line on arm 2: {e}");
}

/// Arm 3 (terminal, no precondition of its own): chunk 0 of card T1 +
/// chunk 1 of card T2, both pinned to chunk-set `22222`, both declaring
/// `total_chunks = 2` -- the codec's own cross-chunk integrity hash
/// refuses it. Same two literals as
/// `seat::input::tests::r5_terminal_cross_chunk_hash_mismatch_classifies_as_terminal`,
/// reaching the real CLI: human sentence first (W16(b)), `error:` line
/// after.
#[test]
fn arm3_terminal_reaches_the_command() {
    let cards = vec![
        "mk1qpyg3zpqqsq4kj90xfeutks2q5zg3vs7rnefw94m5rru59s2su80aw2q4wgdpapgfl4pkhsdyytkwl5z8lphut2hvvpp5fkjjqxnyhx4glde"
            .to_string(),
        "mk1qpyg3zppwyp4dfykwfkgg6fxyxetdcmythf4hsqzd3v879jprztejzs7rlhgvt7a4x7n4h7uagdls".to_string(),
    ];
    let o = seat_cmd("descriptor", V_ARM_THROWAWAY_POLICY, &cards, &[])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(out_of(&o).is_empty(), "nothing on stdout when refusing");
    let e = err_of(&o);
    assert!(e.contains("chunk-set 22222"), "{e}");
    assert!(e.contains("do not form one key card"), "{e}");
    assert!(e.contains("re-scan one card's pieces alone"), "{e}");
    let error_line = e.lines().find(|l| l.starts_with("error: "));
    assert!(
        error_line.is_some_and(|l| l.contains("cross-chunk integrity hash mismatch")),
        "the codec diagnostic is on its own labeled line (W16(b)): {e}"
    );
    assert!(
        e.find("do not form one key card").unwrap() < e.find("error:").unwrap(),
        "human sentence leads, codec line follows (W16(b)): {e}"
    );
}

// ─── SPEC row 10, surplus variant (b): same-id LEGITIMATE extra cards that
// ─── seat, then leftover-refuse with DISTINGUISHABLE labels ────────────

/// A fresh, minimal 1-slot policy, deliberately at an origin NEITHER of
/// `v-collide.txt`'s two cards declares (`m/48'/0'/9'/2'`, vs their
/// `0'/2'`/`1'/2'`), so BOTH seated collided cards become leftover
/// together and their ordinal labels (`12345#1`/`12345#2`) both appear in
/// ONE message. Minted with `md encode "wsh(pk(@0/48'/0'/9'/2'/<0;1>/*))"`.
const V_AP_SURPLUS_B_POLICY: &str = "md1yqfdss5n9gqpsg5n2ysa4r774vcg";

#[test]
fn v_collide_surplus_variant_b_seats_then_refuses_leftover_with_distinguishable_labels() {
    let o = seat_cmd("descriptor", V_AP_SURPLUS_B_POLICY, &mk1(V_COLLIDE), &[])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    let e = err_of(&o);
    assert!(e.contains("1 slot(s) unfilled"), "{e}");
    assert!(e.contains("2 card(s) left over"), "{e}");
    assert!(e.contains("1 slots, 2 cards supplied"), "{e}");
    // Both collided cards seated (auto-partition ran, SPEC §2), then BOTH
    // are the leftover -- named by their DISTINGUISHABLE ordinal labels,
    // never the bare (ambiguous) "12345" a pre-P1 build could never even
    // reach this far to produce.
    assert!(e.contains("12345#1 (stub 5b48af35)"), "{e}");
    assert!(e.contains("12345#2 (stub 5b48af35)"), "{e}");
    assert!(e.contains("[73c5da0a/48'/0'/0'/2']"), "{e}");
    assert!(e.contains("[73c5da0a/48'/0'/1'/2']"), "{e}");
}

// ─── SPEC row 10, surplus variant (c): different-id extra card — the
// ─── existing leftover path, UNCHANGED. Covered by the pre-existing
// ─── `v_leftover_reaches_the_command_naming_the_card` row below
// ─── (PATHOLOGICAL's own 11-card set + V_LEFTOVER's unrelated foreign
// ─── card, no shared id anywhere) — re-confirmed green post-wiring, no
// ─── new test needed.

// ─── V-CSID-WARN — contract 6: the seat-path R2/R6 warning ─────────────

const V_CSID_WARN: &str = include_str!("fixtures/seating/v-csid-warn.txt");

/// A pinned chunk-set-id mismatch that still SEATS gets exactly one extra
/// stderr note; composition, stdout, wallet id and exit code are unchanged
/// from V-USP itself.
///
/// V-USP/V-CSID-WARN are both the V-AMB fixture (same origin at both
/// slots — see `v_amb_the_ambiguity_refusal_reaches_the_operator_with_exit_1`
/// above): seating either needs `--seat '@0=<id>'` to disambiguate, which is
/// orthogonal to contract 6 and asserted elsewhere; here it just needs to
/// hold IDENTICALLY on both sides for the byte-identical-output comparison
/// to mean anything.
#[test]
fn v_csid_seat_warning_fires_on_pinned_mismatch_and_composes_identically() {
    let warned = seat_cmd(
        "descriptor",
        V_CSID_WARN,
        &mk1(V_CSID_WARN),
        &["--seat", "@0=99999"],
    )
    .output()
    .unwrap();
    assert!(warned.status.success(), "{}", err_of(&warned));
    let clean = seat_cmd("descriptor", V_USP, &mk1(V_USP), &["--seat", "@0=69f0e"])
        .output()
        .unwrap();
    assert!(clean.status.success(), "{}", err_of(&clean));
    assert_eq!(
        out_of(&warned),
        out_of(&clean),
        "composition/stdout is byte-identical to the clean twin (contract 6)"
    );

    let e = err_of(&warned);
    let matches: Vec<&str> = e
        .lines()
        .filter(|l| l.contains("was not derived from its content"))
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "exactly one mismatch note, for the one mismatching group: {e}"
    );
    assert!(matches[0].contains("(99999)"), "{}", matches[0]);
    assert!(matches[0].contains("computes 69f0e"), "{}", matches[0]);
    assert!(matches[0].starts_with("warning:"), "{}", matches[0]);
}

/// The clean-twin control (V-USP unmodified): no card's declared id was
/// pinned, so the warning never fires.
#[test]
fn v_csid_seat_no_warning_on_a_clean_card_set() {
    let o = seat_cmd("descriptor", V_USP, &mk1(V_USP), &["--seat", "@0=69f0e"])
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", err_of(&o));
    assert!(
        !err_of(&o).contains("was not derived from its content"),
        "{}",
        err_of(&o)
    );
}

// ─── V-R9 — --from-mk1 arity (design/SPEC_mdcli_mini.md "R9"; FOLLOWUPS
// `from-mk1-arity-spills-card-strings-into-the-md1-positional`) ─────────
//
// `seat_cmd` above always repeats `--from-mk1` once per card -- the
// spelling that already worked before this phase. These rows exercise the
// spelling that did NOT: one `--from-mk1` occurrence carrying several
// values, which is what a natural paste of a scanned card set produces.

/// `md <verb> <phrases...> [--from-mk1 <values...>]`, with ALL of
/// `from_mk1_values` on ONE occurrence of the flag (unlike `seat_cmd`) --
/// this is what a natural multi-value paste looks like on argv, and it is
/// the shape that lets clap's greedy multi-value consumption swallow a
/// trailing positional. The flag is omitted entirely when
/// `from_mk1_values` is empty, since `num_args = 1..` refuses a bare
/// `--from-mk1` with nothing after it.
fn r9_cmd(verb: &str, phrases: &[String], from_mk1_values: &[String]) -> Command {
    let mut c = md();
    c.arg(verb);
    for p in phrases {
        c.arg(p);
    }
    if !from_mk1_values.is_empty() {
        c.arg("--from-mk1");
        for v in from_mk1_values {
            c.arg(v);
        }
    }
    c
}

/// The exact rendered line (Acceptance 4) for every R9 refusal below,
/// asserted the same way `refusal_of`'s siblings do it: exactly one `md: `
/// line, matched in full.
fn assert_one_rendered_line(out: &std::process::Output, expected: &str) {
    let e = err_of(out);
    let lines: Vec<&str> = e.lines().filter(|l| l.starts_with("md: ")).collect();
    assert_eq!(lines.len(), 1, "expected exactly one rendered line:\n{e}");
    assert_eq!(lines[0], expected);
}

/// Row 1 -- POSITIONAL-FIRST composes. The md1 policy phrase(s) precede
/// `--from-mk1` on the command line, so clap claims them for `phrases`
/// before `--from-mk1`'s greedy multi-value consumption ever starts; the
/// single occurrence then takes all 30 key cards, which is exactly the
/// FOLLOWUPS journey (a 30-card vault, pasted once). Proved against the
/// pre-existing repeated-flag spelling (`seat_cmd`), which composes the
/// same wallet by construction (P2, `v_dup_*` above) -- so this row
/// isolates the arity fix from the seating engine itself.
#[test]
fn v_r9_positional_first_natural_paste_composes() {
    let phrases = md1(PATHOLOGICAL);
    let cards = mk1(PATHOLOGICAL);
    assert_eq!(cards.len(), 30, "fixture: the full pathological card set");

    let natural = r9_cmd("descriptor", &phrases, &cards).output().unwrap();
    assert!(
        natural.status.success(),
        "positional-first natural paste did not compose: {}",
        err_of(&natural)
    );

    let repeated = seat_cmd("descriptor", PATHOLOGICAL, &cards, &[])
        .output()
        .unwrap();
    assert!(repeated.status.success(), "{}", err_of(&repeated));
    assert_eq!(
        out_of(&natural),
        out_of(&repeated),
        "a single-occurrence natural paste must compose the identical wallet \
         the repeated-flag spelling does"
    );
}

/// Row 2 -- FLAG-FIRST with a trailing md1 string. Nothing precedes
/// `--from-mk1` on the command line, so ITS single occurrence swallows
/// every value that follows, including the md1 policy phrase a natural
/// paste (policy card typed after the keys, or the keys pasted before
/// scrolling to the policy line) would leave trailing. Per SPEC R9(b) this
/// must NOT surface as clap's own missing-required-argument error -- it
/// must be the symmetric guard's named diagnostic, pointing back at the
/// positional.
fn assert_flag_first_trailing_md1_refuses(verb: &str) {
    let mut values = mk1(V_USP);
    let policy = md1(V_USP);
    assert_eq!(policy.len(), 1, "fixture: a single-chunk policy card");
    values.push(policy[0].clone());

    let out = r9_cmd(verb, &[], &values).output().unwrap();
    assert_eq!(
        out.status.code(),
        Some(1),
        "flag-first with a trailing md1 string did not refuse (or refused \
         with the wrong exit code -- possibly clap's own missing-required- \
         argument error rather than the named guard): {}",
        err_of(&out)
    );
    assert!(out_of(&out).is_empty(), "nothing on stdout when refusing");
    assert_one_rendered_line(
        &out,
        &format!(
            "md: seating refused: `{bad}` is an md1 policy-card string, not an mk1 key card, \
             and does not belong among --from-mk1's values. A single --from-mk1 can take \
             several values now; if this string trailed a run of key cards on one command \
             line, put it back on {verb}'s positional instead, where the policy phrase \
             belongs.",
            bad = policy[0],
        ),
    );
}

#[test]
fn v_r9_flag_first_trailing_md1_string_refuses_naming_the_positional() {
    for verb in ["descriptor", "address"] {
        assert_flag_first_trailing_md1_refuses(verb);
    }
}

/// Row 3 -- an mk1 string DIRECTLY on the positional, no `--from-mk1` at
/// all. The general shape the FOLLOWUPS remedy (b) describes: "which also
/// catches a card string arriving there by any other route."
fn assert_mk1_in_positional_refuses(verb: &str) {
    let cards = mk1(V_USP);
    let out = r9_cmd(verb, &[cards[0].clone()], &[]).output().unwrap();
    assert_eq!(
        out.status.code(),
        Some(1),
        "an mk1 string on the positional did not refuse: {}",
        err_of(&out)
    );
    assert!(out_of(&out).is_empty(), "nothing on stdout when refusing");
    assert_one_rendered_line(
        &out,
        &format!(
            "md: seating refused: `{bad}` is an mk1 key-card string, not an md1 policy card, \
             and does not belong on {verb}'s positional (that positional is for md1 policy \
             phrases only). Pass key cards via --from-mk1 <STRING> (repeatable) or \
             --from-mk1-file <FILE>.",
            bad = cards[0],
        ),
    );
}

#[test]
fn v_r9_mk1_string_in_the_positional_refuses_naming_from_mk1() {
    for verb in ["descriptor", "address"] {
        assert_mk1_in_positional_refuses(verb);
    }
}

/// Row 4 -- `--from-mk1` with values but NO policy card anywhere (no
/// positional, no `--template`). Before this phase's `ArgGroup` widening
/// (needed for Row 2 above) clap's own `required(true)` caught this; the
/// widening reopens it, so `check_from_mk1_arity` must close it itself
/// rather than falling through to `seat::run`'s `reassemble(&[])` -- a
/// bare "chunk set is empty" codec error naming neither `--from-mk1` nor
/// the missing policy card (the FOLLOWUPS class this phase exists to
/// close). Exit 2 (BadArg), matching what clap's own group error would
/// have produced for the same shape.
#[test]
fn v_r9_from_mk1_with_no_policy_card_anywhere_refuses_naming_the_missing_policy() {
    let cards = mk1(V_USP);
    let out = r9_cmd("descriptor", &[], &cards).output().unwrap();
    assert_eq!(
        out.status.code(),
        Some(2),
        "--from-mk1 with no policy card anywhere did not refuse as a usage \
         error, or fell through to seating with an empty policy: {}",
        err_of(&out)
    );
    assert!(out_of(&out).is_empty(), "nothing on stdout when refusing");
    assert_one_rendered_line(
        &out,
        "md: descriptor --from-mk1 supplies key cards, not the policy: no md1 policy phrase is \
         on the positional and no --template was given. Supply the keyless md1 policy card(s) \
         these keys seat into.",
    );
}
