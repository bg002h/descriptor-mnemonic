//! bh1119 FIX #41 (bh-md-I10, bh-md-F1): every `$ md ...` line of an `EXAMPLES:` block in
//! `md <sub> --help` runs as printed under `bash -c` and prints the line below it.
#![allow(missing_docs)]

use std::process::{Command, Output};

fn md_path() -> String {
    let md_bin = assert_cmd::cargo::cargo_bin("md");
    format!(
        "{}:{}",
        md_bin.parent().unwrap().display(),
        std::env::var("PATH").unwrap_or_default()
    )
}

fn help(sub: &str) -> Vec<String> {
    let out = Command::new(assert_cmd::cargo::cargo_bin("md"))
        .args([sub, "--help"])
        .output()
        .unwrap();
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| l.trim().to_string())
        .collect()
}

fn run_bash(cmd: &str) -> Output {
    Command::new("bash")
        .arg("-c")
        .arg(cmd)
        .env("PATH", md_path())
        .output()
        .unwrap()
}

/// `(command, next help line)` for each `$ ` line of `md <sub> --help`.
fn examples(sub: &str) -> Vec<(String, String)> {
    let lines = help(sub);
    let mut v = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        if let Some(cmd) = l.strip_prefix("$ ") {
            v.push((
                cmd.to_string(),
                lines.get(i + 1).cloned().unwrap_or_default(),
            ));
        }
    }
    v
}

/// One `$` line, exit 0, stdout equals the next help line.
fn check_one(sub: &str) {
    let ex = examples(sub);
    assert_eq!(
        ex.len(),
        1,
        "md {sub} --help must print exactly one `$` line"
    );
    let (cmd, expected) = &ex[0];
    let out = run_bash(cmd);
    assert_eq!(
        out.status.code(),
        Some(0),
        "help example failed as printed: {cmd}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        expected.as_str(),
        "help example output differs from the printed line: {cmd}"
    );
}

#[test]
fn i10_encode_help_example_runs_as_printed() {
    check_one("encode");
}

#[test]
fn i10_decompose_help_example_runs_as_printed() {
    check_one("decompose");
}

#[test]
fn c1_decode_help_example_runs_as_printed() {
    check_one("decode");
}

#[test]
fn f1_descriptor_help_example_runs_as_printed() {
    check_one("descriptor");
}

#[test]
fn f1_address_help_example_runs_as_printed() {
    check_one("address");
}

#[test]
fn f1_repair_help_examples_run_as_printed() {
    let ex = examples("repair");
    assert_eq!(
        ex.len(),
        3,
        "md repair --help must print exactly three `$` lines"
    );
    for (cmd, _) in &ex {
        let out = run_bash(cmd);
        assert_eq!(
            out.status.code(),
            Some(5),
            "help example failed as printed: {cmd}\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            !out.stdout.is_empty(),
            "help example printed nothing: {cmd}"
        );
    }
}
