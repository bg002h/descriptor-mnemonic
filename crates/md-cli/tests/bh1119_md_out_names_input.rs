#![allow(missing_docs)]
#![cfg_attr(not(unix), allow(dead_code, unused_imports))]
//! FIX_bh1119_md_out_names_input (`design/FIX_bh1119_md_out_names_input.md`,
//! section 4): an md output that is one of md's inputs is refused. Rows n1-n29
//! fail on the base (exit code or stderr); c1-c7 pass on both. Each test runs
//! the built `md` in a fresh directory holding the section 1 files.

use std::fs::{File, OpenOptions};
use std::path::Path;
use std::process::{Command, Stdio};

const TPL: &str = "wpkh(@0/<0;1>/*)";
const DESC: &str = "wpkh([73c5da0a/48'/0'/0'/2']xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf/<0;1>/*)";

/// Where the child's stdout goes.
enum Out<'a> {
    Pipe,
    /// `>>FILE`
    Append(&'a str),
    /// `>FILE`
    Trunc(&'a str),
}

struct Res {
    rc: i32,
    stdout: Vec<u8>,
    stderr: String,
}

struct Row {
    dir: tempfile::TempDir,
    /// The keyless-card line of the public key-card fixture.
    m: String,
}

impl Row {
    fn new() -> Row {
        let dir = tempfile::tempdir().unwrap();
        let fix = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/seating/v-b1-wallet.txt");
        let text = std::fs::read_to_string(fix).unwrap();
        let m = text
            .lines()
            .find(|l| l.starts_with("md1"))
            .unwrap()
            .to_string();
        let mk: String = text
            .lines()
            .filter(|l| l.starts_with("mk1"))
            .map(|l| format!("{l}\n"))
            .collect();
        let r = Row { dir, m };
        r.put("tpl.txt", &format!("{TPL}\n"));
        r.put("desc.txt", &format!("{DESC}\n"));
        r.put("K.txt", &mk);
        let card = r.run(&["encode", TPL], Out::Pipe, None);
        assert_eq!(card.rc, 0);
        r.put("card.txt", std::str::from_utf8(&card.stdout).unwrap());
        let v = r.run(
            &["descriptor", &r.m, "--from-mk1-file", "K.txt", "--emit", "md1"],
            Out::Pipe,
            None,
        );
        assert_eq!(v.rc, 0);
        r.put("V.txt", std::str::from_utf8(&v.stdout).unwrap());
        r
    }

    fn path(&self, n: &str) -> std::path::PathBuf {
        self.dir.path().join(n)
    }

    fn put(&self, n: &str, body: &str) {
        std::fs::write(self.path(n), body).unwrap();
    }

    fn get(&self, n: &str) -> Vec<u8> {
        std::fs::read(self.path(n)).unwrap()
    }

    fn exists(&self, n: &str) -> bool {
        std::fs::symlink_metadata(self.path(n)).is_ok()
    }

    fn run(&self, args: &[&str], out: Out<'_>, stdin: Option<&str>) -> Res {
        let mut c = Command::new(env!("CARGO_BIN_EXE_md"));
        c.args(args).current_dir(self.dir.path());
        match stdin {
            Some(n) => {
                c.stdin(File::open(self.path(n)).unwrap());
            }
            None => {
                c.stdin(Stdio::null());
            }
        }
        match out {
            Out::Pipe => {
                c.stdout(Stdio::piped());
            }
            Out::Append(n) => {
                let f = OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(self.path(n))
                    .unwrap();
                c.stdout(f);
            }
            Out::Trunc(n) => {
                let f = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(self.path(n))
                    .unwrap();
                c.stdout(f);
            }
        }
        c.stderr(Stdio::piped());
        let o = c.output().unwrap();
        Res {
            rc: o.status.code().unwrap_or(-1),
            stdout: o.stdout,
            stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
        }
    }
}

fn x1(out: &str, input: &str) -> String {
    format!(
        "md: --out {out} is the same file as {input} (the same path, a symlink, a hard link or a standard-stream name), and md does not write into a file it reads its input from. Write to a new name.\n"
    )
}

fn x2(input: &str) -> String {
    format!(
        "md: standard output is the same file as {input}, so the output would go into the file md reads its input from. A > redirect empties that file before md starts, so check that it still holds what you expect. Write to a new name.\n"
    )
}

fn x3(out: &str) -> String {
    format!(
        "md: --out {out} and standard output are the same file (the same path, a symlink or a hard link), so what md prints on stdout would go into the file it writes. A > redirect empties that file before md starts, so check that it still holds what you expect. Write to a new name.\n"
    )
}

/// A refusal: rc 2, nothing on a piped stdout, exactly `line` on stderr.
fn refused(r: &Res, line: &str) {
    assert_eq!(r.rc, 2, "stderr: {}", r.stderr);
    assert!(r.stdout.is_empty(), "stdout: {:?}", r.stdout);
    assert_eq!(r.stderr, line);
}

// ---- X1: --out names an input -------------------------------------------

#[test]
fn n1_encode_out_same_path_as_in_refuses() {
    let w = Row::new();
    let before = w.get("tpl.txt");
    let r = w.run(&["encode", "--in", "tpl.txt", "--out", "tpl.txt"], Out::Pipe, None);
    refused(&r, &x1("tpl.txt", "--in tpl.txt"));
    assert_eq!(w.get("tpl.txt"), before);
}

#[test]
#[cfg(unix)]
fn n2_encode_out_hard_link_to_in_refuses() {
    let w = Row::new();
    std::fs::hard_link(w.path("tpl.txt"), w.path("hl")).unwrap();
    let before = w.get("tpl.txt");
    let r = w.run(&["encode", "--in", "tpl.txt", "--out", "hl"], Out::Pipe, None);
    refused(&r, &x1("hl", "--in tpl.txt"));
    assert_eq!(w.get("tpl.txt"), before);
    assert_eq!(w.get("hl"), before);
}

#[test]
#[cfg(unix)]
fn n3_encode_out_symlink_to_in_refuses() {
    let w = Row::new();
    std::os::unix::fs::symlink("tpl.txt", w.path("sl")).unwrap();
    let before = w.get("tpl.txt");
    let r = w.run(&["encode", "--in", "tpl.txt", "--out", "sl"], Out::Pipe, None);
    refused(&r, &x1("sl", "--in tpl.txt"));
    assert_eq!(w.get("tpl.txt"), before);
}

#[test]
fn n4_encode_out_other_spelling_of_in_refuses() {
    let w = Row::new();
    let before = w.get("tpl.txt");
    let r = w.run(&["encode", "--in", "./tpl.txt", "--out", "tpl.txt"], Out::Pipe, None);
    refused(&r, &x1("tpl.txt", "--in ./tpl.txt"));
    assert_eq!(w.get("tpl.txt"), before);
}

#[test]
fn n5_encode_json_out_same_as_in_refuses() {
    let w = Row::new();
    let before = w.get("tpl.txt");
    let r = w.run(
        &["encode", "--json", "--in", "tpl.txt", "--out", "tpl.txt"],
        Out::Pipe,
        None,
    );
    refused(&r, &x1("tpl.txt", "--in tpl.txt"));
    assert_eq!(w.get("tpl.txt"), before);
}

#[test]
fn n6_descriptor_out_same_as_mk1_file_refuses() {
    let w = Row::new();
    let before = w.get("K.txt");
    let r = w.run(
        &["descriptor", &w.m, "--from-mk1-file", "K.txt", "--emit", "md1", "--out", "K.txt"],
        Out::Pipe,
        None,
    );
    refused(&r, &x1("K.txt", "--from-mk1-file K.txt"));
    assert_eq!(w.get("K.txt"), before);
}

#[test]
#[cfg(unix)]
fn n7_descriptor_out_symlink_to_mk1_file_refuses() {
    let w = Row::new();
    std::os::unix::fs::symlink("K.txt", w.path("kl")).unwrap();
    let before = w.get("K.txt");
    let r = w.run(
        &["descriptor", &w.m, "--from-mk1-file", "K.txt", "--emit", "md1", "--out", "kl"],
        Out::Pipe,
        None,
    );
    refused(&r, &x1("kl", "--from-mk1-file K.txt"));
    assert_eq!(w.get("K.txt"), before);
}

#[test]
fn n8_descriptor_out_same_as_verify_against_file_refuses() {
    let w = Row::new();
    let before = w.get("V.txt");
    let r = w.run(
        &[
            "descriptor", &w.m, "--from-mk1-file", "K.txt", "--emit", "md1", "--group-size", "5",
            "--verify-against", "V.txt", "--out", "V.txt",
        ],
        Out::Pipe,
        None,
    );
    refused(&r, &x1("V.txt", "--verify-against V.txt"));
    assert_eq!(w.get("V.txt"), before);
}

#[test]
#[cfg(unix)]
fn n9_encode_out_dev_stdout_onto_in_refuses() {
    let w = Row::new();
    let before = w.get("tpl.txt");
    let r = w.run(
        &["encode", "--in", "tpl.txt", "--out", "/dev/stdout"],
        Out::Append("tpl.txt"),
        None,
    );
    assert_eq!(r.rc, 2);
    assert_eq!(r.stderr, x1("/dev/stdout", "--in tpl.txt"));
    assert_eq!(w.get("tpl.txt"), before);
}

// ---- X2: standard output names an input ---------------------------------

/// A redirect row: rc 2, exactly `line` on stderr, the file as `expect`.
fn redirect_refused(w: &Row, r: &Res, line: &str, file: &str, expect: &[u8]) {
    assert_eq!(r.rc, 2, "stderr: {}", r.stderr);
    assert_eq!(r.stderr, line);
    assert_eq!(w.get(file), expect);
}

#[test]
#[cfg(unix)]
fn n10_encode_stdout_appended_to_in_refuses() {
    let w = Row::new();
    let before = w.get("tpl.txt");
    let r = w.run(&["encode", "--in", "tpl.txt"], Out::Append("tpl.txt"), None);
    redirect_refused(&w, &r, &x2("--in tpl.txt"), "tpl.txt", &before);
}

#[test]
#[cfg(unix)]
fn n11_encode_stdout_truncating_in_refuses() {
    let w = Row::new();
    let r = w.run(&["encode", "--in", "tpl.txt"], Out::Trunc("tpl.txt"), None);
    redirect_refused(&w, &r, &x2("--in tpl.txt"), "tpl.txt", b"");
}

#[test]
#[cfg(unix)]
fn n12_decode_stdout_appended_to_in_refuses() {
    let w = Row::new();
    let before = w.get("card.txt");
    let r = w.run(&["decode", "--in", "card.txt"], Out::Append("card.txt"), None);
    redirect_refused(&w, &r, &x2("--in card.txt"), "card.txt", &before);
}

#[test]
#[cfg(unix)]
fn n13_verify_stdout_appended_to_in_refuses() {
    let w = Row::new();
    let before = w.get("card.txt");
    let r = w.run(
        &["verify", "--in", "card.txt", "--template", TPL],
        Out::Append("card.txt"),
        None,
    );
    redirect_refused(&w, &r, &x2("--in card.txt"), "card.txt", &before);
}

#[test]
#[cfg(unix)]
fn n14_inspect_stdout_appended_to_in_refuses() {
    let w = Row::new();
    let before = w.get("card.txt");
    let r = w.run(&["inspect", "--in", "card.txt"], Out::Append("card.txt"), None);
    redirect_refused(&w, &r, &x2("--in card.txt"), "card.txt", &before);
}

#[test]
#[cfg(unix)]
fn n15_bytecode_stdout_appended_to_in_refuses() {
    let w = Row::new();
    let before = w.get("card.txt");
    let r = w.run(&["bytecode", "--in", "card.txt"], Out::Append("card.txt"), None);
    redirect_refused(&w, &r, &x2("--in card.txt"), "card.txt", &before);
}

#[test]
#[cfg(unix)]
fn n16_repair_stdout_appended_to_in_refuses() {
    let w = Row::new();
    let before = w.get("card.txt");
    let r = w.run(&["repair", "--in", "card.txt"], Out::Append("card.txt"), None);
    redirect_refused(&w, &r, &x2("--in card.txt"), "card.txt", &before);
}

#[test]
#[cfg(unix)]
fn n17_decompose_stdout_appended_to_in_refuses() {
    let w = Row::new();
    let before = w.get("desc.txt");
    let r = w.run(&["decompose", "--in", "desc.txt"], Out::Append("desc.txt"), None);
    redirect_refused(&w, &r, &x2("--in desc.txt"), "desc.txt", &before);
}

#[test]
#[cfg(unix)]
fn n18_decode_stdin_and_stdout_one_file_refuses() {
    let w = Row::new();
    let before = w.get("card.txt");
    let r = w.run(&["decode", "-"], Out::Append("card.txt"), Some("card.txt"));
    redirect_refused(&w, &r, &x2("standard input"), "card.txt", &before);
}

#[test]
#[cfg(unix)]
fn n19_decompose_stdin_and_stdout_one_file_refuses() {
    let w = Row::new();
    let before = w.get("desc.txt");
    let r = w.run(&["decompose", "-"], Out::Append("desc.txt"), Some("desc.txt"));
    redirect_refused(&w, &r, &x2("standard input"), "desc.txt", &before);
}

#[test]
#[cfg(unix)]
fn n20_descriptor_stdout_appended_to_mk1_file_refuses() {
    let w = Row::new();
    let before = w.get("K.txt");
    let r = w.run(
        &["descriptor", &w.m, "--from-mk1-file", "K.txt"],
        Out::Append("K.txt"),
        None,
    );
    redirect_refused(&w, &r, &x2("--from-mk1-file K.txt"), "K.txt", &before);
}

#[test]
#[cfg(unix)]
fn n21_address_stdout_appended_to_mk1_file_refuses() {
    let w = Row::new();
    let before = w.get("K.txt");
    let r = w.run(
        &["address", &w.m, "--from-mk1-file", "K.txt"],
        Out::Append("K.txt"),
        None,
    );
    redirect_refused(&w, &r, &x2("--from-mk1-file K.txt"), "K.txt", &before);
}

#[test]
#[cfg(unix)]
fn n22_descriptor_stdout_appended_to_verify_against_refuses() {
    let w = Row::new();
    let before = w.get("V.txt");
    let r = w.run(
        &["descriptor", &w.m, "--from-mk1-file", "K.txt", "--verify-against", "V.txt"],
        Out::Append("V.txt"),
        None,
    );
    redirect_refused(&w, &r, &x2("--verify-against V.txt"), "V.txt", &before);
}

#[test]
#[cfg(unix)]
fn n23_encode_json_out_stdout_appended_to_in_refuses() {
    let w = Row::new();
    let before = w.get("tpl.txt");
    let r = w.run(
        &["encode", "--json", "--in", "tpl.txt", "--out", "G"],
        Out::Append("tpl.txt"),
        None,
    );
    redirect_refused(&w, &r, &x2("--in tpl.txt"), "tpl.txt", &before);
    assert!(!w.exists("G"));
}

// ---- X3: --out and standard output are one file -------------------------

#[test]
#[cfg(unix)]
fn n24_encode_json_out_is_stdout_appended_refuses() {
    let w = Row::new();
    w.put("F", "keep\n");
    let r = w.run(&["encode", TPL, "--json", "--out", "F"], Out::Append("F"), None);
    redirect_refused(&w, &r, &x3("F"), "F", b"keep\n");
}

#[test]
#[cfg(unix)]
fn n25_encode_json_out_is_stdout_truncated_refuses() {
    let w = Row::new();
    w.put("F", "keep\n");
    let r = w.run(&["encode", TPL, "--json", "--out", "F"], Out::Trunc("F"), None);
    redirect_refused(&w, &r, &x3("F"), "F", b"");
}

// ---- order, absent paths, first input -----------------------------------

#[test]
#[cfg(unix)]
fn n26_stdout_check_runs_before_out_check() {
    let w = Row::new();
    let before = w.get("tpl.txt");
    let r = w.run(
        &["encode", "--json", "--in", "tpl.txt", "--out", "tpl.txt"],
        Out::Append("tpl.txt"),
        None,
    );
    redirect_refused(&w, &r, &x2("--in tpl.txt"), "tpl.txt", &before);
}

#[test]
#[cfg(unix)]
fn n27_decode_stdin_and_stdout_truncated_refuses() {
    let w = Row::new();
    let r = w.run(&["decode", "-"], Out::Trunc("card.txt"), Some("card.txt"));
    redirect_refused(&w, &r, &x2("standard input"), "card.txt", b"");
}

#[test]
fn n28_encode_absent_in_and_out_one_place_refuses() {
    let w = Row::new();
    let r = w.run(&["encode", "--in", "x", "--out", "x"], Out::Pipe, None);
    refused(&r, &x1("x", "--in x"));
    assert!(!w.exists("x"));
}

#[test]
fn n29_first_matching_input_is_named() {
    let w = Row::new();
    let before = w.get("K.txt");
    let r = w.run(
        &[
            "descriptor", &w.m, "--from-mk1-file", "K.txt", "--emit", "md1", "--verify-against",
            "K.txt", "--out", "K.txt",
        ],
        Out::Pipe,
        None,
    );
    refused(&r, &x1("K.txt", "--from-mk1-file K.txt"));
    assert_eq!(w.get("K.txt"), before);
}

// ---- controls: nothing is refused ---------------------------------------

#[test]
fn c1_encode_out_new_name_writes() {
    let w = Row::new();
    let r = w.run(&["encode", "--in", "tpl.txt", "--out", "new.txt"], Out::Pipe, None);
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(w.get("new.txt"), w.get("card.txt"));
}

#[test]
#[cfg(unix)]
fn c2_decode_stdout_other_file_writes() {
    let w = Row::new();
    let r = w.run(&["decode", "--in", "card.txt"], Out::Append("out.txt"), None);
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(w.get("out.txt"), format!("{TPL}\n").as_bytes());
}

#[test]
fn c3_decode_stdout_pipe_never_matches() {
    let w = Row::new();
    let r = w.run(&["decode", "--in", "card.txt"], Out::Pipe, None);
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(r.stdout, format!("{TPL}\n").as_bytes());
}

#[test]
#[cfg(unix)]
fn c4_descriptor_out_set_stdout_not_checked() {
    let w = Row::new();
    let before = w.get("K.txt");
    let r = w.run(
        &["descriptor", &w.m, "--from-mk1-file", "K.txt", "--emit", "md1", "--out", "G"],
        Out::Append("K.txt"),
        None,
    );
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(w.get("K.txt"), before);
    assert_eq!(w.get("G"), w.get("V.txt"));
}

#[test]
fn c5_verify_against_md1_string_is_not_an_input() {
    let w = Row::new();
    let m = w.m.clone();
    let r = w.run(
        &[
            "descriptor", &m, "--from-mk1-file", "K.txt", "--emit", "md1", "--verify-against",
            &m, "--out", &m,
        ],
        Out::Pipe,
        None,
    );
    assert_eq!(r.rc, 5, "stderr: {}", r.stderr);
    assert_eq!(w.get(&m), w.get("V.txt"));
}

#[test]
#[cfg(unix)]
fn c6_encode_out_set_stdout_not_checked() {
    let w = Row::new();
    let before = w.get("tpl.txt");
    let r = w.run(&["encode", TPL, "--out", "F"], Out::Append("tpl.txt"), None);
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(w.get("tpl.txt"), before);
    assert_eq!(w.get("F"), w.get("card.txt"));
}

#[test]
#[cfg(unix)]
fn c7_encode_text_out_onto_stdout_not_checked() {
    let w = Row::new();
    w.put("F", "keep\n");
    let r = w.run(&["encode", TPL, "--out", "F"], Out::Append("F"), None);
    assert_eq!(r.rc, 0, "stderr: {}", r.stderr);
    assert_eq!(w.get("F"), w.get("card.txt"));
}
