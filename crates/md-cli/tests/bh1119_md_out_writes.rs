//! bughunt-1119 `bh-md-C4`, `bh-md-I1` and `bh-md-I7` (`design/FIX_bh1119_md_out_writes.md`): an
//! `--out` write that fails closed (the old file survives), `md encode --json --out` that writes
//! both, and a failing `md encode` that has written nothing. One test per FIX section 4 row, named
//! exactly as the row. `<dir>` in an expected text is the test's temp dir.
//!
//! FSIZE rows run the built `md` under `RLIMIT_FSIZE` with `SIGXFSZ` ignored, so a write past the
//! limit returns `EFBIG`; this stands in for a full disk or a quota.

#![allow(missing_docs)]

use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Output, Stdio};

const T: &str = "wpkh(@0/<0;1>/*)";
const P: &str = "pkh(@0/<0;1>/*)";
const W: &str = "wsh(pk(@0/<0;1>/*))";
const CARD: &str = "md1yqpqqxqq8xtwhw4xwn4qh\n";
const NOTE: &str = "note: stdout is a keyless descriptor template (no keys)";
const CODEC: &str =
    "md: codec error: non-canonical wrapper requires explicit origin for @0, but none provided";
const V_B1_WALLET: &str = include_str!("fixtures/seating/v-b1-wallet.txt");

fn md_cmd(args: &[&str]) -> Command {
    let mut c = Command::new(assert_cmd::cargo::cargo_bin("md"));
    c.args(args);
    c
}

/// Run with `RLIMIT_FSIZE = limit` and `SIGXFSZ` ignored.
fn fsize(mut c: Command, limit: u64) -> Command {
    unsafe {
        c.pre_exec(move || {
            let lim = libc::rlimit {
                rlim_cur: limit as libc::rlim_t,
                rlim_max: limit as libc::rlim_t,
            };
            libc::signal(libc::SIGXFSZ, libc::SIG_IGN);
            libc::setrlimit(libc::RLIMIT_FSIZE, &lim);
            Ok(())
        });
    }
    c
}

fn run(mut c: Command) -> Output {
    c.output().unwrap()
}
fn rc(o: &Output) -> i32 {
    o.status.code().unwrap()
}
fn so(o: &Output) -> String {
    String::from_utf8(o.stdout.clone()).unwrap()
}
fn se(o: &Output) -> String {
    String::from_utf8(o.stderr.clone()).unwrap()
}
fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}
fn tmp_files(d: &Path) -> Vec<String> {
    std::fs::read_dir(d)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n.ends_with(".tmp"))
        .collect()
}
fn err27(x: &Path) -> String {
    format!(
        "md: cannot write the output: {0}: File too large (os error 27). {0} is unchanged.\n",
        x.display()
    )
}
fn mint(f: &Path, tpl: &str) {
    let o = run(md_cmd(&["encode", tpl, "--out", s(f)]));
    assert_eq!(rc(&o), 0, "{}", se(&o));
}

#[test]
fn o1_failed_overwrite_keeps_previous_file() {
    let d = tempfile::tempdir().unwrap();
    let f = d.path().join("F");
    mint(&f, T);
    assert_eq!(std::fs::read_to_string(&f).unwrap(), CARD);
    let o = run(fsize(md_cmd(&["encode", P, "--out", s(&f)]), 8));
    assert_eq!(rc(&o), 1);
    assert_eq!(se(&o), err27(&f));
    assert_eq!(so(&o), "");
    assert_eq!(std::fs::read_to_string(&f).unwrap(), CARD);
    assert_eq!(tmp_files(d.path()), Vec::<String>::new());
}

#[test]
fn o2_failed_overwrite_keeps_previous_file_descriptor_emit_md1() {
    let d = tempfile::tempdir().unwrap();
    let dd = d.path().join("D");
    let kf = d.path().join("mk1.txt");
    let mk1: Vec<&str> = V_B1_WALLET
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("mk1"))
        .collect();
    std::fs::write(&kf, mk1.join("\n") + "\n").unwrap();
    let md1: Vec<&str> = V_B1_WALLET
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("md1"))
        .collect();
    let mut args: Vec<&str> = vec!["descriptor"];
    args.extend(md1.iter());
    args.extend(["--from-mk1-file", s(&kf), "--emit", "md1", "--out", s(&dd)]);
    let o = run(md_cmd(&args));
    assert_eq!(rc(&o), 0, "{}", se(&o));
    let before = std::fs::read(&dd).unwrap();
    assert!(before.len() > 8);
    let o = run(fsize(md_cmd(&args), 8));
    assert_eq!(rc(&o), 1);
    let want = format!("chunk-set-id: 0x7a2e1\n{}", err27(&dd));
    assert_eq!(se(&o), want);
    assert_eq!(std::fs::read(&dd).unwrap(), before);
}

#[test]
fn o3_out_write_error_text_and_exit() {
    let d = tempfile::tempdir().unwrap();
    let x = d.path().join("nodir").join("x.md1");
    let o = run(md_cmd(&["encode", T, "--out", s(&x)]));
    assert_eq!(rc(&o), 1);
    assert_eq!(
        se(&o),
        format!(
            "md: cannot write the output: {}: No such file or directory (os error 2). Nothing was written.\n",
            x.display()
        )
    );
    assert_eq!(so(&o), "");
}

#[test]
fn o4_read_only_dir_refuses_and_keeps_file() {
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let d = tempfile::tempdir().unwrap();
    let ro = d.path().join("ro");
    std::fs::create_dir(&ro).unwrap();
    let f = ro.join("F");
    mint(&f, T);
    std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o500)).unwrap();
    let o = run(md_cmd(&["encode", P, "--out", s(&f)]));
    std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(rc(&o), 1);
    assert_eq!(
        se(&o),
        format!(
            "md: cannot write the output: {0}: Permission denied (os error 13). {0} is unchanged.\n",
            f.display()
        )
    );
    assert_eq!(std::fs::read_to_string(&f).unwrap(), CARD);
}

#[test]
fn o5_symlink_out_replaces_target_keeps_link() {
    let d = tempfile::tempdir().unwrap();
    let t = d.path().join("T.md1");
    let l = d.path().join("L.md1");
    std::fs::write(&t, "OLD\n").unwrap();
    std::fs::set_permissions(&t, std::fs::Permissions::from_mode(0o644)).unwrap();
    std::os::unix::fs::symlink(&t, &l).unwrap();
    let o = run(md_cmd(&["encode", T, "--out", s(&l)]));
    assert_eq!(rc(&o), 0, "{}", se(&o));
    assert!(
        std::fs::symlink_metadata(&l)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(std::fs::read_to_string(&t).unwrap(), CARD);
    assert_eq!(
        std::fs::metadata(&t).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn o6_json_with_out_writes_both() {
    let d = tempfile::tempdir().unwrap();
    let f = d.path().join("F");
    let o = run(md_cmd(&["encode", T, "--json", "--out", s(&f)]));
    assert_eq!(rc(&o), 0, "{}", se(&o));
    assert_eq!(std::fs::read_to_string(&f).unwrap(), CARD);
    assert_eq!(
        std::fs::metadata(&f).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let plain = run(md_cmd(&["encode", T, "--json"]));
    assert_eq!(so(&o), so(&plain));
    assert_eq!(se(&o), format!("{NOTE}\n"));
}

#[test]
fn o7_json_with_out_grouped_file_unbroken_json() {
    let d = tempfile::tempdir().unwrap();
    let f = d.path().join("F");
    let o = run(md_cmd(&[
        "encode",
        T,
        "--json",
        "--group-size",
        "5",
        "--out",
        s(&f),
    ]));
    assert_eq!(rc(&o), 0, "{}", se(&o));
    assert_eq!(
        std::fs::read_to_string(&f).unwrap(),
        "md1yq pqqxq q8xtw hw4xw n4qh\n"
    );
    let v: serde_json::Value = serde_json::from_str(&so(&o)).unwrap();
    assert_eq!(v["phrase"], "md1yqpqqxqq8xtwhw4xwn4qh");
}

#[test]
fn o8_json_with_unwritable_out_fails() {
    let d = tempfile::tempdir().unwrap();
    let x = d.path().join("nodir").join("x.md1");
    let o = run(md_cmd(&["encode", T, "--json", "--out", s(&x)]));
    assert_eq!(rc(&o), 1);
    assert_eq!(so(&o), "");
    assert_eq!(
        se(&o),
        format!(
            "md: cannot write the output: {}: No such file or directory (os error 2). Nothing was written.\n",
            x.display()
        )
    );
}

#[test]
fn o9_json_with_out_and_full_stdout_keeps_file() {
    if !Path::new("/dev/full").exists() {
        return;
    }
    let d = tempfile::tempdir().unwrap();
    let f = d.path().join("F");
    let full = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();
    let mut c = md_cmd(&["encode", T, "--json", "--out", s(&f)]);
    c.stdout(Stdio::from(full));
    let o = run(c);
    assert_eq!(rc(&o), 1);
    assert_eq!(
        se(&o),
        format!("{NOTE}\nmd: cannot write the output: No space left on device (os error 28)\n")
    );
    assert_eq!(std::fs::read_to_string(&f).unwrap(), CARD);
}

#[test]
fn o10_failed_policy_id_keeps_prior_file() {
    let d = tempfile::tempdir().unwrap();
    let f = d.path().join("F");
    std::fs::write(&f, "PRIOR\n").unwrap();
    let o = run(md_cmd(&[
        "encode",
        W,
        "--policy-id-fingerprint",
        "--out",
        s(&f),
    ]));
    assert_eq!(rc(&o), 1);
    assert_eq!(se(&o), format!("{CODEC}\n"));
    assert_eq!(so(&o), "");
    assert_eq!(std::fs::read_to_string(&f).unwrap(), "PRIOR\n");
}

#[test]
fn o11_failed_policy_id_prints_nothing_on_stdout() {
    let o = run(md_cmd(&["encode", W, "--policy-id-fingerprint"]));
    assert_eq!(rc(&o), 1);
    assert_eq!(so(&o), "");
    assert_eq!(se(&o), format!("{CODEC}\n"));
}

#[test]
fn o12_failed_policy_id_absent_out_stays_absent() {
    let d = tempfile::tempdir().unwrap();
    let n = d.path().join("N");
    let o = run(md_cmd(&[
        "encode",
        W,
        "--policy-id-fingerprint",
        "--out",
        s(&n),
    ]));
    assert_eq!(rc(&o), 1);
    assert!(!n.exists());
}

#[test]
fn o13_failed_policy_id_force_chunked_prints_only_the_error() {
    let o = run(md_cmd(&[
        "encode",
        W,
        "--policy-id-fingerprint",
        "--force-chunked",
    ]));
    assert_eq!(rc(&o), 1);
    assert_eq!(so(&o), "");
    assert_eq!(se(&o), format!("{CODEC}\n"));
}

#[test]
fn o14_failed_policy_id_json_out_keeps_prior_file() {
    let d = tempfile::tempdir().unwrap();
    let f = d.path().join("F");
    std::fs::write(&f, "PRIOR\n").unwrap();
    let o = run(md_cmd(&[
        "encode",
        W,
        "--policy-id-fingerprint",
        "--json",
        "--out",
        s(&f),
    ]));
    assert_eq!(rc(&o), 1);
    assert_eq!(se(&o), format!("{CODEC}\n"));
    assert_eq!(std::fs::read_to_string(&f).unwrap(), "PRIOR\n");
}

#[test]
fn o15_stdio_alias_onto_a_file_names_no_state() {
    if !Path::new("/dev/stdout").exists() {
        return;
    }
    let d = tempfile::tempdir().unwrap();
    let sfile = d.path().join("s.txt");
    std::fs::write(&sfile, "OLD BACKUP\n").unwrap();
    let appended = std::fs::OpenOptions::new()
        .append(true)
        .open(&sfile)
        .unwrap();
    let mut c = fsize(md_cmd(&["encode", T, "--out", "/dev/stdout"]), 0);
    c.stdout(Stdio::from(appended));
    let o = run(c);
    assert_eq!(rc(&o), 1);
    assert_eq!(
        se(&o),
        "md: cannot write the output: /dev/stdout: File too large (os error 27)\n"
    );
    assert_eq!(std::fs::read_to_string(&sfile).unwrap(), "OLD BACKUP\n");
}
