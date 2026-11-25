//! bughunt-1119 `bh-md-M5` and `bh-md-I8` (`design/FIX_bh1119_md_output_advisories.md`): a keyed
//! card is labelled watch-only on `md decode --json` and `md bytecode`; the `--experimental`
//! no-key warning names what opens the path. Rows are named as in the FIX section 4. Every row
//! passes `--network signet` where it mints with a key or a network matters.
//!
//! Fixture provenance: `KEYLESS`, `CARD_AFTER`, `CARD_SHA` and `CARD_ONE` are what the `md encode`
//! commands below print on the base (the md1 string is a function of the template alone); each
//! test that uses one mints it again and asserts it reproduces. `KEYED` is minted inside each
//! test (the chunk-set id is random, so it is not a constant).

#![allow(missing_docs)]

use assert_cmd::Command;

const KA: &str = "tpubDC8msFGeGuwnKG9Upg7DM2b4DaRqg3CUZa5g8v2SRQ6K4NSkxUgd7HsL2XVWbVm39yBA4LAxysQAm397zwQSQoQgewGiYZqrA9DsP4zbQ1M";
const H: &str = "6c60f404f8167a38fc70eaf8aa17ac351023bef86bcb9d1086a19afe95bd5333";
const KEYLESS: &str = "md1yq802gggqpsqwgtua24e7ssf3";
const CARD_AFTER: &str = "md1yq802gggqpsfj2dsqqqqvsf2pjymaa6veav";
const CARD_SHA: &str =
    "md1yq802gggqpsfs2wkcc85qnupv73cl3cw4792z7kr2yprhmuxhjuazzr2rxh7jk74xvc20rflk6gdqmus";
const CARD_ONE: &str = "md1yq802gggqpsfs23s7wv9rxmal4u8q";

const WATCH: &str = "note: stdout is watch-only \u{2014} public keys only, cannot spend";
const TMPL: &str = "note: stdout is a keyless descriptor template (no keys)";

const S_T: &str = " A path there needs no secret at all: once its timelock condition is met, anyone who has this policy and its public keys can spend it.";
const S_H: &str = " Whoever learns a preimage named there can spend that path without a key once any timelock beside it is met. If such a preimage is engraved, THE PLATE IS BEARER ACCESS.";
const TAIL_E: &str =
    " Malleability, resource limits, repeated keys and timelock mixing were still checked.";
const TAIL_D: &str = " It was minted with --experimental; the card carries no record of that, so this is derived from the policy itself.";
const LOCK_AFTER_100: &str = " after(100) is block height 100, not 100 blocks from now; md cannot see the chain, so if the chain is already past height 100, that path is open today.";
const LOCK_OLDER_144: &str = " older(144) is 144 blocks after each coin confirms.";
const LOCK_OLDER_512S: &str = " older(4194305) is 512 seconds after each coin confirms.";
const HEAD_E: &str =
    "warning: --experimental relaxed the signature rule. This descriptor can be spent with NO key";
const HEAD_D: &str = "warning: this card can be spent with NO key";
const Z_BODY: &str = ": anyone who has this policy and its public keys can spend it now.";

fn md(args: &[&str]) -> (String, String, i32) {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(args)
        .output()
        .unwrap();
    (
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
        out.status.code().unwrap(),
    )
}

fn lines(s: &str) -> Vec<&str> {
    s.lines().collect()
}

fn mint_keyed() -> Vec<String> {
    let (o, e, c) = md(&[
        "encode",
        "wpkh(@0/<0;1>/*)",
        "--network",
        "signet",
        "--path",
        "bip84",
        "--key",
        &format!("@0={KA}"),
    ]);
    assert_eq!(c, 0, "{e}");
    let v: Vec<String> = o.lines().map(str::to_string).collect();
    assert_eq!(v.len(), 2, "the keyed card is two chunks: {o}");
    v
}

fn keyed_args<'a>(prefix: &[&'a str], chunks: &'a [String]) -> Vec<&'a str> {
    let mut v = prefix.to_vec();
    v.extend(chunks.iter().map(String::as_str));
    v
}

/// Mints `template` with `--experimental` and returns (stdout, stderr).
fn mint_exp(template: &str) -> (String, String, i32) {
    md(&[
        "encode",
        template,
        "--path",
        "bip84",
        "--network",
        "signet",
        "--experimental",
    ])
}

fn tmpl_after() -> &'static str {
    "wsh(or_i(pk(@0/**),after(100)))"
}

fn assert_mint(template: &str, expect: &str) {
    let (o, e, c) = mint_exp(template);
    assert_eq!(c, 0, "{e}");
    assert_eq!(o, format!("{expect}\n"), "fixture mint drifted");
}

fn e_line(k: &str, pieces: &str) -> String {
    format!("{HEAD_E} when: {k}.{pieces}{TAIL_E}")
}

fn d_line(k: &str, pieces: &str) -> String {
    format!("{HEAD_D} when: {k}.{pieces}{TAIL_D}")
}

fn z_e() -> String {
    format!("{HEAD_E}, NO secret and NO timelock{Z_BODY}{TAIL_E}")
}

fn z_d() -> String {
    format!("{HEAD_D}, NO secret and NO timelock{Z_BODY}{TAIL_D}")
}

// ---------------------------------------------------------------- bh-md-M5

#[test]
fn m5_r1_decode_json_keyed_card_is_watch_only() {
    let k = mint_keyed();
    let (o, e, c) = md(&keyed_args(&["decode", "--json"], &k));
    assert_eq!(c, 0, "{e}");
    let v: serde_json::Value = serde_json::from_str(&o).unwrap();
    assert_eq!(
        v["descriptor"]["tlv"]["pubkeys"].as_array().unwrap().len(),
        1
    );
    assert_eq!(e, format!("{WATCH}\n"));
}

#[test]
fn m5_r2_bytecode_text_keyed_card_is_watch_only() {
    let k = mint_keyed();
    let (o, e, c) = md(&keyed_args(&["bytecode"], &k));
    assert_eq!(c, 0, "{e}");
    let l = lines(&o);
    assert_eq!(l.len(), 3, "{o}");
    assert!(l[0].starts_with("payload-bits:"), "{o}");
    assert!(l[1].starts_with("payload-bytes:"), "{o}");
    assert!(l[2].starts_with("hex:"), "{o}");
    assert_eq!(e, format!("{WATCH}\n"));
}

#[test]
fn m5_r3_bytecode_json_keyed_card_is_watch_only() {
    let k = mint_keyed();
    let (o, e, c) = md(&keyed_args(&["bytecode", "--json"], &k));
    assert_eq!(c, 0, "{e}");
    let v: serde_json::Value = serde_json::from_str(&o).unwrap();
    assert!(v.get("hex").is_some(), "{o}");
    assert_eq!(e, format!("{WATCH}\n"));
}

#[test]
fn m5_c1_decode_text_keyed_card_stays_template() {
    let k = mint_keyed();
    let (o, e, c) = md(&keyed_args(&["decode"], &k));
    assert_eq!(c, 0, "{e}");
    assert_eq!(o, "wpkh(@0/<0;1>/*)\n");
    assert_eq!(e, format!("{TMPL}\n"));
}

#[test]
fn m5_c2_keyless_card_stays_template() {
    for args in [
        vec!["decode", "--json", KEYLESS],
        vec!["bytecode", KEYLESS],
        vec!["bytecode", "--json", KEYLESS],
    ] {
        let (_, e, c) = md(&args);
        assert_eq!(c, 0, "{args:?}: {e}");
        assert_eq!(e, format!("{TMPL}\n"), "{args:?}");
    }
}

#[test]
fn m5_r4_keyed_keyless_card_never_says_cannot_spend() {
    let (o, e, c) = md(&[
        "encode",
        "wsh(or_d(pk(@0/<0;1>/*),1))",
        "--network",
        "signet",
        "--path",
        "bip84",
        "--key",
        &format!("@0={KA}"),
        "--experimental",
    ]);
    assert_eq!(c, 0, "{e}");
    let chunks: Vec<String> = o.lines().map(str::to_string).collect();
    for prefix in [
        vec!["decode", "--json"],
        vec!["bytecode"],
        vec!["bytecode", "--json"],
    ] {
        let (_, e, c) = md(&keyed_args(&prefix, &chunks));
        assert_eq!(c, 0, "{prefix:?}: {e}");
        assert_eq!(e, format!("{}\n", z_d()), "{prefix:?}");
        assert!(!e.contains("cannot spend"), "{prefix:?}: {e}");
    }
}

// ---------------------------------------------------------------- bh-md-I8

#[test]
fn i8_r1_encode_absolute_timelock_path_names_the_lock() {
    let (o, e, c) = mint_exp(tmpl_after());
    assert_eq!(c, 0, "{e}");
    assert_eq!(o, format!("{CARD_AFTER}\n"));
    let l = lines(&e);
    assert_eq!(
        l[0],
        "warning: --experimental relaxed the signature rule. This descriptor can be spent with NO key when: after(100). A path there needs no secret at all: once its timelock condition is met, anyone who has this policy and its public keys can spend it. after(100) is block height 100, not 100 blocks from now; md cannot see the chain, so if the chain is already past height 100, that path is open today. Malleability, resource limits, repeated keys and timelock mixing were still checked."
    );
    assert_eq!(
        l[0],
        e_line("after(100)", &format!("{S_T}{LOCK_AFTER_100}"))
    );
    assert_eq!(l[1], TMPL);
    assert!(!e.contains("preimage") && !e.contains("BEARER"), "{e}");
}

#[test]
fn i8_r2_encode_relative_timelock_path_names_the_lock() {
    let (_, e, c) = mint_exp("wsh(or_d(pk(@0/**),older(144)))");
    assert_eq!(c, 0, "{e}");
    assert_eq!(
        lines(&e)[0],
        e_line("older(144)", &format!("{S_T}{LOCK_OLDER_144}"))
    );
}

#[test]
fn i8_r3_encode_hashlock_path_names_the_hash() {
    let t = format!("wsh(or_d(pk(@0/**),sha256({H})))");
    let (o, e, c) = mint_exp(&t);
    assert_eq!(c, 0, "{e}");
    assert_eq!(o, format!("{CARD_SHA}\n"));
    assert_eq!(lines(&e)[0], e_line(&format!("sha256({H})"), S_H));
}

#[test]
fn i8_r4_encode_no_condition_path_says_no_secret_no_timelock() {
    let t = "wsh(or_d(pk(@0/**),1))";
    let (o, e, c) = mint_exp(t);
    assert_eq!(c, 0, "{e}");
    assert_eq!(o, format!("{CARD_ONE}\n"));
    assert_eq!(lines(&e)[0], z_e());
}

#[test]
fn i8_r5_encode_hash_or_timelock_names_both() {
    let t = format!("tr(@0/**,{{sha256({H}),after(100)}})");
    let (_, e, c) = mint_exp(&t);
    assert_eq!(c, 0, "{e}");
    assert_eq!(
        lines(&e)[0],
        e_line(
            &format!("or(sha256({H}),after(100))"),
            &format!("{S_T}{S_H}{LOCK_AFTER_100}")
        )
    );
}

#[test]
fn i8_r6_encode_json_prints_the_same_warning() {
    let (o, e, c) = md(&[
        "encode",
        "--json",
        tmpl_after(),
        "--path",
        "bip84",
        "--network",
        "signet",
        "--experimental",
    ]);
    assert_eq!(c, 0, "{e}");
    let v: serde_json::Value = serde_json::from_str(&o).unwrap();
    assert_eq!(v["phrase"], CARD_AFTER);
    assert_eq!(
        lines(&e)[0],
        e_line("after(100)", &format!("{S_T}{LOCK_AFTER_100}"))
    );
}

#[test]
fn i8_r7_decode_text_timelock_card_names_the_lock() {
    assert_mint(tmpl_after(), CARD_AFTER);
    let (o, e, c) = md(&["decode", CARD_AFTER]);
    assert_eq!(c, 0, "{e}");
    assert_eq!(o, "wsh(or_i(pk(@0/<0;1>/*),after(100)))\n");
    let l = lines(&e);
    assert_eq!(
        l[0],
        "warning: this card can be spent with NO key when: after(100). A path there needs no secret at all: once its timelock condition is met, anyone who has this policy and its public keys can spend it. after(100) is block height 100, not 100 blocks from now; md cannot see the chain, so if the chain is already past height 100, that path is open today. It was minted with --experimental; the card carries no record of that, so this is derived from the policy itself."
    );
    assert_eq!(
        l[0],
        d_line("after(100)", &format!("{S_T}{LOCK_AFTER_100}"))
    );
    assert_eq!(*l.last().unwrap(), TMPL);
}

#[test]
fn i8_r8_decode_json_timelock_card_warns() {
    assert_mint(tmpl_after(), CARD_AFTER);
    let (o, e, c) = md(&["decode", "--json", CARD_AFTER]);
    assert_eq!(c, 0, "{e}");
    let v: serde_json::Value = serde_json::from_str(&o).unwrap();
    assert_eq!(v["schema"], "md-cli/2");
    assert_eq!(
        e,
        format!(
            "{}\n{TMPL}\n",
            d_line("after(100)", &format!("{S_T}{LOCK_AFTER_100}"))
        )
    );
}

#[test]
fn i8_r9_decode_json_hashlock_card_warns() {
    assert_mint(&format!("wsh(or_d(pk(@0/**),sha256({H})))"), CARD_SHA);
    let (_, e, c) = md(&["decode", "--json", CARD_SHA]);
    assert_eq!(c, 0, "{e}");
    assert_eq!(
        e,
        format!("{}\n{TMPL}\n", d_line(&format!("sha256({H})"), S_H))
    );
}

#[test]
fn i8_r10_decode_no_condition_card_text_and_json() {
    assert_mint("wsh(or_d(pk(@0/**),1))", CARD_ONE);
    let want = "warning: this card can be spent with NO key, NO secret and NO timelock: anyone who has this policy and its public keys can spend it now. It was minted with --experimental; the card carries no record of that, so this is derived from the policy itself.";
    assert_eq!(z_d(), want);
    let (_, e, c) = md(&["decode", CARD_ONE]);
    assert_eq!(c, 0, "{e}");
    assert_eq!(lines(&e)[0], want);
    let (_, e, c) = md(&["decode", "--json", CARD_ONE]);
    assert_eq!(c, 0, "{e}");
    assert_eq!(e, format!("{want}\n{TMPL}\n"));
}

#[test]
fn i8_r11_encode_help_names_timelock_paths() {
    let (o, _, c) = md(&["encode", "--help"]);
    assert_eq!(c, 0);
    let flat = o.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        flat.contains("or a timelock alone (once it is met, anyone who has the policy and its public keys can spend that path)"),
        "{o}"
    );
    assert!(
        !flat.contains("Whoever learns the preimage of a keyless path"),
        "{o}"
    );
}

#[test]
fn i8_r12_refusal_hint_names_no_preimage() {
    let (_, e, c) = md(&[
        "encode",
        tmpl_after(),
        "--path",
        "bip84",
        "--network",
        "signet",
    ]);
    assert_eq!(c, 1, "{e}");
    assert!(e.contains("a spend path that needs no key"), "{e}");
    let low = e.to_lowercase();
    assert!(
        !low.contains("whoever holds the preimage") && !low.contains("bearer"),
        "{e}"
    );
}

#[test]
fn i8_r13_relative_time_lock_in_seconds() {
    let (_, e, c) = mint_exp("wsh(or_d(pk(@0/**),older(4194305)))");
    assert_eq!(c, 0, "{e}");
    assert!(lines(&e)[0].contains(LOCK_OLDER_512S), "{e}");
}

#[test]
fn i8_r14_hash_and_timelock_path_says_no_secret_is_not_free() {
    let t = format!("wsh(or_d(pk(@0/**),and_v(v:sha256({H}),after(100))))");
    let (_, e, c) = mint_exp(&t);
    assert_eq!(c, 0, "{e}");
    let l0 = lines(&e)[0];
    assert_eq!(
        l0,
        e_line(
            &format!("and(sha256({H}),after(100))"),
            &format!("{S_H}{LOCK_AFTER_100}")
        )
    );
    assert!(!l0.contains("needs no secret"), "{l0}");
}

#[test]
fn i8_r15_hash_only_on_a_keyed_path_is_not_bearer() {
    let t = format!("wsh(or_i(and_v(v:pk(@0/**),sha256({H})),after(100)))");
    let (_, e, c) = mint_exp(&t);
    assert_eq!(c, 0, "{e}");
    assert_eq!(
        lines(&e)[0],
        e_line("after(100)", &format!("{S_T}{LOCK_AFTER_100}"))
    );
    assert!(!e.contains("preimage") && !e.contains("BEARER"), "{e}");
}

#[test]
fn i8_c1_encode_not_needed_note_unchanged() {
    let (_, e, c) = mint_exp("wsh(and_v(v:pk(@0/**),after(100)))");
    assert_eq!(c, 0, "{e}");
    assert_eq!(
        lines(&e)[0],
        "note: --experimental was not needed for this descriptor. Every spend path here requires a signature, so the flag relaxed nothing and this card is not bearer access. Re-run without it to get the same plate under the unrelaxed rules."
    );
    assert!(!e.contains("warning:"), "{e}");
}

#[test]
fn i8_c2_decode_json_ordinary_card_has_no_warning() {
    let (_, e, c) = md(&["decode", "--json", KEYLESS]);
    assert_eq!(c, 0, "{e}");
    assert_eq!(e, format!("{TMPL}\n"));
}
