use crate::error::CliError;
use md_codec::encode::strip_display_separators;

/// Strip mstring display separators (SPEC §3.2) from each md1 input string so a
/// grouped or unbroken card both re-ingest. Applied at every md1-intake site
/// that has no stdin channel — `md address` and `md descriptor`, whose
/// positionals are `phrases` rather than the artifact positional §6b binds.
pub fn strip_md1_inputs(strings: &[String]) -> Vec<String> {
    strings
        .iter()
        .map(|s| strip_display_separators(s))
        .collect()
}

/// Read a list of md1 strings: positional `args` minus a leading `"-"`,
/// which means "read one string per line from stdin." `"-"` may appear
/// as any positional value but is processed once across the list.
/// Mirrors mk-cli's `read_mk1_strings` helper (cross-CLI parity).
///
/// **P3 §6b — hoisted here from `cmd::repair` and made `pub`.** It had one
/// caller and `md repair` was the only verb that could read stdin; `decode`,
/// `verify`, `inspect` and `bytecode` each took `-` as a LITERAL positional and
/// failed with `string does not start with HRP md1`. This is the shape `mk`
/// already has at `mk-cli`'s `cmd/mod.rs::read_mk1_strings`, which this
/// function's doc comment already claimed parity with while sitting in a
/// different module with a fifth of the reach.
///
/// It strips display separators **per line** before deciding a line is empty,
/// which is what lets a card copied off the stderr engraving card be pasted
/// straight back in. The shared crate's `records::split_record_stream` filters
/// on `trim().is_empty()` and returns the line unchanged, so it does not do
/// this — which is why P3 declines it and extends this instead.
/// P3 §6b — the md1 intake for the reading verbs, across all three channels.
///
/// `--in FILE` reads the tool's OWN input material: md1 strings, one per line.
/// It goes through the SAME per-line strip as `-` does, so a card copied off
/// the stderr engraving card re-ingests from a file exactly as it does from a
/// pipe.
///
/// `--in` and the positional are declared mutually exclusive in clap, so this
/// function never has to invent a precedence rule.
///
/// `flag` names the ACTUAL flag the caller is reading a file for in every
/// message this function renders (review r1 M3) — every reading verb's own
/// `--in`, but also `md descriptor --verify-against FILE`
/// (`resolve_verify_against`), which reuses this same file-reading
/// convention without owning a `--in` of its own. Before M3 this hardcoded
/// `"--in"`, so `--verify-against` on an empty file told the operator about
/// a flag they never passed.
pub fn read_md1_inputs(
    args: &[String],
    in_file: Option<&std::path::Path>,
    flag: &str,
) -> Result<Vec<String>, CliError> {
    let Some(path) = in_file else {
        return read_md1_strings(args);
    };
    // The path is NAMED in the refusal. A bare "No such file or directory" out
    // of a pipeline that built the path itself is the one message an operator
    // cannot act on.
    let buf = std::fs::read_to_string(path)
        .map_err(|e| CliError::read_failure(format_args!("{flag} {}", path.display()), &e))?;
    let mut out = Vec::new();
    for line in buf.lines() {
        let s = strip_display_separators(line);
        if !s.is_empty() {
            out.push(s);
        }
    }
    if out.is_empty() {
        return Err(CliError::Input(format!(
            "{flag} {}: no md1 strings in this file. An EMPTY file is what a FAILED \
             upstream command leaves behind -- check the command that wrote it.",
            path.display()
        )));
    }
    Ok(out)
}

/// P3 §6b — read a BIP-388 template from `--in FILE` on `md encode`.
///
/// "Own input material" is not a formality (§6b): `encode`'s input is a
/// TEMPLATE, and the reading verbs' input is md1 strings. A single `--in` that
/// meant both would be a filename implying a different artifact, which is what
/// made an earlier acceptance criterion unsatisfiable.
pub fn read_template_file(path: &std::path::Path) -> Result<String, CliError> {
    let buf = std::fs::read_to_string(path)
        .map_err(|e| CliError::read_failure(format_args!("--in {}", path.display()), &e))?;
    let t = buf.trim();
    if t.is_empty() {
        return Err(CliError::Input(format!(
            "--in {}: no BIP-388 template in this file (it is empty or blank)",
            path.display()
        )));
    }
    Ok(t.to_string())
}

/// P3 §6b — write the artifact to `--out FILE`, created **0600**.
///
/// Through the shared crate's `write_private`, never `std::fs::write`: F-244.
/// `write` has no root re-export in `mnemonic-io-lib`, so the path is
/// module-qualified; the unqualified form is an `E0425`.
///
/// `--out` OVERWRITES (operator ruling 2026-08-26), and `write_private` sets
/// the mode a SECOND time on the open file because `OpenOptions::mode()` binds
/// on create only -- an existing 0644 target would otherwise stay 0644, which
/// is the case a re-run actually hits.
///
/// mnemonic-io-lib 0.1.3 writes a temporary beside FILE and renames it over FILE, so a
/// failed write leaves an existing FILE unchanged. A write error exits 32 with
/// `md: cannot write the output: <FILE>: <error>` plus a state sentence only when a second
/// look at FILE confirms it (bh-md-C4, R2); a standard-stream name says nothing about the disk.
pub fn write_artifact(path: &std::path::Path, body: &str) -> Result<(), CliError> {
    let stdio = names_standard_stream(path);
    let before = if stdio { Snap::Other } else { Snap::take(path) };
    match mnemonic_io_lib::write::write_private(path, body.as_bytes()) {
        Ok(()) => Ok(()),
        Err(e) => {
            // R2 (T6): one line, exit 32. The state sentence is said only when a
            // second look confirms it; a standard-stream alias is written into
            // the open stream, so nothing is said about the disk.
            let f = path.display();
            let after = if stdio { Snap::Other } else { Snap::take(path) };
            let tail = state_tail(&f.to_string(), &before, &after);
            Err(CliError::Io(format!(
                "cannot write the output: {f}: {e}{tail}"
            )))
        }
    }
}

/// One input of a verb, for [`refuse_output_names_input`]: where it is read from,
/// and the label the refusal prints for it (`--in tpl.txt`, `standard input`).
pub struct InputName<'a> {
    endpoint: mnemonic_io_lib::write::Endpoint<'a>,
    label: String,
}

impl<'a> InputName<'a> {
    /// An input named by a flag and a path (`--in`, `--from-mk1-file`,
    /// `--verify-against`), the path as typed.
    pub fn path(flag: &str, p: &'a std::path::Path) -> Self {
        InputName {
            endpoint: mnemonic_io_lib::write::Endpoint::Path(p),
            label: format!("{flag} {}", p.display()),
        }
    }

    /// Standard input.
    pub fn stdin() -> Self {
        InputName {
            endpoint: mnemonic_io_lib::write::Endpoint::Stdin,
            label: "standard input".to_string(),
        }
    }

    /// `--in PATH` when given, else standard input when the positional
    /// strings include `-` (what `read_md1_strings` reads), else none.
    pub fn in_or_stdin(
        in_file: Option<&'a std::path::Path>,
        strings: &[String],
    ) -> Vec<InputName<'a>> {
        match in_file {
            Some(p) => vec![InputName::path("--in", p)],
            None if strings.iter().any(|s| s == "-") => vec![InputName::stdin()],
            None => Vec::new(),
        }
    }
}

/// FIX_bh1119_md_out_names_input S1-S4: refuse, before anything is read or
/// printed, a run whose output is one of its own inputs.
///
/// `stdout` is true when the verb will write on standard output; `out` is the
/// `--out FILE`; `json_out` is true for `md encode` with both `--json` and
/// `--out` (X3: stdout and FILE one file). Identity is the shared crate's
/// `same_file` (symlinks followed, hard links equal, pipes and terminals never
/// match). The checks run in order X2 (stdout), X1 (`--out`), X3; the first
/// match refuses with exit 8 (REFUSED: a different destination waives it)
/// and names the first matching input.
pub fn refuse_output_names_input(
    stdout: bool,
    out: Option<&std::path::Path>,
    json_out: bool,
    inputs: &[InputName<'_>],
) -> Result<(), CliError> {
    let eps: Vec<mnemonic_io_lib::write::Endpoint<'_>> =
        inputs.iter().map(|i| i.endpoint).collect();
    if stdout {
        if let Some(i) =
            mnemonic_io_lib::write::same_file(mnemonic_io_lib::write::Endpoint::Stdout, &eps)
        {
            return Err(CliError::Refused(format!(
                "standard output is the same file as {}, so the output would go into the file md reads its input from. A > redirect empties that file before md starts, so check that it still holds what you expect. Write to a new name.",
                inputs[i].label
            )));
        }
    }
    if let Some(o) = out {
        if let Some(i) =
            mnemonic_io_lib::write::same_file(mnemonic_io_lib::write::Endpoint::Path(o), &eps)
        {
            return Err(CliError::Refused(format!(
                "--out {} is the same file as {} (the same path, a symlink, a hard link or a standard-stream name), and md does not write into a file it reads its input from. Write to a new name.",
                o.display(),
                inputs[i].label
            )));
        }
        if json_out
            && mnemonic_io_lib::write::same_file(
                mnemonic_io_lib::write::Endpoint::Stdout,
                &[mnemonic_io_lib::write::Endpoint::Path(o)],
            )
            .is_some()
        {
            return Err(CliError::Refused(format!(
                "--out {} and standard output are the same file (the same path, a symlink or a hard link), so what md prints on stdout would go into the file it writes. A > redirect empties that file before md starts, so check that it still holds what you expect. Write to a new name.",
                o.display()
            )));
        }
    }
    Ok(())
}

/// The state sentence (with its leading `. `) when the two looks at FILE agree, else empty.
fn state_tail(f: &str, before: &Snap, after: &Snap) -> String {
    match (before, after) {
        (Snap::Regular(a), Snap::Regular(b)) if a == b => format!(". {f} is unchanged."),
        (Snap::Absent, Snap::Absent) => ". Nothing was written.".to_string(),
        _ => String::new(),
    }
}

/// What `std::fs::metadata` (symlinks followed) says about an `--out` FILE.
enum Snap {
    /// A regular file: (dev, ino, len, mtime seconds, mtime nanoseconds).
    #[cfg_attr(not(unix), allow(dead_code))]
    Regular((u64, u64, u64, i64, i64)),
    /// NotFound, including a dangling symlink.
    Absent,
    Other,
}

impl Snap {
    fn take(path: &std::path::Path) -> Snap {
        match std::fs::metadata(path) {
            #[cfg(unix)]
            Ok(m) if m.is_file() => {
                use std::os::unix::fs::MetadataExt;
                Snap::Regular((m.dev(), m.ino(), m.len(), m.mtime(), m.mtime_nsec()))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Snap::Absent,
            _ => Snap::Other,
        }
    }
}

/// True when FILE names a standard stream: its typed path, or a hop of its
/// symlink chain, or that path's or hop's canonical parent joined with its
/// name, starts with `/dev/` or `/proc/`.
fn names_standard_stream(path: &std::path::Path) -> bool {
    fn hit(p: &std::path::Path) -> bool {
        let s = p.to_string_lossy();
        s.starts_with("/dev/") || s.starts_with("/proc/")
    }
    // The typed name, or its canonical parent joined with its name.
    fn spelled(p: &std::path::Path) -> bool {
        if hit(p) {
            return true;
        }
        let Some(name) = p.file_name() else {
            return false;
        };
        let parent = match p.parent() {
            Some(d) if !d.as_os_str().is_empty() => d,
            _ => std::path::Path::new("."),
        };
        std::fs::canonicalize(parent).is_ok_and(|c| hit(&c.join(name)))
    }
    let mut cur = path.to_path_buf();
    // A bounded walk: every hop is tested as spelled, and a symlink loop ends the chain.
    for _ in 0..=40 {
        if spelled(&cur) {
            return true;
        }
        match std::fs::read_link(&cur) {
            Ok(t) => {
                cur = if t.is_absolute() {
                    t
                } else {
                    cur.parent().unwrap_or(std::path::Path::new("")).join(t)
                };
            }
            Err(_) => break,
        }
    }
    false
}

pub fn read_md1_strings(args: &[String]) -> Result<Vec<String>, CliError> {
    let mut out = Vec::with_capacity(args.len());
    let mut consumed_stdin = false;
    for a in args {
        if a == "-" && !consumed_stdin {
            consumed_stdin = true;
            let mut buf = String::new();
            std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)
                .map_err(|e| CliError::read_failure(format_args!("stdin read"), &e))?;
            for line in buf.lines() {
                // mstring display-grouping (SPEC §3.2): strip separators so a
                // grouped or unbroken card both re-ingest. (repair OUTPUT stays
                // unbroken — no grouping flags on `md repair`.)
                let s = strip_display_separators(line);
                if !s.is_empty() {
                    out.push(s);
                }
            }
        } else if a == "-" {
            // Already consumed stdin; ignore additional `-` markers.
        } else {
            out.push(strip_display_separators(a));
        }
    }
    if out.is_empty() {
        return Err(CliError::Input(
            "expected at least one md1 string (positional or via stdin with '-')".into(),
        ));
    }
    Ok(out)
}

pub mod address;
pub mod build;
pub mod bytecode;
#[cfg(feature = "cli-compiler")]
pub mod compile;
pub mod compose;
pub mod decode;
pub mod decompose;
pub mod descriptor;
pub mod encode;
pub mod gen_man;
#[cfg(feature = "json")]
pub mod gui_schema;
pub mod inspect;
pub mod partial;
pub mod repair;
pub mod shape_key;
pub mod vectors;
pub mod verdict;
pub mod verify;

#[cfg(test)]
mod state_tail_tests {
    use super::{Snap, state_tail};

    const SAME: (u64, u64, u64, i64, i64) = (1, 2, 3, 4, 5);

    fn reg(x: (u64, u64, u64, i64, i64)) -> Snap {
        Snap::Regular(x)
    }

    #[test]
    fn regular_same_is_unchanged() {
        assert_eq!(state_tail("F", &reg(SAME), &reg(SAME)), ". F is unchanged.");
    }

    #[test]
    fn regular_with_any_field_differing_says_nothing() {
        for x in [
            (9, 2, 3, 4, 5),
            (1, 9, 3, 4, 5),
            (1, 2, 9, 4, 5),
            (1, 2, 3, 9, 5),
            (1, 2, 3, 4, 9),
        ] {
            assert_eq!(state_tail("F", &reg(SAME), &reg(x)), "", "{x:?}");
        }
    }

    #[test]
    fn absent_twice_says_nothing_was_written() {
        assert_eq!(
            state_tail("F", &Snap::Absent, &Snap::Absent),
            ". Nothing was written."
        );
    }

    #[test]
    fn other_combinations_say_nothing() {
        assert_eq!(state_tail("F", &Snap::Absent, &reg(SAME)), "");
        assert_eq!(state_tail("F", &reg(SAME), &Snap::Absent), "");
        assert_eq!(state_tail("F", &Snap::Other, &Snap::Other), "");
        assert_eq!(state_tail("F", &Snap::Other, &Snap::Absent), "");
        assert_eq!(state_tail("F", &reg(SAME), &Snap::Other), "");
    }

    /// Real files: `Snap::take` must see each of ino, len, mtime seconds and mtime nanoseconds
    /// (the other three fields held equal), so a field dropped from the snapshot fails here.
    #[cfg(unix)]
    mod real_files {
        use super::{Snap, state_tail};
        use std::time::{Duration, UNIX_EPOCH};

        fn set(p: &std::path::Path, body: &str, t: Duration) {
            std::fs::write(p, body).unwrap();
            let f = std::fs::OpenOptions::new().write(true).open(p).unwrap();
            f.set_modified(UNIX_EPOCH + t).unwrap();
        }

        /// Snapshot, change the one file in place (same ino) or replace it, snapshot again.
        fn tail(before: &Snap, p: &std::path::Path) -> String {
            state_tail("F", before, &Snap::take(p))
        }

        #[test]
        fn take_sees_ino_len_mtime_sec_and_nsec() {
            let d = tempfile::tempdir().unwrap();
            let t = Duration::new(1_000_000, 500);
            let a = d.path().join("a");
            set(&a, "abc", t);
            let snap = Snap::take(&a);
            assert_eq!(tail(&snap, &a), ". F is unchanged.");
            // len only
            set(&a, "abcd", t);
            assert_eq!(tail(&snap, &a), "", "len");
            // mtime seconds only
            set(&a, "abc", Duration::new(1_000_001, 500));
            assert_eq!(tail(&snap, &a), "", "mtime sec");
            // mtime nanoseconds only
            set(&a, "abc", Duration::new(1_000_000, 501));
            assert_eq!(tail(&snap, &a), "", "mtime nsec");
            // ino only: a second file with the same len and mtime, renamed over a
            set(&a, "abc", t);
            let b = d.path().join("b");
            set(&b, "abc", t);
            std::fs::rename(&b, &a).unwrap();
            assert_eq!(tail(&snap, &a), "", "ino");
        }
    }
}
