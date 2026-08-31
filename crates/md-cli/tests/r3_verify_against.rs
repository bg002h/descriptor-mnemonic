#![allow(missing_docs)]
//! **R3 — `md descriptor --verify-against <md1|FILE>`** (plan P6 step 1;
//! `design/SPEC_mdcli_mini.md` "Riders" R3). Closes FOLLOWUPS
//! `md-verify-against-flag-for-cross-form-comparison`.
//!
//! Wires `seat::compose::spend_equal`/`spend_equal_verdict`
//! (`src/seat/compose.rs`, row-pinned four ways there already by P2's own
//! unit tests — this file drives the CLI ENTRANCE, not the checker again).
//!
//! **Exit codes** (SPEC R3): 0 = spend-equal; 5 = NOT spend-equal (`md
//! repair`'s reserved-5 precedent for a non-error, non-default answer); 1/2
//! = errors, unchanged — so a mistyped `--verify-against` argument can never
//! read as "not equal", the false signal the FOLLOWUP warns invites
//! re-cutting a good plate.
//!
//! **The flag is admissible on every composing input mode** — deliberately
//! NOT the T-row `requires = "template"` + `conflicts_with_all` pattern (r1
//! I5), which would make it unusable on exactly the two modes (the keyed
//! positional, `--from-mk1` seating) the FOLLOWUP exists for. Rows (a) and
//! (b) below exercise both.
//!
//! Five rows (SPEC R3 / plan P6 step 1): (a) an equal cross-form pair via
//! `--from-mk1` against a FILE holding the keyed card; (b) a keyed-card
//! POSITIONAL composition with `--verify-against` — the MODE row; (c) a
//! one-xpub-off negative, naming the values half; (d) an origins-differ
//! pair, still EQUAL; (e) a garbage `--verify-against` argument — a decode
//! error, never a verdict.

use assert_cmd::Command;
use std::io::Write;

const V_B1_WALLET: &str = include_str!("fixtures/seating/v-b1-wallet.txt");
const V_SPENDEQ_KEYED: &str = include_str!("fixtures/seating/v-spendeq-keyed.txt");
const KEYS_TXT: &str = include_str!("fixtures/pathological/keys.txt");

// ─── helpers (mirrors n2_emit_md1.rs's, which pins the same fixtures) ──────

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

fn out_of(o: &std::process::Output) -> String {
    String::from_utf8_lossy(&o.stdout).to_string()
}
fn err_of(o: &std::process::Output) -> String {
    String::from_utf8_lossy(&o.stderr).to_string()
}

/// `md <verb> <policy md1...> --from-mk1 <each mk1> [extra...]` — the same
/// shape `n2_emit_md1.rs::seat_cmd` and `seating_vectors.rs::seat_cmd` build,
/// one flag occurrence per card.
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

/// Acceptance 4: the RENDERED stderr line, from the `md: ` prefix onward,
/// and exactly one of them.
fn assert_one_rendered_line(o: &std::process::Output, expected: &str) {
    let e = err_of(o);
    let l: Vec<&str> = e.lines().filter(|l| l.starts_with("md: ")).collect();
    assert_eq!(l.len(), 1, "expected exactly one rendered line:\n{e}");
    assert_eq!(l[0], expected);
}

/// The n-th (1-based) BIP-380 origin-notated record of the pathological key
/// file, split at the origin bracket: `("73c5da0a/48'/0'/0'/2'", "xpub…")`.
/// Reads the fixture rather than retyping a base58 string by hand.
fn key_record(n: usize) -> (String, String) {
    let recs: Vec<&str> = KEYS_TXT
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with('['))
        .collect();
    assert_eq!(recs.len(), 11, "fixture: keys.txt holds 11 key records");
    let r = recs[n - 1];
    let close = r.find(']').expect("origin notation closes");
    (r[1..close].to_string(), r[close + 1..].to_string())
}

/// Write `lines` (one per line) to `dir/name`, returning the path. The
/// FILE half of `--verify-against <md1|FILE>`.
fn write_lines(dir: &std::path::Path, name: &str, lines: &[String]) -> std::path::PathBuf {
    let p = dir.join(name);
    let mut f = std::fs::File::create(&p).unwrap();
    for l in lines {
        writeln!(f, "{l}").unwrap();
    }
    p
}

/// Mint a 2-of-2 keyed md1 card via `md encode <TPL> --key @0=X --key @1=Y
/// --fingerprint @0=FP --fingerprint @1=FP` — the same oracle construction
/// `n2_emit_md1.rs`'s PRIMARY row uses. Returns the md1 lines.
fn mint_two_of_two(fp: &str, xpub0: &str, xpub1: &str) -> Vec<String> {
    const TPL: &str = "wsh(sortedmulti(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/1'/2'/<0;1>/*))";
    let o = md()
        .args([
            "encode",
            TPL,
            "--key",
            &format!("@0={xpub0}"),
            "--key",
            &format!("@1={xpub1}"),
            "--fingerprint",
            &format!("@0={fp}"),
            "--fingerprint",
            &format!("@1={fp}"),
        ])
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", err_of(&o));
    md1(&out_of(&o))
}

/// Compose a set of md1 card strings back into a concrete descriptor
/// string, via `md descriptor` itself — the same helper
/// `n2_emit_md1.rs::descriptor_of` uses.
fn descriptor_of(cards: &[String]) -> String {
    let mut c = md();
    c.arg("descriptor");
    for s in cards {
        c.arg(s);
    }
    let o = c.output().unwrap();
    assert!(o.status.success(), "{}", err_of(&o));
    out_of(&o).trim_end().to_string()
}

const ORIGIN_NOTE: &str = "Origin metadata (fingerprints, key origins, path declaration) is \
                            excluded from this comparison: it is seating/signing guidance, not \
                            script content.";

fn equal_line() -> String {
    format!(
        "md: --verify-against: SPEND-EQUAL — same template structure, per-slot key values and \
         per-slot use-site paths. {ORIGIN_NOTE}"
    )
}
fn not_equal_line(half: &str) -> String {
    format!("md: --verify-against: NOT spend-equal — the {half} half differs. {ORIGIN_NOTE}")
}

// ─── (a) equal cross-form pair via --from-mk1 vs the keyed card ───────────

#[test]
fn r3_equal_cross_form_pair_via_from_mk1_vs_the_keyed_card() {
    let dir = tempfile::tempdir().unwrap();
    let target = write_lines(dir.path(), "target.txt", &md1(V_SPENDEQ_KEYED));

    let o = seat_cmd(
        "descriptor",
        V_B1_WALLET,
        &mk1(V_B1_WALLET),
        &["--verify-against", target.to_str().unwrap()],
    )
    .output()
    .unwrap();
    assert_eq!(o.status.code(), Some(0), "{}", err_of(&o));
    assert_one_rendered_line(&o, &equal_line());
    // stdout is unaffected: the primary composition still prints.
    assert!(
        out_of(&o).starts_with("wsh(sortedmulti(2,"),
        "{}",
        out_of(&o)
    );
}

// ─── (b) the MODE row — a keyed-card POSITIONAL composition ───────────────

#[test]
fn r3_verify_against_composes_on_a_keyed_card_positional_the_mode_row() {
    // The SAME two wallets as row (a), roles reversed: the PRIMARY
    // composition is the keyed-card POSITIONAL this time
    // (V_SPENDEQ_KEYED), proving the flag is not accidentally scoped to
    // --from-mk1 alone (r1 I5's whole point).
    let split_as_keyed = seat_cmd(
        "descriptor",
        V_B1_WALLET,
        &mk1(V_B1_WALLET),
        &["--emit", "md1"],
    )
    .output()
    .unwrap();
    assert!(
        split_as_keyed.status.success(),
        "{}",
        err_of(&split_as_keyed)
    );
    let target_lines = md1(&out_of(&split_as_keyed));
    assert!(!target_lines.is_empty(), "the split set must mint a card");

    let dir = tempfile::tempdir().unwrap();
    let target = write_lines(dir.path(), "target.txt", &target_lines);

    let mut c = md();
    c.arg("descriptor");
    for p in md1(V_SPENDEQ_KEYED) {
        c.arg(p);
    }
    c.args(["--verify-against", target.to_str().unwrap()]);
    let o = c.output().unwrap();
    assert_eq!(o.status.code(), Some(0), "{}", err_of(&o));
    assert_one_rendered_line(&o, &equal_line());
}

// ─── (c) one-xpub-off negative, names the values half ──────────────────────

#[test]
fn r3_one_xpub_off_is_not_spend_equal_and_names_the_values_half() {
    let (_, x1) = key_record(1);
    let (_, x2) = key_record(2);
    let (_, x3) = key_record(3);
    let card_a = mint_two_of_two("73c5da0a", &x1, &x2);
    let card_b = mint_two_of_two("73c5da0a", &x1, &x3); // slot @1 swapped

    let dir = tempfile::tempdir().unwrap();
    let target = write_lines(dir.path(), "card_b.txt", &card_b);

    let mut c = md();
    c.arg("descriptor");
    for p in &card_a {
        c.arg(p);
    }
    c.args(["--verify-against", target.to_str().unwrap()]);
    let o = c.output().unwrap();
    assert_eq!(o.status.code(), Some(5), "{}", err_of(&o));
    assert_one_rendered_line(&o, &not_equal_line("VALUES"));
    // stdout still carries the PRIMARY composition -- a NOT-equal verdict
    // is not an error, and every A2/A3/A4 rule ran exactly as it does
    // without the flag.
    assert!(
        out_of(&o).starts_with("wsh(sortedmulti(2,"),
        "{}",
        out_of(&o)
    );
}

// ─── (d) origins-differ pair, still EQUAL ──────────────────────────────────

#[test]
fn r3_origins_differ_pair_is_still_spend_equal() {
    let (_, x1) = key_record(1);
    let (_, x2) = key_record(2);
    let card_a = mint_two_of_two("73c5da0a", &x1, &x2);
    let card_a_diff_fp = mint_two_of_two("aabbccdd", &x1, &x2);

    // Prove the premise rather than assume it: the two cards really do
    // carry DIFFERENT declared origins.
    let desc_a = descriptor_of(&card_a);
    let desc_a2 = descriptor_of(&card_a_diff_fp);
    assert_ne!(
        desc_a, desc_a2,
        "different declared fingerprints must render differently"
    );
    assert!(desc_a.contains("73c5da0a"), "{desc_a}");
    assert!(desc_a2.contains("aabbccdd"), "{desc_a2}");

    let dir = tempfile::tempdir().unwrap();
    let target = write_lines(dir.path(), "card_a2.txt", &card_a_diff_fp);

    let mut c = md();
    c.arg("descriptor");
    for p in &card_a {
        c.arg(p);
    }
    c.args(["--verify-against", target.to_str().unwrap()]);
    let o = c.output().unwrap();
    assert_eq!(o.status.code(), Some(0), "{}", err_of(&o));
    assert_one_rendered_line(&o, &equal_line());
}

// ─── (e) garbage --verify-against argument: a decode error, no verdict ────

#[test]
fn r3_garbage_verify_against_argument_is_a_decode_error_never_a_verdict() {
    let (_, x1) = key_record(1);
    let (_, x2) = key_record(2);
    let card_a = mint_two_of_two("73c5da0a", &x1, &x2);

    let mut c = md();
    c.arg("descriptor");
    for p in &card_a {
        c.arg(p);
    }
    c.args(["--verify-against", "not-a-real-md1-or-file-r3-garbage-row"]);
    let o = c.output().unwrap();
    assert_eq!(o.status.code(), Some(1), "{}", err_of(&o));
    let stderr = err_of(&o);
    assert!(
        stderr.starts_with("md: codec error:"),
        "a garbage argument must draw a DECODE error: {stderr}"
    );
    assert!(!stderr.contains("SPEND-EQUAL"), "{stderr}");
    assert!(!stderr.contains("NOT spend-equal"), "{stderr}");
    assert!(
        out_of(&o).is_empty(),
        "nothing on stdout when the target fails to decode: {}",
        out_of(&o)
    );
}

// ─── an EMPTY --verify-against FILE names the flag the operator passed ───
// (whole-diff review r1 M3)
//
// `resolve_verify_against` reuses `cmd::read_md1_inputs`, the same file
// reader every `--in`-taking verb shares — and before this fold that
// function's messages hardcoded "--in", so an empty `--verify-against`
// FILE told the operator about a flag they never typed. Threaded the flag
// name through instead (`cmd::mod::read_md1_inputs`'s new `flag` param).

#[test]
fn r3_an_empty_verify_against_file_names_verify_against_not_in() {
    let (_, x1) = key_record(1);
    let (_, x2) = key_record(2);
    let card_a = mint_two_of_two("73c5da0a", &x1, &x2);

    let dir = tempfile::tempdir().unwrap();
    let empty = write_lines(dir.path(), "empty.txt", &[]);

    let mut c = md();
    c.arg("descriptor");
    for p in &card_a {
        c.arg(p);
    }
    c.args(["--verify-against", empty.to_str().unwrap()]);
    let o = c.output().unwrap();
    assert_eq!(o.status.code(), Some(2), "{}", err_of(&o));
    let stderr = err_of(&o);
    assert!(
        stderr.starts_with("md: --verify-against "),
        "the empty-file refusal must name --verify-against, not --in: {stderr}"
    );
    assert!(
        !stderr.contains("--in "),
        "the flag the operator never typed must not appear: {stderr}"
    );
}
