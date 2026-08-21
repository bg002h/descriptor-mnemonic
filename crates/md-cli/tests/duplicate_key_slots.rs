#![allow(missing_docs)]
//! F-218: `md encode` must refuse a policy that seats ONE key in two slots.
//!
//! Such a policy reads as k-of-n and is satisfiable by fewer parties than it
//! names: with one key twice, its holder produces two of the required
//! signatures alone. The script is legal; the wallet is not what it looks like.
//!
//! The SeedHammer fork has refused this since its multisig build flow shipped,
//! with a named message giving both slots — but only where the DEVICE assembled
//! the policy. A card built by this CLI never reached that check, so the host
//! minted freely what the device would have refused.

use assert_cmd::Command;

const K0: &str = "xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf";
const K1: &str = "xpub6DzhyrnFFYQ1HimDiM388xHnDiRPNdZJFBmmxge3Y1WWcHLtMJLfRuhRHqnQCPbTj3fGKTuKFLHzzwpJkp5Dtc3UtLKZKaVZe1yqMBXd6Vk";

fn md() -> Command {
    Command::cargo_bin("md").unwrap()
}

fn encode(template: &str, k0: &str, k1: &str) -> std::process::Output {
    md().args([
        "encode",
        template,
        "--key",
        &format!("@0={k0}"),
        "--key",
        &format!("@1={k1}"),
        "--path",
        "48'/0'/0'/2'",
        "--group-size",
        "0",
        "--force-chunked",
    ])
    .output()
    .unwrap()
}

const SAME_SITE: &str = "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))";
/// The same policy shape with slot @1 on a DIFFERENT multipath branch.
const SPLIT_SITE: &str = "wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))";

#[test]
fn one_key_in_two_slots_is_refused() {
    let out = encode(SAME_SITE, K0, K0);
    assert!(
        !out.status.success(),
        "a 2-of-2 spendable by one key was minted:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let err = String::from_utf8_lossy(&out.stderr);
    for needle in ["@0", "@1", "same key"] {
        assert!(
            err.contains(needle),
            "the refusal does not mention {needle}, so an operator cannot locate it: {err}"
        );
    }
    assert!(
        !String::from_utf8_lossy(&out.stdout).contains("md1"),
        "a card was printed alongside the refusal"
    );
}

#[test]
fn two_distinct_keys_still_encode() {
    let out = encode(SAME_SITE, K0, K1);
    assert!(
        out.status.success(),
        "an ordinary 2-of-2 was refused: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// THE BOUNDARY THAT MAKES THIS CHECK CORRECT RATHER THAN MERELY STRICT.
///
/// The same xpub at two different multipath branches derives a DIFFERENT child
/// at every address index — two different wallets, not a duplicate. Measured:
/// `<0;1>` and `<2;3>` over one key give different addresses. A check comparing
/// the key alone would refuse this legitimate policy.
#[test]
fn one_key_at_two_different_use_sites_is_not_a_duplicate() {
    let out = encode(SPLIT_SITE, K0, K0);
    assert!(
        out.status.success(),
        "one key at two DIFFERENT branches was refused as a duplicate: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Non-vacuous: the two branch forms must actually be different wallets, or
    // the boundary above is a distinction without a difference.
    let addr = |t: &str| {
        let o = md()
            .args([
                "address",
                "--template",
                t,
                "--key",
                &format!("@0={K0}"),
                "--key",
                &format!("@1={K0}"),
                "--path",
                "48'/0'/0'/2'",
                "--count",
                "1",
            ])
            .output()
            .unwrap();
        String::from_utf8_lossy(&o.stdout)
            .lines()
            .find(|l| l.starts_with("bc1"))
            .unwrap_or_default()
            .to_owned()
    };
    let split = addr(SPLIT_SITE);
    assert!(
        !split.is_empty(),
        "the split-use-site policy derives no address"
    );
    // The same-use-site form is refused at encode but still derivable, so this
    // compares the two POLICIES rather than two cards.
    let same = addr(SAME_SITE);
    assert_ne!(
        split, same,
        "the two use-site forms give the same address — the boundary is meaningless"
    );
}

/// A keyless template has no keys to duplicate, and must not be refused.
#[test]
fn a_keyless_template_is_unaffected() {
    let out = md()
        .args([
            "encode",
            SAME_SITE,
            "--path",
            "48'/0'/0'/2'",
            "--force-chunked",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "a keyless template was refused: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
