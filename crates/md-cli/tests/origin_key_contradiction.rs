#![allow(missing_docs)]
//! F-217: `md encode` must refuse a card that declares ONE key origin for two
//! DIFFERENT keys.
//!
//! BIP-32 is deterministic — a `(fingerprint, path)` pair identifies exactly one
//! extended key — so such a card describes a wallet that cannot exist, and
//! proving it takes no seed, no network and no derivation.
//!
//! **Nothing downstream can catch it.** Addresses derive from the xpubs a card
//! CARRIES, never from the origin it declares, so every address check passes
//! identically either way. That is why it survived a full cross-language
//! conformance corpus: 9 of 9 multi-key keyed vectors were contradictory and
//! zero were consistent, and every address in every one of them matched.

use assert_cmd::Command;

const K0: &str = "xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf";
const K1: &str = "xpub6DzhyrnFFYQ1HimDiM388xHnDiRPNdZJFBmmxge3Y1WWcHLtMJLfRuhRHqnQCPbTj3fGKTuKFLHzzwpJkp5Dtc3UtLKZKaVZe1yqMBXd6Vk";
const FP: &str = "73c5da0a";

fn md() -> Command {
    Command::cargo_bin("md").unwrap()
}

/// `--path` flattens per-key origins to ONE shared path. Over two different
/// keys of the same master that is the contradiction, and it is exactly how the
/// corpus came to hold nine of them.
#[test]
fn a_shared_path_over_two_different_keys_is_refused() {
    let out = md()
        .args([
            "encode",
            "wsh(or_b(pk(@0/<0;1>/*),s:pk(@1/<0;1>/*)))",
            "--key",
            &format!("@0={K0}"),
            "--key",
            &format!("@1={K1}"),
            "--fingerprint",
            &format!("@0={FP}"),
            "--fingerprint",
            &format!("@1={FP}"),
            "--path",
            "48'/0'/0'/2'",
            "--force-chunked",
        ])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "a card declaring one origin for two different keys was minted:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let err = String::from_utf8_lossy(&out.stderr);
    for needle in ["@0", "@1", FP, "48'/0'/0'/2'"] {
        assert!(
            err.contains(needle),
            "the refusal does not name {needle}, so an operator cannot locate the clash: {err}"
        );
    }
    assert!(
        !String::from_utf8_lossy(&out.stdout).contains("md1"),
        "an md1 string was printed alongside the refusal"
    );
}

/// The SAME policy with each key at its TRUE origin still encodes. Without this
/// the refusal above could be satisfied by refusing everything.
#[test]
fn per_key_origins_in_the_template_still_encode() {
    let out = md()
        .args([
            "encode",
            "wsh(or_b(pk(@0/48'/0'/0'/2'/<0;1>/*),s:pk(@1/48'/0'/1'/2'/<0;1>/*)))",
            "--key",
            &format!("@0={K0}"),
            "--key",
            &format!("@1={K1}"),
            "--fingerprint",
            &format!("@0={FP}"),
            "--fingerprint",
            &format!("@1={FP}"),
            "--force-chunked",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "the CORRECT card was refused: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("md1"),
        "no card was emitted"
    );
}

/// Different MASTERS at the same path are consistent — two people can both use
/// `48'/0'/0'/2'`. Refusing that would break every ordinary multisig.
#[test]
fn the_same_path_under_different_fingerprints_is_fine() {
    let out = md()
        .args([
            "encode",
            "wsh(or_b(pk(@0/<0;1>/*),s:pk(@1/<0;1>/*)))",
            "--key",
            &format!("@0={K0}"),
            "--key",
            &format!("@1={K1}"),
            "--fingerprint",
            &format!("@0={FP}"),
            "--fingerprint",
            "@1=00112233",
            "--path",
            "48'/0'/0'/2'",
            "--force-chunked",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "two DIFFERENT masters at one path were refused: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The SAME key in two slots at one origin is CONSISTENT: one origin, one key.
/// It is key reuse — a different hazard with a different remedy (F-218) — and
/// conflating the two would make one refusal explain both badly.
#[test]
fn the_same_key_twice_at_one_origin_is_not_this_error() {
    let out = md()
        .args([
            "encode",
            "wsh(or_b(pk(@0/<0;1>/*),s:pk(@1/<0;1>/*)))",
            "--key",
            &format!("@0={K0}"),
            "--key",
            &format!("@1={K0}"),
            "--fingerprint",
            &format!("@0={FP}"),
            "--fingerprint",
            &format!("@1={FP}"),
            "--path",
            "48'/0'/0'/2'",
            "--force-chunked",
        ])
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        !err.contains("cannot exist"),
        "key reuse was reported as an impossible origin: {err}"
    );
}
