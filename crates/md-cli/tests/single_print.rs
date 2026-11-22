//! `md encode` and `md descriptor --emit md1` print each md1 chunk ONCE, on
//! stdout (Brian 2026-11-22, reversing D4 of mnemonic-engrave
//! `design/SPEC_constellation_cli_uniformity.md`; `design/FIX_md_single_print.md`).
//!
//! The grouped stderr engraving card, and its `group size:` / `separator:`
//! lines, are gone; `chunk-set-id:` stays on stderr. `--group-size N` groups
//! STDOUT on request (default 0, ungrouped), as `mt encode` and
//! `mnemonic bundle` do, and `--out FILE` receives exactly the bytes stdout
//! would have carried.

use assert_cmd::cargo::CommandCargoExt;
use std::process::Command;

/// mk-cli's `V15_84_TEST` account xpub (testnet, m/84'/1'/0', fp 73c5da0a).
const TPUB: &str = "tpubDC9Go1KDateW3gS8VXZ6DD1Xu7PgoTdPcf1MX9Z6qVLiHbaeDJ78swPyuQ8YQY19QjtrzkfkZSXwqCcb7XArtid1iLq8Vy55Ydfm4giZh6X";
const WATCH_NOTE: &str = "note: stdout is watch-only \u{2014} public keys only, cannot spend\n";
const TEMPLATE_NOTE: &str = "note: stdout is a keyless descriptor template (no keys)\n";
const CSID: &str = "chunk-set-id: 0x9c25f\n";
const V_B1_WALLET: &str = include_str!("fixtures/seating/v-b1-wallet.txt");

fn md() -> Command {
    Command::cargo_bin("md").unwrap()
}

fn text(b: &[u8]) -> String {
    String::from_utf8(b.to_vec()).unwrap()
}

/// A keyed signet wpkh wallet policy: three chunks.
fn encode(extra: &[&str]) -> std::process::Output {
    let key = format!("@0={TPUB}");
    let mut args = vec![
        "encode",
        "wpkh(@0/<0;1>/*)",
        "--key",
        &key,
        "--fingerprint",
        "@0=73c5da0a",
        "--path",
        "m/84'/1'/0'",
        "--network",
        "signet",
    ];
    args.extend_from_slice(extra);
    md().args(&args).output().unwrap()
}

fn grouped5(s: &str) -> String {
    let c: Vec<char> = s.chars().collect();
    c.chunks(5)
        .map(|g| g.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(" ")
}

fn regroup5(body: &str) -> String {
    body.lines().map(|l| grouped5(l) + "\n").collect()
}

fn hrp_lines(t: &str, hrp: &str) -> Vec<String> {
    t.lines()
        .map(str::trim)
        .filter(|l| l.starts_with(hrp))
        .map(str::to_string)
        .collect()
}

fn emit_b1(extra: &[&str]) -> std::process::Output {
    let mut c = md();
    c.arg("descriptor");
    for p in hrp_lines(V_B1_WALLET, "md1") {
        c.arg(p);
    }
    for s in hrp_lines(V_B1_WALLET, "mk1") {
        c.args(["--from-mk1", &s]);
    }
    c.args(["--emit", "md1"]);
    c.args(extra);
    c.output().unwrap()
}

#[test]
fn encode_prints_each_chunk_once() {
    let o = encode(&[]);
    assert_eq!(o.status.code(), Some(0));
    let out = text(&o.stdout);
    assert_eq!(out.lines().count(), 3, "three chunks; got {out:?}");
    for l in out.lines() {
        assert!(
            l.starts_with("md1") && !l.contains(' '),
            "ungrouped; got {l:?}"
        );
    }
    assert_eq!(text(&o.stderr), format!("{CSID}{WATCH_NOTE}"));
}

#[test]
fn encode_group_size_groups_stdout() {
    let plain = text(&encode(&[]).stdout);
    let o = encode(&["--group-size", "5"]);
    assert_eq!(o.status.code(), Some(0));
    assert_eq!(text(&o.stdout), regroup5(&plain));
    assert_eq!(text(&o.stderr), format!("{CSID}{WATCH_NOTE}"));
}

#[test]
fn encode_out_writes_the_stdout_bytes() {
    let grouped = text(&encode(&["--group-size", "5"]).stdout);
    let d = tempfile::tempdir().unwrap();
    let f = d.path().join("policy.md1");
    let o = encode(&["--group-size", "5", "--out", f.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(0));
    assert_eq!(text(&o.stdout), "");
    assert_eq!(std::fs::read_to_string(&f).unwrap(), grouped);
    assert_eq!(text(&o.stderr), format!("{CSID}{WATCH_NOTE}"));
}

#[test]
fn encode_single_chunk_template_prints_once() {
    let o = md()
        .args(["encode", "wpkh(@0/<0;1>/*)", "--network", "signet"])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(0));
    assert_eq!(text(&o.stdout), "md1yqpqqxqq8xtwhw4xwn4qh\n");
    assert_eq!(text(&o.stderr), TEMPLATE_NOTE);
}

#[test]
fn descriptor_emit_md1_prints_each_chunk_once() {
    let o = emit_b1(&[]);
    assert_eq!(o.status.code(), Some(0), "{}", text(&o.stderr));
    let out = text(&o.stdout);
    assert!(!out.is_empty());
    for l in out.lines() {
        assert!(
            l.starts_with("md1") && !l.contains(' '),
            "ungrouped; got {l:?}"
        );
    }
    let err = text(&o.stderr);
    for l in err.lines() {
        assert!(
            !l.starts_with("md1") && !l.starts_with("group size:") && !l.starts_with("separator:"),
            "no engraving card on stderr; got {l:?} in {err:?}"
        );
    }
}

#[test]
fn descriptor_emit_md1_group_size_groups_stdout() {
    let plain = text(&emit_b1(&[]).stdout);
    let o = emit_b1(&["--group-size", "5"]);
    assert_eq!(o.status.code(), Some(0), "{}", text(&o.stderr));
    assert_eq!(text(&o.stdout), regroup5(&plain));
}

// ---- the `--separator` refusal remedies run as printed (review r1 I-2) ----

const WPKH: &str = "wpkh(@0/<0;1>/*)";

/// With the default group size 0, `--separator space` alone prints unbroken,
/// so the refusal's remedy for "a grouped card" must carry `--group-size 5`.
/// Both remedies are run exactly as the refusal prints them.
#[test]
fn separator_refusal_remedies_run_as_printed() {
    let refused = md()
        .args([
            "encode",
            WPKH,
            "--network",
            "signet",
            "--separator",
            "hyphen",
        ])
        .output()
        .unwrap();
    assert_eq!(refused.status.code(), Some(2));
    let se = text(&refused.stderr);
    assert!(
        se.contains(
            "Use `--group-size 5 --separator space` (or the literal \" \") for a grouped card, \
             or drop `--separator` for an unbroken one (the default)."
        ),
        "{se}"
    );

    let grouped = md()
        .args([
            "encode",
            WPKH,
            "--network",
            "signet",
            "--group-size",
            "5",
            "--separator",
            "space",
        ])
        .output()
        .unwrap();
    assert_eq!(grouped.status.code(), Some(0));
    assert_eq!(text(&grouped.stdout), "md1yq pqqxq q8xtw hw4xw n4qh\n");

    let unbroken = md()
        .args(["encode", WPKH, "--network", "signet"])
        .output()
        .unwrap();
    assert_eq!(unbroken.status.code(), Some(0));
    assert_eq!(text(&unbroken.stdout), "md1yqpqqxqq8xtwhw4xwn4qh\n");

    let other = md()
        .args(["encode", WPKH, "--network", "signet", "--separator", "x"])
        .output()
        .unwrap();
    assert_eq!(other.status.code(), Some(2));
    let se = text(&other.stderr);
    assert!(
        se.contains(
            "Drop `--separator` for an unbroken card (the default), or use \
             `--group-size 5 --separator space` for a grouped one."
        ),
        "{se}"
    );
}

#[test]
fn descriptor_emit_md1_out_writes_the_stdout_bytes() {
    let grouped = text(&emit_b1(&["--group-size", "5"]).stdout);
    assert!(grouped.contains(' '));
    let d = tempfile::tempdir().unwrap();
    let f = d.path().join("policy.md1");
    let o = emit_b1(&["--group-size", "5", "--out", f.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(0), "{}", text(&o.stderr));
    assert_eq!(text(&o.stdout), "");
    assert_eq!(std::fs::read_to_string(&f).unwrap(), grouped);
}
