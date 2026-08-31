#![allow(missing_docs)]
//! SPEC P1 -- per-slot origins on the read side
//! (`design/SPEC_wallet_form_converter.md` "The three pieces"). C1 wires the
//! origin-notated `--key '@i=[fp/path]xpub'` form (BIP-380 origin notation,
//! mk's own file format; C0 landed the parser, `parse_key_with_origin`,
//! unwired) into `md descriptor`/`md address`, plus the per-datum precedence
//! that makes it useful ("the sources never overlap on the same datum, so
//! precedence is per-DATUM, not per-source"):
//!
//! - PATHS: inline template origin where present, else shared `--path`, else
//!   today's non-canonical-wrapper refusal stands.
//! - FINGERPRINTS: `--fingerprint @i=` or the origin-notated `--key`
//!   bracket; when both name a slot they must AGREE or refuse (never a
//!   silent override).
//! - An origin-notated `--key`'s bracket PATH must AGREE with the slot's
//!   inline template path when both exist -- agreement is not an override.
//!
//! Rows: V-KEYORIG, V-FPAGREE, V-PATHAGREE, V-PRECEDENCE. (V-HSPELL lives in
//! `parse::template`'s own test module, `v_hspell_*` -- it is a
//! template-lexer fix, not a `--key`-flag one, so its RED/GREEN cycle ran
//! against the lexer directly.)
//!
//! `md descriptor` renders a key's origin bracket `[fingerprint/path]xpub`
//! ONLY when a fingerprint is present (`assemble_origin_and_xkey` in
//! `md-codec/src/to_miniscript.rs` gates the whole bracket -- including the
//! PATH -- on `fingerprint.is_some()`), so every test that needs to observe
//! which path won supplies a fingerprint for that slot.

use assert_cmd::Command;
use bitcoin::Network;
use bitcoin::bip32::{DerivationPath, Xpriv, Xpub};
use bitcoin::secp256k1::Secp256k1;
use std::str::FromStr;

const ABANDON: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon \
                        abandon abandon about";

/// A real, on-curve xpub at an exact derivation path -- same construction as
/// `cli_path_override_reaches_noncanonical.rs`'s `xpub_at`.
fn xpub_at(path: &str) -> Xpub {
    let mn = bip39::Mnemonic::parse(ABANDON).unwrap();
    let seed = mn.to_seed("");
    let secp = Secp256k1::new();
    let master = Xpriv::new_master(Network::Bitcoin, &seed).unwrap();
    let dp = DerivationPath::from_str(path).unwrap();
    Xpub::from_priv(&secp, &master.derive_priv(&secp, &dp).unwrap())
}

fn md() -> Command {
    Command::cargo_bin("md").unwrap()
}

fn stdout_of(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn stderr_of(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

// ─── V-KEYORIG ──────────────────────────────────────────────────────────
//
// A real multi-slot template with per-slot inline origin paths -- the
// spec's motivation-case shape (SPEC "Motivation, measured": the
// pathological vault's slots declare different accounts). Origin-notated
// `--key`, with NO `--fingerprint` flags at all, composes to the SAME
// concrete descriptor as the already-proven bare-key + `--fingerprint`
// route -- an independent cross-check rather than a hand-written expected
// string.

#[test]
fn v_keyorig_template_and_origin_notated_keys_compose_end_to_end() {
    let x0 = xpub_at("48'/0'/0'/2'");
    let x1 = xpub_at("48'/0'/1'/2'");
    let template = "wsh(multi(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/1'/2'/<0;1>/*))";

    let via_origin = md()
        .args([
            "descriptor",
            "--template",
            template,
            "--key",
            &format!("@0=[deadbeef/48'/0'/0'/2']{x0}"),
            "--key",
            &format!("@1=[cafebabe/48'/0'/1'/2']{x1}"),
        ])
        .output()
        .unwrap();
    assert!(
        via_origin.status.success(),
        "stderr: {}",
        stderr_of(&via_origin)
    );
    let via_origin_out = stdout_of(&via_origin);

    let via_bare = md()
        .args([
            "descriptor",
            "--template",
            template,
            "--key",
            &format!("@0={x0}"),
            "--key",
            &format!("@1={x1}"),
            "--fingerprint",
            "@0=deadbeef",
            "--fingerprint",
            "@1=cafebabe",
        ])
        .output()
        .unwrap();
    assert!(
        via_bare.status.success(),
        "stderr: {}",
        stderr_of(&via_bare)
    );
    let via_bare_out = stdout_of(&via_bare);

    assert_eq!(
        via_origin_out, via_bare_out,
        "origin-notated --key must compose the SAME concrete descriptor as \
         bare --key + --fingerprint"
    );
    assert!(
        via_origin_out.starts_with("wsh("),
        "not a concrete descriptor: {via_origin_out}"
    );
    assert!(
        via_origin_out.contains("[deadbeef/48'/0'/0'/2']")
            && via_origin_out.contains("[cafebabe/48'/0'/1'/2']"),
        "composed descriptor must carry both origins: {via_origin_out}"
    );
}

/// The brief's task 1 names BOTH `md descriptor` and `md address` — they
/// share `build_descriptor` (`cmd/build.rs`) so the wiring is structurally
/// one change, but that is a claim about the implementation, not a
/// substitute for exercising the second command directly.
#[test]
fn v_keyorig_address_also_accepts_the_origin_notated_key() {
    let x0 = xpub_at("48'/0'/0'/2'");
    let out = md()
        .args([
            "address",
            "--template",
            SINGLE_SLOT_TEMPLATE,
            "--key",
            &format!("@0=[deadbeef/48'/0'/0'/2']{x0}"),
            "--count",
            "1",
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    let stdout = stdout_of(&out);
    assert!(
        stdout.starts_with("bc1"),
        "expected a derived address: {stdout}"
    );
}

// ─── V-FPAGREE ──────────────────────────────────────────────────────────

const SINGLE_SLOT_TEMPLATE: &str = "wpkh(@0/48'/0'/0'/2'/<0;1>/*)";

#[test]
fn v_fpagree_matching_fingerprints_succeed_identically_to_either_alone() {
    let x0 = xpub_at("48'/0'/0'/2'");

    let via_flag_only = md()
        .args([
            "descriptor",
            "--template",
            SINGLE_SLOT_TEMPLATE,
            "--key",
            &format!("@0={x0}"),
            "--fingerprint",
            "@0=cafebabe",
        ])
        .output()
        .unwrap();
    assert!(
        via_flag_only.status.success(),
        "stderr: {}",
        stderr_of(&via_flag_only)
    );

    let via_key_only = md()
        .args([
            "descriptor",
            "--template",
            SINGLE_SLOT_TEMPLATE,
            "--key",
            &format!("@0=[cafebabe]{x0}"),
        ])
        .output()
        .unwrap();
    assert!(
        via_key_only.status.success(),
        "stderr: {}",
        stderr_of(&via_key_only)
    );

    let via_both_agreeing = md()
        .args([
            "descriptor",
            "--template",
            SINGLE_SLOT_TEMPLATE,
            "--key",
            &format!("@0=[cafebabe]{x0}"),
            "--fingerprint",
            "@0=cafebabe",
        ])
        .output()
        .unwrap();
    assert!(
        via_both_agreeing.status.success(),
        "stderr: {}",
        stderr_of(&via_both_agreeing)
    );

    let a = stdout_of(&via_flag_only);
    let b = stdout_of(&via_key_only);
    let c = stdout_of(&via_both_agreeing);
    assert_eq!(
        a, b,
        "fingerprint-via-flag and fingerprint-via-key-bracket must produce \
         the same descriptor"
    );
    assert_eq!(a, c, "agreeing both must equal either alone");
    assert!(
        a.contains("[cafebabe/"),
        "expected the fingerprint in the origin: {a}"
    );
}

#[test]
fn v_fpagree_disagreeing_fingerprints_refuse_naming_the_slot() {
    let x0 = xpub_at("48'/0'/0'/2'");
    let out = md()
        .args([
            "descriptor",
            "--template",
            SINGLE_SLOT_TEMPLATE,
            "--key",
            &format!("@0=[cafebabe]{x0}"),
            "--fingerprint",
            "@0=deadbeef",
        ])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "disagreeing fingerprints were accepted: {}",
        stdout_of(&out)
    );
    let err = stderr_of(&out);
    for needle in ["@0", "cafebabe", "deadbeef"] {
        assert!(
            err.contains(needle),
            "the refusal does not name {needle}, so an operator cannot locate \
             the clash: {err}"
        );
    }
}

// ─── V-PATHAGREE ────────────────────────────────────────────────────────

#[test]
fn v_pathagree_disagreeing_key_path_refuses_naming_both_paths() {
    // Any xpub works here -- the disagreement is caught before depth or
    // secp validity would matter to the OUTCOME, but a real depth-4 xpub
    // keeps the fixture realistic and avoids masking the path check behind
    // an unrelated depth refusal.
    let x0 = xpub_at("48'/0'/0'/3'");
    let out = md()
        .args([
            "descriptor",
            "--template",
            SINGLE_SLOT_TEMPLATE, // inline path 48'/0'/0'/2'
            "--key",
            &format!("@0=[deadbeef/48'/0'/0'/3']{x0}"),
        ])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "disagreeing origin-notated --key path was accepted: {}",
        stdout_of(&out)
    );
    let err = stderr_of(&out);
    for needle in ["@0", "48'/0'/0'/2'", "48'/0'/0'/3'"] {
        assert!(
            err.contains(needle),
            "the refusal does not name {needle} (slot + BOTH paths): {err}"
        );
    }
}

#[test]
fn v_pathagree_agreeing_key_path_succeeds() {
    let x0 = xpub_at("48'/0'/0'/2'");
    let out = md()
        .args([
            "descriptor",
            "--template",
            SINGLE_SLOT_TEMPLATE, // inline path 48'/0'/0'/2'
            "--key",
            &format!("@0=[deadbeef/48'/0'/0'/2']{x0}"),
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    // Agreement is not an override: the descriptor still carries the ONE
    // agreed path, not two.
    let stdout = stdout_of(&out);
    assert!(stdout.contains("[deadbeef/48'/0'/0'/2']"), "got: {stdout}");
}

// ─── V-PRECEDENCE ───────────────────────────────────────────────────────
//
// Both tests supply --fingerprint for every slot under test: md's origin
// bracket (and therefore the PATH) renders only when a fingerprint is
// present (see the module doc comment), so a path-precedence assertion
// needs one to be observable in the composed descriptor at all.

#[test]
fn v_precedence_inline_path_wins_over_conflicting_shared_path() {
    let x0 = xpub_at("48'/0'/0'/2'"); // matches the INLINE path
    let out = md()
        .args([
            "descriptor",
            "--template",
            SINGLE_SLOT_TEMPLATE, // inline: 48'/0'/0'/2'
            "--key",
            &format!("@0={x0}"),
            "--fingerprint",
            "@0=deadbeef",
            "--path",
            "84'/0'/0'", // conflicting SHARED path
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    let stdout = stdout_of(&out);
    assert!(
        stdout.contains("[deadbeef/48'/0'/0'/2']"),
        "the composed descriptor must keep the INLINE origin path, not the \
         shared --path: {stdout}"
    );
    assert!(
        !stdout.contains("84'/0'/0'"),
        "the shared --path must NOT have overridden the inline origin: {stdout}"
    );
}

#[test]
fn v_precedence_shared_path_fills_only_the_slot_without_an_inline_origin() {
    // @0 has NO inline origin; @1 does. This is what falsifies the OLD
    // behaviour (`apply_path_override` flattening the WHOLE descriptor to
    // one Shared path): a per-descriptor override would either wipe @1's
    // inline path or leave @0 unfillable.
    let template = "wsh(multi(2,@0/<0;1>/*,@1/48'/0'/1'/2'/<0;1>/*))";
    let x0 = xpub_at("84'/0'/0'"); // matches the SHARED --path
    let x1 = xpub_at("48'/0'/1'/2'"); // matches the INLINE path

    let out = md()
        .args([
            "descriptor",
            "--template",
            template,
            "--key",
            &format!("@0={x0}"),
            "--fingerprint",
            "@0=aaaaaaaa",
            "--key",
            &format!("@1={x1}"),
            "--fingerprint",
            "@1=bbbbbbbb",
            "--path",
            "84'/0'/0'",
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    let stdout = stdout_of(&out);
    assert!(
        stdout.contains("[aaaaaaaa/84'/0'/0']"),
        "slot @0 (no inline origin) must be filled by the shared --path: {stdout}"
    );
    assert!(
        stdout.contains("[bbbbbbbb/48'/0'/1'/2']"),
        "slot @1 (has an inline origin) must keep it, not the shared --path: {stdout}"
    );
}

// ─── V-PATHEFF (REVIEW-converter-whole-diff-r1 I1) ──────────────────────
//
// P1's agreement rule was written as bracket-vs-INLINE. The defect is
// bracket-vs-EFFECTIVE: the bracket path must agree with whatever source
// actually WINS for that slot, and when NOTHING wins the bracket path is
// silently discarded.
//
// Measured at 9d0c30dc, both manifestations:
//
//   A. TRUNCATION. `md decode` prints a template with no inline origins (the
//      origins go to a stderr note), and the operator's keys are in the
//      `[fp/path]xpub` form `md decompose --emit keys` and `mk encode --keys`
//      both use. Combining them emitted `[73c5da0a]` on a depth-0 xpub for
//      every slot -- a BIP-380 statement that the key IS master 73c5da0a,
//      exit 0, no warning. Ground truth for that wallet is
//      `[73c5da0a/48'/0'/0'/2']` etc. AT THE TIME, I1's fix made this refuse
//      rather than truncate -- "never emit an origin md already knows is
//      incomplete." N3 (ruled 2026-08-31, `design/SPEC_mdcli_mini.md` "N3 --
//      the --key bracket path becomes a last-resort source") SUPERSEDES that
//      disposition for this manifestation specifically: with nothing else
//      supplying a path for the slot, the bracket path itself now becomes
//      the source, so this composes with the FULL origin instead of either
//      truncating or refusing. The two tests below that exercised this exact
//      shape (single winning bracket, no inline, no --path) are updated in
//      place rather than duplicated -- see `v_n3_*` below.
//   B. SILENT OVERRIDE. With `--path` supplied, a bracket path never enters
//      `inline_declared`, so `--path` overwrote it: slot @1 declared
//      `[73c5da0a/48'/0'/0'/2']` for a key the operator had explicitly said
//      was at `48'/0'/1'/2'`. Worse than truncation -- it looks complete and
//      it is false. And it is a silent override on the PATH datum, which is
//      exactly what P1 forbids on the sibling FINGERPRINT datum. STILL A
//      REFUSAL under N3 -- precedence is inline > --path > bracket, so
//      wherever --path speaks for a slot the bracket is a cross-check only.
//
// Addresses are unaffected either way (both forms derive the same address);
// what breaks is SPENDING, because a signer handed the wrong origin derives
// the wrong child and never matches its key.

/// A template whose single slot declares NO inline origin, so `--path` is the
/// source that wins for it.
const PATHLESS_SINGLE_SLOT: &str = "wpkh(@0/<0;1>/*)";

#[test]
fn v_patheff_bracket_path_disagreeing_with_shared_path_refuses() {
    let x0 = xpub_at("48'/0'/0'/2'");
    let out = md()
        .args([
            "descriptor",
            "--template",
            PATHLESS_SINGLE_SLOT,
            "--key",
            &format!("@0=[deadbeef/48'/0'/0'/2']{x0}"),
            "--path",
            "84'/0'/0'", // the source that WINS, and it disagrees
        ])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "the shared --path silently overrode the --key bracket path: {}",
        stdout_of(&out)
    );
    let err = stderr_of(&out);
    for needle in ["@0", "48'/0'/0'/2'", "84'/0'/0'", "--path"] {
        assert!(
            err.contains(needle),
            "the refusal must name the slot, BOTH paths and which source wins; \
             missing {needle}: {err}"
        );
    }
}

#[test]
fn v_patheff_bracket_path_agreeing_with_shared_path_succeeds() {
    // The control that keeps the row above from being a blanket refusal:
    // agreement is not an override, exactly as for the inline pair.
    let x0 = xpub_at("48'/0'/0'/2'");
    let out = md()
        .args([
            "descriptor",
            "--template",
            PATHLESS_SINGLE_SLOT,
            "--key",
            &format!("@0=[deadbeef/48'/0'/0'/2']{x0}"),
            "--path",
            "48'/0'/0'/2'",
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    assert!(
        stdout_of(&out).contains("[deadbeef/48'/0'/0'/2']"),
        "got: {}",
        stdout_of(&out)
    );
}

/// N3 (ruled 2026-08-31): with no inline origin and no `--path`, the
/// bracket path becomes the slot's SOURCE rather than being discarded or
/// refused. WAS `v_patheff_bracket_path_with_no_winning_source_refuses_instead_of_truncating`
/// before N3 -- same shape (single winning bracket, no other source),
/// opposite disposition.
#[test]
fn v_n3_bracket_path_with_no_other_source_becomes_the_path_source() {
    let x0 = xpub_at("48'/0'/0'/2'");
    let out = md()
        .args([
            "descriptor",
            "--template",
            PATHLESS_SINGLE_SLOT, // no inline origin
            "--key",
            &format!("@0=[deadbeef/48'/0'/0'/2']{x0}"),
            // and no --path: the bracket is the only source left
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    let stdout = stdout_of(&out);
    assert!(
        stdout.contains("[deadbeef/48'/0'/0'/2']"),
        "the bracket's full origin (fingerprint AND path) must appear, not a \
         truncated [deadbeef]-only origin: {stdout}"
    );
}

/// THE JOURNEY, end to end -- N3's own motivating case
/// (`design/FOLLOWUPS.md` `descriptor-key-bracket-path-as-a-last-resort-source`):
/// `md decode` hands over a pathless template, the operator's key file is in
/// `[fp/path]xpub` form, and the three slots sit at three DIFFERENT accounts
/// -- so `--path`, which is shared, cannot express this wallet at all. Before
/// I1 this composed at exit 0 with three `[fp]`-only (TRUNCATED) origins;
/// after I1 and before N3 it refused outright (this test's previous name was
/// `v_patheff_the_divergent_origin_journey_refuses_rather_than_emitting_false_origins`).
/// N3 makes it compose again, but with the FULL origins this time -- and
/// PROVES it against an independent construction: pasting the same three
/// origins directly into the template (no brackets) must produce the
/// byte-identical descriptor (the plan's row-1 equality obligation).
#[test]
fn v_n3_divergent_origin_wallet_composes_and_equals_inline_pasted_origins() {
    let x0 = xpub_at("48'/0'/0'/2'");
    let x1 = xpub_at("48'/0'/1'/2'");
    let x2 = xpub_at("48'/0'/2'/2'");

    let via_bracket = md()
        .args([
            "descriptor",
            "--template",
            "wsh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))",
            "--key",
            &format!("@0=[73c5da0a/48'/0'/0'/2']{x0}"),
            "--key",
            &format!("@1=[73c5da0a/48'/0'/1'/2']{x1}"),
            "--key",
            &format!("@2=[73c5da0a/48'/0'/2'/2']{x2}"),
        ])
        .output()
        .unwrap();
    assert!(
        via_bracket.status.success(),
        "stderr: {}",
        stderr_of(&via_bracket)
    );
    let via_bracket_out = stdout_of(&via_bracket);
    assert!(
        !via_bracket_out.contains("[73c5da0a]"),
        "a truncated fingerprint-only origin shipped: {via_bracket_out}"
    );
    assert!(
        via_bracket_out.contains("[73c5da0a/48'/0'/0'/2']")
            && via_bracket_out.contains("[73c5da0a/48'/0'/1'/2']")
            && via_bracket_out.contains("[73c5da0a/48'/0'/2'/2']"),
        "all three full per-slot origins must appear: {via_bracket_out}"
    );

    let via_inline = md()
        .args([
            "descriptor",
            "--template",
            "wsh(sortedmulti(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/1'/2'/<0;1>/*,\
             @2/48'/0'/2'/2'/<0;1>/*))",
            "--key",
            &format!("@0={x0}"),
            "--key",
            &format!("@1={x1}"),
            "--key",
            &format!("@2={x2}"),
            "--fingerprint",
            "@0=73c5da0a",
            "--fingerprint",
            "@1=73c5da0a",
            "--fingerprint",
            "@2=73c5da0a",
        ])
        .output()
        .unwrap();
    assert!(
        via_inline.status.success(),
        "stderr: {}",
        stderr_of(&via_inline)
    );
    let via_inline_out = stdout_of(&via_inline);

    assert_eq!(
        via_bracket_out, via_inline_out,
        "the bracket-as-source composition must equal, byte for byte, the \
         same wallet built by pasting the origins inline into the template"
    );
}

/// N3's row 3 (whole-diff review r1 M1): a bracket sources SOME slots while
/// another has NO source at all -- the shape N3 could plausibly have broken,
/// because it makes `apply_path_override_per_slot` PROCEED where it used to
/// early-return the instant any slot lacked a path source. Rows 1 and 2
/// above cover "bracket wins" and "bracket disagrees with --path"; neither
/// exercises a slot N3's bracket source cannot reach at all.
///
/// `@0` has a bracket origin (wins per N3); `@1` has none -- no inline
/// origin, no `--path`, no bracket -- so it must still hit today's
/// non-canonical-wrapper refusal exactly as it did before N3 existed
/// (`tests/cli_path_override_reaches_noncanonical.rs::
/// address_without_path_still_refuses_a_noncanonical_wrapper`, which uses
/// bare keys and so never enters N3's code path at all).
#[test]
fn v_n3_a_slot_with_no_path_from_any_source_still_refuses() {
    let x0 = xpub_at("48'/0'/0'/2'");
    let x1 = xpub_at("48'/0'/1'/2'");
    let out = md()
        .args([
            "descriptor",
            "--template",
            "tr(@0/<0;1>/*,pk(@1/<0;1>/*))",
            "--key",
            &format!("@0=[73c5da0a/48'/0'/0'/2']{x0}"),
            "--key",
            &format!("@1={x1}"),
            // no --path, and @1's --key carries no bracket at all
        ])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "a slot with NO path source composed: {}",
        stdout_of(&out)
    );
    let err = stderr_of(&out);
    assert!(
        err.contains("non-canonical wrapper requires explicit origin"),
        "the refusal must name the missing origin, unchanged by N3's bracket \
         source existing for a DIFFERENT slot: {err}"
    );
    assert!(err.contains("@1"), "the refusal must name the slot: {err}");
}

/// A bracket carrying a fingerprint and NO path states nothing about the
/// path, so there is nothing to agree or disagree about and the slot still
/// composes on the shared `--path`. Without this row the fix above could be
/// a blanket "any bracket needs a path source".
#[test]
fn v_patheff_a_fingerprint_only_bracket_is_unaffected() {
    let x0 = xpub_at("48'/0'/0'/2'");
    let out = md()
        .args([
            "descriptor",
            "--template",
            PATHLESS_SINGLE_SLOT,
            "--key",
            &format!("@0=[cafebabe]{x0}"),
            "--path",
            "48'/0'/0'/2'",
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    assert!(stdout_of(&out).contains("[cafebabe/48'/0'/0'/2']"));
}
