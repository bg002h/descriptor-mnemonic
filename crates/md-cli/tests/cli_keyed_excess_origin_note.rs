//! F-411 — `md encode` notes a declared origin that runs DEEPER than the xpub
//! seated in that slot, when the excess steps are unhardened.
//!
//! ## The two tiers, and why only one of them is keyed
//!
//! In an md template the path written after `@i` **is** that key's origin
//! declaration — the same grammatical slot that carries `@0/48'/0'/0'/2'`.
//! Nothing is relocated and nothing is dropped, so `wpkh(@0/0/*)` and
//! `wpkh(@0/*)` derive the identical address. The risk is a reader's INTENT: a
//! reader arriving from descriptors reads the trailing steps as derivation
//! ("give me `/0/i` from the key I supply"), and the two readings diverge the
//! moment a non-master xpub is seated.
//!
//! **Keyless (F-410, `cli_unhardened_origin_note.rs`)** — with no key seated,
//! a trailing-unhardened origin is indistinguishable from every ordinary
//! single-chain template, so the note is deliberately narrow: it fires only
//! when the origin has NO hardened component at all. It is not that the risk is
//! smaller there, it is **undecidable**, and silence beats note fatigue.
//!
//! **Keyed (this file)** — a seated xpub carries its own BIP-32 depth, which
//! makes the question decidable. If the declared origin is longer than the
//! seated key's depth and every step past that depth is unhardened, the excess
//! is exactly what a descriptor-thinker meant as derivation and exactly what
//! that xpub COULD have derived. Standard workflows stay silent: an account
//! xpub with a matching-depth origin has no excess at all.
//!
//! ## Measured, not asserted
//!
//! With the depth-3 key below seated, the excess `/0` changes no address —
//! `md address` gives `bc1qr932kkqd95r3chv9sh36wkjez4jvsmlf46xuc9` for BOTH
//! `wpkh(@0/84'/0'/0'/0/*)` and `wpkh(@0/84'/0'/0'/*)`, because md derives
//! nothing through an origin. What the descriptor-style reading meant, `X/0/i`,
//! is `bc1qmxrw6qdh5g3ztfcwm0et5l8mvws4eva24kmp8m`. That gap is the finding.
//!
//! ## Note, never a refusal
//!
//! Both spellings are legitimate origin declarations, so refusing would reject
//! correct templates to catch a misreading. Every golden `stdout` below was
//! captured from the binary BEFORE this note existed, so a note that ever leaks
//! onto stdout — or moves the exit code — fails here.

#![allow(missing_docs)]

use std::process::Command as StdCommand;

/// Abandon-mnemonic account xpub at DEPTH 3 (parent fp `155bca59`).
const K3: &str = "xpub6BosfCnifzxcFwrSzQiqu2DBVTshkCXacvNsWGYJVVhhawA7d4R5WSWGFNbi8Aw6ZRc1brxMyWMzG3DSSSSoekkudhUd9yLb6qx39T9nMdj";
/// Abandon-mnemonic account xpub at DEPTH 4 (m/48'/0'/0'/2').
const K4: &str = "xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf";
/// BIP-32 master (depth 0), for the case the CLI refuses outright.
const MASTER: &str = "xpub661MyMwAqRbcFkPHucMnrGNzDwb6teAX1RbKQmqtEF8kK3Z7LZ59qafCjB9eCRLiTVG3uxBxgKvRgbubRhqSKXnGGb1aoaqLrpMBDrVxga8";

/// This note's distinguishing phrase, in one place.
const KEYED: &str = "declared origin runs DEEPER than the xpub seated there";
/// The F-410 keyless note's phrase — asserted ABSENT/PRESENT independently, so
/// the two tiers cannot be confused for one another.
const NARROW: &str = "key ORIGIN, not a derivation step";

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

fn encode_keyed(template: &str, keys: &[&str]) -> (String, String, i32) {
    let mut args = vec!["encode", "--group-size", "0", template];
    for k in keys {
        args.push("--key");
        args.push(k);
    }
    encode(&args)
}

// ---------------------------------------------------------------- it FIRES

#[test]
fn excess_unhardened_suffix_under_a_depth3_key_fires() {
    // origin `84'/0'/0'/0` is 4 levels; the seated key is depth 3; the excess
    // `/0` is unhardened. All three conditions hold.
    let (stdout, stderr, code) = encode_keyed("wpkh(@0/84'/0'/0'/0/*)", &[&format!("@0={K3}")]);
    assert_eq!(
        code, 0,
        "the note must never change the exit code: {stderr}"
    );
    assert!(stderr.contains(KEYED), "expected the note, got: {stderr}");
    let note = stderr
        .lines()
        .find(|l| l.contains(KEYED))
        .expect("note line");
    // It must name the slot, the origin AND the excess, or nobody can act on it.
    assert!(note.contains("@0"), "must name the slot: {note}");
    assert!(note.contains("84'/0'/0'/0"), "must echo the origin: {note}");
    assert!(
        note.contains("depth 3"),
        "must name the seated depth: {note}"
    );
    // stdout is BYTE-IDENTICAL to the pre-note binary's output.
    assert_eq!(
        stdout,
        "md1f3cfxqspqzt6jzqqqp2sg8kjtcxg2y6qpz8f3lt0aeyzl9flkeemudugfjxg3dujn6sed8nhuja7e2ff\n\
         md1f3cfxqsf5g5seqdm5eyg0eurl495gd6nefux4etke4l3sk39c8alzzwae9ycw0h6t6qv0qnr7vdt9l8a\n"
    );
}

#[test]
fn the_clause_is_not_hardcoded_to_depth_three() {
    // A DEPTH-4 key with a 5-level origin: same shape, different depth. Without
    // this, a `depth == 3` constant would pass every other test in this file.
    let (stdout, stderr, code) = encode_keyed(
        "wsh(multi(1,@0/48'/0'/0'/2'/0/<0;1>/*))",
        &[&format!("@0={K4}")],
    );
    assert_eq!(code, 0, "{stderr}");
    let note = stderr
        .lines()
        .find(|l| l.contains(KEYED))
        .unwrap_or_else(|| panic!("expected the note, got: {stderr}"));
    assert!(
        note.contains("depth 4"),
        "must name the seated depth: {note}"
    );
    assert_eq!(
        stdout,
        "md1f2aecqspqzmvyyy5pqqxppsqq4gythgx8egtq4pcwl6u5p2us6r6zsnl2rd0q6gghvalgyp4qsrh3qg2wnd\n\
         md1f2aecqsflcdlz64mrqgdrha0m7umapumfj075dhzfzvynh66n94j5lcxlmx9ayav9mj0jjvk8xuss4gqmme\n"
    );
}

#[test]
fn only_the_deeper_slot_is_named() {
    // @0: depth-3 key under a 4-level origin -> fires.
    // @1: depth-4 key under its matching 4-level BIP-48 origin -> silent.
    let (stdout, stderr, code) = encode_keyed(
        "wsh(multi(2,@0/84'/0'/0'/0/<0;1>/*,@1/48'/0'/0'/2'/<0;1>/*))",
        &[&format!("@0={K3}"), &format!("@1={K4}")],
    );
    assert_eq!(code, 0, "{stderr}");
    let note = stderr
        .lines()
        .find(|l| l.contains(KEYED))
        .unwrap_or_else(|| panic!("expected the note, got: {stderr}"));
    assert!(note.contains("@0"), "must name @0: {note}");
    assert!(
        !note.contains("@1"),
        "must NOT sweep in the matching-depth slot @1: {note}"
    );
    assert_eq!(
        stdout,
        "md1f8hlzps9q2t6jzqp9kzzz2sqrqscy9zhqjrmf9ury9zdqq3r5cl4h7ujp0j5lmvua7yyvn8ruaupa650\n\
         md1f8hlzpsdugfjxg3dujn6s6y2gvsxa6vjy8u7pl6j6yxafu57r2u4mv6lcctgjur7l3qnxmqrye92d3ej\n\
         md1f8hlzpsjwae9ycw0h6tmhwsv0jskp2rsal4egz4ep5859p875x67p5s3wem7sgluxls4ehwsr65xy5ml\n\
         md1f8hlzps664mrqgdrha0m7umapumfj075dhzfzvynh66n94j5lcxlmx9ayav9mj0jjfqrlyl2akgw4a\n"
    );
}

/// ONE LINE PER FIRING SLOT — the rule the emitter states beside itself, and
/// the one that distinguishes this tier from F-410's joined list: each line
/// carries its own depth, level count and excess, which do not collapse into a
/// shared sentence.
///
/// WHAT THIS ROW USED TO SAY, AND WHY IT CHANGED. It used to put ONE
/// placeholder at two occurrences and assert a single line, pinning "per
/// declaration, not per occurrence". Since N1 (`design/SPEC_mdcli_mini.md`
/// R-N1a) a placeholder at two use sites with the same path expression is
/// refused outright, so that template no longer reaches the emitter at all and
/// the per-occurrence half of the rule is unconstructible through the CLI. The
/// per-declaration half is what remains observable, and it is what this row
/// now measures: two firing slots, two lines, neither swallowed.
#[test]
fn note_is_said_once_per_firing_slot() {
    let (_, stderr, code) = encode_keyed(
        "wsh(or_d(pk(@0/84'/0'/0'/0/<0;1>/*),pk(@1/48'/0'/0'/2'/0/<0;1>/*)))",
        &[&format!("@0={K3}"), &format!("@1={K4}")],
    );
    assert_eq!(code, 0, "{stderr}");
    let lines: Vec<&str> = stderr.lines().filter(|l| l.contains(KEYED)).collect();
    assert_eq!(
        lines.len(),
        2,
        "one line per firing slot, and both slots fire here: {stderr}"
    );
    assert!(lines.iter().any(|l| l.contains("@0")), "{stderr}");
    assert!(lines.iter().any(|l| l.contains("@1")), "{stderr}");
}

#[cfg(feature = "json")]
#[test]
fn json_branch_emits_the_note_too() {
    // Advisory parity: `--json` moves the artifact's SHAPE, not which advisories
    // fire. A note that vanished under `--json` would be a silent hole.
    //
    // `--force-chunked` is not incidental: this keyed payload is 118 data
    // symbols and the regular code caps a single string at 80, so the text
    // branch auto-chunks while `--json` refuses (exit 1) without the flag.
    let (stdout, stderr, code) = encode(&[
        "encode",
        "--json",
        "--force-chunked",
        "wpkh(@0/84'/0'/0'/0/*)",
        "--key",
        &format!("@0={K3}"),
    ]);
    assert_eq!(code, 0, "{stderr}");
    assert!(
        stderr.contains(KEYED),
        "expected the note on --json: {stderr}"
    );
    // The artifact is untouched, and these are the SAME two chunk strings the
    // pre-note binary printed on the text branch above — so this is a
    // cross-branch check, not a golden restating itself.
    for chunk in [
        "md1f3cfxqspqzt6jzqqqp2sg8kjtcxg2y6qpz8f3lt0aeyzl9flkeemudugfjxg3dujn6sed8nhuja7e2ff",
        "md1f3cfxqsf5g5seqdm5eyg0eurl495gd6nefux4etke4l3sk39c8alzzwae9ycw0h6t6qv0qnr7vdt9l8a",
    ] {
        assert!(stdout.contains(chunk), "json artifact changed: {stdout}");
    }
}

// --------------------------------------------------------------- it is SILENT

#[test]
fn matching_depth_origin_is_silent() {
    // CLEAN NEGATIVE, and the one that matters most: a depth-3 account xpub
    // under its own 3-level origin, the ordinary single-sig workflow. There is
    // no excess, so there is nothing to misread.
    let (stdout, stderr, code) = encode_keyed("wpkh(@0/84'/0'/0'/<0;1>/*)", &[&format!("@0={K3}")]);
    assert_eq!(code, 0, "{stderr}");
    assert!(
        !stderr.contains(KEYED),
        "the standard account-xpub workflow must be SILENT: {stderr}"
    );
    assert_eq!(
        stdout,
        "md1ftl2gqspqpm6jzzqqvqz4qs0dyhsvs5f5qzywnr7klmjg972nldnnhcmcsnyv3zme984qfn0j0qnz9cyfq\n\
         md1ftl2gqsg6y2gvsxa6vjy8u7pl6j6yxafu57r2u4mv6lcctgjur7l3p8wujjv88ma9aqzeyjq30dq4hzv\n"
    );
}

#[test]
fn excess_hardened_suffix_is_silent() {
    // CLEAN NEGATIVE. `84'/0'/0'/0'` is 4 levels over a depth-3 key, so the
    // origin does run deeper — but the excess step is HARDENED, and an xpub
    // cannot derive a hardened child. No descriptor-style reading is available,
    // so there is no ambiguity to report.
    let (stdout, stderr, code) = encode_keyed("wpkh(@0/84'/0'/0'/0'/*)", &[&format!("@0={K3}")]);
    assert_eq!(code, 0, "{stderr}");
    assert!(
        !stderr.contains(KEYED),
        "a hardened excess step must be SILENT: {stderr}"
    );
    assert_eq!(
        stdout,
        "md1f5j46qspqzt6jzzqqp2sg8kjtcxg2y6qpz8f3lt0aeyzl9flkeemudugfjxg3dujn6sufnq67c2ehzh8\n\
         md1f5j46qsf5g5seqdm5eyg0eurl495gd6nefux4etke4l3sk39c8alzzwae9ycw0h6t6q9ae683czvlzs6\n"
    );
}

#[test]
fn the_same_template_with_no_key_is_silent() {
    // THE TIER BOUNDARY, in one pair with the first test in this file: the
    // identical template, minus the key. Without a seated xpub there is no
    // depth to compare against, and `84'/0'/0'/0` is indistinguishable from
    // every ordinary single-chain template — undecidable, so silence wins.
    let (stdout, stderr, code) = encode_keyed("wpkh(@0/84'/0'/0'/0/*)", &[]);
    assert_eq!(code, 0, "{stderr}");
    assert!(!stderr.contains(KEYED), "no key means no clause: {stderr}");
    assert!(
        !stderr.contains(NARROW),
        "the keyless note is narrow — a hardened component silences it: {stderr}"
    );
    assert_eq!(stdout, "md1yqf02ggqqq8s2g74h4s3yj2\n");
}

#[test]
fn the_keyless_narrow_note_is_unchanged_by_a_seated_key() {
    // REGRESSION GUARD on the F-410 tier: `@0/0/*` is all-unhardened, so the
    // narrow note fires whether or not a key is seated, and it must keep firing
    // with its own wording. The keyed clause does not apply here (a 1-level
    // origin is not deeper than a depth-3 key), so exactly one note is said.
    let (stdout, stderr, code) = encode_keyed("wpkh(@0/0/*)", &[&format!("@0={K3}")]);
    assert_eq!(code, 0, "{stderr}");
    assert!(
        stderr.contains(NARROW),
        "the F-410 note must still fire with a key seated: {stderr}"
    );
    assert!(
        !stderr.contains(KEYED),
        "a SHALLOWER origin than the seated key is not this clause's business: {stderr}"
    );
    assert_eq!(
        stdout,
        "md1fmxtuqspqqsqq92pq76f0qepgngqygax8adlhystu487m8803h3pxgez9hj202rg3xshruum4rpzsh\n\
         md1fmxtuqs2gvsxa6vjy8u7pl6j6yxafu57r2u4mv6lcctgjur7l3p8wujjv88ma9aqwdlkhp49hwah3\n"
    );
}

#[test]
fn a_master_xpub_cannot_be_seated_at_all() {
    // Condition 1 of the clause (`depth >= 1`) excludes master, because the two
    // readings PROVABLY agree there — unhardened steps commute. Through the CLI
    // the case is unreachable for a stronger reason: `parse_key` admits only
    // depth 3 or 4, so a master xpub is refused before any advisory runs. This
    // pins WHY the CLI cannot exercise the guard; `emit_unhardened_origin_note`'s
    // own unit test reaches depth 0 directly.
    let (stdout, stderr, code) = encode_keyed("wpkh(@0/0/*)", &[&format!("@0={MASTER}")]);
    assert_eq!(code, 1, "a master xpub must be refused: {stderr}");
    assert!(
        stderr.contains("account-level xpub at depth 3 or 4") && stderr.contains("got 0"),
        "the refusal must name the depth: {stderr}"
    );
    assert_eq!(stdout, "", "nothing on stdout for a refused key");
}
