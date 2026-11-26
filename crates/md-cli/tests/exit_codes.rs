#![allow(missing_docs)]

use assert_cmd::Command;

#[test]
fn no_args_returns_64() {
    Command::cargo_bin("md").unwrap().assert().code(64);
}

#[test]
fn unknown_subcommand_returns_64() {
    Command::cargo_bin("md")
        .unwrap()
        .arg("bogus")
        .assert()
        .code(64);
}

#[test]
fn encode_bad_template_returns_2() {
    Command::cargo_bin("md")
        .unwrap()
        .args(["encode", "this is not a template"])
        .assert()
        .code(2);
}

#[test]
fn decode_bad_string_returns_2() {
    Command::cargo_bin("md")
        .unwrap()
        .args(["decode", "not-a-valid-md-string"])
        .assert()
        .code(2);
}

// ─── The constellation exit codes (mnemonic-engrave
// `design/SPEC_constellation_exit_codes.md`, §3 and §7a) ─────────────────────

/// The three chunks of `md address --help`'s example card.
const C1: &str = "md1fwkhxpqpqztvyyy4qqxqqks883w6pg25z9m5rru59s2su80aw2q4qg9e343jwu6tsn";
const C2: &str = "md1fwkhxpq0yxs7s5yl6smtcxjz9m806prlsm794tkxqs6806lhaeh6qwafumf4dq3lcd";
const C3: &str = "md1fwkhxpq3umfj075dhzfzvynh66n94j5lcxlmx9ayav9mj0jjqdxqaazpst259n";
/// `C2` with one substitution (`md repair --help`'s example).
const C2_BAD: &str = "md1fwkhxpq0yxs7q5yl6smtcxjz9m806prlsm794tkxqs6806lhaeh6qwafumf4dq3lcd";
/// A keyless single-string card (`md decode --help`).
const OK: &str = "md1yqpqqxqq8xtwhw4xwn4qh";

fn run(args: &[&str]) -> (i32, String, String) {
    run_stdin(args, b"")
}

fn run_stdin(args: &[&str], stdin: &[u8]) -> (i32, String, String) {
    let o = Command::cargo_bin("md")
        .unwrap()
        .args(args)
        .write_stdin(stdin.to_vec())
        .output()
        .unwrap();
    (
        o.status.code().expect("exited normally"),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

/// R11: the root `--help` ends with the spec's legend, word for word.
#[test]
fn root_help_ends_with_the_exit_status_legend() {
    let (code, out, _) = run(&["--help"]);
    assert_eq!(code, 0);
    assert!(
        out.trim_end().ends_with(
            "Exit status: 0 = done, every check passed. 64 = bad command line, nothing ran. \
             101 = internal error. 130 = stopped by the operator. Other codes from 128 up = \
             killed by a signal. Otherwise the sum (1 to 63) of: 1 a check said no; 2 an input \
             is invalid; 4 verify the result before trusting it; 8 refused (network mismatch, \
             secret on the command line, unsafe output); 16 a correction was applied; 32 a \
             file, device or helper failed."
        ),
        "{out}"
    );
}

/// Decision 2: every parse error is 64, non-UTF-8 argv included.
#[test]
fn usage_errors_are_64() {
    for args in [
        &["decode", "--bogus"][..],
        &["decode"][..],
        &["encode"][..],
        &["descriptor", OK, "--emit", "bogus"][..],
    ] {
        assert_eq!(run(args).0, 64, "{args:?}");
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let o = std::process::Command::new(assert_cmd::cargo::cargo_bin("md"))
            .arg("decode")
            .arg(std::ffi::OsStr::from_bytes(b"\xff\xfe"))
            .output()
            .unwrap();
        assert_eq!(o.status.code(), Some(64), "non-UTF-8 argv");
    }
    // R2: a usage error is 64 before any input file is read, so a missing
    // file does not turn it into 32.
    let missing = "/nonexistent/md-exit-codes/missing.txt";
    let (code, _, err) = run(&["encode", "--in", missing, "--force-long-code"]);
    assert_eq!(code, 64, "{err}");
    for verb in ["descriptor", "address"] {
        let (code, _, err) = run(&[verb, "--from-mk1-file", missing]);
        assert_eq!(code, 64, "{verb}: {err}");
        let (code, _, err) = run(&[verb, "--from-mk1", "mk1qqqq", "--from-mk1-file", missing]);
        assert_eq!(code, 64, "{verb}: {err}");
    }
}

/// Decision 3 and test 2: empty input and a file that is not text are 2; a
/// missing input file is 32 (IO).
#[test]
fn empty_input_is_2_and_a_missing_file_is_32() {
    let dir = std::env::temp_dir().join(format!("md-exit-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let bin = dir.join("bin.dat");
    std::fs::write(&bin, b"\xff\xfe\n").unwrap();
    let empty = dir.join("empty.txt");
    std::fs::write(&empty, b"").unwrap();
    let missing = dir.join("missing.txt");
    assert_eq!(run(&["decode", "-"]).0, 2, "empty stdin");
    assert_eq!(run(&["decode", "--in", empty.to_str().unwrap()]).0, 2);
    assert_eq!(run(&["decode", "--in", bin.to_str().unwrap()]).0, 2);
    for verb in ["decode", "decompose"] {
        let (code, _, err) = run_stdin(&[verb, "-"], b"\xff\xfe\n");
        assert_eq!(code, 2, "non-UTF-8 stdin on {verb}: {err}");
    }
    assert_eq!(run(&["decode", "--in", missing.to_str().unwrap()]).0, 32);
    assert_eq!(run(&["encode", "--in", missing.to_str().unwrap()]).0, 32);
    let (code, _, err) = run(&[
        "descriptor",
        OK,
        "--from-mk1-file",
        missing.to_str().unwrap(),
    ]);
    assert_eq!(code, 32, "{err}");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Test 2: an output directory that cannot be made is 32, and so is a
/// feature this build lacks.
#[test]
fn io_and_missing_features_are_32() {
    assert_eq!(run(&["gen-man", "--out", "/proc/nonexistent/man"]).0, 32);
    if !cfg!(feature = "cli-compiler") {
        assert_eq!(run(&["compile", "pk(A)", "--context", "tap"]).0, 32);
    }
}

/// Test 6: an invalid card is 2 on every reading verb, `inspect` included.
#[test]
fn an_invalid_card_is_2_on_every_reading_verb() {
    for verb in ["decode", "inspect", "bytecode", "repair", "shape-key"] {
        assert_eq!(run(&[verb, "not-a-card"]).0, 2, "{verb}");
    }
    assert_eq!(
        run(&["verify", "not-a-card", "--template", "wpkh(@0/<0;1>/*)"]).0,
        2
    );
}

/// Test 4: a verify mismatch and NOT spend-equal are 1.
#[test]
fn a_comparison_that_came_out_no_is_1() {
    assert_eq!(run(&["verify", OK, "--template", "wpkh(@0/<2;3>/*)"]).0, 1);
    assert_eq!(run(&["verify", OK, "--template", "wpkh(@0/<0;1>/*)"]).0, 0);
}

/// R2: a `--chain` the card does not have is 2, and fails BEFORE the
/// `--verify-against` verdict is printed.
#[test]
fn chain_out_of_range_is_2_before_the_verdict() {
    let (code, out, err) = run(&[
        "descriptor",
        C1,
        C2,
        C3,
        "--verify-against",
        OK,
        "--chain",
        "7",
    ]);
    assert_eq!(code, 2, "{err}");
    assert!(out.is_empty(), "{out}");
    assert!(
        !err.contains("spend-equal"),
        "the verdict printed first: {err}"
    );
}

/// Test 3: a key of the other network is 8; an xprv is 2.
#[test]
fn a_wrong_network_key_is_8_and_an_xprv_is_2() {
    const TPUB: &str = "tpubDC8msFGeGuwnKG9Upg7DM2b4DaRqg3CUZa5g8v2SRQ6K4NSkxUgd7HsL2XVWbVm39yBA4LAxysQAm397zwQSQoQgewGiYZqrA9DsP4zbQ1M";
    // BIP-32 test vector 1, master: a published private key.
    const XPRV: &str = "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWbtVgs3ccS8wYZmCRDRcGWDdKKzfLHzSc1Vp5wN";
    let t = "wpkh(@0/48'/0'/0'/2'/<0;1>/*)";
    let (code, _, err) = run(&["encode", t, "--key", &format!("@0={TPUB}")]);
    assert_eq!(code, 8, "{err}");
    let (code, _, err) = run(&["encode", t, "--key", &format!("@0={XPRV}")]);
    assert_eq!(code, 2, "{err}");
}

/// Test 5: an output that is one of the inputs is refused, 8.
#[test]
fn an_output_naming_its_input_is_8() {
    let dir = std::env::temp_dir().join(format!("md-exit-out-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("tpl.txt");
    std::fs::write(&f, "wpkh(@0/<0;1>/*)\n").unwrap();
    let p = f.to_str().unwrap();
    assert_eq!(run(&["encode", "--in", p, "--out", p]).0, 8);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Decision 7: an incomplete set is corrected chunk by chunk and printed:
/// 16 when a correction was applied, 0 when clean. A set that reassembles
/// is 16; a single string's correction (no second check) is 20.
#[test]
fn repair_codes_follow_decisions_6_and_7() {
    let (code, out, _) = run(&["repair", C1]);
    assert_eq!((code, out.as_str()), (0, format!("{C1}\n").as_str()));
    let (code, out, err) = run(&["repair", C2_BAD]);
    assert_eq!(code, 16, "{err}");
    assert!(out.ends_with(&format!("{C2}\n")), "{out}");
    assert!(err.contains("incomplete (1 of 3 chunks)"), "{err}");
    let (code, out, _) = run(&["repair", C1, C2_BAD]);
    assert_eq!(code, 16);
    assert!(out.ends_with(&format!("{C1}\n{C2}\n")), "{out}");
    assert_eq!(run(&["repair", C1, C2_BAD, C3]).0, 16);
    assert_eq!(run(&["repair", C1, C2, C3]).0, 0);
    let (code, _, err) = run(&["repair", "md1yqpqqxqq8xtwhw4xwn4qq"]);
    assert_eq!(code, 20, "{err}");
    assert!(err.contains("UNVERIFIED"), "{err}");
    assert_eq!(run(&["repair", "md1qqqqqqqqqqqqqqqqqqqqqq"]).0, 2);
    // A set that is not short of chunks is not "incomplete" (I-1, M-6):
    // more chunks than the set holds, a chunk given twice, or a chunk whose
    // index is outside its set is 2 with nothing printed.
    for args in [
        &["repair", C1, C2_BAD, C3, C2_BAD][..],
        &["repair", C1, C2, C3, C2][..],
        &["repair", C1, C1][..],
        &["repair", C1, C2_BAD, C2_BAD][..],
    ] {
        let (code, out, err) = run(args);
        assert_eq!((code, out.as_str()), (2, ""), "{args:?}: {err}");
        assert!(!err.contains("incomplete"), "{args:?}: {err}");
    }
    let stray = with_chunk_index(C1, 40);
    let (code, out, err) = run(&["repair", &stray]);
    assert_eq!((code, out.as_str()), (2, ""), "{err}");
}

/// `chunk` with its header's index field set to `index`, re-checksummed.
fn with_chunk_index(chunk: &str, index: u64) -> String {
    use md_codec::bitstream::{BitReader, BitWriter};
    let (bytes, bits) = md_codec::codex32::unwrap_string(chunk).unwrap();
    let mut r = BitReader::with_bit_limit(&bytes, bits);
    let mut w = BitWriter::new();
    w.write_bits(r.read_bits(31).unwrap(), 31); // version, flag, id, count
    r.read_bits(6).unwrap();
    w.write_bits(index, 6);
    for _ in 37..bits {
        w.write_bits(r.read_bits(1).unwrap(), 1);
    }
    md_codec::codex32::wrap_payload(&w.into_bytes(), bits).unwrap()
}
