//! F-410 item 2 — `md encode` notes a placeholder origin with no hardened
//! component.
//!
//! WHAT THE NOTE IS FOR. In an md template the path written after `@i` IS that
//! key's origin declaration — the same grammatical slot that carries
//! `@0/48'/0'/0'/2'`. Nothing is relocated and nothing is dropped. But a reader
//! coming from descriptors reads `wpkh(@0/0/*)` as "derive `/0/i` from the key
//! I supply", and md reads it as "the key I supply lives at origin `m/0`, use
//! site `/i`".
//!
//! THE TWO READINGS AGREE ON A MASTER XPUB, which is precisely what makes the
//! misreading survive a spot check — unhardened steps commute, so both spellings
//! derive the same address:
//!
//! ```text
//! wpkh(@0/0/*)  ->  bc1qr932kkqd95r3chv9sh36wkjez4jvsmlf46xuc9
//! wpkh(@0/*)    ->  bc1qr932kkqd95r3chv9sh36wkjez4jvsmlf46xuc9
//! ```
//!
//! They DIVERGE the moment a NON-master xpub is supplied: the plate then backs
//! `X/i` where the user meant `X/0/i`. A hardened component cannot be misread
//! this way at all — an xpub cannot derive one — so the note is keyed on an
//! origin whose every component is unhardened.
//!
//! NOTE, NEVER A REFUSAL. This is a user-INTENT risk, not a codec defect: the
//! encoding is correct, and the same grammatical slot carries every legitimate
//! origin declaration, so refusing would reject correct templates to catch a
//! misreading. stdout and the exit code are therefore untouched, and this file
//! pins BOTH — the goldens below were captured from the binary BEFORE the note
//! existed, so a note that ever leaked onto stdout fails here.

#![allow(missing_docs)]

use std::process::Command as StdCommand;

fn encode(args: &[&str]) -> (String, String, i32) {
    let out = StdCommand::new(assert_cmd::cargo::cargo_bin("md"))
        .args(args)
        .output()
        .expect("invoke md encode");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().expect("md exited normally"),
    )
}

fn encode_template(template: &str) -> (String, String, i32) {
    encode(&["encode", "--group-size", "0", template])
}

/// The note's distinguishing phrase, in one place.
const NEEDLE: &str = "key ORIGIN, not a derivation step";

#[test]
fn unhardened_origin_emits_the_note() {
    let (stdout, stderr, code) = encode_template("wpkh(@0/0/*)");
    assert_eq!(
        code, 0,
        "the note must never change the exit code: {stderr}"
    );
    assert!(stderr.contains(NEEDLE), "expected the note, got: {stderr}");
    // It must name the slot AND the path, or the reader cannot act on it.
    assert!(
        stderr.contains("@0") && stderr.contains("/0"),
        "the note must name the slot and the path it is about: {stderr}"
    );
    // stdout is BYTE-IDENTICAL to the pre-note binary's output.
    assert_eq!(stdout, "md1yqzqqqtk8nf99an9vzl\n");
}

#[test]
fn multi_component_unhardened_origin_emits_the_note() {
    let (stdout, stderr, code) = encode_template("wpkh(@0/0/1/*)");
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.contains(NEEDLE), "expected the note, got: {stderr}");
    assert!(
        stderr.contains("/0/1"),
        "the note must echo the caller's OWN path: {stderr}"
    );
    assert_eq!(stdout, "md1yqyqrqqk0kh8ld3y8dp6\n");
}

#[test]
fn hardened_origin_is_silent() {
    // CLEAN NEGATIVE. A hardened component cannot be read as a use-site step at
    // all, so the ambiguity the note describes does not exist here. If this
    // fired, every standard BIP-48/84 template would carry a pointless warning.
    let (stdout, stderr, code) = encode_template("wpkh(@0/84'/0'/0'/<0;1>/*)");
    assert_eq!(code, 0, "{stderr}");
    assert!(
        !stderr.contains(NEEDLE),
        "a hardened origin must be SILENT: {stderr}"
    );
    assert_eq!(stdout, "md1yq802gggqpsqwgtua24e7ssf3\n");
}

#[test]
fn no_origin_at_all_is_silent() {
    // CLEAN NEGATIVE. With no path after the placeholder there is no
    // declaration to misread.
    let (stdout, stderr, code) = encode_template("wpkh(@0/*)");
    assert_eq!(code, 0, "{stderr}");
    assert!(
        !stderr.contains(NEEDLE),
        "a pathless placeholder must be SILENT: {stderr}"
    );
    assert_eq!(stdout, "md1yqqqq2wy43c0uqcl29\n");
}

#[test]
fn only_the_affected_slot_is_named() {
    // @0 is all-unhardened, @1 is a standard BIP-48 origin. The note must name
    // @0 and must NOT sweep @1 in with it.
    let (stdout, stderr, code) =
        encode_template("wsh(multi(2,@0/0/<0;1>/*,@1/48'/0'/0'/2'/<0;1>/*))");
    assert_eq!(code, 0, "{stderr}");
    let note = stderr
        .lines()
        .find(|l| l.contains(NEEDLE))
        .unwrap_or_else(|| panic!("expected the note, got: {stderr}"));
    assert!(note.contains("@0"), "must name @0: {note}");
    assert!(
        !note.contains("@1"),
        "must NOT name the hardened slot @1: {note}"
    );
    assert_eq!(stdout, "md15pzqjmppp9gqpsgvzzshrwxrtzllpy97\n");
}

/// Two firing DECLARATIONS collapse into ONE line here — the tier-1 note joins
/// its slots into a shared sentence, unlike F-411's per-slot tier.
///
/// The template used to be one placeholder at two occurrences. Since N1
/// (`design/SPEC_mdcli_mini.md` R-N1a) that shape is refused before any
/// advisory runs, so the occurrence spelling is unconstructible; two slots is
/// the remaining way to ask the same question of the emitter, and it is the
/// stronger one — a per-slot regression here would print two lines.
#[test]
fn note_is_emitted_once_per_run() {
    let (_, stderr, code) = encode_template("wsh(or_d(pk(@0/0/<0;1>/*),pk(@1/0/<0;1>/*)))");
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(
        stderr.lines().filter(|l| l.contains(NEEDLE)).count(),
        1,
        "exactly one note per run: {stderr}"
    );
}

#[cfg(feature = "json")]
#[test]
fn json_branch_emits_the_note_too() {
    // Advisory parity: `--json` moves the artifact's SHAPE, not which advisories
    // fire. Every other stderr advisory in `md encode` is emitted on both
    // branches; a note that vanished under `--json` would be a silent hole.
    let (stdout, stderr, code) = encode(&["encode", "--json", "wpkh(@0/0/*)"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(
        stderr.contains(NEEDLE),
        "expected the note on --json: {stderr}"
    );
    assert!(
        stdout.contains("\"phrase\""),
        "json artifact still emitted: {stdout}"
    );
}
