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
