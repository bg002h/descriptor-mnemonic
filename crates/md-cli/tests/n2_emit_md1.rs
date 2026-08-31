#![allow(missing_docs)]
//! **N2 — `md descriptor … --emit md1`, the S → K cell** (plan P5;
//! `design/SPEC_mdcli_mini.md` "N2 — mint a keyed card from a seating
//! result"). Closes FOLLOWUPS `md-cannot-mint-a-keyed-card-from-a-split-set`.
//!
//! ## What the cell was blocked on, and why this is not that bridge
//!
//! The converter cycle left S → keyed card ✗ because the only route to a
//! keyed card ran through `md encode --key`, which admits an account-level
//! xpub at depth 3 or 4 — and a descriptor composed from mk1 cards carries
//! DEPTH-0 keys, because md's `Pubkeys` TLV holds 65 bytes (chain code ‖
//! compressed point) and no depth field. `--emit md1` does not use that
//! bridge and does not relax its rule: it mints from the SEATING RESULT,
//! whose 65 bytes are exactly what a keyed card needs. Nothing is lost in the
//! trip.
//!
//! ## Input modes
//!
//! `--emit md1` is admissible ONLY with `--from-mk1`/`--from-mk1-file`, and
//! the two refusals are rendered-line rows (Acceptance 4).

use assert_cmd::Command;
use std::io::Write;

const V_B1_WALLET: &str = include_str!("fixtures/seating/v-b1-wallet.txt");
const V_SPENDEQ_KEYED: &str = include_str!("fixtures/seating/v-spendeq-keyed.txt");
const KEYS_TXT: &str = include_str!("fixtures/pathological/keys.txt");

/// V-B1-WALLET's own mint command, from the provenance header of
/// `fixtures/seating/v-b1-wallet.txt`.
const B1_TPL: &str = "wsh(sortedmulti(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/1'/2'/<0;1>/*))";

// ─── helpers ────────────────────────────────────────────────────────────

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

/// `md <verb> <policy md1...> --from-mk1 <each mk1> [extra...]` — the same
/// shape `seating_vectors.rs::seat_cmd` builds, one flag occurrence per card.
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

/// Acceptance 4: the RENDERED stderr line, from the `md: ` prefix onward,
/// and exactly one of them.
fn assert_one_rendered_line(out: &std::process::Output, expected: &str) {
    let e = err_of(out);
    let l: Vec<&str> = e.lines().filter(|l| l.starts_with("md: ")).collect();
    assert_eq!(l.len(), 1, "expected exactly one rendered line:\n{e}");
    assert_eq!(l[0], expected);
}

/// The n-th (1-based) BIP-380 origin-notated record of the pathological key
/// file, split at the origin bracket: `("73c5da0a/48'/0'/0'/2'", "xpub…")`.
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

/// `md descriptor <v-b1-wallet policy> --from-mk1 <its cards> --emit md1`.
fn emit_b1() -> std::process::Output {
    seat_cmd(
        "descriptor",
        V_B1_WALLET,
        &mk1(V_B1_WALLET),
        &["--emit", "md1"],
    )
    .output()
    .unwrap()
}

/// Compose a set of md1 card strings back into a concrete descriptor.
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

// ─── step 1 — emission from the seating result ──────────────────────────

/// The cell itself: a split card set in, a KEYED card out.
#[test]
fn n2_emit_md1_mints_a_keyed_card_from_the_seating_result() {
    let o = emit_b1();
    assert!(o.status.success(), "{}", err_of(&o));
    let cards: Vec<String> = out_of(&o).lines().map(str::to_string).collect();
    assert!(!cards.is_empty(), "stdout carries the card");
    assert!(
        cards.iter().all(|s| s.starts_with("md1")),
        "stdout is the card and nothing else: {cards:?}"
    );
    assert!(
        !out_of(&o).contains("wsh("),
        "--emit md1 replaces the descriptor on stdout, it does not add to it"
    );
    // The chunk-set id is the only thing tying a multi-chunk card together,
    // so it belongs on stderr exactly as `md encode` puts it there.
    if cards.len() > 1 {
        assert!(
            err_of(&o).contains("chunk-set-id: 0x"),
            "a chunked mint names its set id: {}",
            err_of(&o)
        );
    }
    // And what came out is a KEYED card: it composes a concrete wallet with
    // no key cards supplied at all.
    let composed = descriptor_of(&cards);
    assert!(composed.starts_with("wsh(sortedmulti(2,"), "{composed}");
}

/// The `--from-mk1-file` spelling — the one the FOLLOWUPS journey recommends
/// for a 30-card set, and a second entrance that must reach the same mint.
#[test]
fn n2_emit_md1_mints_the_same_card_from_the_from_mk1_file_spelling() {
    let cards = mk1(V_B1_WALLET);
    let mut f = tempfile::NamedTempFile::new().unwrap();
    writeln!(f, "# the v-b1-wallet key cards, one per line").unwrap();
    for s in &cards {
        writeln!(f, "{s}").unwrap();
    }
    let path = f.into_temp_path();

    let mut c = md();
    c.arg("descriptor");
    for p in md1(V_B1_WALLET) {
        c.arg(p);
    }
    c.args(["--from-mk1-file", path.to_str().unwrap()]);
    c.args(["--emit", "md1"]);
    let via_file = c.output().unwrap();
    assert!(via_file.status.success(), "{}", err_of(&via_file));

    let via_flags = emit_b1();
    assert_eq!(
        out_of(&via_file),
        out_of(&via_flags),
        "both card channels mint one card"
    );
}

/// **Plan P4 guard-scope note (r1 M6), discharged.** P4's md1-prefix guard
/// scopes to `--from-mk1`'s values and the positional ONLY. `--emit`'s own
/// literal value is the string `md1`, so a guard reading one flag too widely
/// would refuse this invocation — and `--from-mk1`'s `num_args = 1..` greedy
/// consumption would swallow the value outright if `--emit` did not stop it.
/// Spelled flag-first, with the cards run together on one occurrence, which
/// is the ordering that makes both failures reachable.
#[test]
fn n2_emit_md1_is_not_swallowed_or_refused_by_the_from_mk1_arity_guard() {
    let mut c = md();
    c.arg("descriptor");
    for p in md1(V_B1_WALLET) {
        c.arg(p);
    }
    c.arg("--from-mk1");
    for s in mk1(V_B1_WALLET) {
        c.arg(s);
    }
    c.args(["--emit", "md1"]);
    let o = c.output().unwrap();
    assert!(
        o.status.success(),
        "the literal --emit value must not reach the md1-prefix guard: {}",
        err_of(&o)
    );
    assert_eq!(
        out_of(&o),
        out_of(&emit_b1()),
        "one occurrence carrying every card mints the same card"
    );
}

// ─── step 2 — the input-mode refusals (Acceptance 4 rendered lines) ──────

/// `--emit md1` with `--template`: the template row already has a minting
/// tool, and the refusal's whole content is saying which.
#[test]
fn n2_emit_md1_with_a_template_refuses_naming_md_encode() {
    let (_, k1) = key_record(1);
    let (_, k2) = key_record(2);
    let o = md()
        .args([
            "descriptor",
            "--template",
            B1_TPL,
            "--key",
            &format!("@0={k1}"),
            "--key",
            &format!("@1={k2}"),
            "--emit",
            "md1",
        ])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(2), "{}", err_of(&o));
    assert!(out_of(&o).is_empty(), "nothing on stdout when refusing");
    assert_one_rendered_line(
        &o,
        "md: --emit md1 mints a keyed card from a SEATED card set, so it needs the keyless \
         policy card together with its mk1 key cards (--from-mk1 <STRING>, repeatable, or \
         --from-mk1-file <FILE>). Minting a card from a template plus keys is what `md encode \
         <TEMPLATE> --key @i=XPUB` does -- use that.",
    );
}

/// `--emit md1` on a card positional: there is nothing to mint, because the
/// thing it would mint is the argument.
#[test]
fn n2_emit_md1_on_a_keyed_card_positional_refuses_as_a_re_emit() {
    let mut c = md();
    c.arg("descriptor");
    for p in md1(V_SPENDEQ_KEYED) {
        c.arg(p);
    }
    c.args(["--emit", "md1"]);
    let o = c.output().unwrap();
    assert_eq!(o.status.code(), Some(2), "{}", err_of(&o));
    assert!(out_of(&o).is_empty(), "nothing on stdout when refusing");
    assert_one_rendered_line(
        &o,
        "md: --emit md1 mints a keyed card from a SEATED card set, so it needs the keyless \
         policy card together with its mk1 key cards (--from-mk1 <STRING>, repeatable, or \
         --from-mk1-file <FILE>). These md1 phrases are a card already, and re-emitting a card \
         you are holding would hand back what you pasted in. Drop --emit md1 to render this \
         card as a descriptor.",
    );
}

/// `--json` names an envelope with a `descriptor` field, which `--emit md1`
/// does not produce. Declared a clap conflict rather than answered by a
/// diagnostic of md's own: the cycle ships no new JSON envelope (SPEC
/// "Non-goals"), and a silently discarded `--json` is the defect class
/// REVIEW-converter-whole-diff-r1 I4 found on this very verb.
#[test]
fn n2_emit_md1_and_json_are_declared_mutually_exclusive() {
    let o = seat_cmd(
        "descriptor",
        V_B1_WALLET,
        &mk1(V_B1_WALLET),
        &["--emit", "md1", "--json"],
    )
    .output()
    .unwrap();
    assert_eq!(o.status.code(), Some(2), "{}", err_of(&o));
    assert!(err_of(&o).contains("cannot be used with"), "{}", err_of(&o));
    assert!(
        !out_of(&o).contains("md1") && !out_of(&o).contains("wsh("),
        "nothing composed: {}",
        out_of(&o)
    );
}
