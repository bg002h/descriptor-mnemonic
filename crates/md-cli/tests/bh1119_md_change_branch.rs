//! FIX_bh1119_md_change_branch (bughunt-1119 row #30): `md descriptor --change`
//! / `--chain N` refuses a branch no key has. Each test is named exactly as its
//! TRACE row (`design/FIX_bh1119_md_change_branch.md` section 4).

#![allow(missing_docs)]

use std::process::Command;

const Y: &str = "xpub6DzhyrnFFYQ1HimDiM388xHnDiRPNdZJFBmmxge3Y1WWcHLtMJLfRuhRHqnQCPbTj3fGKTuKFLHzzwpJkp5Dtc3UtLKZKaVZe1yqMBXd6Vk";

const WPKH_X_STAR: &str = "wpkh(xpub661MyMwAqRbcGQnC8zMGwRc4EXYJCgXrxx9kXtw1RXqu4TcW26PxHqssgp6sU4N5CR5o9QcUZPG31fzPeUHEVPkFit1WpVZTmqZLzpvZG2s/*)#fn4fyxff\n";
const KX: &str = "@0=xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf";

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

fn oor(n: u32, w: u32) -> String {
    format!("md: codec error: chain index {n} out of range; use-site multipath alt-count is {w}\n")
}

#[test]
fn r1_descriptor_change_on_single_chain_wallet_refused() {
    let (code, out, err) = md(&[
        "descriptor",
        "--template",
        "wpkh(@0/*)",
        "--key",
        KX,
        "--change",
    ]);
    assert_eq!((code, out, err), (1, String::new(), oor(1, 1)));
}

#[test]
fn r2_descriptor_chain2_on_single_chain_wallet_refused() {
    let (code, out, err) = md(&[
        "descriptor",
        "--template",
        "wpkh(@0/*)",
        "--key",
        KX,
        "--chain",
        "2",
    ]);
    assert_eq!((code, out, err), (1, String::new(), oor(2, 1)));
}

#[test]
fn r3_descriptor_change_json_on_single_chain_wallet_refused() {
    let (code, out, err) = md(&[
        "descriptor",
        "--template",
        "wpkh(@0/*)",
        "--key",
        KX,
        "--change",
        "--json",
    ]);
    assert_eq!((code, out, err), (1, String::new(), oor(1, 1)));
}

#[test]
fn r5_descriptor_chain1_hardened_wildcard_single_chain_refused() {
    let (code, out, err) = md(&[
        "descriptor",
        "--template",
        "wpkh(@0/*h)",
        "--key",
        KX,
        "--chain",
        "1",
    ]);
    assert_eq!((code, out, err), (1, String::new(), oor(1, 1)));
}

#[test]
fn c1_descriptor_chain0_on_single_chain_wallet_unchanged() {
    let (code, out, _) = md(&[
        "descriptor",
        "--template",
        "wpkh(@0/*)",
        "--key",
        KX,
        "--chain",
        "0",
    ]);
    assert_eq!(code, 0);
    assert_eq!(out, WPKH_X_STAR);
}

#[test]
fn c2_descriptor_change_on_mixed_wallet_follows_address_rule() {
    let ky = format!("@1={Y}");
    let args = [
        "--template",
        "wsh(multi(2,@0/<0;1>/*,@1/*))",
        "--key",
        KX,
        "--key",
        ky.as_str(),
        "--change",
    ];
    let mut d = vec!["descriptor"];
    d.extend_from_slice(&args);
    let (code, out, err) = md(&d);
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        out,
        "wsh(multi(2,xpub661MyMwAqRbcGQnC8zMGwRc4EXYJCgXrxx9kXtw1RXqu4TcW26PxHqssgp6sU4N5CR5o9QcUZPG31fzPeUHEVPkFit1WpVZTmqZLzpvZG2s/1/*,xpub661MyMwAqRbcG5161axqKvt7Kx7XBe4pWwNMvgbwdffMtvzPnXA85ToGs3EtpEVAAYf9PggopL6xt7ySJw5Kc7ELWVcwopEjYXVHaHy6tFz/*))#4a8tvuad\n"
    );
    let mut a = vec!["address"];
    a.extend_from_slice(&args);
    let (code, _, err) = md(&a);
    assert_eq!(code, 0, "{err}");
}

#[test]
fn c3_descriptor_chain2_counts_per_slot_override() {
    let ky = format!("@1={Y}");
    let (code, out, err) = md(&[
        "descriptor",
        "--template",
        "wsh(multi(2,@0/*,@1/<0;1;2>/*))",
        "--key",
        KX,
        "--key",
        ky.as_str(),
        "--chain",
        "2",
    ]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        out,
        "wsh(multi(2,xpub661MyMwAqRbcGQnC8zMGwRc4EXYJCgXrxx9kXtw1RXqu4TcW26PxHqssgp6sU4N5CR5o9QcUZPG31fzPeUHEVPkFit1WpVZTmqZLzpvZG2s/*,xpub661MyMwAqRbcG5161axqKvt7Kx7XBe4pWwNMvgbwdffMtvzPnXA85ToGs3EtpEVAAYf9PggopL6xt7ySJw5Kc7ELWVcwopEjYXVHaHy6tFz/2/*))#250az8uk\n"
    );
}

#[test]
fn c4_address_change_on_single_chain_wallet_unchanged() {
    let (code, out, err) = md(&[
        "address",
        "--template",
        "wpkh(@0/*)",
        "--key",
        KX,
        "--change",
    ]);
    assert_eq!((code, out, err), (1, String::new(), oor(1, 1)));
}

#[test]
fn c5_descriptor_chain2_on_two_branch_wallet_unchanged() {
    let (code, out, err) = md(&[
        "descriptor",
        "--template",
        "wpkh(@0/<0;1>/*)",
        "--key",
        KX,
        "--chain",
        "2",
    ]);
    assert_eq!((code, out, err), (1, String::new(), oor(2, 2)));
}
