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
            // Plan 1b: no coordinator imports a keyless path.
            "--md-only",
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

/// F-551: `md compose`'s hash refusals name the clause the input VIOLATED.
///
/// Two of them named a rule the input already satisfied:
///
///   `ripemd160=09E7BB..` (40 chars, uppercase) -> "needs 40 hex characters,
///   lowercase" — the length is right, so the message reads as md being wrong
///   about the length.
///
///   `RIPEMD160=..` -> a bare "unknown option `RIPEMD160`". `ms hashlock`'s own
///   card says: "If it answers `unknown option ripemd160`, that support has not
///   shipped in your md yet" — so a CURRENT md printing that for a CASE error
///   sent the operator hunting a newer release of a tool already correct.
///
/// MUTATION: restore the combined width-or-case check -> the uppercase row
/// fails. MUTATION: drop the `looks_like_a_kind` hint -> the last row fails.
#[test]
fn compose_hash_refusals_name_the_clause_that_was_violated() {
    const D40: &str = "09e7bb5051d89788fb4e4b374126721dbcc2946b";

    // Uppercase at the RIGHT width: the case clause, not the width one.
    let r = md()
        .args([
            "compose",
            "--wrapper",
            "wsh",
            "--path",
            "2of3",
            "--path",
            &format!("keyless,ripemd160={}", D40.to_ascii_uppercase()),
            "--experimental",
        ])
        .assert()
        .failure();
    let e = String::from_utf8_lossy(&r.get_output().stderr).to_string();
    assert!(
        e.contains("LOWERCASE") && e.contains("never folded"),
        "an uppercase digest of the RIGHT length must be refused for its CASE:\n{e}"
    );
    assert!(
        !e.contains("needs exactly 40"),
        "the refusal still names the width, which this input satisfies:\n{e}"
    );

    // Wrong width: the width clause, and it says what was counted.
    let r = md()
        .args([
            "compose",
            "--wrapper",
            "wsh",
            "--path",
            "2of3",
            "--path",
            "keyless,sha256=abc",
            "--experimental",
        ])
        .assert()
        .failure();
    let e = String::from_utf8_lossy(&r.get_output().stderr).to_string();
    assert!(
        e.contains("needs exactly 64") && e.contains("got 3"),
        "a wrong-width digest must be told the number it gave:\n{e}"
    );

    // An uppercase OPTION NAME must not read as "your md is too old".
    let r = md()
        .args([
            "compose",
            "--wrapper",
            "wsh",
            "--path",
            "2of3",
            "--path",
            &format!("keyless,RIPEMD160={D40}"),
            "--experimental",
        ])
        .assert()
        .failure();
    let e = String::from_utf8_lossy(&r.get_output().stderr).to_string();
    assert!(
        e.contains("This md supports all four"),
        "a mis-CASED kind name still reads as missing support, which is what \
         `ms hashlock`'s card teaches the operator to conclude:\n{e}"
    );
}

/// F-548: `md decompose --emit commands` adds `--experimental` for exactly the
/// template that needs it.
///
/// That block prints under a "ready to run" banner and, for a keyless path,
/// printed `md encode` commands that are NOT — they fail with "All spend paths
/// must require a signature". The emitter already knew: it appends a note
/// saying the template may not be accepted, and printed the command without
/// the flag that accepts it.
///
/// THIS PINS THE CONDITION, not the rendering. The emitter keys on
/// `parse_template` failing with the signature rule, and scopes the flag to
/// that string — a template md rejects for some OTHER reason must not be handed
/// `--experimental`, which would only move the failure and imply the flag was
/// the answer. If that error text ever changes, the flag silently stops being
/// emitted and no rendering test would notice; this one does.
///
/// (The end-to-end `decompose` walk needs a concrete descriptor whose xpubs are
/// account-level at a 4-deep origin, which the fixtures here do not carry.)
#[test]
fn the_keyless_signature_rule_is_what_decompose_keys_on() {
    // A keyless hashlock path: refused, and BY THE SIGNATURE RULE.
    let keyless = md()
        .args([
            "compose",
            "--wrapper",
            "wsh",
            "--path",
            "2of2",
            "--path",
            "keyless,ripemd160=09e7bb5051d89788fb4e4b374126721dbcc2946b",
            "--experimental",
            "--md-only",
        ])
        .assert()
        .success();
    let t = String::from_utf8_lossy(&keyless.get_output().stdout)
        .lines()
        .next()
        .unwrap()
        .to_string();

    // `md descriptor` without the flag is the same gate decompose consults.
    let r = md()
        .args(["descriptor", "--template", &t, "--key", K0, "--key", K1])
        .assert()
        .failure();
    let e = String::from_utf8_lossy(&r.get_output().stderr).to_string();
    assert!(
        e.contains("require a signature"),
        "decompose scopes its --experimental to this exact string; if the wording \
         moved, the flag stops being emitted silently:\n{e}"
    );

    // And a KEYED policy must not trip it, or every decomposition would be
    // handed a flag it does not need.
    let keyed = md()
        .args(["compose", "--wrapper", "wsh", "--path", "2of2"])
        .assert()
        .success();
    let t2 = String::from_utf8_lossy(&keyed.get_output().stdout)
        .lines()
        .next()
        .unwrap()
        .to_string();
    md().args(["descriptor", "--template", &t2, "--key", K0, "--key", K1])
        .assert()
        .success();
}
