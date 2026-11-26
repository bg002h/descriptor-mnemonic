//! Kills for the md-cli baseline GAP-B mutation survivors (tests only).
//! Each test names the survivor(s) it kills in its doc comment. The private
//! items (`resolve_placeholders`, `partition`, `comparison_form`) are killed by
//! unit tests inside `src/` (see the commit).

#![allow(missing_docs)]

#[path = "liana_cases.rs"]
#[allow(dead_code)]
mod liana_cases;
use liana_cases::{case, kofn_with_internal_key};

use md_codec::identity::{
    compute_md1_encoding_id, compute_wallet_descriptor_template_id, compute_wallet_policy_id,
};
use std::process::Command as StdCommand;

/// Run `md` with `args`: `(stdout, stderr, exit code)`.
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

fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or("").to_string()
}

fn hex(b: &[u8]) -> String {
    use std::fmt::Write;
    b.iter().fold(String::new(), |mut a, x| {
        write!(a, "{x:02x}").unwrap();
        a
    })
}

// ─── compose.rs ─────────────────────────────────────────────────────────

const TWO_OF_THREE: &str = "2of3";

fn compose_with(second: &str) -> (String, String, i32) {
    md(&[
        "compose",
        "--wrapper",
        "wsh",
        "--path",
        TWO_OF_THREE,
        "--path",
        second,
    ])
}

/// Kills compose.rs:46:5 `parse_u16 -> Ok(0)` and `-> Ok(1)`: `older=100u` is
/// 100 units of 512 s, which the template spells `older(4194304 + 100)`
/// (BIP 68: type-flag bit 22 plus the value).
#[test]
fn compose_older_units_carries_the_typed_value() {
    let (out, err, code) = compose_with("1of1,older=100u");
    assert_eq!(code, 0, "compose failed: {err}");
    assert!(out.contains("older(4194404)"), "{out}");
}

/// Kills compose.rs:89:24 (`older` guard -> true): a second lock option must
/// refuse, whichever lock came first.
#[test]
fn compose_refuses_older_after_another_lock() {
    for second in ["1of1,older=10,older=20", "1of1,after=5,older=10"] {
        let (out, err, code) = compose_with(second);
        assert_eq!(code, 1, "`{second}` must refuse; stdout: {out}");
        assert!(err.contains("at most one lock per path"), "{second}: {err}");
    }
}

/// Kills compose.rs:106:24 (`after` guard -> true).
#[test]
fn compose_refuses_after_after_another_lock() {
    for second in ["1of1,after=5,after=7", "1of1,older=10,after=5"] {
        let (out, err, code) = compose_with(second);
        assert_eq!(code, 1, "`{second}` must refuse; stdout: {out}");
        assert!(err.contains("at most one lock per path"), "{second}: {err}");
    }
}

// ─── decompose/walk.rs ──────────────────────────────────────────────────

/// Kills walk.rs:248:52 `||`->`&&` in `liana_internal_key_match`: the Liana
/// recipe xpub is only the unspendable internal key at EXACTLY `<0;1>/*`; a
/// key whose multipath is `<0;2>` or `<2;1>` (one alternative standard, one
/// not) must not be relabelled `UNSPENDABLE(liana)`.
#[test]
fn decompose_does_not_recognise_a_liana_key_at_a_nonstandard_multipath() {
    let xpub = case("preset-kofn-recovery-tr").expected_xpub;
    for mp in ["<0;2>", "<2;1>"] {
        let desc = kofn_with_internal_key(&format!("{xpub}/{mp}/*"));
        let (out, err, _code) = md(&["decompose", &desc, "--emit", "template"]);
        assert!(
            !out.contains("UNSPENDABLE(liana)") && !err.contains("UNSPENDABLE(liana)"),
            "{mp} wrongly recognised as the Liana internal key: {out} {err}"
        );
    }
}

/// Kills walk.rs:332/335/338/341 (the four `Translator` hash literal methods,
/// each `-> String::new()` and `-> "xyzzy"`): the template carries each digest
/// verbatim.
#[test]
fn decompose_template_carries_each_hash_literal_verbatim() {
    let k0 = "[73c5da0a/48'/0'/0'/2']xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf";
    let h32 = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
    let h20 = "000102030405060708090a0b0c0d0e0f10111213";
    for (kind, h) in [
        ("sha256", h32),
        ("hash256", h32),
        ("ripemd160", h20),
        ("hash160", h20),
    ] {
        let desc = format!("wsh(and_v(v:pk({k0}/<0;1>/*),{kind}({h})))");
        let (out, err, code) = md(&["decompose", &desc, "--emit", "template"]);
        assert_eq!(code, 0, "{kind}: {err}");
        assert_eq!(
            first_line(&out),
            format!("wsh(and_v(v:pk(@0/48'/0'/0'/2'/<0;1>/*),{kind}({h})))"),
            "{kind}"
        );
    }
}

// ─── format/text.rs via `md inspect` ────────────────────────────────────

/// Kills text.rs:322:5, 330:5, 338:5 (`fmt_md1_id`, `fmt_template_id`,
/// `fmt_policy_id`, each `-> String::new()` and `-> "xyzzy"`): the three id
/// values `md inspect` prints equal the ones md-codec computes for the card.
#[test]
fn inspect_prints_the_three_ids_in_full() {
    let (enc, err, code) = md(&["encode", "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))"]);
    assert_eq!(code, 0, "{err}");
    let card = first_line(&enc);
    let d = md_codec::decode_md1_string_with_opts(&card, md_codec::DecodeOpts::partial()).unwrap();
    let md1 = hex(compute_md1_encoding_id(&d).unwrap().as_bytes());
    let tpl = hex(compute_wallet_descriptor_template_id(&d)
        .unwrap()
        .as_bytes());
    let pid = hex(compute_wallet_policy_id(&d).unwrap().as_bytes());
    assert_eq!(md1.len(), 32);
    let (out, err, code) = md(&["inspect", &card]);
    assert_eq!(code, 0, "{err}");
    let lines: Vec<&str> = out.lines().collect();
    assert!(
        lines.contains(&format!("md1-encoding-id: {md1}").as_str()),
        "{out}"
    );
    assert!(
        lines.contains(&format!("wallet-descriptor-template-id: {tpl}").as_str()),
        "{out}"
    );
    assert!(
        lines.contains(&format!("wallet-policy-id: {pid}").as_str()),
        "{out}"
    );
}

// ─── parse/template.rs: tag mapping on the encode path ──────────────────

/// Kills template.rs:1731:9 (`DupIf`) and 1752:9 (`False`): each fragment
/// encodes and decodes back to the same template.
///
/// The bare `pk_k` / `pk_h` arms (template.rs 1542:9 / 1548:9) are reached
/// only under a `c:` over a non-key fragment, which md now refuses at mint
/// (bug hunt 2026-11-25 md#4): the card decoded to `c:and_v(v:pk(..),pk(..))`,
/// which no miniscript parser accepts. They are killed by
/// [`check_over_non_key_reaches_the_mint_refusal`] below instead.
#[test]
fn encode_decode_round_trips_pk_k_pk_h_dupif_and_false() {
    for (tpl, expect) in [
        (
            "wsh(and_v(v:pk(@0/<0;1>/*),dv:older(10)))",
            "wsh(and_v(v:pk(@0/<0;1>/*),dv:older(10)))",
        ),
        (
            "wsh(and_v(v:pk(@0/<0;1>/*),or_i(0,pk(@1/<0;1>/*))))",
            "wsh(and_v(v:pk(@0/<0;1>/*),or_i(0,pk(@1/<0;1>/*))))",
        ),
    ] {
        let (enc, err, code) = md(&["encode", "--path", "bip48", tpl]);
        assert_eq!(code, 0, "`{tpl}` refused: {err}");
        let (dec, err, code) = md(&["decode", &first_line(&enc)]);
        assert_eq!(code, 0, "{err}");
        assert_eq!(first_line(&dec), expect, "{tpl}");
    }
}

/// Kills template.rs:2937:37 and 2940:37 (`minting` guard -> false): under
/// `--experimental` the sanity rules other than the signature rule still apply
/// to wsh and sh(wsh) minting; a heightlock/timelock mix is refused.
#[test]
fn experimental_still_refuses_timelock_mixing_under_wsh_and_sh_wsh() {
    let body = "and_v(v:pk(@0/<0;1>/*),and_v(v:after(100),after(500000001)))";
    for tpl in [format!("wsh({body})"), format!("sh(wsh({body}))")] {
        let (out, err, code) = md(&["encode", "--experimental", "--path", "bip48", &tpl]);
        assert_eq!(code, 1, "`{tpl}` must refuse; stdout: {out}");
        assert!(err.contains("heightlock and timelock"), "`{tpl}`: {err}");
    }
}

/// Kills template.rs:1542:9 (delete `Terminal::PkK`) and 1548:9 (`PkH`):
/// with either arm gone the walker fails first with a different error, so
/// these templates must reach md#4's mint refusal by name.
#[test]
fn check_over_non_key_reaches_the_mint_refusal() {
    for tpl in [
        "wsh(c:and_v(v:pk(@0/<0;1>/*),pk_k(@1/<0;1>/*)))",
        "wsh(c:and_v(v:pk(@0/<0;1>/*),pk_h(@1/<0;1>/*)))",
    ] {
        let (enc, err, code) = md(&["encode", "--path", "bip48", tpl]);
        assert_eq!((code, enc.as_str()), (1, ""), "{tpl}: {err}");
        assert!(
            err.starts_with(
                "md: unsupported: this template wraps a fragment other than a single key in `c:`"
            ),
            "{tpl}: {err}"
        );
    }
}
