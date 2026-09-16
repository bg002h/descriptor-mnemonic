//! F-547: `md descriptor` and `md address` accept `--experimental`, so the
//! refusal that PRESCRIBES it is a remedy the operator can actually follow.
//!
//! Before: both verbs refused a keyless hashlock with *"pass --experimental
//! here"*, and passing it produced `error: unexpected argument '--experimental'
//! found` (exit 2). The flag existed only on the minting verbs. So a wallet
//! composed with `md compose --experimental` could be encoded and engraved and
//! then never turned into a descriptor or an address — the operator would hold
//! a plate and have no way to learn where to send funds.
//!
//! That is `md verify --experimental`'s own stated reasoning, verbatim: "a card
//! authored with `--experimental` cannot be verified at all — the operator would
//! hold a plate and have no way to check it, which is worse than not authoring
//! it." It applies unchanged to rendering.
//!
//! A WRONG REMEDY COSTS MORE THAN NO REMEDY: the operator's next action was
//! guaranteed to fail, and the failure named the flag rather than the shape.

use assert_cmd::Command;

fn md() -> Command {
    Command::cargo_bin("md").unwrap()
}

const K0: &str = "@0=[73c5da0a/48'/0'/0'/2']xpub6BosfCnifzxcFwrSzQiqu2DBVTshkCXacvNsWGYJVVhhawA7d4R5WSWGFNbi8Aw6ZRc1brxMyWMzG3DSSSSoekkudhUd9yLb6qx39T9nMdj";
const K1: &str = "@1=[73c5da0a/48'/0'/1'/2']xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz";

/// Every kind, because the refusal is about the KEYLESS PATH and not about the
/// hash — a fix that worked only for sha256 would leave the three this cycle
/// added unreachable, which is the shape of defect this whole cycle exists for.
fn template_for(kind: &str) -> String {
    let h = match kind {
        "sha256" | "hash256" => "3cf5d421caf2a9c8eb9de1d400866ea7d475e6ba978861bb0167a37cb70a4c12",
        _ => "09e7bb5051d89788fb4e4b374126721dbcc2946b",
    };
    let out = md()
        .args([
            "compose",
            "--wrapper",
            "wsh",
            "--path",
            "2of2",
            "--path",
            &format!("keyless,{kind}={h}"),
            "--experimental",
        ])
        .assert()
        .success();
    String::from_utf8_lossy(&out.get_output().stdout)
        .lines()
        .next()
        .expect("compose prints the template")
        .to_string()
}

/// MUTATION: remove `experimental` from DescriptorInput (or stop threading it)
/// -> every `with` row fails, the verb refusing its own prescribed remedy.
#[test]
fn a_keyless_hashlock_reaches_a_descriptor_and_an_address_at_every_kind() {
    for kind in ["sha256", "hash256", "ripemd160", "hash160"] {
        let t = template_for(kind);

        // WITHOUT the flag it still refuses — the guard is not simply gone.
        let r = md()
            .args(["descriptor", "--template", &t, "--key", K0, "--key", K1])
            .assert()
            .failure();
        let err = String::from_utf8_lossy(&r.get_output().stderr).to_string();
        assert!(
            err.contains("require a signature"),
            "{kind}: the keyless guard stopped firing without the flag:\n{err}"
        );
        // And the refusal still names the remedy, which is now reachable.
        assert!(
            err.contains("--experimental"),
            "{kind}: the refusal no longer names its remedy:\n{err}"
        );

        // WITH the flag, a concrete descriptor.
        let r = md()
            .args([
                "descriptor",
                "--template",
                &t,
                "--key",
                K0,
                "--key",
                K1,
                "--experimental",
            ])
            .assert()
            .success();
        let out = String::from_utf8_lossy(&r.get_output().stdout).to_string();
        assert!(
            out.contains("wsh(") && out.contains(kind),
            "{kind}: --experimental did not yield a descriptor naming the kind:\n{out}"
        );

        // And an address, which is the thing the operator actually needs.
        let r = md()
            .args([
                "address",
                "--template",
                &t,
                "--key",
                K0,
                "--key",
                K1,
                "--experimental",
            ])
            .assert()
            .success();
        let addr = String::from_utf8_lossy(&r.get_output().stdout).to_string();
        assert!(
            addr.trim_start().starts_with("bc1"),
            "{kind}: no mainnet address:\n{addr}"
        );
    }
}
