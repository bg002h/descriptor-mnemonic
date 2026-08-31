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
//! trip, and the primary row below measures that as byte-identity rather than
//! asserting it.
//!
//! ## The oracle (SPEC N2 "Oracle", r1 I4)
//!
//! > Byte-identity is decided by the FULL input set: template, keys, per-slot
//! > origins, per-slot FINGERPRINTS …, and the path-declaration SHAPE.
//!
//! The PRIMARY row is the spec's primary form, measured executable rather
//! than falling back: `md encode <template with INLINE per-slot origins>
//! --key @i=XPUB --fingerprint @i=HEX`, one `--fingerprint` per fingerprint
//! the policy card declares. Its stdout and `--emit md1`'s stdout are
//! compared byte for byte.
//!
//! The SECONDARY rows are `spend_equal` and address-0 equality against the
//! KEYED fixture card `v-spendeq-keyed.txt`, which is the same wallet minted
//! with DIFFERENT declared fingerprints. Both are computed in
//! `tests/common/facts.rs` from a rust-miniscript parse of the emitted
//! descriptor STRING — never by asking `src/seat` whether it succeeded.
//!
//! ## Input modes
//!
//! `--emit md1` is admissible ONLY with `--from-mk1`/`--from-mk1-file`, and
//! the two refusals are rendered-line rows (Acceptance 4). It changes ONLY
//! the output form: a seating refusal is pinned byte-identical with and
//! without it, and it composes with `--seat`.

use assert_cmd::Command;
use std::io::Write;

#[path = "common/facts.rs"]
mod common_facts;
use common_facts::{Facts, facts, spend_equal, spend_equal_report};

const V_B1_WALLET: &str = include_str!("fixtures/seating/v-b1-wallet.txt");
const V_SPENDEQ_KEYED: &str = include_str!("fixtures/seating/v-spendeq-keyed.txt");
const V_USP: &str = include_str!("fixtures/seating/v-usp.txt");
const V_LEGACY_P2SH: &str = include_str!("fixtures/seating/v-legacy-p2sh.txt");
const KEYS_TXT: &str = include_str!("fixtures/pathological/keys.txt");

/// V-B1-WALLET's own mint command, from the provenance header of
/// `fixtures/seating/v-b1-wallet.txt`. `fixture_header_still_records_this_mint`
/// asserts the fixture still says so, so a regenerated fixture cannot leave
/// the oracle row measuring a template nobody minted.
const B1_TPL: &str = "wsh(sortedmulti(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/1'/2'/<0;1>/*))";
/// The fingerprint the policy card declares for BOTH slots.
const B1_FP: &str = "73c5da0a";

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

/// Address 0 of the receive chain, derived by rust-miniscript from the
/// descriptor STRING — not by a second run of md's own engine.
fn address_zero_of(desc: &str) -> String {
    use miniscript::descriptor::{Descriptor, DescriptorPublicKey};
    use std::str::FromStr;
    let d = Descriptor::<DescriptorPublicKey>::from_str(desc)
        .unwrap_or_else(|e| panic!("the composed descriptor must parse: {e}"));
    d.into_single_descriptors()
        .expect("multipath descriptor splits into single-path forms")
        .into_iter()
        .next()
        .expect("chain 0 exists")
        .derive_at_index(0)
        .expect("index 0 derives")
        .address(bitcoin::Network::Bitcoin)
        .expect("a wsh descriptor has an address")
        .to_string()
}

// ─── the fixture provenance pin ─────────────────────────────────────────

/// The oracle row below hands `md encode` a template and two `--fingerprint`
/// flags typed out as constants. This is what keeps them the FIXTURE's, not
/// this file's opinion of it: v-b1-wallet.txt records its own mint command in
/// its provenance header, and if the fixture is ever regenerated from a
/// different template the oracle stops measuring anything and this row says
/// so first.
#[test]
fn n2_fixture_header_still_records_the_mint_the_oracle_reproduces() {
    let header: String = V_B1_WALLET
        .lines()
        .filter(|l| l.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        header.contains(B1_TPL),
        "v-b1-wallet.txt's header no longer records the template the oracle uses:\n{header}"
    );
    for i in [0, 1] {
        assert!(
            header.contains(&format!("--fingerprint @{i}={B1_FP}")),
            "v-b1-wallet.txt's header no longer declares @{i}={B1_FP}:\n{header}"
        );
    }
    // And the two keys the oracle seats are the records those origins name.
    for (n, want) in [(1usize, "48'/0'/0'/2'"), (2usize, "48'/0'/1'/2'")] {
        let (origin, xpub) = key_record(n);
        assert_eq!(origin, format!("{B1_FP}/{want}"), "keys.txt record {n}");
        assert!(xpub.starts_with("xpub"), "keys.txt record {n}: {xpub}");
    }
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

// ─── step 3 — the oracle ────────────────────────────────────────────────

/// **PRIMARY.** The minted card is byte-identical to the one `md encode`
/// mints from the same wallet, given the template with INLINE per-slot
/// origins and one `--fingerprint @i=HEX` per fingerprint the policy card
/// declares — SPEC N2's primary oracle form, which the spec records as
/// measured-executable at the cycle baseline.
///
/// stdout against stdout: `md encode` puts the artifact there unbroken, and
/// so does this.
#[test]
fn n2_emit_md1_is_byte_identical_to_the_md_encode_mint_of_the_same_wallet() {
    let (_, k1) = key_record(1);
    let (_, k2) = key_record(2);
    let oracle = md()
        .args([
            "encode",
            B1_TPL,
            "--key",
            &format!("@0={k1}"),
            "--key",
            &format!("@1={k2}"),
            "--fingerprint",
            &format!("@0={B1_FP}"),
            "--fingerprint",
            &format!("@1={B1_FP}"),
        ])
        .output()
        .unwrap();
    assert!(oracle.status.success(), "{}", err_of(&oracle));

    let minted = emit_b1();
    assert!(minted.status.success(), "{}", err_of(&minted));

    assert_eq!(
        out_of(&minted),
        out_of(&oracle),
        "the seated mint and the `md encode` mint are one card"
    );
    // The comparison is only worth something if there is a card in it.
    assert!(
        out_of(&oracle).starts_with("md1"),
        "the oracle minted a card: {}",
        out_of(&oracle)
    );
    // Both name the same chunk set, which is the id an operator writes down.
    let id_of = |o: &std::process::Output| -> String {
        err_of(o)
            .lines()
            .find_map(|l| l.strip_prefix("chunk-set-id: ").map(str::to_string))
            .expect("a chunked mint names its set id")
    };
    assert_eq!(id_of(&minted), id_of(&oracle));
}

/// **SECONDARY.** The minted card is SPEND-EQUAL to the keyed fixture card of
/// the same wallet, and derives the same address 0 — with the fixture card
/// deliberately declaring DIFFERENT fingerprints, so the row exercises the
/// relation rather than string equality.
#[test]
fn n2_emit_md1_is_spend_equal_to_the_keyed_fixture_card_and_shares_address_zero() {
    let minted = emit_b1();
    assert!(minted.status.success(), "{}", err_of(&minted));
    let minted_cards: Vec<String> = out_of(&minted).lines().map(str::to_string).collect();

    let a = descriptor_of(&minted_cards);
    let b = descriptor_of(&md1(V_SPENDEQ_KEYED));

    // The two forms declare different origin metadata...
    assert_ne!(a, b, "the fixture card declares another fingerprint");
    let (fa, fb): (Facts, Facts) = (facts(&a), facts(&b));
    assert_ne!(fa.origins, fb.origins, "which is what differs");
    // ...and are still spend-equal, origin metadata excluded.
    assert!(spend_equal(&fa, &fb), "{}", spend_equal_report(&fa, &fb));
    assert_eq!(address_zero_of(&a), address_zero_of(&b));
}

/// **`--emit` changes ONLY the output form.** Every A2/A3/A4 seating rule is
/// untouched, so a refusal is the same refusal — byte for byte on stderr, and
/// still nothing on stdout.
#[test]
fn n2_emit_md1_leaves_a_seating_refusal_exactly_as_it_was() {
    let plain = seat_cmd("descriptor", V_USP, &mk1(V_USP), &[])
        .output()
        .unwrap();
    let emitted = seat_cmd("descriptor", V_USP, &mk1(V_USP), &["--emit", "md1"])
        .output()
        .unwrap();
    assert_eq!(plain.status.code(), Some(1), "{}", err_of(&plain));
    assert_eq!(emitted.status.code(), plain.status.code());
    assert!(
        out_of(&emitted).is_empty(),
        "nothing on stdout when refusing"
    );
    assert_eq!(err_of(&emitted), err_of(&plain), "one refusal, one wording");
}

/// And it composes with `--seat`: the assertion that resolves that refusal
/// resolves it here too, and what comes back is a card.
#[test]
fn n2_emit_md1_composes_with_seat() {
    let refused = seat_cmd("descriptor", V_USP, &mk1(V_USP), &[])
        .output()
        .unwrap();
    let id = err_of(&refused)
        .lines()
        .find_map(|l| l.trim().strip_prefix("card "))
        .and_then(|s| s.split(' ').next().map(str::to_string))
        .expect("the refusal lists the cards that fit more than one slot");
    assert_eq!(id.len(), 5, "a full five-hex-digit id: {id}");

    let o = seat_cmd(
        "descriptor",
        V_USP,
        &mk1(V_USP),
        &["--seat", &format!("@0={id}"), "--emit", "md1"],
    )
    .output()
    .unwrap();
    assert!(o.status.success(), "{}", err_of(&o));
    assert!(
        out_of(&o).starts_with("md1"),
        "a seated mint is a card: {}",
        out_of(&o)
    );
}

// ─── step 4 — advisories carry across the new minting surface (r1 I3) ───────
//
// `--emit md1` is a minting surface too, and which command engraved the
// plate makes no difference to what it claims. `md encode` emits five things
// around a mint; this cell only carried `emit_output_class_advisory`.
// F-227 and F-410 are genuinely inapplicable ONLY here (a keyed card and no
// `--template`), so this row measures F-A4 only, on a policy shape no other
// fixture in this directory carries: a bare `sh(sortedmulti(...))`.

/// Measured ABSENT before this fold: `md descriptor <V-LEGACY-P2SH>
/// --from-mk1 <its cards> --emit md1` minted at exit 0 with zero occurrences
/// of "legacy P2SH" on stderr, while `md encode` of the identical wallet (the
/// oracle row's shape, `sh(sortedmulti(...))` instead of `wsh(...)`) warns
/// every time. Same wallet, same defect class, two commands, one answer now.
#[test]
fn n2_emit_md1_carries_the_legacy_p2sh_advisory_across() {
    let o = seat_cmd(
        "descriptor",
        V_LEGACY_P2SH,
        &mk1(V_LEGACY_P2SH),
        &["--emit", "md1"],
    )
    .output()
    .unwrap();
    assert!(o.status.success(), "{}", err_of(&o));
    assert!(
        out_of(&o).starts_with("md1"),
        "the mint still happened -- this is warn-only: {}",
        out_of(&o)
    );
    assert!(
        err_of(&o).contains(
            "sh(multi)/sh(sortedmulti) is legacy P2SH multisig \u{2014} susceptible to \
             third-party txid malleability"
        ),
        "the F-A4 advisory did not carry across to --emit md1: {}",
        err_of(&o)
    );
}
