#![allow(missing_docs)]
//! P3 §6b — the `--in` / `--out` channels on `md`.
//!
//! `--in FILE` reads the tool's OWN input material: a BIP-388 template on
//! `encode`, md1 strings on the reading verbs. `--out FILE` writes the artifact
//! through the shared crate's `write_private`, which creates it **0600** —
//! something a shell redirect cannot do, which is the whole of F-244.
//!
//! **`OpenOptions::mode()` binds on CREATE ONLY.** An implementation that sets
//! the mode when opening leaves an ALREADY-EXISTING 0644 file at 0644, and that
//! is the case an operator re-running a command actually hits. The overwrite
//! half is asserted here for exactly that reason.

use assert_cmd::Command;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

const CARD: &str = "md1yqpqqxqq8xtwhw4xwn4qh";
const TEMPLATE: &str = "wpkh(@0/<0;1>/*)";

fn out_of(args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(args)
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[cfg(unix)]
fn mode_of(p: &std::path::Path) -> u32 {
    std::fs::metadata(p).unwrap().permissions().mode() & 0o777
}

/// `md encode --in <template file>` is byte-equal to the positional run, on
/// both streams. §10's acceptance pipeline opens with exactly this call.
#[test]
fn encode_in_reads_a_template_file() {
    let dir = tempfile::tempdir().unwrap();
    let tpl = dir.path().join("wallet.template");
    std::fs::write(&tpl, format!("{TEMPLATE}\n")).unwrap();
    let (p_code, p_out, p_err) = out_of(&["encode", TEMPLATE]);
    let (i_code, i_out, i_err) = out_of(&["encode", "--in", tpl.to_str().unwrap()]);
    assert_eq!(p_code, Some(0));
    assert_eq!(i_code, Some(0), "stderr={i_err}");
    assert_eq!(i_out, p_out, "--in stdout must equal the positional run");
    assert_eq!(i_err, p_err, "--in stderr must equal the positional run");
}

/// Supplying both the positional template and `--in` is a usage error, not a
/// silent precedence rule.
#[test]
fn encode_refuses_both_the_positional_and_in() {
    let dir = tempfile::tempdir().unwrap();
    let tpl = dir.path().join("wallet.template");
    std::fs::write(&tpl, format!("{TEMPLATE}\n")).unwrap();
    let (code, _, err) = out_of(&["encode", TEMPLATE, "--in", tpl.to_str().unwrap()]);
    assert_eq!(code, Some(2), "stderr={err}");
    // The exit code ALONE is a false PASS before this entry lands: `--in` was an
    // unknown flag then, so clap already exited 2 with "unexpected argument".
    // The refusal must name the CONFLICT.
    assert!(
        err.contains("cannot be used with"),
        "must refuse as a conflict, not as an unknown flag; got {err:?}"
    );
}

/// `--out` on a path that does NOT exist: created 0600, holding the artifact,
/// with stdout left empty.
#[test]
fn out_creates_the_file_owner_only() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("wallet.md1");
    let (code, stdout, stderr) = out_of(&["encode", TEMPLATE, "--out", f.to_str().unwrap()]);
    assert_eq!(code, Some(0), "stderr={stderr}");
    assert_eq!(stdout, "", "--out routes the artifact off stdout");
    assert_eq!(std::fs::read_to_string(&f).unwrap(), format!("{CARD}\n"));
    #[cfg(unix)]
    assert_eq!(mode_of(&f), 0o600, "--out must CREATE 0600");
}

/// **F-244, the half that is easy to leave out.** The target already exists at
/// 0644. `OpenOptions::mode()` does nothing here, so a mode-on-create
/// implementation leaves it world-readable and this assertion reds.
#[test]
fn out_tightens_an_existing_0644_file_to_0600() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("stale.md1");
    std::fs::write(&f, "stale contents that are longer than the artifact\n").unwrap();
    #[cfg(unix)]
    std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o644)).unwrap();
    #[cfg(unix)]
    assert_eq!(mode_of(&f), 0o644, "fixture must start world-readable");

    let (code, _, stderr) = out_of(&["encode", TEMPLATE, "--out", f.to_str().unwrap()]);
    assert_eq!(code, Some(0), "stderr={stderr}");
    #[cfg(unix)]
    assert_eq!(
        mode_of(&f),
        0o600,
        "--out must tighten an EXISTING file to 0600"
    );
    // And it TRUNCATED: no tail of the longer previous contents survives.
    assert_eq!(std::fs::read_to_string(&f).unwrap(), format!("{CARD}\n"));
}

/// `--out` OVERWRITES — ruled by the operator 2026-08-26. Running the same
/// command twice must succeed and leave one artifact, not two appended.
#[test]
fn out_overwrites_on_a_second_run() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("wallet.md1");
    let a = out_of(&["encode", TEMPLATE, "--out", f.to_str().unwrap()]);
    let b = out_of(&["encode", TEMPLATE, "--out", f.to_str().unwrap()]);
    assert_eq!(a.0, Some(0));
    assert_eq!(b.0, Some(0), "stderr={}", b.2);
    assert_eq!(std::fs::read_to_string(&f).unwrap(), format!("{CARD}\n"));
    #[cfg(unix)]
    assert_eq!(mode_of(&f), 0o600);
}

/// `--out` SUPPRESSES NOTHING ELSE. The stderr engraving card, the chunk-set-id
/// and the output-class advisory all still fire; only the artifact moved.
#[test]
fn out_suppresses_nothing_on_stderr() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("wallet.md1");
    let (_, _, plain_err) = out_of(&["encode", TEMPLATE]);
    let (code, _, out_err) = out_of(&["encode", TEMPLATE, "--out", f.to_str().unwrap()]);
    assert_eq!(code, Some(0));
    assert_eq!(
        out_err, plain_err,
        "--out must not change what stderr carries"
    );
    assert!(out_err.contains("md1yq pqqxq q8xtw hw4xw n4qh"));
    assert!(out_err.contains("group size: 5"));
}

/// A CHUNK SET through `--out`: every chunk lands in the file, unbroken, one
/// per line — the form `mk encode --from-md1-set` and `me sysw pack --in` read.
#[test]
fn out_writes_a_whole_chunk_set() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("wallet.md1");
    let (code, _, _) = out_of(&[
        "encode",
        TEMPLATE,
        "--force-chunked",
        "--out",
        f.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(0));
    let body = std::fs::read_to_string(&f).unwrap();
    let (_, stdout, _) = out_of(&["encode", TEMPLATE, "--force-chunked"]);
    assert_eq!(
        body, stdout,
        "the file must carry exactly what stdout would"
    );
    for l in body.lines() {
        assert!(
            l.starts_with("md1") && !l.contains(char::is_whitespace),
            "{l:?}"
        );
    }
}

/// `--in FILE` on every reading verb, asserted as byte-equality with the
/// positional run — the same gate shape as `-`, for the same reason.
#[test]
fn in_reads_md1_strings_on_every_reading_verb() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("card.md1");
    std::fs::write(&f, format!("{CARD}\n")).unwrap();
    let path = f.to_str().unwrap();
    let cases: Vec<(&str, Vec<&str>, Vec<&str>)> = vec![
        ("decode", vec!["decode", CARD], vec!["decode", "--in", path]),
        (
            "inspect",
            vec!["inspect", CARD],
            vec!["inspect", "--in", path],
        ),
        (
            "bytecode",
            vec!["bytecode", CARD],
            vec!["bytecode", "--in", path],
        ),
        ("repair", vec!["repair", CARD], vec!["repair", "--in", path]),
        (
            "verify",
            vec!["verify", CARD, "--template", TEMPLATE],
            vec!["verify", "--in", path, "--template", TEMPLATE],
        ),
    ];
    for (verb, pos_args, in_args) in cases {
        let (p_code, p_out, p_err) = out_of(&pos_args);
        let (i_code, i_out, i_err) = out_of(&in_args);
        assert_eq!(i_code, p_code, "`md {verb} --in` exit; stderr={i_err}");
        assert_eq!(i_out, p_out, "`md {verb} --in` stdout");
        assert_eq!(i_err, p_err, "`md {verb} --in` stderr");
    }
}

/// A GROUPED card in the `--in` file re-ingests, because `--in` goes through
/// the same reader `-` does.
#[test]
fn in_accepts_a_grouped_card() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("card.md1");
    std::fs::write(&f, "md1yq pqqxq q8xtw hw4xw n4qh\n").unwrap();
    let (code, stdout, err) = out_of(&["decode", "--in", f.to_str().unwrap()]);
    assert_eq!(code, Some(0), "stderr={err}");
    assert_eq!(stdout, format!("{TEMPLATE}\n"));
}

/// A missing `--in` file is a usage error naming the path, not a panic.
#[test]
fn in_refuses_a_missing_file() {
    let (code, _, err) = out_of(&["decode", "--in", "/nonexistent/nope.md1"]);
    assert_eq!(code, Some(2), "stderr={err}");
    assert!(err.contains("/nonexistent/nope.md1"), "stderr={err}");
}

/// The help surface actually carries the flags. This is the plan's own
/// "fails today" measurement, kept as a regression: the sweep counted 0
/// `--in`/`--out` across all five verbs before this entry.
#[test]
fn the_help_surface_carries_the_channels() {
    for verb in [
        "encode", "decode", "verify", "inspect", "bytecode", "repair",
    ] {
        let (_, help, _) = out_of(&[verb, "--help"]);
        assert!(help.contains("--in "), "`md {verb} --help` lacks --in");
    }
    let (_, help, _) = out_of(&["encode", "--help"]);
    assert!(help.contains("--out "), "`md encode --help` lacks --out");
}

/// **THE REQUIREMENT DID NOT EVAPORATE.** Making the positional
/// `required_unless_present = "in_file"` is exactly the kind of change that can
/// silently make a verb accept NOTHING — and a decode of nothing that exits 0
/// is worse than any refusal. Every verb whose positional was relaxed is
/// asserted here to still refuse when neither channel is supplied.
#[test]
fn every_relaxed_verb_still_refuses_when_no_input_is_supplied() {
    let cases: Vec<Vec<&str>> = vec![
        vec!["decode"],
        vec!["inspect"],
        vec!["bytecode"],
        vec!["repair"],
        vec!["verify", "--template", TEMPLATE],
        vec!["encode"],
    ];
    for args in cases {
        let out = Command::cargo_bin("md")
            .unwrap()
            .args(&args)
            .write_stdin(String::new())
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(2),
            "`md {}` with no input must still refuse at 2; stderr={}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

/// `--from-policy` also supplies the template, so `--in` conflicts with it too.
/// Without the conflict the file would be silently ignored — a precedence rule
/// nobody wrote down, and the flag would look like it worked.
#[test]
fn encode_refuses_from_policy_together_with_in() {
    let dir = tempfile::tempdir().unwrap();
    let tpl = dir.path().join("wallet.template");
    std::fs::write(&tpl, format!("{TEMPLATE}\n")).unwrap();
    let (code, _, err) = out_of(&[
        "encode",
        "--from-policy",
        "pk(@0)",
        "--in",
        tpl.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(2), "stderr={err}");
    assert!(err.contains("cannot be used with"), "stderr={err}");
}
