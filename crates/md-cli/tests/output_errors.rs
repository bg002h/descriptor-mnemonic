//! What md does when stdout cannot take its output (crash sweep, the family
//! rule of grade-mk-crash-sweep-1 Q4; toolkit manual `42-md.md`
//! "When stdout fails").
//!
//! - A reader that closed the pipe only stops the output: no text, no
//!   signal, the exit code is the verb's own.
//! - Any other write error (`/dev/full`, ENOSPC), from a verb or `--help`:
//!   "md: cannot write the output: <error>" on stderr, exit 1. A code that
//!   means success with the output as the result (0, partial decode 4,
//!   repair 5) becomes 1; a failing code is kept (pinned by the unit test in
//!   `src/out.rs`, since no md verb fails after printing).
//! - No file of the crate but `src/out.rs` names stdout.
#![allow(missing_docs)]
#![cfg(target_os = "linux")]

use std::fs::OpenOptions;
use std::os::fd::{FromRawFd, OwnedFd};
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Output, Stdio};

const XPUB: &str = "xpub6BmeGmRo4LosAcU21HDaGcvtaQ7GrqQcY48nBkE22qM6KVwQUjRJ1BGzk84SFVHgLcd61Vcnhr8petHexjjn5WbQ9PriVrRhphw4oCp2z6a";
const TEMPLATE: &str = "wpkh(@0/<0;1>/*)";
const CARD: &str = "md1yqpqqxqq8xtwhw4xwn4qh";
/// `CARD` with its last character changed: repair corrects it and exits 5.
const DAMAGED: &str = "md1yqpqqxqq8xtwhw4xwn4qx";
const FULL: &str = "md: cannot write the output: No space left on device (os error 28)";

fn md() -> Command {
    Command::new(env!("CARGO_BIN_EXE_md"))
}

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| (*x).to_owned()).collect()
}

fn address(extra: &[&str]) -> Vec<String> {
    let mut v = s(&[
        "address",
        "--template",
        TEMPLATE,
        "--key",
        &format!("@0={XPUB}"),
        "--path",
        "m/84'/0'/0'",
    ]);
    v.extend(extra.iter().map(|x| (*x).to_owned()));
    v
}

fn closed_pipe() -> OwnedFd {
    let mut fds = [0i32; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    unsafe { libc::close(fds[0]) }; // the reader is gone before md starts
    unsafe { OwnedFd::from_raw_fd(fds[1]) }
}

fn on_full(a: &[String]) -> Output {
    let full = OpenOptions::new().write(true).open("/dev/full").unwrap();
    md().args(a)
        .stdout(Stdio::from(full))
        .stderr(Stdio::piped())
        .output()
        .unwrap()
}

fn on_closed(a: &[String], stderr_too: bool) -> Output {
    let err = if stderr_too {
        Stdio::from(closed_pipe())
    } else {
        Stdio::piped()
    };
    md().args(a)
        .stdout(Stdio::from(closed_pipe()))
        .stderr(err)
        .output()
        .unwrap()
}

fn full_case(a: &[String], code: i32) {
    let o = on_full(a);
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(!err.contains("panicked"), "{a:?}: panic: {err}");
    assert_eq!(
        o.status.code(),
        Some(code),
        "{a:?}: status {:?}: {err}",
        o.status
    );
    assert_eq!(err.lines().last(), Some(FULL), "{a:?}: stderr: {err}");
}

fn closed_case(a: &[String], code: i32, stderr_too: bool) {
    let o = on_closed(a, stderr_too);
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(
        o.status.signal(),
        None,
        "{a:?}: killed by a signal: {:?}",
        o.status
    );
    assert_eq!(
        o.status.code(),
        Some(code),
        "{a:?}: status {:?}: {err}",
        o.status
    );
    assert!(
        !err.contains("panicked") && !err.contains("cannot write"),
        "{a:?}: {err}"
    );
}

#[test]
fn full_stdout_exits_1_with_the_line() {
    let mut rows = vec![
        s(&["gui-schema"]),
        s(&["encode", TEMPLATE, "--group-size", "0"]),
        s(&["encode", "--json", TEMPLATE]),
        s(&["decode", CARD]),
        s(&["decode", "--json", CARD]),
        s(&["inspect", CARD]),
        s(&["inspect", "--json", CARD]),
        s(&["bytecode", CARD]),
        s(&["verify", CARD, "--template", TEMPLATE]),
        s(&["repair", CARD]),
        s(&["repair", "--json", CARD]),
        s(&["shape-key", CARD]),
        s(&[
            "descriptor",
            "--template",
            TEMPLATE,
            "--key",
            &format!("@0={XPUB}"),
            "--path",
            "m/84'/0'/0'",
        ]),
        s(&["compose", "--wrapper", "wsh", "--path", "2of3"]),
        s(&["compose", "--wrapper", "wsh", "--path", "2of3", "--json"]),
        s(&[
            "decompose",
            &format!("wpkh([aabbccdd/84'/0'/0']{XPUB}/<0;1>/*)"),
        ]),
        s(&[
            "decompose",
            "--emit",
            "all",
            &format!("wpkh([aabbccdd/84'/0'/0']{XPUB}/<0;1>/*)"),
        ]),
        address(&["--count", "2"]),
        address(&["--count", "2", "--json"]),
        s(&["--help"]),
        s(&["--version"]),
        s(&["decode", "--help"]),
        s(&["help", "encode"]),
    ];
    if cfg!(feature = "cli-compiler") {
        rows.push(s(&["compile", "pk(@0)", "--context", "segwitv0"]));
    }
    for a in rows {
        full_case(&a, 1);
    }
}

#[test]
fn full_stdout_after_a_repair_exits_1_not_5() {
    let o = md().args(["repair", DAMAGED]).output().unwrap();
    assert_eq!(o.status.code(), Some(5), "fixture: repair should exit 5");
    full_case(&s(&["repair", DAMAGED]), 1);
    full_case(&s(&["repair", "--json", DAMAGED]), 1);
}

#[test]
fn full_stdout_after_a_partial_decode_exits_1_not_4() {
    // A dead shape (no origin to resolve) decodes partially and exits 4.
    let t = "wsh(pk(@0/<0;1>/*))";
    let enc = md()
        .args(["encode", "--group-size", "0", t])
        .output()
        .unwrap();
    let card = String::from_utf8(enc.stdout)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .to_owned();
    let o = md().args(["decode", &card]).output().unwrap();
    assert_eq!(
        o.status.code(),
        Some(4),
        "fixture: partial decode should exit 4"
    );
    full_case(&s(&["decode", &card]), 1);
    full_case(&s(&["inspect", &card]), 1);
}

#[test]
fn full_stdout_with_nothing_to_print_keeps_the_verbs_code() {
    // A verb that fails before printing never touches stdout: its own code
    // and no "cannot write" line.
    let a = s(&["decode", "not-a-card"]);
    let o = on_full(&a);
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{err}");
    assert!(!err.contains("cannot write"), "{err}");
    let a = s(&["repair", "md1qqqqqqqqqqqqqqqqqqqqqqqq"]);
    let o = on_full(&a);
    assert_eq!(o.status.code(), Some(2));
}

#[test]
fn closed_reader_exits_with_the_verbs_code() {
    let mut rows = vec![
        (s(&["gui-schema"]), 0),
        (s(&["encode", TEMPLATE]), 0),
        (s(&["decode", CARD]), 0),
        (address(&["--count", "1000"]), 0),
        (address(&["--count", "1000", "--json"]), 0),
        (s(&["--help"]), 0),
        (s(&["--version"]), 0),
        (s(&["repair", DAMAGED]), 5),
    ];
    rows.push((s(&["decode", "--help"]), 0));
    for (a, code) in rows {
        closed_case(&a, code, false);
    }
}

#[test]
fn closed_reader_on_stdout_and_stderr_exits_0() {
    // These verbs print on stderr with `eprintln!` (encode's policy-id line,
    // compose's notes, a failing verb's `md: ...` line) after or before their
    // stdout: with stderr on the closed pipe too, no panic and the verb's own
    // code.
    closed_case(
        &s(&["encode", TEMPLATE, "--policy-id-fingerprint"]),
        0,
        true,
    );
    closed_case(
        &s(&["compose", "--wrapper", "wsh", "--path", "2of3"]),
        0,
        true,
    );
    closed_case(&address(&["--count", "2"]), 0, true);
    closed_case(&s(&["decode", CARD]), 0, true);
    closed_case(&s(&["decode", "not-a-card"]), 1, true);
    closed_case(&s(&["repair", "md1qqqqqqqqqqqqqqqqqqqqqqqq"]), 2, true);
}

#[test]
fn a_stderr_that_cannot_take_text_never_changes_the_result() {
    // Rule (c): stderr write errors are ignored. compose notes on stderr
    // first; its card on stdout must still arrive, exit 0, whether stderr is
    // a closed pipe or /dev/full.
    let a = s(&["compose", "--wrapper", "wsh", "--path", "2of3"]);
    let full = || OpenOptions::new().write(true).open("/dev/full").unwrap();
    for err in [Stdio::from(closed_pipe()), Stdio::from(full())] {
        let o = md().args(&a).stderr(err).output().unwrap();
        assert_eq!(o.status.code(), Some(0), "{:?}", o.status);
        assert!(String::from_utf8_lossy(&o.stdout).starts_with("wsh("));
    }
    let o = md()
        .args(["decode", "not-a-card"])
        .stderr(Stdio::from(full()))
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
}

#[test]
fn only_out_rs_names_stdout() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut bad = Vec::new();
    let mut stack = vec![dir];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") && !p.ends_with("src/out.rs") {
                let t = std::fs::read_to_string(&p).unwrap();
                for n in ["stdout(", "std::print", "std::eprint", "core::print"] {
                    if t.contains(n) {
                        bad.push(format!("{}: {n}", p.display()));
                    }
                }
            }
        }
    }
    assert!(bad.is_empty(), "stdout named outside src/out.rs: {bad:?}");
}

#[test]
fn every_binary_includes_the_out_module() {
    // `src/bin/gen_ap2_grind.rs` is a second binary of the package (too slow to
    // run here, a ~2^32 grind): it must take its print macros from `out.rs`.
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut bins = vec![src.join("main.rs")];
    for e in std::fs::read_dir(src.join("bin")).unwrap() {
        bins.push(e.unwrap().path());
    }
    for p in bins {
        let t = std::fs::read_to_string(&p).unwrap();
        assert!(
            t.contains("mod out;") && t.contains("#[macro_use]"),
            "{} does not take its print macros from out.rs",
            p.display()
        );
        assert!(
            t.contains("out::exit_code("),
            "{} does not return its exit code through out::exit_code",
            p.display()
        );
    }
}

/// std's stdout reports EBADF as a successful write, so a stdout opened
/// read-only (`md gui-schema 1</dev/null`) lost the output and exited 0.
#[test]
fn read_only_stdout_exits_1_with_the_line() {
    const BADF: &str = "md: cannot write the output: Bad file descriptor (os error 9)";
    let on_read_only = |a: &[String]| {
        let ro = std::fs::File::open("/dev/null").unwrap();
        md().args(a)
            .stdout(Stdio::from(ro))
            .stderr(Stdio::piped())
            .output()
            .unwrap()
    };
    for a in [
        s(&["gui-schema"]),
        s(&["decode", CARD]),
        address(&["--count", "1000"]),
        s(&["--help"]),
        s(&["--version"]),
    ] {
        let o = on_read_only(&a);
        let err = String::from_utf8_lossy(&o.stderr);
        assert_eq!(o.status.code(), Some(1), "{a:?}: {err}");
        assert_eq!(err.lines().last(), Some(BADF), "{a:?}: {err}");
    }
    // A verb that fails before printing keeps its code and prints no line.
    let o = on_read_only(&s(&["decode", "not-a-card"]));
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{err}");
    assert!(!err.contains("cannot write"), "{err}");
}

/// Linux accepts access mode 3 (`O_WRONLY | O_RDWR`): the fd can neither read
/// nor write, and a write gets EBADF, which std reports as a success.
#[test]
fn no_access_mode_stdout_exits_1_with_the_line() {
    const BADF: &str = "md: cannot write the output: Bad file descriptor (os error 9)";
    let fd = unsafe { libc::open(c"/dev/null".as_ptr(), libc::O_ACCMODE) };
    assert!(fd >= 0);
    let o = md()
        .args(["gui-schema"])
        .stdout(Stdio::from(unsafe { OwnedFd::from_raw_fd(fd) }))
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{err}");
    assert_eq!(err.lines().last(), Some(BADF), "{err}");
}

/// A stdout opened read-write (a terminal, `1<>file`) is writable: the probe
/// must not treat it as read-only.
#[test]
fn read_write_stdout_keeps_the_output_and_exits_0() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("out");
    let rw = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&p)
        .unwrap();
    let o = md()
        .arg("gui-schema")
        .stdout(Stdio::from(rw))
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(0), "{err}");
    assert!(!err.contains("cannot write"), "{err}");
    assert!(std::fs::metadata(&p).unwrap().len() > 0, "output lost");
}
