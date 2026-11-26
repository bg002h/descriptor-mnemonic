//! `md compose --scheme` is the flag's name; `--preset`, its old spelling, is
//! kept as a HIDDEN clap alias. The alias must parse to the very same argument:
//! byte-identical stdout and stderr and the same exit code as `--scheme`, on
//! success and on refusal, and `md compose --help` must show `--scheme` and
//! never `--preset`.

use assert_cmd::Command;

fn md(args: &[&str]) -> (Vec<u8>, Vec<u8>, Option<i32>) {
    let out = Command::cargo_bin("md")
        .expect("md binary")
        .args(args)
        .output()
        .expect("run md");
    (out.stdout, out.stderr, out.status.code())
}

/// Runs `md compose --wrapper <w> <flag> <spec> <extra...>` with `flag` set
/// to each spelling (both the separate-value and the `=` form) and asserts
/// every run matches the `--scheme <spec>` run byte for byte.
fn assert_alias_identical(wrapper: &str, spec: &str, extra: &[&str]) -> Option<i32> {
    let run = |flag: &str, joined: bool| {
        let eq = format!("{flag}={spec}");
        let mut args = vec!["compose", "--wrapper", wrapper];
        if joined {
            args.push(&eq);
        } else {
            args.extend_from_slice(&[flag, spec]);
        }
        args.extend_from_slice(extra);
        md(&args)
    };
    let want = run("--scheme", false);
    for (flag, joined) in [("--scheme", true), ("--preset", false), ("--preset", true)] {
        let got = run(flag, joined);
        let form = if joined { "=" } else { " " };
        assert_eq!(
            got.2, want.2,
            "{flag}{form}{spec}: exit code differs from --scheme"
        );
        assert_eq!(
            String::from_utf8_lossy(&got.0),
            String::from_utf8_lossy(&want.0),
            "{flag}{form}{spec}: stdout differs from --scheme"
        );
        assert_eq!(
            String::from_utf8_lossy(&got.1),
            String::from_utf8_lossy(&want.1),
            "{flag}{form}{spec}: stderr differs from --scheme"
        );
    }
    want.2
}

#[test]
fn preset_alias_is_byte_identical_to_scheme() {
    // Success, plain text and --json.
    assert_eq!(
        assert_alias_identical("wsh", "plain-multisig,2of3", &[]),
        Some(0)
    );
    assert_eq!(
        assert_alias_identical("tr", "kofn-recovery,2of3,older=26280", &["--json"]),
        Some(0)
    );
    // Refusals: an unknown name, and a parameter error.
    assert_ne!(
        assert_alias_identical("wsh", "frobnicate,2of3", &[]),
        Some(0)
    );
    assert_ne!(
        assert_alias_identical("wsh", "kofn-recovery,2of3", &[]),
        Some(0)
    );
    // Clap's own refusal: the alias still conflicts with --path.
    assert_ne!(
        assert_alias_identical("wsh", "plain-multisig,2of3", &["--path", "2of3"]),
        Some(0)
    );
}

#[test]
fn compose_refusals_name_the_scheme_flag() {
    let (_, err, code) = md(&["compose", "--wrapper", "wsh", "--preset", "frobnicate,2of3"]);
    let err = String::from_utf8_lossy(&err);
    assert_eq!(code, Some(1), "{err}");
    assert!(err.contains("--scheme frobnicate"), "{err}");
    assert!(!err.contains("preset"), "{err}");

    let (_, err, code) = md(&[
        "compose",
        "--wrapper",
        "wsh",
        "--preset",
        "plain-multisig,2of3",
        "--path",
        "2of3",
    ]);
    let err = String::from_utf8_lossy(&err);
    assert_ne!(code, Some(0), "{err}");
    assert!(err.contains("--scheme"), "{err}");
    assert!(!err.contains("--preset"), "{err}");
}

#[test]
fn compose_help_shows_scheme_and_not_preset() {
    for help in ["-h", "--help"] {
        let (out, err, code) = md(&["compose", help]);
        let out = String::from_utf8_lossy(&out);
        assert_eq!(code, Some(0), "{}", String::from_utf8_lossy(&err));
        assert!(out.contains("--scheme <SCHEME>"), "compose {help}:\n{out}");
        assert!(
            out.contains("named wallet scheme"),
            "compose {help}:\n{out}"
        );
        assert!(
            !out.contains("--preset"),
            "compose {help} still names the old spelling:\n{out}"
        );
    }
}
