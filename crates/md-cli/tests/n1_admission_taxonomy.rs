#![allow(missing_docs)]
//! N1 — the placeholder/key-reuse admission taxonomy
//! (`design/SPEC_mdcli_mini.md` §N1).
//!
//! Two halves, in phase order: the MINT/COMPOSE surface reached through
//! `--template` (plan P2), then the same taxonomy reached through an md1
//! CARD plus the READ side that must keep reading one (plan P3, from
//! "P3 — the CARD input and the READ side" below). They share this file
//! because they must share the RENDERED LINE — one predicate, one message,
//! the disposition the only difference.
//!
//! Every row here asserts the **RENDERED stderr line, from the `md:` prefix
//! onward** (Acceptance 4). A body substring would pass under a prefix that
//! blames the input — "template parse error:" — which is exactly what R-N1c's
//! message mandate forbids, so the prefix is part of the contract and the
//! rows pin the whole line.
//!
//! WHAT IS NOT HERE. The SHIPPED same-use-site refusal
//! (`validate_no_duplicate_key_slots`, F-218) keeps its own wording and its
//! own rows in `duplicate_key_slots.rs`; R-N1d here is the DISJOINT
//! use-site DELTA only. The SEATING side — the door check and
//! `check_no_repeated_xpub` — is pinned where it lives, in `src/seat/` and
//! `tests/seating_vectors.rs`.

use assert_cmd::Command;

/// KEY 1 of `tests/fixtures/pathological/keys.txt` — `[73c5da0a/48'/0'/0'/2']`.
const K0: &str = "xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf";
/// KEY 2 — `[73c5da0a/48'/0'/1'/2']`. Same master, a DIFFERENT account.
const K1: &str = "xpub6DzhyrnFFYQ1HimDiM388xHnDiRPNdZJFBmmxge3Y1WWcHLtMJLfRuhRHqnQCPbTj3fGKTuKFLHzzwpJkp5Dtc3UtLKZKaVZe1yqMBXd6Vk";

const PATH: &str = "48'/0'/0'/2'";

fn md() -> Command {
    Command::cargo_bin("md").unwrap()
}

/// The single rendered stderr line, from the `md:` prefix onward.
///
/// Asserts there is EXACTLY ONE `md:` line, so a row cannot pass by finding
/// its text beside a second diagnostic the operator would also have to read.
fn rendered_line(out: &std::process::Output) -> String {
    let err = String::from_utf8_lossy(&out.stderr);
    let lines: Vec<&str> = err.lines().filter(|l| l.starts_with("md: ")).collect();
    assert_eq!(
        lines.len(),
        1,
        "expected exactly one rendered `md:` line, got {}:\n{err}",
        lines.len()
    );
    lines[0].to_owned()
}

fn encode(template: &str, keys: &[&str]) -> std::process::Output {
    let mut c = md();
    c.args(["encode", template, "--path", PATH, "--group-size", "0"]);
    for k in keys {
        c.args(["--key", k]);
    }
    c.output().unwrap()
}

fn descriptor(template: &str, keys: &[&str]) -> std::process::Output {
    let mut c = md();
    c.args(["descriptor", "--template", template, "--path", PATH]);
    for k in keys {
        c.args(["--key", k]);
    }
    c.output().unwrap()
}

fn address(template: &str, keys: &[&str]) -> std::process::Output {
    let mut c = md();
    c.args([
        "address",
        "--template",
        template,
        "--path",
        PATH,
        "--count",
        "1",
    ]);
    for k in keys {
        c.args(["--key", k]);
    }
    c.output().unwrap()
}

/// A refusal must be exit 1 (a content refusal, not a usage error at 2), must
/// print NOTHING on stdout, and must never call the operator's wallet invalid.
fn assert_refused(out: &std::process::Output, expected_line: &str) {
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(1),
        "expected a content refusal at exit 1; stdout was:\n{stdout}\nstderr was:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.trim().is_empty(),
        "a wallet was printed alongside the refusal:\n{stdout}"
    );
    let line = rendered_line(out);
    assert_eq!(line, expected_line);
    assert!(
        !line.to_lowercase().contains("invalid"),
        "the operator's wallet must never be called invalid (Principle, operator \
         ruling 2026-08-30): {line}"
    );
}

// ─── R-N1a — one placeholder, several use sites, IDENTICAL triples ──────────
//
// BIP 388 forbids it by name. Refused on all three mint/compose entrances,
// and — because the taxonomy has ONE implementation per predicate and the
// per-verb disposition is only a parameter — with the SAME rendered line on
// each, which the shared constant below is what proves.

const T_N1A: &str = "wsh(sortedmulti(2,@0/<0;1>/*,@0/<0;1>/*))";

const MSG_N1A: &str = "md: unsupported: @0 appears at 2 use sites in this template with the same \
path expression, so ONE key would fill every one of them. That is forbidden by BIP 388's \
disjointness rule (\"if two KEY are KP/<M;N>/* and KP/<P;Q>/* for the same key placeholder KP, \
then the sets {M, N} and {P, Q} must be disjoint\"), whose forbidden-example list names \
sh(multi(1,@0/**,@0/**)) — \"Repeated keys with the same path expression\". md declines to mint \
or compose this shape: give each distinct key its own placeholder.";

/// The READ-side WARN rendering of the same finding (review r1 M4): the tail
/// is disposition-aware, so this is NOT `as_warning(MSG_N1A)` — the body up
/// to the forbidden-example quote is identical, only the remedy differs.
const MSG_N1A_WARN: &str = "md: warning: @0 appears at 2 use sites in this template with the \
same path expression, so ONE key would fill every one of them. That is forbidden by BIP 388's \
disjointness rule (\"if two KEY are KP/<M;N>/* and KP/<P;Q>/* for the same key placeholder KP, \
then the sets {M, N} and {P, Q} must be disjoint\"), whose forbidden-example list names \
sh(multi(1,@0/**,@0/**)) — \"Repeated keys with the same path expression\". This shape can no \
longer be minted or composed; the card remains readable.";

#[test]
fn r_n1a_refuses_at_encode() {
    assert_refused(&encode(T_N1A, &[&format!("@0={K0}")]), MSG_N1A);
}

#[test]
fn r_n1a_refuses_at_descriptor_template() {
    assert_refused(&descriptor(T_N1A, &[&format!("@0={K0}")]), MSG_N1A);
}

#[test]
fn r_n1a_refuses_at_address_template() {
    assert_refused(&address(T_N1A, &[&format!("@0={K0}")]), MSG_N1A);
}

/// Family 1 needs no key bindings, so the KEYLESS spelling refuses too — the
/// shape is a property of the template, not of what is seated in it.
#[test]
fn r_n1a_refuses_the_keyless_spelling_too() {
    assert_refused(&encode(T_N1A, &[]), MSG_N1A);
}

// ─── R-N1b — multipath sets differ and OVERLAP ─────────────────────────────

const MSG_N1B: &str = "md: unsupported: @0 appears at use sites whose multipath sets OVERLAP — \
<0;1> and <1;2> share 1. BIP 388 requires two key expressions on one placeholder to have DISJOINT \
multipath sets, so this shape is forbidden; md1 could not carry two use sites for one key slot in \
any case (one path per key slot, F-417), but the disjointness rule is the primary ground. md \
declines to mint or compose this shape: give each use site its own placeholder.";

#[test]
fn r_n1b_overlapping_multipath_sets_refuse() {
    assert_refused(
        &encode(
            "wsh(multi(2,@0/<0;1>/*,@0/<1;2>/*))",
            &[&format!("@0={K0}")],
        ),
        MSG_N1B,
    );
}

// ─── R-N1c — multipath sets differ and are DISJOINT ────────────────────────
//
// The one row where the refusal is NOT about a defect in the wallet: BIP 388
// permits this shape outright. The message mandate is therefore the whole
// point of the row — the wallet is legal, md1 deliberately cannot express it
// (F-417), and the operator is handed a RUNNABLE escape rather than a dead
// end. It must not render through `CliError::TemplateParse`, whose prefix
// ("template parse error:") would blame an input that is not at fault.

const MSG_N1C: &str = "md: unsupported: @0 appears at use sites with DISJOINT multipath sets — \
<0;1> and <2;3>. The WALLET is legal under BIP 388, which permits exactly this on one \
placeholder; md1 deliberately cannot express it, because an md1 card carries ONE path per key \
slot and that narrowness is a design decision the wire format will not widen (F-417). md declines \
to mint or compose this shape. Keep this wallet as a descriptor instead: me sysw pack --as \
descriptor --in <your export file>";

#[test]
fn r_n1c_disjoint_multipath_sets_refuse_with_the_honest_message() {
    let out = encode(
        "wsh(multi(2,@0/<0;1>/*,@0/<2;3>/*))",
        &[&format!("@0={K0}")],
    );
    assert_refused(&out, MSG_N1C);
    let line = rendered_line(&out);
    // The three content statements and the escape, named individually so a
    // future edit that drops one says WHICH one.
    assert!(!line.contains("template parse error"), "{line}");
    assert!(line.contains("legal under BIP 388"), "{line}");
    assert!(line.contains("F-417"), "{line}");
    assert!(
        line.contains("me sysw pack --as descriptor --in <your export file>"),
        "{line}"
    );
}

// ─── R-N1-origin — the inline ORIGIN axis ──────────────────────────────────
//
// md's own representability limit. The message must cite NO BIP rule at all:
// inline origins are md's normal template spelling (`--emit template` prints
// them, `md encode` mints them), not an error class, so a BIP citation here
// would be a false record about a normative document.

const MSG_N1_ORIGIN: &str = "md: unsupported: @0 appears at use sites declaring DIFFERENT key \
origins — /48'/0'/0'/2' and /48'/0'/1'/2'. One placeholder is one key slot and an md1 card records \
ONE origin per key slot, so it cannot carry both. Inline origins are md's own normal template \
spelling, so this is md's representability limit and not a statement about your wallet. md \
declines to mint or compose this shape: give each origin its own placeholder.";

#[test]
fn r_n1_origin_refuses_naming_the_origin_axis_and_cites_no_bip() {
    let out = encode(
        "wsh(multi(2,@0/48'/0'/0'/2'/<0;1>/*,@0/48'/0'/1'/2'/<0;1>/*))",
        &[&format!("@0={K0}")],
    );
    assert_refused(&out, MSG_N1_ORIGIN);
    let line = rendered_line(&out);
    assert!(
        !line.contains("BIP") && !line.contains("bip"),
        "R-N1-origin must cite no BIP rule at all: {line}"
    );
    assert!(
        !line.contains("pairwise distinct"),
        "and in particular not the repeated-key rule: {line}"
    );
}

// ─── R-N1-hardening — the wildcard HARDENING axis ──────────────────────────
//
// REACHABILITY, MEASURED at the plan's baseline binary (b8a64938 surface):
// the single-site hardened wildcard is NOT refused — `md descriptor
// --template "wpkh(@0/<0;1>/*')" --key @0=<KEY 1> --path 48'/0'/0'/2'`
// composed at exit 0 and printed `.../<0;1>/*h#hs2ar46p`, and `md encode` on
// the same template exited 0. The two-occurrence differing-hardening form
// lexed cleanly and reached `resolve_placeholders`, which refused it with the
// generic "@0 appears with inconsistent path/multipath/hardening". So the
// case IS reachable past the single-site surface, and per the plan its row
// lands rather than being recorded as unreachable.

const MSG_N1_HARDENING: &str = "md: unsupported: @0 appears at use sites whose wildcards differ in \
HARDENING — <0;1>/* and <0;1>/*'. One placeholder is one key slot and an md1 card records ONE \
use-site wildcard per slot, so it cannot carry both; an xpub derives no hardened child either, so \
one key could not serve both use sites even if the card could carry them. This is md's own \
derivability limit, not a statement about your wallet. md declines to mint or compose this shape: \
give each use site its own placeholder.";

#[test]
fn r_n1_hardening_refuses_naming_the_hardening_axis_and_cites_no_bip() {
    let out = encode(
        "wsh(multi(2,@0/<0;1>/*,@0/<0;1>/*'))",
        &[&format!("@0={K0}")],
    );
    assert_refused(&out, MSG_N1_HARDENING);
    let line = rendered_line(&out);
    assert!(
        !line.contains("BIP") && !line.contains("bip"),
        "R-N1-hardening must cite no BIP rule: {line}"
    );
}

/// The reachability probe itself, kept as a row so the determination above
/// cannot rot silently: if the single-site hardened wildcard ever starts
/// refusing, THIS row goes red and the R-N1-hardening row's ground is what
/// needs re-deciding — not the other way round.
#[test]
fn r_n1_hardening_reachability_probe_single_site_still_composes() {
    let out = descriptor("wpkh(@0/<0;1>/*')", &[&format!("@0={K0}")]);
    assert!(
        out.status.success(),
        "the single-site hardened wildcard now refuses, which would make \
         R-N1-hardening unreachable: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// ─── R-N1d — ONE KEY at TWO placeholders, DISJOINT use sites ───────────────
//
// The delta over the shipped codec floor, which refuses the SAME-use-site
// case (F-218) and keeps its own wording. Here the wallet is legal and the
// SPELLING is not, so the message may not reuse the floor's sentences: "at
// the same use-site" is false for the delta, and "a card minted from it could
// never be read back" is false AND contradicts Acceptance 5 — a delta card
// exists (tests/fixtures/n1/r-n1d-delta.txt, chunk-set-id 0x00ee4) and the
// reading verbs must keep reading it.

const T_N1D: &str = "wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))";

const MSG_N1D: &str = "md: unsupported: @0 and @1 were given the SAME extended public key at \
DIFFERENT use sites — <0;1>/* and <2;3>/*. Spelled with two placeholders, this policy lists that \
key TWICE in BIP 388's key information vector, and BIP 388's pairwise-distinctness rule requires \
\"the public keys obtained by \
deserializing elements of the key information vector must be pairwise distinct\" — so what BIP 388 \
forbids is THIS SPELLING's key vector, not the wallet it describes. The wallet — one key at two \
disjoint path sets — is a legal descriptor, and BIP 388 writes it with ONE placeholder carrying \
both sets; md1 cannot write that spelling either, because an md1 card carries one path per key \
slot (F-417). md declines to mint or compose this shape. Keep this wallet as a descriptor: me \
sysw pack --as descriptor --in <your export file>";

#[test]
fn r_n1d_delta_refuses_at_encode() {
    assert_refused(
        &encode(T_N1D, &[&format!("@0={K0}"), &format!("@1={K0}")]),
        MSG_N1D,
    );
}

#[test]
fn r_n1d_delta_refuses_at_descriptor_template() {
    assert_refused(
        &descriptor(T_N1D, &[&format!("@0={K0}"), &format!("@1={K0}")]),
        MSG_N1D,
    );
}

#[test]
fn r_n1d_delta_refuses_at_address_template() {
    assert_refused(
        &address(T_N1D, &[&format!("@0={K0}"), &format!("@1={K0}")]),
        MSG_N1D,
    );
}

/// The message mandate, clause by clause — each named so a drift says which
/// obligation it broke.
#[test]
fn r_n1d_message_meets_its_mandate() {
    let out = encode(T_N1D, &[&format!("@0={K0}"), &format!("@1={K0}")]);
    let line = rendered_line(&out);
    assert!(
        line.contains("THIS SPELLING's key vector, not the wallet"),
        "must attribute the pairwise-distinct violation to the SPELLING: {line}"
    );
    assert!(
        line.contains("is a legal descriptor"),
        "must state the wallet is expressible as a descriptor: {line}"
    );
    assert!(
        line.contains("me sysw pack --as descriptor --in <your export file>"),
        "must name the same runnable escape as R-N1c: {line}"
    );
    assert!(
        !line.contains("at the same use-site"),
        "must not reuse the shipped same-use-site wording — it is FALSE here: {line}"
    );
    assert!(
        !line.contains("could never be read back"),
        "must not reuse the shipped read-back claim — it is false here and \
         contradicts Acceptance 5: {line}"
    );
    assert!(!line.to_lowercase().contains("invalid"), "{line}");
}

/// The shipped SAME-use-site refusal is untouched: it keeps its own variant,
/// its own wording, and its position at the codec floor. A classifier that
/// swallowed this case would be a second implementation of a shipped
/// predicate, which the single-source rule forbids.
#[test]
fn the_shipped_same_use_site_refusal_still_owns_its_own_case() {
    let out = encode(
        "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))",
        &[&format!("@0={K0}"), &format!("@1={K0}")],
    );
    let line = rendered_line(&out);
    assert!(
        line.starts_with("md: key reuse refused: ")
            || line.starts_with("md: codec error: ")
            || line.contains("at the same use-site"),
        "the same-use-site case must keep the SHIPPED refusal, not the N1 \
         taxonomy's: {line}"
    );
    assert!(!line.starts_with("md: unsupported: "), "{line}");
}

// ─── the anti-over-refusal controls ────────────────────────────────────────

/// THE OPERATOR-PROBE ROW (plan P2 step 4). One master, two DIFFERENT account
/// paths, therefore two DIFFERENT derived xpubs: no reuse, and the post-walk
/// ruling ("No carve out for reused keys unless different origin paths",
/// operator 2026-08-31) keeps it a legitimate wallet. It MUST compose.
///
/// This is what catches a fingerprint-keyed misimplementation: both slots
/// declare fingerprint 73c5da0a, so a check comparing masters instead of key
/// material refuses here and this row is the only thing that says so.
#[test]
fn control_same_fingerprint_different_accounts_still_composes() {
    let out = md()
        .args([
            "descriptor",
            "--template",
            "wsh(multi(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/1'/2'/<0;1>/*))",
            "--key",
            &format!("@0={K0}"),
            "--key",
            &format!("@1={K1}"),
            "--fingerprint",
            "@0=73c5da0a",
            "--fingerprint",
            "@1=73c5da0a",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "one master at two accounts was refused: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("wsh(multi(2,"));
}

/// Two placeholders, two DIFFERENT keys, disjoint use sites — nothing to
/// refuse on either axis.
#[test]
fn control_distinct_keys_at_disjoint_use_sites_still_compose() {
    let out = encode(T_N1D, &[&format!("@0={K0}"), &format!("@1={K1}")]);
    assert!(
        out.status.success(),
        "an ordinary two-key policy was refused: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// One placeholder used ONCE is not a use-site repeat, however many
/// occurrences the rest of the template has.
#[test]
fn control_a_single_use_site_per_placeholder_still_composes() {
    let out = encode(
        "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))",
        &[&format!("@0={K0}"), &format!("@1={K1}")],
    );
    assert!(
        out.status.success(),
        "a plain 2-of-2 was refused: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// ─── the READ side is not the mint/compose side ────────────────────────────

/// `md verify --template` carries the WARN disposition (SPEC N1 "Verb
/// dispositions"), so an operator can still check a legacy plate that carries
/// a refused shape. The classifier is ONE implementation; only the
/// disposition differs, which is why the warning body is the refusal body.
#[test]
fn verify_template_warns_and_completes_on_a_refused_shape() {
    let fixture = include_str!("fixtures/n1/r-n1a-keyed.txt");
    let chunks: Vec<&str> = fixture.lines().filter(|l| l.starts_with("md1")).collect();
    assert_eq!(chunks.len(), 2, "the R-N1a fixture is a 2-chunk card");

    let mut c = md();
    c.arg("verify");
    for ch in &chunks {
        c.arg(ch);
    }
    c.args([
        "--template",
        T_N1A,
        "--key",
        &format!("@0={K0}"),
        "--path",
        PATH,
    ]);
    let out = c.output().unwrap();
    assert!(
        out.status.success(),
        "verify refused a legacy plate instead of warning: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("OK"));
    let line = rendered_line(&out);
    // NOT `as_warning(MSG_N1A)` / a strip-prefix of MSG_N1A — R-N1a's tail is
    // disposition-aware (M4), so the WARN rendering is `MSG_N1A_WARN`
    // verbatim, not MSG_N1A with the prefix swapped.
    assert_eq!(line, MSG_N1A_WARN);
}

// ─── `md compile` cannot open a mint path for a refused shape ──────────────
//
// Plan P2 step 8. The refusal is rust-miniscript's own ("Policy contains
// duplicate keys"), pinned locally by nothing, so an upstream bump could
// silently start EMITTING a Family-1 template from a duplicate-key policy.
// These rows are what would notice.

#[cfg(feature = "cli-compiler")]
#[test]
fn md_compile_refuses_a_duplicate_key_policy() {
    for (expr, ctx) in [
        ("thresh(2,pk(@0),pk(@0),pk(@1))", "segwitv0"),
        ("and(pk(@0),pk(@0))", "segwitv0"),
        ("or(pk(@0),pk(@0))", "segwitv0"),
        ("thresh(2,pk(@0),pk(@0),pk(@1))", "tap"),
    ] {
        let out = md()
            .args(["compile", expr, "--context", ctx])
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(1),
            "`{expr}` (--context {ctx}) did not refuse; stdout:\n{}",
            String::from_utf8_lossy(&out.stdout)
        );
        let line = rendered_line(&out);
        assert_eq!(
            line, "md: compile error: compile: Policy contains duplicate keys",
            "the upstream duplicate-key refusal changed shape for `{expr}`"
        );
    }
}

/// Non-vacuous: a distinct-key policy of the same shape still compiles, so
/// the rows above are pinning the duplicate-key rule and not a broken verb.
#[cfg(feature = "cli-compiler")]
#[test]
fn md_compile_still_accepts_a_distinct_key_policy() {
    let out = md()
        .args([
            "compile",
            "thresh(2,pk(@0),pk(@1),pk(@2))",
            "--context",
            "segwitv0",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "the distinct-key control failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// ═══ P3 — the CARD input and the READ side ═════════════════════════════════
//
// Everything above enters through `--template`. Everything below enters
// through an md1 CARD, which is the other half of the spec's verb
// dispositions and the whole of Acceptance 5.
//
// The two fixture cards are FROZEN (`tests/fixtures/n1/`): they were minted
// from the b8a64938 baseline binary while a shipped `md encode` still would,
// and from P2 onward none can. They are the already-engraved plates the C1
// placement constraint exists for, so they are also the only possible input
// to these rows.
//
// THE ROWS QUOTE THE SAME TWO CONSTANTS AS THE TEMPLATE ROWS, deliberately.
// "each predicate has ONE implementation; the per-verb disposition is a
// parameter" is a claim about the code that only a shared constant can
// falsify: if the card path ever grew its own copy of the predicate, its
// message would drift and these `assert_eq!`s are what would say so.

const CARD_N1A: &str = include_str!("fixtures/n1/r-n1a-keyed.txt");
const CARD_N1D: &str = include_str!("fixtures/n1/r-n1d-delta.txt");

/// The md1 chunks of a fixture file; `#` lines are provenance.
fn chunks(fixture: &str) -> Vec<&str> {
    fixture
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("md1"))
        .collect()
}

/// `md <verb> <every md1 chunk of the fixture> [extra…]`.
fn card_cmd(verb: &str, fixture: &str, extra: &[&str]) -> std::process::Output {
    let mut c = md();
    c.arg(verb);
    for ch in chunks(fixture) {
        c.arg(ch);
    }
    c.args(extra);
    c.output().unwrap()
}

/// The WARN rendering of a refusal's body — the same text under the other
/// disposition, which is the observable form of "one implementation".
fn as_warning(refusal: &str) -> String {
    format!(
        "md: warning: {}",
        refusal.strip_prefix("md: unsupported: ").unwrap()
    )
}

/// A reading verb must COMPLETE at exit 0 (Acceptance 5), still produce its
/// output, and say what is wrong with the card on stderr.
fn assert_read_warns(out: &std::process::Output, expected_line: &str) {
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(0),
        "a reading verb refused an already-engraved plate; stderr was:\n{stderr}"
    );
    assert!(
        !String::from_utf8_lossy(&out.stdout).trim().is_empty(),
        "the verb warned but produced no output, so the plate is still unreadable"
    );
    let line = rendered_line(out);
    assert_eq!(line, expected_line);
    assert!(
        line.contains("BIP 388"),
        "the read-side warning must name the BIP-388 violation: {line}"
    );
    assert!(
        !line.to_lowercase().contains("invalid"),
        "the operator's wallet must never be called invalid: {line}"
    );
}

// ─── P3.1 — the card-input COMPOSING refusals ──────────────────────────────
//
// `descriptor` and `address` are on the REFUSE (mint/compose) surface for
// BOTH inputs (SPEC N1 "Verb dispositions"). Measured before this phase:
// every one of these four read at exit 0 and printed a wallet, because the
// card branch of `cmd/build.rs` ran no reuse check at all.

#[test]
fn r_n1a_card_refuses_at_descriptor() {
    assert_refused(&card_cmd("descriptor", CARD_N1A, &[]), MSG_N1A);
}

#[test]
fn r_n1a_card_refuses_at_address() {
    assert_refused(&card_cmd("address", CARD_N1A, &["--count", "1"]), MSG_N1A);
}

#[test]
fn r_n1d_card_refuses_at_descriptor() {
    assert_refused(&card_cmd("descriptor", CARD_N1D, &[]), MSG_N1D);
}

#[test]
fn r_n1d_card_refuses_at_address() {
    assert_refused(&card_cmd("address", CARD_N1D, &["--count", "1"]), MSG_N1D);
}

// ─── P3.2 — the READ side keeps reading ────────────────────────────────────
//
// Acceptance 5, row-pinned on both plates: `decode`, `inspect`, `bytecode`
// and `verify` complete at exit 0 on a card carrying a shape this cycle
// newly refuses, WITH a warning naming the BIP-388 violation. This is what
// the C1 placement constraint buys — a check inside `encode_payload` would
// have made these same plates uninspectable and unverifiable.

#[test]
fn r_n1a_card_decodes_at_exit_0_with_a_warning() {
    assert_read_warns(&card_cmd("decode", CARD_N1A, &[]), MSG_N1A_WARN);
}

#[test]
fn r_n1a_card_inspects_at_exit_0_with_a_warning() {
    assert_read_warns(&card_cmd("inspect", CARD_N1A, &[]), MSG_N1A_WARN);
}

#[test]
fn r_n1a_card_bytecodes_at_exit_0_with_a_warning() {
    assert_read_warns(&card_cmd("bytecode", CARD_N1A, &[]), MSG_N1A_WARN);
}

#[test]
fn r_n1d_card_decodes_at_exit_0_with_a_warning() {
    assert_read_warns(&card_cmd("decode", CARD_N1D, &[]), &as_warning(MSG_N1D));
}

#[test]
fn r_n1d_card_inspects_at_exit_0_with_a_warning() {
    assert_read_warns(&card_cmd("inspect", CARD_N1D, &[]), &as_warning(MSG_N1D));
}

#[test]
fn r_n1d_card_bytecodes_at_exit_0_with_a_warning() {
    assert_read_warns(&card_cmd("bytecode", CARD_N1D, &[]), &as_warning(MSG_N1D));
}

/// `md verify` needs a `--template` (clap `required = true`), so its CARD
/// row is the card checked against its own spelling.
///
/// NO SECOND CHECK IS WIRED ON `verify`'s DECODED SIDE, and that is a
/// decision rather than an omission: `verify` returns MISMATCH unless the
/// decoded card and the parsed template encode to the identical payload, so
/// the only card that can reach exit 0 here is one whose shape the template
/// also carries — and the template side already warns (P2). A second
/// invocation would print the identical line twice on every such run.
/// These rows are what would notice if that reasoning ever stopped holding:
/// they assert the warning on the CARD's own journey, not on the mechanism.
fn verify_card(fixture: &str, template: &str, keys: &[&str]) -> std::process::Output {
    let mut c = md();
    c.arg("verify");
    for ch in chunks(fixture) {
        c.arg(ch);
    }
    c.args(["--template", template, "--path", PATH]);
    for k in keys {
        c.args(["--key", k]);
    }
    c.output().unwrap()
}

#[test]
fn r_n1a_card_verifies_at_exit_0_with_a_warning() {
    let out = verify_card(CARD_N1A, T_N1A, &[&format!("@0={K0}")]);
    assert_read_warns(&out, MSG_N1A_WARN);
    assert!(String::from_utf8_lossy(&out.stdout).contains("OK"));
}

#[test]
fn r_n1d_card_verifies_at_exit_0_with_a_warning() {
    let out = verify_card(CARD_N1D, T_N1D, &[&format!("@0={K0}"), &format!("@1={K0}")]);
    assert_read_warns(&out, &as_warning(MSG_N1D));
    assert!(String::from_utf8_lossy(&out.stdout).contains("OK"));
}

// ─── P3.3 — `Body::Tr`'s internal-key arm (whole-diff review r1 N5) ────────
//
// Every row above puts its repeat in `Body::MultiKeys`. `count_occurrences`
// (`src/parse/reuse.rs`) also has a `Body::Tr` arm — the tap internal key is
// a BARE index, not a child `Node`, so a walker that only recursed through
// `Body::Children` would miss it — and until this fixture, no CARD ever
// exercised it: `md_codec::tree::Body::Tr`'s doc-comment traces back to
// v0.30 Phase C/F, well before this cycle, but the R-N1a/R-N1d predicate
// that walks it card-side is this cycle's own.
//
// FROZEN for the identical reason as `r-n1a-keyed.txt` (same header, this
// file's own header explains the mint command and baseline commit): the
// current binary refuses `tr(@0/<0;1>/*,pk(@0/<0;1>/*))` at `md encode`
// (measured, exit 1, "unsupported: @0 appears at 2 use sites..."), so this
// card could only be minted from the pre-R-N1a baseline. `@0` is the Tr
// internal key AND a leaf `pk(@0/<0;1>/*)`, IDENTICAL triples — the same
// R-N1a shape as `CARD_N1A`, spelled through the other arm. The rendered
// message is BYTE-IDENTICAL to `MSG_N1A`/`MSG_N1A_WARN`: `Finding::message`
// never quotes the surrounding template shape, only `@{i}` and `{sites}`,
// so reusing the same constants here is itself a check that one classifier
// serves both arms rather than a second, drifted copy.

const CARD_N1A_TR: &str = include_str!("fixtures/n1/r-n1a-tr-internal-key.txt");

#[test]
fn r_n1a_tr_internal_key_card_refuses_at_descriptor() {
    assert_refused(&card_cmd("descriptor", CARD_N1A_TR, &[]), MSG_N1A);
}

#[test]
fn r_n1a_tr_internal_key_card_refuses_at_address() {
    assert_refused(
        &card_cmd("address", CARD_N1A_TR, &["--count", "1"]),
        MSG_N1A,
    );
}

#[test]
fn r_n1a_tr_internal_key_card_decodes_at_exit_0_with_a_warning() {
    assert_read_warns(&card_cmd("decode", CARD_N1A_TR, &[]), MSG_N1A_WARN);
}

// ─── the anti-over-refusal duty, on the CARD path ──────────────────────────
//
// A card-path check that fired on every card would pass every row above and
// break every wallet. The control is minted here rather than taken from a
// fixture so it is unarguably a card a shipped binary still writes.

/// A clean 2-of-2 from two DIFFERENT keys, minted by this same binary.
fn clean_card() -> Vec<String> {
    let out = encode(
        "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))",
        &[&format!("@0={K0}"), &format!("@1={K1}")],
    );
    assert!(
        out.status.success(),
        "the control card could not be minted: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("md1"))
        .map(str::to_owned)
        .collect()
}

fn run_on(verb: &str, card: &[String], extra: &[&str]) -> std::process::Output {
    let mut c = md();
    c.arg(verb);
    for ch in card {
        c.arg(ch);
    }
    c.args(extra);
    c.output().unwrap()
}

#[test]
fn control_a_clean_card_still_composes_and_reads_without_a_diagnostic() {
    let card = clean_card();
    assert!(!card.is_empty(), "the control card has no chunks");
    for (verb, extra) in [
        ("descriptor", &[][..]),
        ("address", &["--count", "1"][..]),
        ("decode", &[][..]),
        ("inspect", &[][..]),
        ("bytecode", &[][..]),
    ] {
        let out = run_on(verb, &card, extra);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(
            out.status.code(),
            Some(0),
            "`md {verb}` refused a clean card: {stderr}"
        );
        assert!(
            !stderr.lines().any(|l| l.starts_with("md: ")),
            "`md {verb}` emitted a diagnostic on a clean card: {stderr}"
        );
    }
}
