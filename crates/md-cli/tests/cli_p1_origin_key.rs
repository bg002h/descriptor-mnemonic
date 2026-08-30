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
