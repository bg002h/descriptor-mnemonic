//! Coordinator-compat plan 1b, the host half (design §4, "`md` on the host"):
//! `md shape-key`, and the verdict notice on `md compose` and `md descriptor`.
//!
//! Every test names the mutation that reds it.

#![allow(missing_docs)]

use std::collections::BTreeSet;
use std::process::Command as StdCommand;

use md_codec::chunk::split;
use md_codec::descriptor_route::{descriptor_from_text, skeleton_key_of_text};

/// `(stdout, stderr, exit code)`; the same helper `cli_compose_unspendable.rs`
/// and `liana_input_side.rs` define.
fn md(args: &[&str]) -> (String, String, i32) {
    let out = StdCommand::new(assert_cmd::cargo::cargo_bin("md"))
        .args(args)
        .output()
        .expect("invoke md");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().expect("md exited normally"),
    )
}

fn evidence() -> Vec<serde_json::Value> {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../md-codec/tests/fixtures/coordinator/evidence.jsonl"
    );
    std::fs::read_to_string(p)
        .unwrap_or_else(|e| panic!("{p}: {e}"))
        .lines()
        .map(|l| serde_json::from_str(l).expect("row parses"))
        .collect()
}

fn descriptor_named(name: &str) -> String {
    evidence()
        .into_iter()
        .find(|r| r["name"] == name)
        .unwrap_or_else(|| panic!("no evidence row named {name}"))["descriptor"]
        .as_str()
        .unwrap()
        .to_string()
}

/// The md1 chunks of a keyed card, minted by the library from a descriptor.
fn card_of(descriptor: &str) -> Vec<String> {
    split(&descriptor_from_text(descriptor).unwrap()).unwrap()
}

/// Design §5 step 1: "`md shape-key` is a thin CLI over the same library
/// function, and a test asserts the two agree" -- over every distinct
/// keyable evidence descriptor, i.e. every key the table was built from.
/// Mutation: print the key with its U+001F separators replaced by spaces ->
/// every row reds.
#[test]
fn shape_key_agrees_with_the_library_on_every_evidence_descriptor() {
    let descriptors: BTreeSet<String> = evidence()
        .into_iter()
        .filter(|r| r.get("unkeyable").is_none())
        .map(|r| r["descriptor"].as_str().unwrap().to_string())
        .collect();
    assert!(descriptors.len() >= 73, "{} descriptors", descriptors.len());
    for d in &descriptors {
        let (out, err, code) = md(&["shape-key", "--descriptor", d]);
        assert_eq!(code, 0, "{err}");
        let want = skeleton_key_of_text(d).unwrap();
        assert_eq!(out.strip_suffix('\n'), Some(want.as_str()), "{d}");
    }
}

/// A card and its descriptor are one key. Mutation: key the card through
/// `DecodeOpts::partial()` with its TLV dropped -> the partitions render
/// empty and this reds.
#[test]
fn shape_key_of_a_card_equals_the_key_of_its_descriptor() {
    for name in [
        "CONTROL-accept-kofn-recovery-flat",
        "X24-wsh-2of3-1of1-unlocked-plus-rec",
    ] {
        let d = descriptor_named(name);
        let card = card_of(&d);
        let mut args = vec!["shape-key"];
        args.extend(card.iter().map(String::as_str));
        let (from_card, err, code) = md(&args);
        assert_eq!(code, 0, "{name}: {err}");
        let (from_text, _, _) = md(&["shape-key", "--descriptor", &d]);
        assert_eq!(from_card, from_text, "{name}");
    }
}

/// A single-chain descriptor cannot be keyed: the key keeps `<0;1>`.
/// Mutation: accept single-chain text by guessing `<0;1>` -> exit 0, red.
#[test]
fn shape_key_refuses_a_single_chain_descriptor() {
    let d = descriptor_named("X24-wsh-2of3-1of1-unlocked-plus-rec").replace("<0;1>", "0");
    let d = d.split('#').next().unwrap().to_string();
    let (out, err, code) = md(&["shape-key", "--descriptor", &d]);
    assert_ne!(code, 0, "{out}");
    assert!(err.contains("multipath"), "{err}");
}

// ---------------------------------------------------------------------------
// The verdict on `md compose` and `md descriptor` (Task 8).
// ---------------------------------------------------------------------------

const H: &str = "a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8";

/// F-644's md-cli half, re-owned by plan 1b: each shape it names now carries
/// Liana's refusal, keyed on the composed shape, with or without the flag.
/// Mutation: remove the `verdict::notice` call from `compose::run` -> red.
#[test]
fn compose_names_liana_refusal_for_every_f644_shape() {
    for spelling in [
        &["--path", "2of3,unsorted", "--path", "2of2"][..],
        &["--path", "2of2", "--path", "1of1,after=800000"][..],
        &[
            "--path",
            "2of2",
            "--path",
            "1of1,older=100",
            "--path",
            "1of1,older=100",
        ][..],
        &["--path", "2of3,unsorted", "--experimental"][..],
    ] {
        for flag in [&["--unspendable", "liana"][..], &[][..]] {
            let mut args = vec!["compose", "--wrapper", "tr"];
            args.extend_from_slice(spelling);
            args.extend_from_slice(flag);
            let (out, err, code) = md(&args);
            assert_eq!(code, 0, "{args:?}: {err}");
            assert!(!out.is_empty(), "{args:?}");
            assert!(err.contains("Liana 8.0-15.0: refuses ("), "{args:?}: {err}");
        }
    }
}

/// A template never claims an import (design §1 (a2)); Core's structural
/// refusal before 26.0 still prints. Mutation: drop the `KeysAbsent` return
/// in `md_codec::coordinator::at_version` -> the lines change and this reds.
#[test]
fn compose_prints_refusals_and_silences_never_imports() {
    let (_, err, code) = md(&[
        "compose",
        "--wrapper",
        "tr",
        "--preset",
        "kofn-recovery,2of3,older=26280",
    ]);
    assert_eq!(code, 0, "{err}");
    assert!(
        err.contains("Bitcoin Core 24.2-25.2: refuses (miniscript under tr)"),
        "{err}"
    );
    assert!(
        err.contains("Bitcoin Core 26.0-31.1: unproven (a template has no keys"),
        "{err}"
    );
    assert!(!err.contains(": imports"), "{err}");
}

/// Ruling 2 and design §4: compose refuses the none case, exits non-zero,
/// names `--md-only`, and emits nothing; `--md-only` proceeds. Mutation:
/// drop `&& !md_only` from the refusal -> the second half reds.
#[test]
fn compose_refuses_the_none_case_and_md_only_proceeds() {
    let keyless = format!("keyless,sha256={H}");
    let base = [
        "compose",
        "--wrapper",
        "wsh",
        "--path",
        "2of3",
        "--path",
        &keyless,
        "--experimental",
    ];
    let (out, err, code) = md(&base);
    assert_eq!(code, 1, "{err}");
    assert!(out.is_empty(), "a refusal printed a template: {out}");
    assert!(err.contains("--md-only"), "{err}");

    let mut args = base.to_vec();
    args.push("--md-only");
    let (out, err, code) = md(&args);
    assert_eq!(code, 0, "{err}");
    assert!(out.starts_with("wsh("), "{out}");
}

/// `md descriptor` READS a card, so it never refuses on a verdict -- not
/// even the none case -- and it names the spelling it printed. Mutation:
/// route `md descriptor` through compose's none-case refusal -> the keyless
/// card exits 1 and this reds.
#[test]
fn descriptor_names_the_form_and_never_refuses() {
    let card = card_of(&descriptor_named("CONTROL-accept-kofn-recovery-flat"));
    let mut args = vec!["descriptor"];
    args.extend(card.iter().map(String::as_str));
    let (_, err, code) = md(&args);
    assert_eq!(code, 0, "{err}");
    assert!(err.contains("multipath form"), "{err}");
    assert!(
        err.contains("Bitcoin Core 24.2-28.4: refuses (the <0;1> multipath spelling"),
        "{err}"
    );
    args.extend(["--chain", "0"]);
    let (_, err, code) = md(&args);
    assert_eq!(code, 0, "{err}");
    assert!(
        err.contains("Bitcoin Core 26.0-31.1: imports the chain0 form"),
        "{err}"
    );

    let card = card_of(&descriptor_named("keyless-hash-path-wsh"));
    let mut args = vec!["descriptor", "--experimental"];
    args.extend(card.iter().map(String::as_str));
    let (out, err, code) = md(&args);
    assert_eq!(code, 0, "{err}");
    assert!(out.starts_with("wsh("), "{out}");
    assert!(
        err.contains("Liana 8.0-15.0: refuses (a path with no key)"),
        "{err}"
    );
}

/// F-674 item 3, as the user sees it: `md descriptor` on one of the composer
/// tr wallets the Core e2e run imported (agent-reports/e2e-live-site-wallets.md,
/// "Date 2026-09-23", mainnet) dates the Core import 2026-09-23, the day it
/// was measured in local time. It printed 2026-09-24, the UTC date of the
/// engrave commit that recorded it.
/// Mutation: vendor with the old git-blame rule -> this reds.
#[test]
fn descriptor_dates_the_core_import_the_day_it_was_measured() {
    let card = card_of(&descriptor_named("kofn-nums"));
    let mut args = vec!["descriptor"];
    args.extend(card.iter().map(String::as_str));
    let (_, err, code) = md(&args);
    assert_eq!(code, 0, "{err}");
    let core = err
        .lines()
        .find(|l| l.contains("Bitcoin Core") && l.contains("imports the multipath form"))
        .unwrap_or_else(|| panic!("no Core multipath import line: {err}"));
    assert!(core.ends_with("(measured 2026-09-23)"), "{core}");
    assert!(!err.contains("2026-09-24"), "{err}");
}

// ---------------------------------------------------------------------------
// F-677: `md shape-key --descriptor` on a WALLET's export.
// ---------------------------------------------------------------------------

/// Liana's own re-render of the composer's `preset-simple-timelocked-
/// inheritance-wsh` wallet (mnemonic-engrave
/// design/evidence/composer-fable-r0/fable-liana-parse-out.jsonl, `liana_desc`
/// of the `ok` rows), in the two variants the harness fed it: `md` (the xpubs
/// `md descriptor` prints, parent fingerprint `00000000`) and `real-xpub` (the
/// same keys as a wallet exports them, real parent fingerprints `0x1cf29716`
/// and `0xee71f8c5`). Same chain codes, same points: one wallet.
const LIANA_WSH_MD: &str = "wsh(or_i(pkh([73c5da0a/48'/0'/0'/2']xpub6DXuQW1Q2JpZxsEnFKrPvDuiRMmQgU4fzHU1wsvM5EqgGAWRJ3cmwbtS8u1HQjrEHg3YFb7XGnFovPydJ8qpaGNNd2hSEPoheWd27EABdGH/<0;1>/*),and_v(v:pkh([3f635a63/48'/0'/0'/2']xpub6DXuQW1Q2JpZwZhyeFyRwoVcxxRQUvWjfWf5X5tre7aRCMTYwNR1DnwZAehowmtGsB2oEka2aWofzRgVnexutt2KVBZfRcPtuxS6JYwywD5/<0;1>/*),older(26280))))#mxj7j54n";
const LIANA_WSH_WALLET: &str = "wsh(or_i(pkh([73c5da0a/48'/0'/0'/2']xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf/<0;1>/*),and_v(v:pkh([3f635a63/48'/0'/0'/2']xpub6FHZCoNb3tg3o1GAJQxSwgFNF8mLRtTk2GgkF7n5rwzoxBhUEdFWa8cyZRHqytAzKZWsKz8627cQEMCCfR5GDSv6yXegqirpgDUX41Pxybr/<0;1>/*),older(26280))))#0gya4r2c";
/// The same pair for `X09-tr-1of1-2of3older` (tr, a multi_a leaf).
const LIANA_TR_MD: &str = "tr([73c5da0a/48'/0'/0'/3']xpub6DXuQW1Q2JpZyweiMewTZuMPvjG8hKhV2qoF6wL9VFxsMBExtbfqAAoR4oMG4GyxFzVdfas1v2eAdfLxyjc4Ceo5B6w6zTpf7F2BuXCJ52i/<0;1>/*,and_v(v:multi_a(2,[3f635a63/48'/0'/1'/3']xpub6DXuQW1Q2Jpa1UavR7Se7NgKcQRQXyxqRjsqLybQB5JiYZcfLV93enNpsEdDHHWMjqiRqht5LHedAHPGsT3qLegcbtdn5f7nWaXPPMS4ERE/<0;1>/*,[66d455ea/48'/0'/2'/3']xpub6DXuQW1Q2JpZzLpJ3h1hHgFWxTCpQ6suGvKzYrSZgvhnsh1TyH4xKA3Mf3pXEmh8Wb94GJMj7CHvAcTpJLjG8MpsVfojpdjumU6FcKW5gFK/<0;1>/*,[73c5da0a/48'/0'/3'/3']xpub6DXuQW1Q2Jpa17KCHfNUuv5BiEM3eYamrmGaFCycMLvntLaosd1Jkd9XWgw7WFnhYc31z3G4aPnGT6W7tzcXfvzLKt8UWsTLq3X2XFBGeeF/<0;1>/*),older(26280)))#udsskqh4";
const LIANA_TR_WALLET: &str = "tr([73c5da0a/48'/0'/0'/3']xpub6DkFAXWQ2dHxr7LX1ByDVebj6u3C5KSKTVXWkiVKb3tdYfh9t7FhXzvUVSxNSikoVTRb2bGjvYoW8PqYBReMeswi3megtqDwRCeVs3vxMeH/<0;1>/*,and_v(v:multi_a(2,[3f635a63/48'/0'/1'/3']xpub6EM8uMGUZMyVTMTbYY2XpJCM56KMvje5p5UC8U5qhRZvXWrAmRoHFwJJHpGtJbiFMhMPxMTiMfMF35TDvCkHYMpgXhnY9cdk1G4SWFqQT1g/<0;1>/*,[66d455ea/48'/0'/2'/3']xpub6DcNBiPgXULnCqCYT2PVqc9PPpkomu8w2goEGoP4hMgeScHnognE9twZH9zGAQbEajtDxSkQu8LyehuZXX9sTh6Ze5mSrDSBaLETy38DwA1/<0;1>/*,[73c5da0a/48'/0'/3'/3']xpub6E6Z3Ss5TXJYQKLeD76XTFYJXyVQzT5FBKY3a7evG61SuqJKBVF2EqzMWydzSEbhyj4ESvnBLpdL8Pde5sSUNL9Y9d6mY214mwuvbspUMK5/<0;1>/*),older(26280)))#fhc8ztmz";

fn shape_key_of_card(card: &[String]) -> String {
    let mut args = vec!["shape-key"];
    args.extend(card.iter().map(String::as_str));
    let (out, err, code) = md(&args);
    assert_eq!(code, 0, "{err}");
    out
}

/// F-677: a wallet's export keys exactly like `md descriptor`'s rendering of
/// the same wallet, and like the card minted from it. It was refused:
/// "the reconstruction does not round-trip chain 0", because the self-check
/// demanded the xpub's parent fingerprint byte-for-byte and a card cannot
/// carry one (F-611). Mutation: compare `got != want` again in
/// `descriptor_from_chains` -> the wallet exports exit 1 and this reds.
#[test]
fn shape_key_keys_a_wallet_export_like_its_card() {
    for (md_form, wallet) in [
        (LIANA_WSH_MD, LIANA_WSH_WALLET),
        (LIANA_TR_MD, LIANA_TR_WALLET),
    ] {
        let (from_md, err, code) = md(&["shape-key", "--descriptor", md_form]);
        assert_eq!(code, 0, "{err}");
        let (from_wallet, err, code) = md(&["shape-key", "--descriptor", wallet]);
        assert_eq!(code, 0, "{wallet}: {err}");
        assert_eq!(from_wallet, from_md, "{wallet}");
        assert_eq!(shape_key_of_card(&card_of(wallet)), from_wallet, "{wallet}");
    }
}

/// F-677's round trip: card -> `md descriptor` -> `md shape-key --descriptor`
/// is the key `md shape-key` prints for the card itself, over every keyable
/// evidence descriptor and both wallet exports above. Mutation: compare
/// `got != want` again in `descriptor_from_chains` -> `card_of` panics on the
/// wallet exports and this reds.
#[test]
fn md_descriptor_round_trips_through_shape_key() {
    let mut descriptors: BTreeSet<String> = evidence()
        .into_iter()
        .filter(|r| r.get("unkeyable").is_none())
        .map(|r| r["descriptor"].as_str().unwrap().to_string())
        .collect();
    descriptors.extend([LIANA_WSH_WALLET, LIANA_TR_WALLET].map(String::from));
    for d in &descriptors {
        let card = card_of(d);
        let from_card = shape_key_of_card(&card);
        let mut args = vec!["descriptor"];
        args.extend(card.iter().map(String::as_str));
        let (text, err, code) = md(&args);
        assert_eq!(code, 0, "{d}: {err}");
        let (from_text, err, code) = md(&["shape-key", "--descriptor", text.trim_end()]);
        assert_eq!(code, 0, "{d}: {err}");
        assert_eq!(from_text, from_card, "{d}");
    }
}

/// The relaxation is the parent fingerprint and nothing else: the SAME
/// wallet export with one xpub's DEPTH changed (a field the origin
/// determines, and which is still compared) is refused. Mutation: normalize
/// the whole xpub metadata (depth and child number too) -> exit 0, red.
#[test]
fn shape_key_still_refuses_an_xpub_whose_depth_disagrees_with_its_origin() {
    use std::str::FromStr as _;
    let real = "xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf";
    let mut x = bitcoin::bip32::Xpub::from_str(real).unwrap();
    x.depth = 3;
    let body = LIANA_WSH_WALLET
        .split('#')
        .next()
        .unwrap()
        .replace(real, &x.to_string());
    let (out, err, code) = md(&["shape-key", "--descriptor", &body]);
    assert_ne!(code, 0, "{out}");
    assert!(err.contains("does not round-trip"), "{err}");
}
