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
    // The contrast is drawn on ONE slot, not on SAME_SITE.
    //
    // It used to be `addr(SAME_SITE)`: the same-use-site form was refused at
    // encode but still DERIVABLE through `md address`, so the two policies
    // could be compared. REVIEW-converter-whole-diff-r1 C1 closed that route
    // (`t_row_one_key_in_two_slots_is_refused_by_address` below), so
    // `addr(SAME_SITE)` now returns the empty string and the old `assert_ne!`
    // would have passed for the wrong reason — a refusal compared against an
    // address. The single-slot pair asserts the same underlying fact directly:
    // one xpub at `<0;1>` and at `<2;3>` is two different children.
    let one_slot = |branch: &str| {
        let o = md()
            .args([
                "address",
                "--template",
                &format!("wpkh(@0/{branch}/*)"),
                "--key",
                &format!("@0={K0}"),
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
    let recv = one_slot("<0;1>");
    let other = one_slot("<2;3>");
    assert!(
        !recv.is_empty() && !other.is_empty(),
        "the single-slot control derived nothing: `{recv}` / `{other}`"
    );
    assert_ne!(
        recv, other,
        "the two use-site branches give the same address — the boundary is meaningless"
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

// ─── THE T ROW (REVIEW-converter-whole-diff-r1 C1) ──────────────────────────
//
// `md encode` has refused shape (1) since F-218 (rows above). `md descriptor`
// and `md address` build the same wallet through `cmd::build::build_descriptor`
// and RENDER it without ever encoding, so the guard never ran on that route:
// measured 2026-08-30, `sortedmulti(2,X,X,Y)` shipped with exit 0 and no
// warning — a 2-of-3 one key alone can spend — and `md decompose` refused to
// read back the exact string `md descriptor` had just emitted.
//
// The rows below are the compose side of SPEC A3's "refuses BOTH forbidden
// shapes in BOTH directions". The boundary row is the one that makes the check
// correct rather than merely strict, and it is the same boundary the encode
// rows above pin: DISJOINT use-sites are two wallets, not one key twice.

const K2: &str = "xpub6EGx8sPr9FxPPE1rbZazhqWwpMXA3Hf5DYKtZbL7c4BSddzmQktp96UaTvecEkoCZysuaj79GMCFZYT1KKk7Ph2M3Kf5g8B82KZ8TZ9SKQR";

/// The review's reproduction verbatim: three slots, one xpub on two of them.
const THREE_SLOT: &str = "wsh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))";

fn compose(verb: &str, template: &str, keys: &[&str]) -> std::process::Output {
    let mut c = md();
    c.args([verb, "--template", template]);
    for k in keys {
        c.args(["--key", k]);
    }
    if verb == "address" {
        c.args(["--count", "1"]);
    }
    c.output().unwrap()
}

/// SPEC A3's diagnostic rule, on the compose side: cite BIP 388, say
/// UNSUPPORTED, never call the input invalid.
fn assert_t_row_reuse_refusal(out: &std::process::Output, verb: &str) {
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !out.status.success(),
        "md {verb} composed a wallet one key alone can spend: {stdout}"
    );
    assert!(
        !stdout.contains("wsh(") && !stdout.contains("bc1"),
        "md {verb} printed a wallet alongside the refusal: {stdout}"
    );
    let err = String::from_utf8_lossy(&out.stderr);
    for needle in ["@0", "@1", "BIP 388", "pairwise distinct"] {
        assert!(
            err.contains(needle),
            "the md {verb} refusal does not name {needle}: {err}"
        );
    }
    assert!(
        err.contains("UNSUPPORTED") || err.contains("unsupported"),
        "the md {verb} refusal must say UNSUPPORTED: {err}"
    );
    assert!(
        !err.to_lowercase().contains("invalid"),
        "the md {verb} refusal must never call the input invalid: {err}"
    );
}

#[test]
fn t_row_one_key_in_two_slots_is_refused_by_descriptor() {
    let out = compose(
        "descriptor",
        THREE_SLOT,
        &[
            &format!("@0={K0}"),
            &format!("@1={K0}"),
            &format!("@2={K1}"),
        ],
    );
    assert_t_row_reuse_refusal(&out, "descriptor");
}

#[test]
fn t_row_one_key_in_two_slots_is_refused_by_address() {
    let out = compose(
        "address",
        THREE_SLOT,
        &[
            &format!("@0={K0}"),
            &format!("@1={K0}"),
            &format!("@2={K1}"),
        ],
    );
    assert_t_row_reuse_refusal(&out, "address");
}

/// The origin-notated two-slot form the review measured composing with a
/// checksum (`#xn0gcxt8`) — same defect, reached through C1's `--key` bracket.
///
/// The template carries INLINE origins here, unlike the review's command,
/// which used the pathless form. That is deliberate: the I1 fold in the same
/// review makes a bracket path with no winning source a refusal in its own
/// right, so the pathless spelling would stop at THAT gate and this row would
/// no longer measure key reuse. Both spellings refuse; this one refuses for
/// the reason the row is named for.
#[test]
fn t_row_one_key_in_two_slots_is_refused_through_the_origin_notated_key() {
    let out = compose(
        "descriptor",
        "wsh(sortedmulti(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/0'/2'/<0;1>/*))",
        &[
            &format!("@0=[73c5da0a/48'/0'/0'/2']{K0}"),
            &format!("@1=[73c5da0a/48'/0'/0'/2']{K0}"),
        ],
    );
    assert_t_row_reuse_refusal(&out, "descriptor");
}

/// NOT MERELY STRICT, half 1: three distinct cosigners still compose.
#[test]
fn t_row_three_distinct_keys_still_compose() {
    let out = compose(
        "descriptor",
        THREE_SLOT,
        &[
            &format!("@0={K0}"),
            &format!("@1={K1}"),
            &format!("@2={K2}"),
        ],
    );
    assert!(
        out.status.success(),
        "an ordinary 2-of-3 was refused: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("wsh(sortedmulti(2,"));
}

/// NOT MERELY STRICT, half 2: the DISJOINT use-site form BIP 388 permits and
/// `md encode` accepts (row `one_key_at_two_different_use_sites_is_not_a_duplicate`
/// above) must keep composing here too, or `md descriptor` would start refusing
/// what `md encode` mints — a fourth answer from one binary.
#[test]
fn t_row_one_key_at_two_disjoint_use_sites_still_composes() {
    let out = compose(
        "descriptor",
        SPLIT_SITE,
        &[&format!("@0={K0}"), &format!("@1={K0}")],
    );
    assert!(
        out.status.success(),
        "the BIP-legal disjoint form was refused: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// THE SAME SLOT SUPPLIED TWICE IS NOT SHAPE (1) and keeps today's behaviour,
/// which is FIRST-WINS — measured 2026-08-30 at 9d0c30dc: `--key @0=X --key
/// @0=Y --key @1=Y` composed byte-identically to `--key @0=X --key @1=Y`
/// (`#jywr5cj9`). One placeholder cannot be two cosigners, so there is no
/// reuse to refuse; pinned here so the C1 check cannot silently absorb it.
#[test]
fn t_row_the_same_slot_supplied_twice_keeps_first_wins() {
    let two_slot = "wsh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*))";
    let once = compose(
        "descriptor",
        two_slot,
        &[&format!("@0={K0}"), &format!("@1={K1}")],
    );
    let twice = compose(
        "descriptor",
        two_slot,
        &[
            &format!("@0={K0}"),
            &format!("@0={K1}"),
            &format!("@1={K1}"),
        ],
    );
    assert!(
        once.status.success() && twice.status.success(),
        "supplying @0 twice must not refuse: {}",
        String::from_utf8_lossy(&twice.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&once.stdout),
        String::from_utf8_lossy(&twice.stdout),
        "a repeated --key for ONE slot must still be first-wins"
    );
}
