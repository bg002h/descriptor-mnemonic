//! Bughunt-1119 #30 Q1: the coordinator verdict `md descriptor` prints for a
//! wallet where no key has a multipath names no spelling, so it never claims
//! the `<0;1>` refusal or prints its `--chain 1` remedy (refused there).

#![allow(missing_docs)]

use std::process::Command;

const X: &str = "xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf";
const Y: &str = "xpub6DzhyrnFFYQ1HimDiM388xHnDiRPNdZJFBmmxge3Y1WWcHLtMJLfRuhRHqnQCPbTj3fGKTuKFLHzzwpJkp5Dtc3UtLKZKaVZe1yqMBXd6Vk";

const NO_FORM_BLOCK: &str =
    "note: coordinators for this descriptor (verified versions only; newer: unmeasured):
  Liana 8.0-15.0: refuses (legacy wrapper)
  Nunchuk 2.1.1: unproven (not measured for this policy)
  Bitcoin Core 24.2-31.1: unproven (not measured for this policy)
";

const WPKH_X_STAR: &str = "wpkh(xpub661MyMwAqRbcGQnC8zMGwRc4EXYJCgXrxx9kXtw1RXqu4TcW26PxHqssgp6sU4N5CR5o9QcUZPG31fzPeUHEVPkFit1WpVZTmqZLzpvZG2s/*)#fn4fyxff\n";

fn md(args: &[&str]) -> (i32, String, String) {
    let o = Command::new(assert_cmd::cargo::cargo_bin("md"))
        .args(args)
        .output()
        .expect("invoke md");
    (
        o.status.code().expect("md exited normally"),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

fn key(slot: &str, xpub: &str) -> String {
    format!("{slot}={xpub}")
}

#[test]
fn q1_r1_descriptor_single_path_no_chain_names_no_form() {
    let kx = key("@0", X);
    let (code, out, err) = md(&["descriptor", "--template", "wpkh(@0/*)", "--key", &kx]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, WPKH_X_STAR);
    assert!(err.contains(NO_FORM_BLOCK), "{err}");
    assert!(!err.contains("--chain"), "{err}");
    assert!(!err.contains(" form"), "{err}");
}

#[test]
fn q1_r2_descriptor_single_path_chain0_same_verdict_as_no_chain() {
    let kx = key("@0", X);
    let (code, out, err) = md(&[
        "descriptor",
        "--template",
        "wpkh(@0/*)",
        "--key",
        &kx,
        "--chain",
        "0",
    ]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, WPKH_X_STAR);
    assert!(err.contains(NO_FORM_BLOCK), "{err}");
    assert!(!err.contains(" form"), "{err}");
}

#[test]
fn q1_r3_descriptor_single_alt_multipath_names_no_form() {
    let kx = key("@0", X);
    let (code, out, err) = md(&["descriptor", "--template", "wpkh(@0/<5>/*)", "--key", &kx]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        out,
        "wpkh(xpub661MyMwAqRbcGQnC8zMGwRc4EXYJCgXrxx9kXtw1RXqu4TcW26PxHqssgp6sU4N5CR5o9QcUZPG31fzPeUHEVPkFit1WpVZTmqZLzpvZG2s/5/*)#q45wdwf5\n"
    );
    assert!(err.contains(NO_FORM_BLOCK), "{err}");
    assert!(!err.contains("--chain"), "{err}");
}

/// Every `--chain N` a no-chain run prints is accepted as printed.
#[test]
fn q1_r4_every_printed_chain_remedy_runs() {
    let kx = key("@0", X);
    let ky = key("@1", Y);
    let cases: &[&[&str]] = &[
        &["--template", "wpkh(@0/*)", "--key", &kx],
        &["--template", "wpkh(@0/<5>/*)", "--key", &kx],
        &["--template", "wpkh(@0/<0;1>/*)", "--key", &kx],
        &[
            "--template",
            "wsh(multi(2,@0/<0;1>/*,@1/*))",
            "--key",
            &kx,
            "--key",
            &ky,
        ],
        &[
            "--template",
            "wsh(multi(2,@0/*,@1/<0;1;2>/*))",
            "--key",
            &kx,
            "--key",
            &ky,
        ],
    ];
    for case in cases {
        let mut args = vec!["descriptor"];
        args.extend_from_slice(case);
        let (code, _, err) = md(&args);
        assert_eq!(code, 0, "{args:?}: {err}");
        for n in err.split("--chain ").skip(1) {
            let n: String = n.chars().take_while(char::is_ascii_digit).collect();
            let mut again = args.clone();
            again.extend(["--chain", n.as_str()]);
            let (code, _, err2) = md(&again);
            assert_eq!(code, 0, "printed remedy {again:?} failed: {err2}");
        }
    }
}

#[test]
fn q1_c1_descriptor_multipath_no_chain_unchanged() {
    let kx = key("@0", X);
    let (code, _, err) = md(&["descriptor", "--template", "wpkh(@0/<0;1>/*)", "--key", &kx]);
    assert_eq!(code, 0, "{err}");
    assert!(
        err.contains(
            "note: coordinators for this descriptor, multipath form (verified versions only; newer: unmeasured):
  Liana 8.0-15.0: refuses (legacy wrapper)
  Nunchuk 2.1.1: unproven (not measured for this policy)
  Bitcoin Core 24.2-28.4: refuses (the <0;1> multipath spelling (use --chain 0 and --chain 1))
  Bitcoin Core 29.4-31.1: unproven (not measured for this policy)
"
        ),
        "{err}"
    );
}

#[test]
fn q1_c2_descriptor_mixed_and_override_multipath_keep_multipath_form() {
    let kx = key("@0", X);
    let ky = key("@1", Y);
    for t in [
        "wsh(multi(2,@0/<0;1>/*,@1/*))",
        "wsh(multi(2,@0/*,@1/<0;1;2>/*))",
    ] {
        let (code, _, err) = md(&["descriptor", "--template", t, "--key", &kx, "--key", &ky]);
        assert_eq!(code, 0, "{t}: {err}");
        assert!(
            err.contains("note: coordinators for this descriptor, multipath form ("),
            "{t}: {err}"
        );
        assert!(
            err.contains(
                "  Bitcoin Core 24.2-28.4: refuses (the <0;1> multipath spelling (use --chain 0 and --chain 1))\n"
            ),
            "{t}: {err}"
        );
    }
}
