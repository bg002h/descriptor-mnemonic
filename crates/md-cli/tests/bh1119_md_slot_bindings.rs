//! bughunt-1119 row #29 (`design/FIX_bh1119_md_slot_bindings.md`, bh-md-C3): one `--key` and one
//! `--fingerprint` per slot, on `encode`, `verify`, `descriptor` and `address`. Rows are named as in
//! the FIX section 4. Every row passes `--network signet`.

#![allow(missing_docs)]

use assert_cmd::Command;

const KA: &str = "tpubDC8msFGeGuwnKG9Upg7DM2b4DaRqg3CUZa5g8v2SRQ6K4NSkxUgd7HsL2XVWbVm39yBA4LAxysQAm397zwQSQoQgewGiYZqrA9DsP4zbQ1M";
const KB: &str = "tpubDC8msFGeGuwnP2xwTZBBZSie1BLgRAkJhgzpFYTdpGgZNzguXQhNDVWp7mJbHJUjQQvV2myLU9dkx67a7VAUnzY7yT7nvhHj7FgS4oNivvq";
const T: &str = "wpkh(@0/<0;1>/*)";
const T2: &str = "wsh(sortedmulti(1,@0/<0;1>/*,@1/<0;1>/*))";

fn run(args: &[&str]) -> (String, String, i32) {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(args)
        .arg("--network")
        .arg("signet")
        .output()
        .unwrap();
    (
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
        out.status.code().unwrap(),
    )
}

fn k_line(i: u8, a: usize, b: usize) -> String {
    format!(
        "md: @{i}: --key arguments {a} and {b} both bind this slot, with different values; a slot takes exactly one key, and md never picks one for you. Keep the --key @{i}=... you mean and remove the other.\n"
    )
}

fn f_line(i: u8, x: &str, y: &str) -> String {
    format!(
        "md: @{i}: --fingerprint {x} and --fingerprint {y} both bind this slot; a slot takes exactly one master fingerprint, and md never picks one for you. Keep the one you mean and remove the other.\n"
    )
}

fn ek_line(i: u8, a: usize, b: usize) -> String {
    format!(
        "md: @{i}: --key arguments {a} and {b} bind this slot to the same value; give each slot's --key once.\n"
    )
}

fn ef_line(i: u8, x: &str) -> String {
    format!(
        "md: @{i}: --fingerprint {x} is given more than once for this slot; give each slot's --fingerprint once.\n"
    )
}

/// stdout empty, stderr exactly `line`, exit `rc`.
fn refused(args: &[&str], rc: i32, line: &str) {
    let (o, e, c) = run(args);
    assert_eq!(o, "", "stdout must be empty");
    assert_eq!(e, line, "stderr");
    assert_eq!(c, rc, "exit");
}

fn card() -> Vec<String> {
    let (o, e, c) = run(&["encode", T, "--key", &format!("@0={KA}")]);
    assert_eq!(c, 0, "{e}");
    o.lines().map(str::to_owned).collect()
}

fn ka() -> String {
    format!("@0={KA}")
}
fn kb() -> String {
    format!("@0={KB}")
}

#[test]
fn s1_address_two_keys_one_slot() {
    refused(
        &["address", "--template", T, "--key", &ka(), "--key", &kb()],
        2,
        &k_line(0, 1, 2),
    );
}

#[test]
fn s2_address_two_keys_one_slot_reversed() {
    refused(
        &["address", "--template", T, "--key", &kb(), "--key", &ka()],
        2,
        &k_line(0, 1, 2),
    );
}

#[test]
fn s3_descriptor_two_keys_one_slot() {
    refused(
        &[
            "descriptor",
            "--template",
            T,
            "--key",
            &ka(),
            "--key",
            &kb(),
        ],
        2,
        &k_line(0, 1, 2),
    );
}

#[test]
fn s4_descriptor_two_keys_one_slot_reversed() {
    refused(
        &[
            "descriptor",
            "--template",
            T,
            "--key",
            &kb(),
            "--key",
            &ka(),
        ],
        2,
        &k_line(0, 1, 2),
    );
}

#[test]
fn s5_descriptor_two_fingerprints_one_slot() {
    refused(
        &[
            "descriptor",
            "--template",
            T,
            "--key",
            &ka(),
            "--fingerprint",
            "@0=11223344",
            "--fingerprint",
            "@0=aabbccdd",
        ],
        2,
        &f_line(0, "11223344", "aabbccdd"),
    );
}

#[test]
fn s6_descriptor_two_fingerprints_one_slot_reversed() {
    refused(
        &[
            "descriptor",
            "--template",
            T,
            "--key",
            &ka(),
            "--fingerprint",
            "@0=aabbccdd",
            "--fingerprint",
            "@0=11223344",
        ],
        2,
        &f_line(0, "aabbccdd", "11223344"),
    );
}

#[test]
fn s7_address_two_fingerprints_one_slot() {
    refused(
        &[
            "address",
            "--template",
            T,
            "--key",
            &ka(),
            "--fingerprint",
            "@0=11223344",
            "--fingerprint",
            "@0=aabbccdd",
        ],
        2,
        &f_line(0, "11223344", "aabbccdd"),
    );
}

#[test]
fn s8_descriptor_two_bracket_paths_one_slot() {
    refused(
        &[
            "descriptor",
            "--template",
            T,
            "--key",
            &format!("@0=[73c5da0a/84'/1'/0']{KA}"),
            "--key",
            &format!("@0=[73c5da0a/84'/1'/1']{KA}"),
        ],
        2,
        &k_line(0, 1, 2),
    );
}

#[test]
fn s9_descriptor_two_bracket_fingerprints_one_slot() {
    refused(
        &[
            "descriptor",
            "--template",
            T,
            "--key",
            &format!("@0=[73c5da0a/84'/1'/0']{KA}"),
            "--key",
            &format!("@0=[aabbccdd/84'/1'/0']{KA}"),
        ],
        2,
        &k_line(0, 1, 2),
    );
}

#[test]
fn s10_descriptor_bare_and_bracketed_one_slot() {
    refused(
        &[
            "descriptor",
            "--template",
            T,
            "--key",
            &ka(),
            "--key",
            &format!("@0=[73c5da0a/84'/1'/0']{KA}"),
        ],
        2,
        &k_line(0, 1, 2),
    );
}

#[test]
fn s11_encode_two_keys_one_slot() {
    refused(
        &["encode", T, "--key", &ka(), "--key", &kb()],
        2,
        &k_line(0, 1, 2),
    );
}

#[test]
fn s12_encode_two_fingerprints_one_slot() {
    refused(
        &[
            "encode",
            T,
            "--key",
            &ka(),
            "--fingerprint",
            "@0=11223344",
            "--fingerprint",
            "@0=aabbccdd",
        ],
        2,
        &f_line(0, "11223344", "aabbccdd"),
    );
}

#[test]
fn s13_verify_two_keys_one_slot() {
    let c = card();
    let (ka, kb) = (ka(), kb());
    let mut a: Vec<&str> = vec!["verify"];
    a.extend(c.iter().map(String::as_str));
    a.extend(["--template", T, "--key", &ka, "--key", &kb]);
    refused(&a, 2, &k_line(0, 1, 2));
}

#[test]
fn s14_verify_two_fingerprints_one_slot() {
    let c = card();
    let ka = ka();
    let mut a: Vec<&str> = vec!["verify"];
    a.extend(c.iter().map(String::as_str));
    a.extend([
        "--template",
        T,
        "--key",
        &ka,
        "--fingerprint",
        "@0=11223344",
        "--fingerprint",
        "@0=aabbccdd",
    ]);
    refused(&a, 2, &f_line(0, "11223344", "aabbccdd"));
}

#[test]
fn s15_first_conflict_in_argument_order() {
    let (a0, a1) = (format!("@1={KB}"), format!("@1={KA}"));
    refused(
        &[
            "descriptor",
            "--template",
            T2,
            "--key",
            &ka(),
            "--key",
            &a0,
            "--key",
            &a1,
            "--key",
            &kb(),
        ],
        2,
        &k_line(1, 2, 3),
    );
}

#[test]
fn s16_keys_before_fingerprints() {
    refused(
        &[
            "descriptor",
            "--template",
            T,
            "--key",
            &ka(),
            "--key",
            &kb(),
            "--fingerprint",
            "@0=11223344",
            "--fingerprint",
            "@0=aabbccdd",
        ],
        2,
        &k_line(0, 1, 2),
    );
}

#[test]
fn s17_one_key_per_slot_still_composes() {
    let (o, e, c) = run(&[
        "descriptor",
        "--template",
        T2,
        "--key",
        &ka(),
        "--key",
        &format!("@1={KB}"),
    ]);
    assert_eq!(c, 0, "{e}");
    assert!(o.contains("#5zanlgag"), "{o}");
}

#[test]
fn e1_descriptor_same_key_twice() {
    refused(
        &[
            "descriptor",
            "--template",
            T,
            "--key",
            &ka(),
            "--key",
            &ka(),
        ],
        2,
        &ek_line(0, 1, 2),
    );
}

#[test]
fn e2_encode_same_key_twice() {
    refused(
        &["encode", T, "--key", &ka(), "--key", &ka()],
        2,
        &ek_line(0, 1, 2),
    );
}

#[test]
fn e3_descriptor_same_fingerprint_twice() {
    refused(
        &[
            "descriptor",
            "--template",
            T,
            "--key",
            &ka(),
            "--fingerprint",
            "@0=11223344",
            "--fingerprint",
            "@0=11223344",
        ],
        2,
        &ef_line(0, "11223344"),
    );
}

#[test]
fn e4_encode_same_fingerprint_twice() {
    refused(
        &[
            "encode",
            T,
            "--key",
            &ka(),
            "--fingerprint",
            "@0=11223344",
            "--fingerprint",
            "@0=11223344",
        ],
        2,
        &ef_line(0, "11223344"),
    );
}

#[test]
fn e5_descriptor_same_fingerprint_two_spellings() {
    refused(
        &[
            "descriptor",
            "--template",
            T,
            "--key",
            &ka(),
            "--fingerprint",
            "@0=AABBCCDD",
            "--fingerprint",
            "@0=0xaabbccdd",
        ],
        2,
        &ef_line(0, "aabbccdd"),
    );
}

#[test]
fn e6_descriptor_same_bracket_two_spellings() {
    refused(
        &[
            "descriptor",
            "--template",
            T,
            "--key",
            &format!("@0=[73c5da0a/84'/1'/0']{KA}"),
            "--key",
            &format!("@0=[73C5DA0A/84h/1h/0h]{KA}"),
        ],
        2,
        &ek_line(0, 1, 2),
    );
}
