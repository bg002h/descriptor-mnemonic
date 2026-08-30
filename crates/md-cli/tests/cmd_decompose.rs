//! `md decompose` — the D row (SPEC `design/SPEC_wallet_form_converter.md`
//! "P3 — the concrete descriptor becomes an entrance"; plan §3 C3).
//!
//! Roster rows pinned here (plan §4): **V-D-DEPTH, V-D-NOORIG, V-D-REUSE,
//! V-D-SHAPE2, V-D-JSON, V-D-PAIR, V-D-CHKSUM**. V-D-RT — the acceptance-grade
//! round trip through `md encode --key` and `mk encode --keys` — lives in
//! `cmd_decompose_roundtrip.rs`, because it needs the generated fixture.
//!
//! Every test drives the REAL binary: P3's refusals are a CLI contract (the
//! operator reads them off a terminal), and a unit test on the refusal
//! constructor would pass while the dispatch never reached it.

#![allow(missing_docs)]

use assert_cmd::Command;

// ─── fixture key material ───────────────────────────────────────────────────
//
// PROVENANCE: records 1-3 of `tests/fixtures/pathological/keys.txt` — the
// PINNED depth-consistent fixture SPEC Acceptance 1(c) names (r2 M5). Each is a
// depth-4 xpub whose origin path has four components ending in the xpub's own
// child number, so `mk encode --keys` accepts them (measured).

pub const K0: &str = "[73c5da0a/48'/0'/0'/2']xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf";
pub const K1: &str = "[73c5da0a/48'/0'/1'/2']xpub6DzhyrnFFYQ1HimDiM388xHnDiRPNdZJFBmmxge3Y1WWcHLtMJLfRuhRHqnQCPbTj3fGKTuKFLHzzwpJkp5Dtc3UtLKZKaVZe1yqMBXd6Vk";
pub const K2: &str = "[73c5da0a/48'/0'/2'/2']xpub6EGx8sPr9FxPPE1rbZazhqWwpMXA3Hf5DYKtZbL7c4BSddzmQktp96UaTvecEkoCZysuaj79GMCFZYT1KKk7Ph2M3Kf5g8B82KZ8TZ9SKQR";

/// The bare xpub of record 1 — used to build the origin-less and key-reuse
/// shapes without introducing new key material.
pub const K0_BARE: &str = "xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf";

/// The pinned 2-of-3 the round trip is measured on.
pub fn fixture_descriptor() -> String {
    format!("wsh(sortedmulti(2,{K0}/<0;1>/*,{K1}/<0;1>/*,{K2}/<0;1>/*))")
}

pub fn run(args: &[&str]) -> (i32, String, String) {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(args)
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

/// Every P3 refusal must say "unsupported" / "forbidden by BIP 388" and must
/// NEVER say "invalid" of the input (SPEC A3, operator ruling 2026-08-30 —
/// "Bad ideas can be valid, but we don't want to support BIP forbidden
/// wallets"). Asserted as a helper so no row can forget half of it.
fn assert_bip388_wording(stderr: &str) {
    assert!(
        stderr.contains("forbidden by BIP 388") || stderr.contains("BIP 388"),
        "refusal must cite BIP 388; got: {stderr}"
    );
    assert!(
        stderr.contains("UNSUPPORTED") || stderr.contains("unsupported"),
        "refusal must say UNSUPPORTED; got: {stderr}"
    );
    assert!(
        !stderr.to_lowercase().contains("invalid descriptor")
            && !stderr.to_lowercase().contains("is invalid"),
        "refusal must NEVER call the input invalid; got: {stderr}"
    );
}

// ─── the happy path the refusal rows are measured against ───────────────────

#[test]
fn decompose_emits_template_keys_and_fingerprints() {
    let d = fixture_descriptor();
    let (code, stdout, stderr) = run(&["decompose", &d]);
    assert_eq!(code, 0, "stderr: {stderr}");
    // The keyless template, in md's admitted surface and `'` hardened
    // spelling (SPEC "Canonicalisation").
    assert!(
        stdout.contains(
            "wsh(sortedmulti(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/1'/2'/<0;1>/*,@2/48'/0'/2'/2'/<0;1>/*))"
        ),
        "template missing from: {stdout}"
    );
    // `'` hardened spelling, zero `h`-forms (SPEC "Canonicalisation": md emits
    // `'`, and the spelling changes the checksum).
    assert!(
        !stdout.contains("48h/") && !stdout.contains("0h/") && !stdout.contains("2h/"),
        "an `h` hardened spelling leaked into the emission: {stdout}"
    );
    // Key lines AS PARSED — depth 4, child 2', origin from the input.
    for k in [K0, K1, K2] {
        assert!(stdout.contains(k), "key line {k} missing from: {stdout}");
    }
    // Per-slot fingerprint flags.
    for f in [
        "--fingerprint @0=73c5da0a",
        "--fingerprint @1=73c5da0a",
        "--fingerprint @2=73c5da0a",
    ] {
        assert!(stdout.contains(f), "{f} missing from: {stdout}");
    }
}

#[test]
fn emit_keys_is_a_valid_mk_encode_keys_file() {
    let d = fixture_descriptor();
    let (code, stdout, stderr) = run(&["decompose", &d, "--emit", "keys"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(
        lines,
        vec![K0, K1, K2],
        "one origin-notated record per line"
    );
}

#[test]
fn emit_template_is_exactly_one_line_and_carries_no_checksum() {
    let d = fixture_descriptor();
    let (code, stdout, _) = run(&["decompose", &d, "--emit", "template"]);
    assert_eq!(code, 0);
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 1, "one template line, got: {stdout}");
    assert!(
        !lines[0].contains('#'),
        "a BIP-388 template carries no BIP-380 checksum -- md's parser computes \
         the checksum over the SYNTHETIC-substituted form, so any suffix here is \
         one md refuses. got: {}",
        lines[0]
    );
}

#[test]
fn emit_descriptor_is_canonical_and_carries_a_recomputed_checksum() {
    // SPEC "Canonicalisation": emitted DESCRIPTORS use `'`, preserve input key
    // order, and always recompute and append the checksum.
    let d = fixture_descriptor();
    let (code, stdout, _) = run(&["decompose", &d, "--emit", "descriptor"]);
    assert_eq!(code, 0);
    let line = stdout.trim();
    assert_eq!(line, format!("{d}#auwzhqew"), "canonical form drifted");
    // Supplying the SAME descriptor with its checksum already attached must
    // reach the identical byte string (recompute, never copy).
    let (code2, stdout2, _) = run(&[
        "decompose",
        &format!("{d}#auwzhqew"),
        "--emit",
        "descriptor",
    ]);
    assert_eq!(code2, 0);
    assert_eq!(stdout2.trim(), line);
}

#[test]
fn emit_commands_prints_runnable_md_and_mk_lines() {
    let d = fixture_descriptor();
    let (code, stdout, stderr) = run(&["decompose", &d, "--emit", "commands"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stdout.contains("md encode "), "no md encode line: {stdout}");
    assert!(stdout.contains("mk encode "), "no mk encode line: {stdout}");
    assert!(
        stdout.contains("--fingerprint '@0=73c5da0a'"),
        "the keyed command must carry the per-slot fingerprints: {stdout}"
    );
    assert!(
        stdout.contains("--from-md1-set"),
        "the split command must bind the mk1 cards to the policy card: {stdout}"
    );
}

// ─── V-D-CHKSUM ─────────────────────────────────────────────────────────────

#[test]
fn v_d_chksum_bare_descriptor_without_a_checksum_is_accepted() {
    let (code, stdout, stderr) = run(&["decompose", &fixture_descriptor(), "--emit", "template"]);
    assert_eq!(
        code, 0,
        "a checksum-free descriptor must be accepted: {stderr}"
    );
    assert!(stdout.contains("@0"));
}

#[test]
fn v_d_chksum_correct_checksum_is_accepted() {
    let d = format!("{}#auwzhqew", fixture_descriptor());
    let (code, stdout, stderr) = run(&["decompose", &d, "--emit", "template"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stdout.contains("@0"));
}

#[test]
fn v_d_chksum_wrong_checksum_draws_a_named_error_not_the_f420_class() {
    let d = format!("{}#00000000", fixture_descriptor());
    let (code, _, stderr) = run(&["decompose", &d]);
    assert_ne!(code, 0, "a wrong checksum must refuse");
    assert!(
        stderr.contains("checksum"),
        "the refusal must name the checksum; got: {stderr}"
    );
    assert!(
        stderr.contains("00000000") && stderr.contains("auwzhqew"),
        "the refusal must print BOTH the supplied and the computed checksum, so \
         the operator can tell a mistyped suffix from an altered descriptor; got: {stderr}"
    );
    assert!(
        stderr.contains("drop the `#"),
        "the refusal must name the remedy (drop the suffix; md recomputes); got: {stderr}"
    );
    // NOT the F-420 class: `md encode`'s "no @i placeholders" dead-end.
    assert!(
        !stderr.contains("no @i placeholders") && !stderr.contains("me sysw pack"),
        "the F-420-class misdirect must not appear here; got: {stderr}"
    );
}

// ─── V-D-JSON ───────────────────────────────────────────────────────────────

#[test]
fn v_d_json_core_listdescriptors_output_refuses_with_guidance() {
    let json = format!(
        r#"{{"wallet_name":"w","descriptors":[{{"desc":"{}#auwzhqew","active":true}}]}}"#,
        fixture_descriptor()
    );
    let (code, _, stderr) = run(&["decompose", &json]);
    assert_ne!(code, 0);
    assert!(
        stderr.contains("listdescriptors"),
        "the refusal must name what the input IS; got: {stderr}"
    );
    assert!(
        stderr.contains("\"desc\""),
        "the refusal must name the field to extract; got: {stderr}"
    );
    // NOT the bare checksum error (SPEC P3 "Input boundary", the F-420 class).
    assert!(
        !stderr.contains("checksum"),
        "a JSON blob must not be reported as a checksum problem; got: {stderr}"
    );
}

#[test]
fn v_d_json_bare_array_form_also_refuses_with_guidance() {
    let json = format!(r#"[{{"desc":"{}"}}]"#, fixture_descriptor());
    let (code, _, stderr) = run(&["decompose", &json]);
    assert_ne!(code, 0);
    assert!(stderr.contains("listdescriptors"), "got: {stderr}");
}

// ─── V-D-PAIR ───────────────────────────────────────────────────────────────

#[test]
fn v_d_pair_two_descriptors_refuse_with_the_multipath_remedy() {
    let recv = format!("wsh(sortedmulti(2,{K0}/0/*,{K1}/0/*,{K2}/0/*))");
    let chg = format!("wsh(sortedmulti(2,{K0}/1/*,{K1}/1/*,{K2}/1/*))");
    let (code, _, stderr) = run(&["decompose", &recv, &chg]);
    assert_ne!(code, 0);
    assert!(
        stderr.contains("<0;1>"),
        "the refusal must name the combined multipath form; got: {stderr}"
    );
    assert!(
        stderr.contains("ONE descriptor") || stderr.contains("one descriptor"),
        "the refusal must say decompose takes one; got: {stderr}"
    );
    assert!(
        !stderr.contains("checksum"),
        "a pair must not be reported as a checksum problem; got: {stderr}"
    );
}

#[test]
fn v_d_pair_in_file_holding_two_descriptors_refuses_the_same_way() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("pair.txt");
    std::fs::write(
        &p,
        format!(
            "wsh(sortedmulti(2,{K0}/0/*,{K1}/0/*,{K2}/0/*))\nwsh(sortedmulti(2,{K0}/1/*,{K1}/1/*,{K2}/1/*))\n"
        ),
    )
    .unwrap();
    let (code, _, stderr) = run(&["decompose", "--in", p.to_str().unwrap()]);
    assert_ne!(code, 0);
    assert!(stderr.contains("<0;1>"), "got: {stderr}");
}

#[test]
fn v_d_pair_a_single_fixed_path_descriptor_is_still_accepted() {
    // SPEC P3 "Input boundary": D accepts a bare descriptor "multipath or
    // FIXED-PATH". Only the PAIR refuses -- without this the pair refusal
    // could be implemented as "refuse every single-chain descriptor" and the
    // row above would still pass.
    let recv = format!("wsh(sortedmulti(2,{K0}/0/*,{K1}/0/*,{K2}/0/*))");
    let (code, stdout, stderr) = run(&["decompose", &recv, "--emit", "template"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stdout.contains("@0/48'/0'/0'/2'/0/*"), "got: {stdout}");
}

// ─── V-D-DEPTH ──────────────────────────────────────────────────────────────

#[test]
fn v_d_depth_inconsistent_input_key_refuses_naming_mks_constraint() {
    // The xpub is depth 4 / child 2'; the origin path states three components
    // ending 0'. `mk encode --keys` refuses exactly this record, measured:
    //   "xpub origin-path mismatch: xpub depth 4 / child 2' vs origin_path
    //    depth 3 / last Some(Hardened { index: 0 })"
    let bad = format!("[73c5da0a/48'/0'/0']{K0_BARE}");
    let d = format!("wsh(sortedmulti(2,{bad}/<0;1>/*,{K1}/<0;1>/*))");
    let (code, _, stderr) = run(&["decompose", &d]);
    assert_ne!(code, 0);
    assert!(
        stderr.contains("depth 4") && stderr.contains("depth 3"),
        "the refusal must print BOTH depths; got: {stderr}"
    );
    assert!(
        stderr.contains("mk"),
        "the refusal must name mk's constraint -- that is the whole reason it \
         exists (SPEC P3 'Key emission is round-trip-grade'); got: {stderr}"
    );
    assert!(
        stderr.contains("origin path") || stderr.contains("origin-path"),
        "got: {stderr}"
    );
}

#[test]
fn v_d_depth_a_consistent_key_is_not_refused() {
    // The negative half: without it the check could refuse EVERY key and the
    // row above would still pass.
    let (code, _, stderr) = run(&["decompose", &fixture_descriptor(), "--emit", "keys"]);
    assert_eq!(code, 0, "stderr: {stderr}");
}

#[test]
fn v_d_depth_child_number_disagreement_alone_refuses() {
    // Same component COUNT (4), different terminal component: the origin says
    // `.../3'` but the xpub's child number is 2'. mk reconstructs the child
    // number from the path, so this is lossy exactly as the depth case is.
    let bad = format!("[73c5da0a/48'/0'/0'/3']{K0_BARE}");
    let d = format!("wsh(sortedmulti(2,{bad}/<0;1>/*,{K1}/<0;1>/*))");
    let (code, _, stderr) = run(&["decompose", &d]);
    assert_ne!(code, 0, "a child-number disagreement must refuse too");
    assert!(stderr.contains("child"), "got: {stderr}");
}

// ─── V-D-NOORIG ─────────────────────────────────────────────────────────────

#[test]
fn v_d_noorig_bare_key_line_is_emitted_and_template_still_works() {
    let d = format!("wsh(sortedmulti(2,{K0_BARE}/<0;1>/*,{K1}/<0;1>/*))");
    let (code, stdout, stderr) = run(&["decompose", &d, "--emit", "keys"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let lines: Vec<&str> = stdout
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .collect();
    assert_eq!(
        lines,
        vec![K0_BARE, K1],
        "the origin-less key is a BARE line"
    );
    assert!(
        stderr.contains("@0") && stderr.contains("mk"),
        "the note must name the excluded slot and why; got: {stderr}"
    );

    // The template and descriptor outputs still work (SPEC P3).
    let (tcode, tout, _) = run(&["decompose", &d, "--emit", "template"]);
    assert_eq!(tcode, 0);
    assert!(
        tout.contains("wsh(sortedmulti(2,@0/<0;1>/*,@1/48'/0'/1'/2'/<0;1>/*))"),
        "got: {tout}"
    );
    let (dcode, dout, _) = run(&["decompose", &d, "--emit", "descriptor"]);
    assert_eq!(dcode, 0);
    assert!(dout.contains(K0_BARE) && dout.contains('#'), "got: {dout}");
}

#[test]
fn v_d_noorig_emit_commands_refuses_naming_the_keys_and_the_reason() {
    let d = format!("wsh(sortedmulti(2,{K0_BARE}/<0;1>/*,{K1}/<0;1>/*))");
    let (code, stdout, stderr) = run(&["decompose", &d, "--emit", "commands"]);
    assert_ne!(code, 0, "--emit commands must refuse; stdout was: {stdout}");
    assert!(
        stderr.contains("@0"),
        "the refusal must NAME the key(s) without an origin; got: {stderr}"
    );
    assert!(
        stderr.contains(&K0_BARE[..16]),
        "the refusal must name the key itself, not just the slot; got: {stderr}"
    );
    assert!(
        stderr.contains("origin"),
        "the refusal must give the reason -- an mk1 card binds key to origin by \
         design; got: {stderr}"
    );
    assert!(
        stderr.contains("--emit template") || stderr.contains("--emit keys"),
        "the refusal must name the emissions that DO work; got: {stderr}"
    );
}

#[test]
fn v_d_noorig_emit_commands_succeeds_when_every_key_has_an_origin() {
    // The negative half: `--emit commands` must not be refusing unconditionally.
    let (code, _, stderr) = run(&["decompose", &fixture_descriptor(), "--emit", "commands"]);
    assert_eq!(code, 0, "stderr: {stderr}");
}

// ─── V-D-REUSE ──────────────────────────────────────────────────────────────

#[test]
fn v_d_reuse_same_xpub_at_two_positions_refuses_as_bip388_forbidden() {
    // Two DIFFERENT key expressions (different origin paths) that deserialize
    // to the SAME public key -- BIP 388 rule (1), pairwise distinctness.
    // rust-miniscript parses this and `sanity_check` does NOT catch it (the two
    // `DescriptorPublicKey`s differ), measured 2026-08-30 -- so the check is
    // md's own.
    let twin = format!("[73c5da0a/48'/0'/9'/2']{K0_BARE}");
    let d = format!("wsh(sortedmulti(2,{K0}/<0;1>/*,{twin}/<0;1>/*))");
    let (code, _, stderr) = run(&["decompose", &d]);
    assert_ne!(code, 0);
    assert_bip388_wording(&stderr);
    assert!(
        stderr.contains("pairwise distinct") || stderr.contains("pairwise-distinct"),
        "the refusal must cite the rule it is enforcing; got: {stderr}"
    );
    assert!(
        stderr.contains("48'/0'/0'/2'") && stderr.contains("48'/0'/9'/2'"),
        "the refusal must name BOTH positions by their origins; got: {stderr}"
    );
}

#[test]
fn v_d_reuse_distinct_keys_are_not_refused() {
    // The negative half: the reuse check must not fire on a wallet whose keys
    // merely share a master fingerprint (all three fixture keys do).
    let (code, _, stderr) = run(&["decompose", &fixture_descriptor(), "--emit", "template"]);
    assert_eq!(code, 0, "stderr: {stderr}");
}

// ─── V-D-SHAPE2 ─────────────────────────────────────────────────────────────

#[test]
fn v_d_shape2_same_placeholder_with_non_disjoint_multipath_refuses() {
    // BIP 388's own invalid example, in concrete form: one key expression at
    // two positions with IDENTICAL multipath sets. Reachable here because
    // rust-miniscript parses it (measured; `sanity_check` reports
    // `RepeatedPubkeys`, but decompose must refuse in its own words).
    let d = format!("wsh(sortedmulti(2,{K0}/<0;1>/*,{K0}/<0;1>/*))");
    let (code, _, stderr) = run(&["decompose", &d]);
    assert_ne!(code, 0);
    assert_bip388_wording(&stderr);
    assert!(
        stderr.contains("disjoint"),
        "the refusal must name the disjointness rule; got: {stderr}"
    );
    assert!(
        stderr.contains("0, 1") || stderr.contains("{0,1}") || stderr.contains("0;1"),
        "the refusal must print the overlapping multipath set; got: {stderr}"
    );
}

#[test]
fn v_d_shape2_partially_overlapping_multipath_sets_also_refuse() {
    // {0,1} and {1,2} share 1 -- non-disjoint without being equal, so an
    // implementation that only compared the two sets for EQUALITY would pass
    // the row above and fail here.
    let d = format!("wsh(sortedmulti(2,{K0}/<0;1>/*,{K0}/<1;2>/*))");
    let (code, _, stderr) = run(&["decompose", &d]);
    assert_ne!(code, 0);
    assert_bip388_wording(&stderr);
    assert!(stderr.contains("disjoint"), "got: {stderr}");
}

#[test]
fn v_d_shape2_disjoint_sets_refuse_naming_mds_narrower_template_surface() {
    // {0,1} and {2,3} are DISJOINT, so BIP 388 permits this shape. md's own
    // template surface does not -- `md encode` refuses `@0` at two positions
    // ("@0 appears with inconsistent path/multipath/hardening", measured
    // 2026-08-30, SPEC A3's "measured scope note"). Decompose must not emit a
    // template md cannot ingest, so it refuses -- but NOT as a BIP violation,
    // because there is none.
    let d = format!("wsh(sortedmulti(2,{K0}/<0;1>/*,{K0}/<2;3>/*))");
    let (code, _, stderr) = run(&["decompose", &d]);
    assert_ne!(code, 0);
    assert!(
        stderr.contains("unsupported") || stderr.contains("UNSUPPORTED"),
        "got: {stderr}"
    );
    assert!(
        stderr.contains("BIP 388 permits") || stderr.contains("permitted by BIP 388"),
        "this shape is BIP-LEGAL and the refusal must say so rather than blaming \
         the BIP; got: {stderr}"
    );
    assert!(
        stderr.contains("md encode") || stderr.contains("md's template"),
        "the refusal must name whose limit it is; got: {stderr}"
    );
}

// ─── the emitted commands must actually RUN ─────────────────────────────────

#[test]
fn emit_commands_route1_line_actually_runs() {
    // Every md template carries `'` as its hardened marker, so a naively
    // single-quoted command line breaks at the FIRST hardened step and the
    // shell sees a different template than the one printed. Proven by RUNNING
    // the emitted line, not by reading it.
    let d = fixture_descriptor();
    let (_, stdout, _) = run(&["decompose", &d, "--emit", "commands"]);
    let route1 = stdout
        .lines()
        .skip_while(|l| !l.starts_with("md encode "))
        .take_while(|l| !l.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        route1.starts_with("md encode "),
        "no route-1 command line in: {stdout}"
    );
    let md = assert_cmd::cargo::cargo_bin("md");
    let dir = tempfile::tempdir().unwrap();
    let path = format!(
        "{}:{}",
        md.parent().unwrap().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = std::process::Command::new("sh")
        .arg("-c")
        .arg(&route1)
        .current_dir(dir.path())
        .env("PATH", path)
        .output()
        .expect("sh spawn");
    assert!(
        out.status.success(),
        "the emitted command did not run:\n{route1}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).starts_with("md1"),
        "the emitted command produced no md1 artifact: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}
