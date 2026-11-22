//! FIX_bh1119_md_decompose_routes (row #4): `md decompose --emit commands`
//! prints only the mint routes it has proven, and no emission prints a
//! template an md1 card cannot hold. Each test is named exactly as its TRACE
//! row (`design/FIX_bh1119_md_decompose_routes.md` section 4). Signet only.
//!
//! This file asserts only what `md decompose` prints, and runs route 1's
//! printed block with `bash`. It never spawns `mk`: "route 2 runs and
//! reproduces" is checked by `scripts/fix-bh1119-md-criticals-gate.sh` with
//! the installed `mk`.

#![allow(missing_docs)]

use assert_cmd::Command;
use bitcoin::hashes::{Hash, sha256};

const KA: &str = "tpubDFH9dgzveyD8zTbPUFuLrGmCydNvxehyNdUXKJAQN8x4aZ4j6UZqGfnqFrD4NqyaTVGKbvEW54tsvPTK2UoSbCC1PJY8iCNiwTL3RWZEheQ";
const K2: &str = "tpubDB5GSFrKWykZsJgwdd9ViGUECU8AQvTxig4iZonrS5BJ2k9V4ucfHifnXTNfRExUaDsLvZedPfbEN2iphbq11CMDe4WJQAvQRqXaGJuwhwk";
const M0: &str = "tpubD6NzVbkrYhZ4XYa9MoLt4BiMZ4gkt2faZ4BcmKu2a9te4LDpQmvEz2L2yDERivHxFPnxXXhqDRkUNnQCpZggCyEZLBktV7VaSmwayqMJy1s";
const K1: &str = "tpubD97UxEEVXiRvic42NcHLjLAWv5L3CEkuj587dZtEmb4T4Da74DpjbC5tjm7qBq3G4LxjU9SYuTv7edTvTefctXWPkvgSn7MHUmiPCsSyzyJ";
const O: &str = "[73c5da0a/48'/1'/0'/2']";

const DEPTH_REASON: &str = "md encode refuses it: --key @0: expected an account-level xpub at depth 3 or 4 (this script context conventionally uses 3), got";

fn md(args: &[&str]) -> (i32, String, String) {
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

fn decompose(desc: &str, emit: &str) -> (i32, String, String) {
    md(&["decompose", desc, "--network", "signet", "--emit", emit])
}

/// The 2.0 refusal for key @`i`; `use_site` is the occurrence's use-site text
/// or `None` for "has no use-site".
fn template_refusal(i: usize, use_site: Option<&str>) -> String {
    let has = match use_site {
        Some(u) => format!("has use-site `{u}`"),
        None => "has no use-site".to_string(),
    };
    format!(
        "md: decompose: this descriptor has no md1 template: key @{i} {has}, which an md1 card cannot hold (after the origin it holds only a wildcard, optionally after one multipath step such as `/<0;1>/*`), so a template, and any card minted from it, would describe a different wallet. If this is the receive (`/0/*`) or change (`/1/*`) half of a wallet, give the descriptor with `/<0;1>/*` in its place. The emissions that need no template still work: `--emit keys`, `--emit fingerprints`, `--emit descriptor`.\n"
    )
}

fn assert_template_refused(desc: &str, emit: &str, i: usize, use_site: Option<&str>) {
    let (rc, out, err) = decompose(desc, emit);
    assert_eq!(rc, 1, "stdout: {out} stderr: {err}");
    assert_eq!(out, "", "stdout must be empty");
    assert_eq!(err, template_refusal(i, use_site));
}

// ─── C1: --emit commands, fixed use-sites ───────────────────────────────────

#[test]
fn d1_c1_fixed_step_use_site() {
    assert_template_refused(&format!("wpkh({O}{KA}/0/*)"), "commands", 0, Some("/0/*"));
}

#[test]
fn d2_no_wildcard() {
    assert_template_refused(&format!("wpkh({O}{KA})"), "commands", 0, None);
}

#[test]
fn d3_fixed_step_then_multipath() {
    assert_template_refused(
        &format!("wpkh({O}{KA}/7/<0;1>/*)"),
        "commands",
        0,
        Some("/7/<0;1>/*"),
    );
}

// ─── New-2: --emit template / all ───────────────────────────────────────────

#[test]
fn d1t_c1_fixed_step_template() {
    assert_template_refused(&format!("wpkh({O}{KA}/0/*)"), "template", 0, Some("/0/*"));
}

#[test]
fn d2t_no_wildcard_template() {
    assert_template_refused(&format!("wpkh({O}{KA})"), "template", 0, None);
}

#[test]
fn d3t_fixed_step_then_multipath_template() {
    assert_template_refused(
        &format!("wpkh({O}{KA}/7/<0;1>/*)"),
        "template",
        0,
        Some("/7/<0;1>/*"),
    );
}

#[test]
fn d3u_trailing_step_after_multipath_template() {
    assert_template_refused(
        &format!("wpkh({O}{KA}/<0;1>/5/*)"),
        "template",
        0,
        Some("/<0;1>/5/*"),
    );
}

#[test]
fn d3h_hardened_multipath_alt_template() {
    // [md-bh4-hardened-alt-row]: the lexer-error branch also refuses a
    // hardened multipath alternative. The use-site is named as the input
    // renders it (miniscript spells the hardening `'`).
    for spelt in ["/<0h;1h>/*", "/<0;1h>/*", "/<0';1'>/*"] {
        let rendered = match spelt {
            "/<0;1h>/*" => "/<0;1'>/*",
            _ => "/<0';1'>/*",
        };
        assert_template_refused(
            &format!("wpkh({O}{KA}{spelt})"),
            "template",
            0,
            Some(rendered),
        );
    }
}

#[test]
fn d1a_c1_fixed_step_all() {
    assert_template_refused(&format!("wpkh({O}{KA}/0/*)"), "all", 0, Some("/0/*"));
}

// ─── I9 and the depth-1/2 keys: the route proofs ────────────────────────────

#[test]
fn d4_i9_master_key() {
    let (rc, out, err) = decompose(&format!("wpkh([73c5da0a]{M0}/<0;1>/*)"), "commands");
    assert_eq!(rc, 1, "stderr: {err}");
    assert_eq!(out, "");
    assert_eq!(
        err,
        format!(
            "md: decompose: --emit commands cannot be produced: no mint route reproduces this descriptor. route 1: {DEPTH_REASON} 0. route 2: mk encode refuses it: keys.txt:1: origin \"73c5da0a\" has no derivation path; a key card must declare one (e.g. `[73c5da0a/48'/0'/0'/2']`). The emissions that mint nothing still work: `--emit template`, `--emit keys`, `--emit descriptor`.\n"
        )
    );
}

#[test]
fn d5_depth2_route1_withheld() {
    let (rc, out, err) = decompose(&format!("wpkh([73c5da0a/84'/1']{K2}/<0;1>/*)"), "commands");
    assert_eq!(rc, 0, "stderr: {err}");
    let first = out.lines().next().unwrap();
    assert_eq!(
        first,
        format!("# ── route 1 withheld: {DEPTH_REASON} 2; see `md decompose --emit template` ──")
    );
    assert_eq!(out.lines().nth(1), Some(""));
    assert!(
        out.lines()
            .nth(2)
            .unwrap()
            .starts_with("# ── route 2: the SPLIT set"),
        "got: {out}"
    );
    assert!(!out.contains("route 1: the KEYED"));
    assert!(out.contains("mk encode --keys keys.txt --from-md1-set policy.md1"));
}

#[test]
fn d10_depth1_wrong_fingerprint() {
    let (rc, out, err) = decompose(&format!("wpkh([deadbeef/84']{K1}/<0;1>/*)"), "commands");
    assert_eq!(rc, 1, "stderr: {err}");
    assert_eq!(out, "");
    assert_eq!(
        err,
        format!(
            "md: decompose: --emit commands cannot be produced: no mint route reproduces this descriptor. route 1: {DEPTH_REASON} 1. route 2: mk encode refuses it: origin fingerprint deadbeef does not match the xpub: at depth 1 the xpub's parent is the master key, so the fingerprint must be 73c5da0a. The record pairs a key with an origin that is not its own.. The emissions that mint nothing still work: `--emit template`, `--emit keys`, `--emit descriptor`.\n"
        )
    );
}

#[test]
fn d10b_depth1_right_fingerprint() {
    let (rc, out, err) = decompose(&format!("wpkh([73c5da0a/84']{K1}/<0;1>/*)"), "commands");
    assert_eq!(rc, 0, "stderr: {err}");
    assert_eq!(
        out.lines().next().unwrap(),
        format!("# ── route 1 withheld: {DEPTH_REASON} 1; see `md decompose --emit template` ──")
    );
    assert!(
        out.lines()
            .nth(2)
            .unwrap()
            .starts_with("# ── route 2: the SPLIT set"),
        "got: {out}"
    );
}

// ─── origin-less keys ───────────────────────────────────────────────────────

#[test]
fn d11_origin_less_fixed_step() {
    let d = format!("wpkh({KA}/0/*)");
    let (rc, out, err) = decompose(&d, "commands");
    assert_eq!(rc, 1);
    assert_eq!(out, "");
    assert!(
        err.contains("`--emit keys`, `--emit descriptor`. To mint"),
        "got: {err}"
    );
    assert!(!err.contains("`--emit template`"), "got: {err}");
    assert_template_refused(&d, "template", 0, Some("/0/*"));
    let (rc, out, err) = decompose(&d, "keys");
    assert_eq!(rc, 0, "stderr: {err}");
    assert!(out.contains(KA));
    assert!(
        err.contains(
            "The descriptor emission is unaffected; `--emit template` and `--emit commands` refuse."
        ),
        "got: {err}"
    );
}

#[test]
fn d9_origin_less_unchanged() {
    let (rc, out, err) = decompose(&format!("wpkh({KA}/<0;1>/*)"), "commands");
    assert_eq!(rc, 1);
    assert_eq!(out, "");
    assert!(
        err.contains("`--emit template`, `--emit keys`, `--emit descriptor`. To mint"),
        "got: {err}"
    );
}

#[test]
fn d12_template_controls() {
    let cases: [(String, &str); 4] = [
        (
            format!("wpkh({O}{KA}/<2;3>/*)"),
            "wpkh(@0/48'/1'/0'/2'/<2;3>/*)",
        ),
        (format!("wpkh({O}{KA}/*)"), "wpkh(@0/48'/1'/0'/2'/*)"),
        (
            format!("wpkh({O}{KA}/<0;1;2>/*)"),
            "wpkh(@0/48'/1'/0'/2'/<0;1;2>/*)",
        ),
        (format!("wpkh([73c5da0a]{M0}/<0;1>/*)"), "wpkh(@0/<0;1>/*)"),
    ];
    for (d, want) in cases {
        let (rc, out, err) = decompose(&d, "template");
        assert_eq!(rc, 0, "{d}: {err}");
        assert_eq!(out, format!("{want}\n"), "{d}");
    }
}

// ─── controls: both routes printed, byte for byte as today ──────────────────

fn control(desc: &str) -> String {
    let (rc, out, err) = decompose(desc, "commands");
    assert_eq!(rc, 0, "stderr: {err}");
    assert!(
        out.starts_with("# ── route 1: the KEYED card"),
        "got: {out}"
    );
    assert!(out.contains("# ── route 2: the SPLIT set"), "got: {out}");
    assert!(!out.contains("withheld"));
    out
}

#[test]
fn d6_control_standard() {
    let out = control(&format!("wpkh({O}{KA}/<0;1>/*)"));
    // Route 1's printed block runs and yields a card md decodes.
    let route1 = out
        .lines()
        .skip_while(|l| !l.starts_with("md encode "))
        .take_while(|l| !l.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    let md_bin = assert_cmd::cargo::cargo_bin("md");
    let path = format!(
        "{}:{}",
        md_bin.parent().unwrap().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let ran = std::process::Command::new("bash")
        .arg("-c")
        .arg(&route1)
        .env("PATH", path)
        .output()
        .unwrap();
    assert!(
        ran.status.success(),
        "{}",
        String::from_utf8_lossy(&ran.stderr)
    );
    let card = String::from_utf8(ran.stdout).unwrap();
    let mut args = vec!["decode"];
    args.extend(card.split_whitespace());
    let (rc, _, err) = md(&args);
    assert_eq!(rc, 0, "{err}");
}

#[test]
fn d6b_control_stdout_unchanged() {
    let (_, out, _) = decompose(&format!("wpkh({O}{KA}/<0;1>/*)"), "commands");
    let h = sha256::Hash::hash(out.as_bytes()).to_string();
    assert_eq!(&h[..16], "db14761d61c955d2");
}

#[test]
fn d7_control_other_multipath() {
    control(&format!("wpkh({O}{KA}/<2;3>/*)"));
}

#[test]
fn d8_control_bare_wildcard() {
    control(&format!("wpkh({O}{KA}/*)"));
}
