//! FIX_bh1119_md_encoder_admission, md-cli rows R6 and K4: `md encode`
//! refuses a template nested deeper than the decoder's limit.
//! See `design/FIX_bh1119_md_encoder_admission.md`.

#![allow(missing_docs)]

use std::process::Command;

fn template(n: usize) -> String {
    format!("wsh({}:pk(@0/<0;1>/*))", "n".repeat(n))
}

fn run(args: &[&str]) -> (i32, String, String) {
    let o = Command::new(assert_cmd::cargo::cargo_bin("md"))
        .args(args)
        .output()
        .expect("invoke md");
    (
        o.status.code().expect("normal exit"),
        String::from_utf8(o.stdout).unwrap(),
        String::from_utf8(o.stderr).unwrap(),
    )
}

#[test]
fn bh_md_row10_cli_depth_127_encode_refused() {
    let t = template(127);
    let got = run(&["encode", &t, "--path", "bip84"]);
    assert_eq!(
        got,
        (
            1,
            String::new(),
            "md: codec error: decode recursion depth 128 exceeded maximum 128\n".to_string()
        )
    );
}

#[test]
fn bh_md_row10_cli_depth_126_encodes_and_decodes() {
    let t = template(126);
    let (code, out, _) = run(&["encode", &t, "--path", "bip84"]);
    assert_eq!(code, 0);
    let lines: Vec<&str> = out.lines().filter(|l| l.starts_with("md1")).collect();
    assert!(!lines.is_empty());
    let mut args = vec!["decode"];
    args.extend(lines);
    let (dcode, _, derr) = run(&args);
    assert_eq!(dcode, 0, "decode stderr: {derr}");
}
