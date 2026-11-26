//! Brian's decision of 2026-11-26 (bh1125-r3-i1 / bh1125-r4-i1): `md address`
//! and `md descriptor` refuse, by default, to render an EXISTING card whose
//! policy is not valid miniscript (md-codec `policy_type_checks`: untyped,
//! a non-B root or tap leaf, a tap leaf that can never be satisfied), and
//! render it only under `--allow-invalid-miniscript`, with a loud warning.
//! Reading verbs (`decode`, `inspect`, `bytecode`, `verify`) are unchanged.

#![allow(missing_docs)]

use std::process::Command;

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

/// Hand-built (the parser refuses the text): `tr(NUMS,v:pk(@0))` with a
/// pubkey. Its only leaf leaves an empty stack.
const NON_B_LEAF: [&str; 2] = [
    "md1fzsn0qs9qpcu49cqps8xjs4gy2329g4z52329g4z52329g4z52329g4z52329g4z52329gpszdz0l0uvdrg",
    "md1fzsn0qs252329gqfx5kmx3xcgr9j5m62htnkgq4fx5tk02mq9vjs3nwnz9rfe9f5rkvqqwal30ddk2rurt",
];
/// Minted by the md before the r4 fold (`md encode
/// 'tr(NUMS,and_v(v:pk(@0/<0;1>/*),0))' --key @0=… --path bip86`): no one
/// can spend it.
const DEAD_LEAF: [&str; 2] = [
    "md1ferkvqspqpm66zzqqvp6v623p2sg4z52329g4z52329g4z52329g4z52329g4z52329g4ze6y2gmx0r9xpr",
    "md1ferkvqs2329g4z52sqjdfdkdzdssxt9fh54wh8vsp2jdghv74kq2e9prxaxy2xnj2ng8vcrq7u7r9c0yhs2",
];
/// Also pre-r4 (`--experimental`): a dead leaf beside `older(10)`, which
/// anyone can spend 10 blocks after each coin confirms.
const DEAD_LEAF_AND_NO_KEY: [&str; 3] = [
    "md1fm0cjpqpqpm66zzqqvpc4xd9gnsqqqqq2z4q329g4z52329g4z52sx9mzfr7vlfutr",
    "md1fm0cjpqg4z52329g4z52329g4z52329g4z52329g4z5qyn2tdngnq8g8fhdtw9j5mu",
    "md1fm0cjpqkzqev4x7j46uajq92f4zan6kcpty5yvm5c3g6wf2dqanqqazn3pfcgnlw60",
];
const FLAG: &str = "--allow-invalid-miniscript";
const REFUSAL: &str = "this card's policy is not valid miniscript";
const WARNING: &str = "WARNING: --allow-invalid-miniscript";

fn cards() -> Vec<(&'static str, Vec<&'static str>)> {
    vec![
        ("non_b_leaf", NON_B_LEAF.to_vec()),
        ("dead_leaf", DEAD_LEAF.to_vec()),
        ("dead_leaf_and_no_key", DEAD_LEAF_AND_NO_KEY.to_vec()),
    ]
}

#[test]
fn address_and_descriptor_refuse_an_invalid_miniscript_card_by_default() {
    for verb in ["address", "descriptor"] {
        for (name, card) in cards() {
            let mut args = vec![verb];
            args.extend(&card);
            let (code, out, err) = run(&args);
            assert_eq!(code, 1, "{verb} {name}: stderr: {err}");
            assert_eq!(out, "", "{verb} {name}");
            assert!(
                err.contains(REFUSAL) && err.contains(FLAG),
                "{verb} {name}: stderr: {err}"
            );
        }
    }
}

#[test]
fn the_override_renders_it_with_a_loud_warning() {
    for verb in ["address", "descriptor"] {
        for (name, card) in cards() {
            let mut args = vec![verb, FLAG];
            args.extend(&card);
            let (code, out, err) = run(&args);
            assert_eq!(code, 0, "{verb} {name}: stderr: {err}");
            let want = if verb == "address" { "bc1p" } else { "tr(" };
            assert!(out.starts_with(want), "{verb} {name}: stdout: {out}");
            assert!(
                err.contains(WARNING)
                    && err.contains("UNSPENDABLE")
                    && err.contains("WITHOUT A SIGNATURE"),
                "{verb} {name}: stderr: {err}"
            );
        }
    }
}

/// A valid card is untouched, and the flag changes nothing for it (no
/// warning: there is nothing to override).
#[test]
fn a_valid_card_is_unaffected() {
    let x = "xpub6BmeGmRo4LosAcU21HDaGcvtaQ7GrqQcY48nBkE22qM6KVwQUjRJ1BGzk84SFVHgLcd61Vcnhr8petHexjjn5WbQ9PriVrRhphw4oCp2z6a";
    let key = format!("@0={x}");
    let (code, out, err) = run(&[
        "encode",
        "tr(@0/<0;1>/*)",
        "--key",
        &key,
        "--path",
        "bip86",
        "--group-size",
        "0",
    ]);
    assert_eq!(code, 0, "stderr: {err}");
    let card: Vec<&str> = out.split_whitespace().collect();
    for flag in [None, Some(FLAG)] {
        let mut args = vec!["address"];
        args.extend(flag);
        args.extend(&card);
        let (code, out, err) = run(&args);
        assert_eq!(code, 0, "{flag:?}: stderr: {err}");
        assert!(out.starts_with("bc1p"), "{flag:?}");
        assert!(!err.contains(WARNING), "{flag:?}: stderr: {err}");
    }
}

/// The reading verbs still read such a card.
#[test]
fn reading_verbs_are_unchanged() {
    for verb in ["decode", "inspect", "bytecode"] {
        for (name, card) in cards() {
            let mut args = vec![verb];
            args.extend(&card);
            let (code, _, err) = run(&args);
            assert_eq!(code, 0, "{verb} {name}: stderr: {err}");
            assert!(!err.contains(REFUSAL), "{verb} {name}: stderr: {err}");
        }
    }
}
