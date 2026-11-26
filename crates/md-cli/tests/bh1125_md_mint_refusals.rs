//! Bug hunt 2026-11-25 (descriptor-mnemonic report): mint-time refusals and
//! notes for md#2, md#3, md#4, md#5, md#6 and X4. Every refusal here is at
//! MINT; the `*_existing_card_*` rows pin that reading an already-minted card
//! of the same shape is unchanged.

#![allow(missing_docs)]

use std::process::Command;

/// Depth 4, child 2' (m/48'/0'/0'/2' of the abandon mnemonic), mainnet.
const X0: &str = "xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf";
/// X0's point with depth 3, child 0', parent fingerprint 0 and an ALL-ZERO
/// chain code: the shape a WIF slot is written in (X4), at an admitted depth.
const ZERO_CHAIN_CODE: &str = "xpub6BemYiVNp19Zz8cJwY3X663dNHj5QRjN4dgPByiKNEFvKSBotr7D79u3ESCFsmxoEhJcCAxQ15SEryxWU34nZiiCw8F3XiSZvbpACHhR7tu";

fn run(args: &[&str]) -> (i32, String, String) {
    let o = Command::new(assert_cmd::cargo::cargo_bin("md"))
        .args(args)
        .output()
        .expect("invoke md");
    (
        o.status.code().expect("normal exit"),
        String::from_utf8(o.stdout).unwrap(),
        String::from_utf8(o.stderr).unwrap(),
    )
}

fn key(i: u8, x: &str) -> String {
    format!("@{i}={x}")
}

// ─── md#2: the xpub header contradicts the declared origin ──────────────────

#[test]
fn md2_depth4_key_under_depth3_origin_refused() {
    let k = key(0, X0);
    let (code, out, err) = run(&["encode", "wpkh(@0/84'/0'/0'/<0;1>/*)", "--key", &k]);
    assert_eq!(code, 1, "stderr: {err}");
    assert_eq!(out, "");
    assert!(
        err.contains("--key @0: the extended key states depth 4, but the origin this card declares for @0 is `m/84'/0'/0'`"),
        "stderr: {err}"
    );
}

#[test]
fn md2_child_number_contradicting_last_step_refused() {
    let k = key(0, X0);
    let (code, out, err) = run(&[
        "encode",
        "wsh(sortedmulti(1,@0/48'/0'/0'/1'/<0;1>/*))",
        "--key",
        &k,
    ]);
    assert_eq!(code, 1, "stderr: {err}");
    assert_eq!(out, "");
    assert!(
        err.contains("states depth 4 and child number 2'") && err.contains("whose step 4 is 1'"),
        "stderr: {err}"
    );
}

#[test]
fn md2_path_override_is_what_is_compared() {
    let k = key(0, X0);
    let (code, _, err) = run(&[
        "encode",
        "wsh(sortedmulti(1,@0/<0;1>/*))",
        "--key",
        &k,
        "--path",
        "m/48'/0'/0'/1'",
    ]);
    assert_eq!(code, 1, "stderr: {err}");
    assert!(err.contains("whose step 4 is 1'"), "stderr: {err}");
}

#[test]
fn md2_consistent_origin_still_mints() {
    let k = key(0, X0);
    let (code, out, err) = run(&[
        "encode",
        "wsh(sortedmulti(1,@0/48'/0'/0'/2'/<0;1>/*))",
        "--key",
        &k,
    ]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(out.starts_with("md1"));
}

// ─── md#3: a placeholder with no wildcard ───────────────────────────────────

#[test]
fn md3_multipath_without_wildcard_refused_at_mint() {
    let (code, out, err) = run(&["encode", "wpkh(@0/<0;1>)", "--path", "bip84"]);
    assert_eq!(code, 1, "stderr: {err}");
    assert_eq!(out, "");
    assert!(
        err.starts_with("md: unsupported: @0 has use-site `/<0;1>` with no trailing `/*`"),
        "stderr: {err}"
    );
}

/// r1 A2 (owner ruling 2026-11-25): a placeholder with no use-site tail at
/// all (`@0`, `@0/84'/0'/0'`) is refused at mint, as the multipath case is.
#[test]
fn a2_bare_placeholder_refused_at_mint() {
    for (t, i) in [
        ("wpkh(@0)", "0"),
        ("wpkh(@0/84'/0'/0')", "0"),
        ("wsh(multi(1,@0/<0;1>/*,@1))", "1"),
    ] {
        let (code, out, err) = run(&["encode", t, "--path", "bip84"]);
        assert_eq!(code, 1, "{t}: stderr: {err}");
        assert_eq!(out, "", "{t}");
        assert_eq!(
            err,
            format!(
                "md: unsupported: @{i} has no use-site tail, which an md1 card cannot hold: the \
                 card records every key as ranged, so a card minted from this template would \
                 describe `@{i}/*`, a different wallet from the single key `@{i}` names. Write \
                 `@{i}/<0;1>/*` for a receive and change pair, or `@{i}/*` for one ranged \
                 chain.\n"
            ),
            "{t}"
        );
    }
}

/// The other minting verbs share the minting parse, so they refuse too.
#[test]
fn a2_bare_placeholder_refused_by_descriptor_verb() {
    let k = key(0, X0);
    let (code, out, err) = run(&["descriptor", "--template", "wpkh(@0)", "--key", &k]);
    assert_ne!(code, 0, "stderr: {err}");
    assert_eq!(out, "");
    assert!(err.contains("@0 has no use-site tail"), "stderr: {err}");
}

/// A card minted before the ruling (`wpkh(@0)` minted as `/*`) still decodes
/// and re-encodes to the same string.
#[test]
fn a2_existing_bare_card_still_decodes_and_reemits() {
    let card = "md1yqqqq2wy43c0uqcl29";
    let (code, out, err) = run(&["decode", card]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(out.contains("wpkh(@0/*)"), "stdout: {out}");
    let (code, again, err) = run(&["encode", "wpkh(@0/*)"]);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(again, format!("{card}\n"));
}

// ─── md#4: `c:` over a non-key child ────────────────────────────────────────

#[test]
fn md4_check_over_non_key_refused_at_mint() {
    for t in [
        "wsh(c:or_i(pk_k(@0/<0;1>/*),pk_k(@1/<0;1>/*)))",
        "wsh(c:and_v(v:pk(@0/<0;1>/*),pk_k(@1/<0;1>/*)))",
        "wsh(c:or_i(pk_k(@0/<0;1>/*),pk_h(@1/<0;1>/*)))",
    ] {
        let (code, out, err) = run(&["encode", t, "--path", "bip48"]);
        assert_eq!(code, 1, "{t}: stderr: {err}");
        assert_eq!(out, "", "{t}");
        assert!(
            err.starts_with(
                "md: unsupported: this template wraps a fragment other than a single key in `c:`"
            ),
            "{t}: stderr: {err}"
        );
    }
}

#[test]
fn md4_check_over_key_still_mints() {
    let (code, _, err) = run(&["encode", "wsh(c:pk_k(@0/<0;1>/*))", "--path", "bip48"]);
    assert_eq!(code, 0, "stderr: {err}");
}

#[test]
fn md4_existing_card_still_decodes() {
    // Minted by md before the fix (bug hunt reproduction).
    let (code, out, err) = run(&["decode", "md1ypfdsssj5qqcyvvjs495khr3xmjp6j6"]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(out.contains("c:or_i("), "stdout: {out}");
}

// ─── md#5: a hardened wildcard on a keyed card ──────────────────────────────

#[test]
fn md5_keyed_hardened_wildcard_refused_at_mint() {
    let k = key(0, X0);
    let (code, out, err) = run(&[
        "encode",
        "wsh(sortedmulti(1,@0/48'/0'/0'/2'/<0;1>/*h))",
        "--key",
        &k,
    ]);
    assert_eq!(code, 1, "stderr: {err}");
    assert_eq!(out, "");
    assert!(
        err.starts_with("md: unsupported: the shared use-site ends in a hardened wildcard"),
        "stderr: {err}"
    );
}

#[test]
fn md5_keyless_hardened_wildcard_still_mints() {
    let (code, _, err) = run(&["encode", "wpkh(@0/<0;1>/*h)", "--path", "bip84"]);
    assert_eq!(code, 0, "stderr: {err}");
}

// ─── X4: an all-zero chain code (a WIF written as an xpub) ──────────────────

#[test]
fn x4_zero_chain_code_key_refused_at_mint() {
    let k = key(0, ZERO_CHAIN_CODE);
    let (code, out, err) = run(&["encode", "wpkh(@0/84'/0'/0'/<0;1>/*)", "--key", &k]);
    assert_eq!(code, 1, "stderr: {err}");
    assert_eq!(out, "");
    assert!(
        err.starts_with("md: unsupported: the key at @0 has an all-zero chain code"),
        "stderr: {err}"
    );
}

// ─── md#6: fingerprints are privacy-sensitive ───────────────────────────────

const FP_WARNING: &str = "warning: this card will carry master-key fingerprints";

#[test]
fn md6_encode_with_fingerprint_warns() {
    let k = key(0, X0);
    let (code, _, err) = run(&[
        "encode",
        "wsh(sortedmulti(1,@0/48'/0'/0'/2'/<0;1>/*))",
        "--key",
        &k,
        "--fingerprint",
        "@0=aabbccdd",
    ]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(err.contains(FP_WARNING), "stderr: {err}");
}

#[test]
fn md6_encode_without_fingerprint_is_silent() {
    let k = key(0, X0);
    let (code, _, err) = run(&[
        "encode",
        "wsh(sortedmulti(1,@0/48'/0'/0'/2'/<0;1>/*))",
        "--key",
        &k,
    ]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(!err.contains(FP_WARNING), "stderr: {err}");
}

#[test]
fn md6_inspect_and_decode_flag_fingerprints() {
    let (_, card, _) = run(&[
        "encode",
        "wsh(sortedmulti(1,@0/48'/0'/0'/2'/<0;1>/*))",
        "--fingerprint",
        "@0=aabbccdd",
    ]);
    let card = card.trim();
    for verb in ["inspect", "decode"] {
        let (code, out, err) = run(&[verb, card]);
        assert!(code == 0 || code == 4, "{verb}: {code} {err}");
        assert!(
            out.contains("aabbccdd") || err.contains("aabbccdd"),
            "{verb}: {out} {err}"
        );
        assert!(
            err.contains("note: the fingerprints shown are privacy-sensitive"),
            "{verb} stderr: {err}"
        );
    }
}

// ─── md review r4 I-1: a tap leaf that can never be satisfied ───────────────

const NUMS: &str = "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0";

#[test]
fn r4_unsatisfiable_tap_leaf_refused_at_mint() {
    for t in [
        format!("tr({NUMS},and_v(v:pk(@0/<0;1>/*),0))"),
        format!("tr({NUMS},{{pk(@0/<0;1>/*),and_v(v:pk(@1/<0;1>/*),0)}})"),
        "tr(@0/<0;1>/*,0)".to_string(),
    ] {
        let (code, out, err) = run(&["encode", &t, "--path", "bip48"]);
        assert_eq!(code, 1, "{t}: stderr: {err}");
        assert_eq!(out, "", "{t}");
        assert!(
            err.contains("a taproot leaf can never be satisfied") && err.contains("; remove it"),
            "{t}: stderr: {err}"
        );
    }
}

/// The satisfiable control still mints.
#[test]
fn r4_satisfiable_tap_leaves_still_mint() {
    let t = format!("tr({NUMS},{{pk(@0/<0;1>/*),and_v(v:pk(@1/<0;1>/*),older(10))}})");
    let (code, _, err) = run(&["encode", &t, "--path", "bip48"]);
    assert_eq!(code, 0, "stderr: {err}");
}

// ─── md review r5 I-1: the dead-leaf mint rule must not reach read verbs ─────
//
// Both cards were minted by the md before the r4 fold (`md encode
// --experimental 'tr(NUMS,{and_v(v:pk(@0/<0;1>/*),0),older(10)})'`, the
// second with `--key @0=… --path bip86`): one dead leaf, and one leaf anyone
// can spend after 10 blocks. Reading them must still warn about the latter.

const DEAD_LEAF_KEYLESS: &str = "md1yqfdsssj5qqcr32v6238qqqqqq587m7pw4u7l9p3";
const DEAD_LEAF_WALLET: [&str; 3] = [
    "md1fm0cjpqpqpm66zzqqvpc4xd9gnsqqqqq2z4q329g4z52329g4z52sx9mzfr7vlfutr",
    "md1fm0cjpqg4z52329g4z52329g4z52329g4z52329g4z5qyn2tdngnq8g8fhdtw9j5mu",
    "md1fm0cjpqkzqev4x7j46uajq92f4zan6kcpty5yvm5c3g6wf2dqanqqazn3pfcgnlw60",
];
const NO_KEY: &str = "warning: this card can be spent with NO key when: older(10)";

#[test]
fn r5_existing_dead_leaf_card_still_warns_on_decode_bytecode_and_json() {
    for verb in [
        vec!["decode"],
        vec!["decode", "--json"],
        vec!["bytecode"],
        vec!["bytecode", "--json"],
    ] {
        let mut args = verb.clone();
        args.push(DEAD_LEAF_KEYLESS);
        let (code, _, err) = run(&args);
        assert_eq!(code, 0, "{verb:?}: stderr: {err}");
        assert!(err.contains(NO_KEY), "{verb:?}: stderr: {err}");
    }
}

#[test]
fn r5_existing_dead_leaf_wallet_card_is_not_called_watch_only() {
    for verb in [
        vec!["decode", "--json"],
        vec!["bytecode"],
        vec!["bytecode", "--json"],
    ] {
        let mut args = verb.clone();
        args.extend(DEAD_LEAF_WALLET);
        let (code, _, err) = run(&args);
        assert_eq!(code, 0, "{verb:?}: stderr: {err}");
        assert!(err.contains(NO_KEY), "{verb:?}: stderr: {err}");
        assert!(!err.contains("watch-only"), "{verb:?}: stderr: {err}");
    }
}
