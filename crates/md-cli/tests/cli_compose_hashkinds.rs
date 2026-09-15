//! `md compose` authors all four miniscript hash kinds (SPEC_hashlock_kinds
//! phase 1, §9 row 1).
//!
//! WHY THIS FILE EXISTS. `md-codec` could already decode and render all four —
//! an `md1` carrying a `ripemd160` hashlock read back correctly — while
//! `md compose` accepted only `sha256=`. The gap was the AUTHORING side, and
//! `ms hashlock --kind ripemd160` hands an operator exactly the operand this
//! refused: `--path ... ripemd160=<40 hex>`.
//!
//! The 20-byte kinds carry the risk. Their hex is 40 characters, not 64, and
//! every length rule has to come from `HashKind::digest_len()` — the spec makes
//! that the only place a length is written precisely so the parser, the
//! validator and the formatter cannot drift apart.

use assert_cmd::Command;

const H32: &str = "a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8";
const H20: &str = "09e7bb5051d89788fb4e4b374126721dbcc2946b";

fn md(args: &[&str]) -> (bool, String, String) {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(args)
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn hex_for(kind: &str) -> &'static str {
    match kind {
        "sha256" | "hash256" => H32,
        _ => H20,
    }
}

/// Each kind is its own `--path` option, and the composed template names that
/// fragment. `sha256=` keeps working byte for byte.
#[test]
fn every_kind_is_a_path_option_and_reaches_the_template() {
    for kind in ["sha256", "hash256", "ripemd160", "hash160"] {
        let h = hex_for(kind);
        let (ok, so, se) = md(&[
            "compose",
            "--wrapper",
            "wsh",
            "--path",
            &format!("1of1,{kind}={h}"),
        ]);
        assert!(ok, "{kind}: refused:\n{se}");
        assert!(
            so.contains(&format!("{kind}({h})")),
            "{kind}: template does not carry the fragment at its own width:\n{so}"
        );
    }
}

/// The hex length rule is `digest_len() * 2`, so a 64-hex `ripemd160` is a
/// refusal, not a silent truncation — and a 40-hex `sha256` likewise.
#[test]
fn each_kind_refuses_the_other_width() {
    for (kind, wrong) in [
        ("sha256", H20),
        ("hash256", H20),
        ("ripemd160", H32),
        ("hash160", H32),
    ] {
        let (ok, _, se) = md(&[
            "compose",
            "--wrapper",
            "wsh",
            "--path",
            &format!("1of1,{kind}={wrong}"),
        ]);
        assert!(!ok, "{kind}: accepted a digest of the wrong width");
        let want = if kind == "sha256" || kind == "hash256" {
            64
        } else {
            40
        };
        assert!(
            se.contains(&format!("{want} hex")),
            "{kind}: the refusal must name ITS OWN width, not a default: {se}"
        );
    }
}

/// Uppercase is rejected, never folded — the same rule `ms hashlock --kind`
/// applies, so an operator copying between the two tools meets one behaviour.
#[test]
fn an_unknown_or_uppercase_kind_is_refused() {
    let (ok, _, se) = md(&[
        "compose",
        "--wrapper",
        "wsh",
        "--path",
        &format!("1of1,RIPEMD160={H20}"),
    ]);
    assert!(!ok, "RIPEMD160 must be refused, not folded");
    assert!(se.contains("unknown option"), "{se}");
}

/// At most one hash per path, whichever kinds are named.
#[test]
fn two_hashes_on_one_path_are_refused_across_kinds() {
    let (ok, _, se) = md(&[
        "compose",
        "--wrapper",
        "wsh",
        "--path",
        &format!("1of1,sha256={H32},ripemd160={H20}"),
    ]);
    assert!(!ok, "two kinds on one path must be refused");
    assert!(se.contains("at most one hash per path"), "{se}");
}

/// The `hashlock-gated` preset takes the same sibling options, and its `--json`
/// key names the kind rather than saying `sha256` under all four.
#[test]
fn the_preset_takes_every_kind_and_the_json_names_it() {
    for kind in ["sha256", "hash256", "ripemd160", "hash160"] {
        let h = hex_for(kind);
        let (ok, so, se) = md(&[
            "compose",
            "--wrapper",
            "wsh",
            "--preset",
            &format!("hashlock-gated,{kind}={h},older=144"),
            "--json",
        ]);
        assert!(ok, "{kind}: preset refused:\n{se}");
        let v: serde_json::Value = serde_json::from_str(&so).unwrap_or_else(|e| {
            panic!("{kind}: --json did not parse: {e}\n{so}");
        });
        let p = &v["preset"]["params"];
        assert_eq!(
            p["kind"], kind,
            "{kind}: the params must name the kind:\n{p}"
        );
        assert_eq!(p["digest"], h, "{kind}: digest at its own width:\n{p}");
        assert!(
            p.get("sha256").is_none(),
            "{kind}: the old sha256-only key is wrong for three of four kinds:\n{p}"
        );
    }
}
