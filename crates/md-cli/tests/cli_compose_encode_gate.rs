//! `md encode` and `md compose` must agree on the EXPERIMENTAL gate: a spend
//! path that needs no signature is refused under EVERY wrapper unless
//! `--experimental`, which then warns. Before this task only `tr` was gated.

use assert_cmd::Command;
use predicates::prelude::*;

fn md() -> Command {
    Command::cargo_bin("md").expect("md binary")
}

const H: &str = "a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8";
const XPUB: [&str; 3] = [
    "xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf",
    "xpub6DzhyrnFFYQ1HimDiM388xHnDiRPNdZJFBmmxge3Y1WWcHLtMJLfRuhRHqnQCPbTj3fGKTuKFLHzzwpJkp5Dtc3UtLKZKaVZe1yqMBXd6Vk",
    "xpub6EGx8sPr9FxPPE1rbZazhqWwpMXA3Hf5DYKtZbL7c4BSddzmQktp96UaTvecEkoCZysuaj79GMCFZYT1KKk7Ph2M3Kf5g8B82KZ8TZ9SKQR",
];

fn sigless_wsh() -> String {
    format!(
        "wsh(or_d(multi(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/1'/2'/<0;1>/*,@2/48'/0'/2'/2'/<0;1>/*),and_v(v:sha256({H}),after(1383520))))"
    )
}

#[test]
fn encode_refuses_a_sigless_wsh_path_unkeyed_unless_experimental() {
    md().args(["encode", &sigless_wsh()])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("must require a signature"));
    md().args(["encode", "--experimental", &sigless_wsh()])
        .assert()
        .success()
        .stderr(predicate::str::contains("relaxed the signature rule"));
}

#[test]
fn encode_refuses_a_sigless_wsh_path_keyed_unless_experimental() {
    let keyed = |extra: &[&str]| {
        let mut args: Vec<String> = vec!["encode".into()];
        args.extend(extra.iter().map(|s| s.to_string()));
        args.push(sigless_wsh());
        for (i, x) in XPUB.iter().enumerate() {
            args.push("--key".into());
            args.push(format!("@{i}={x}"));
            args.push("--fingerprint".into());
            args.push(format!("@{i}=73c5da0a"));
        }
        md().args(&args).assert()
    };
    keyed(&[])
        .failure()
        .code(1)
        .stderr(predicate::str::contains("must require a signature"));
    keyed(&["--experimental"])
        .success()
        .stderr(predicate::str::contains("relaxed the signature rule"));
}

#[test]
fn encode_still_admits_a_signed_wsh_policy_without_the_flag() {
    let two_path = "wsh(or_d(multi(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/1'/2'/<0;1>/*,@2/48'/0'/2'/2'/<0;1>/*),and_v(v:pkh(@3/48'/0'/3'/2'/<0;1>/*),older(26280))))";
    md().args(["encode", two_path])
        .assert()
        .success()
        .stderr(predicate::str::contains("signature").not());
}

/// F-600: `md compose` emitted, at EXIT 0, a template that every downstream
/// `md` verb refuses as malleable — whenever two key-less hash paths sit side
/// by side, which is precisely the mixed-kind shape the hashlock-kinds work
/// exists for.
///
/// ```text
/// md compose --wrapper wsh --experimental \
///   --path 2of3 --path keyless,sha256=<h> --path keyless,ripemd160=<h>
///   -> exit 0, prints wsh(or_d(multi(2,…),or_i(sha256(…),ripemd160(…))))
/// md encode --in that --experimental
///   -> "Miniscript is malleable"
/// ```
///
/// The operator got a plausible template, a zero exit and a wall at the next
/// verb. `compose` read back what it was about to emit, using the SAME parser
/// `encode` uses — not a second copy of the rules, which is what keeps the two
/// from drifting.
///
/// WHICH LAYER REFUSES IT CHANGED IN md-codec 0.45 (composer fable review r0,
/// lens 1 C-1). A read-back is not portable: the device's Go port has no
/// miniscript library, so it could not reach this verdict at all and cut such
/// a policy into steel. The rule is now STATED in `compose::validate` — at
/// most one key-less path — so this shape is refused BEFORE lowering, and the
/// message names the rule and the remedy instead of quoting the parser. The
/// read-back stays behind it for everything else `md encode` can refuse.
///
/// What this test pins is the CONTRACT, not the layer: for this path list
/// `md compose` exits non-zero, prints no template, and says why.
///
/// MUTATION: delete the `TooManyKeylessPaths` arm in `compose::validate` ->
/// the last two assertions fail (the message becomes the parser's "Miniscript
/// is malleable" quoted by the read-back); delete the read-back too and the
/// first two fail (exit 0, and a template on stdout).
#[test]
fn compose_refuses_to_emit_a_template_encode_would_reject() {
    let out = md()
        .args([
            "compose",
            "--wrapper",
            "wsh",
            "--experimental",
            "--path",
            "2of3",
            "--path",
            "keyless,sha256=443816468ff6e25c543904c92a3b2cdaceca70ef366b5829462b0fc7f608b34c",
            "--path",
            "keyless,ripemd160=dc3004b2228ca886cd79dce3d8af04aff87dfc83",
        ])
        .output()
        .unwrap();

    assert!(
        !out.status.success(),
        "compose emitted a template encode refuses, at exit 0"
    );
    assert!(
        out.stdout.is_empty(),
        "compose printed a template it knows is unusable: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let err = String::from_utf8_lossy(&out.stderr);
    // It must name WHICH paths, not just "invalid" -- naming them is what tells
    // the operator what to change. (Paths 2 and 3 are the two key-less ones.)
    assert!(
        err.contains("paths 2 and 3"),
        "the refusal does not name the offending paths:\n{err}"
    );
    // And the rule that was broken.
    assert!(
        err.contains("malleable"),
        "the refusal does not name the rule that was broken:\n{err}"
    );
    // And a remedy that WORKS. The pre-0.45 hint offered a timelock, which does
    // not help at all: `older`/`after` need no signature either, so the path
    // stays unsafe and the `or_i` stays malleable. A remedy that does not work
    // is worse than no remedy -- it costs the operator a second attempt.
    assert!(
        err.contains("a timelock does not help"),
        "the refusal offers no working remedy, or offers the one that fails:\n{err}"
    );
}

/// The control for the test above: an ordinary compose must still emit. A
/// read-back that refused everything would pass the assertions there and make
/// the verb useless.
#[test]
fn compose_still_emits_a_template_that_encode_accepts() {
    let out = md()
        .args(["compose", "--wrapper", "wsh", "--path", "2of3"])
        .output()
        .unwrap();
    assert!(out.status.success(), "an ordinary compose was refused");
    let tmpl = String::from_utf8_lossy(&out.stdout).trim().to_string();
    assert!(tmpl.starts_with("wsh("), "no template emitted: {tmpl}");

    // And the claim compose makes about itself -- "what `md encode` reads back
    // to the same card" -- is asserted rather than assumed.
    md().args(["encode", &tmpl, "--group-size", "0"])
        .assert()
        .success()
        .stdout(predicate::str::contains("md1"));
}
