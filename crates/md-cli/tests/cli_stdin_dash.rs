#![allow(missing_docs)]
//! P3 §6b — `-` as a positional reads stdin, on all four remaining `md` verbs.
//!
//! **THE GATE IS EQUALITY, NOT SUCCESS, AND THAT IS LOAD-BEARING.** Before this
//! entry, `-` was accepted as a LITERAL positional value and failed with
//! `codex32 decode error: string does not start with HRP md1` — measured at
//! exit 1 on `decode`, `inspect` and `bytecode`, and at exit 1 on `verify` once
//! `--template` is supplied. It was never clap's `unexpected argument` at exit
//! 2, so a gate written as *"the command fails today"* would have passed both
//! before and after the fix and proved nothing. What distinguishes the two
//! worlds is that the piped run and the positional run produce the SAME bytes.
//!
//! `verify` supplies `--template` in every assertion here. Bare `md verify -`
//! exits 2 from clap (*"the following required arguments were not provided:
//! --template"*), so a `verify` gate written without it measures clap and never
//! reaches the reader at all.

use assert_cmd::Command;

const CARD: &str = "md1yqpqqxqq8xtwhw4xwn4qh";
const TEMPLATE: &str = "wpkh(@0/<0;1>/*)";

struct Run {
    code: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn run(args: &[&str], stdin: Option<&str>) -> Run {
    let mut cmd = Command::cargo_bin("md").unwrap();
    cmd.args(args);
    if let Some(s) = stdin {
        cmd.write_stdin(s.to_string());
    }
    let out = cmd.output().unwrap();
    Run {
        code: out.status.code(),
        stdout: out.stdout,
        stderr: out.stderr,
    }
}

/// The four verbs, each asserted three ways: the positional run succeeds (the
/// CONTROL — a non-dash positional is still parsed as a card), the piped run
/// exits 0, and the two are byte-equal on both streams.
#[test]
fn dash_reads_stdin_on_all_four_verbs_byte_for_byte() {
    let cases: Vec<(&str, Vec<&str>, Vec<&str>)> = vec![
        ("decode", vec!["decode", CARD], vec!["decode", "-"]),
        ("inspect", vec!["inspect", CARD], vec!["inspect", "-"]),
        ("bytecode", vec!["bytecode", CARD], vec!["bytecode", "-"]),
        (
            "verify",
            vec!["verify", CARD, "--template", TEMPLATE],
            vec!["verify", "-", "--template", TEMPLATE],
        ),
    ];
    for (verb, pos_args, dash_args) in cases {
        let positional = run(&pos_args, None);
        assert_eq!(
            positional.code,
            Some(0),
            "control: `md {verb} <card>` must still parse the positional as a card; stderr={}",
            String::from_utf8_lossy(&positional.stderr)
        );
        let piped = run(&dash_args, Some(&format!("{CARD}\n")));
        assert_eq!(
            piped.code,
            Some(0),
            "`md {verb} -` must read stdin; stderr={}",
            String::from_utf8_lossy(&piped.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&piped.stdout),
            String::from_utf8_lossy(&positional.stdout),
            "`md {verb} -` stdout must be byte-equal to the positional run"
        );
        assert_eq!(
            String::from_utf8_lossy(&piped.stderr),
            String::from_utf8_lossy(&positional.stderr),
            "`md {verb} -` stderr must be byte-equal to the positional run"
        );
    }
}

/// The multi-line path: a chunk set piped one chunk per line is equal to the
/// same chunks as repeated positionals. This is the shape `md encode` now
/// writes to stdout, so it is the shape a pipeline actually produces.
#[test]
fn dash_reads_a_whole_chunk_set_from_stdin() {
    let enc = Command::cargo_bin("md")
        .unwrap()
        .args(["encode", TEMPLATE, "--force-chunked"])
        .output()
        .unwrap();
    assert!(enc.status.success());
    let stdout = String::from_utf8(enc.stdout).unwrap();
    let chunks: Vec<&str> = stdout.lines().collect();
    assert!(!chunks.is_empty());

    let mut pos_args = vec!["decode"];
    pos_args.extend(chunks.iter().copied());
    let positional = run(&pos_args, None);
    let piped = run(&["decode", "-"], Some(&stdout));
    assert_eq!(positional.code, Some(0));
    assert_eq!(piped.code, Some(0));
    assert_eq!(
        String::from_utf8_lossy(&piped.stdout),
        String::from_utf8_lossy(&positional.stdout)
    );
}

/// A GROUPED card pasted back in over stdin re-ingests, because the reader
/// strips display separators per line before deciding a line is empty. This is
/// the whole of the mstring-grouping contract, and it is the behaviour the
/// crate's `records::split_record_stream` does NOT have — which is why P3
/// declines it and extends this reader instead.
#[test]
fn dash_accepts_a_grouped_card_from_the_engraving_card() {
    let grouped = "md1yq pqqxq q8xtw hw4xw n4qh\n";
    let piped = run(&["decode", "-"], Some(grouped));
    let positional = run(&["decode", CARD], None);
    assert_eq!(
        piped.code,
        Some(0),
        "stderr={}",
        String::from_utf8_lossy(&piped.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&piped.stdout),
        String::from_utf8_lossy(&positional.stdout)
    );
}
