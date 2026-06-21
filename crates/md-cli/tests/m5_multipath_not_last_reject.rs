#![allow(missing_docs)]

//! cycle-9 M5 (md-cli leg, FUNDS) — a multipath group that is NOT the final
//! derivation step (`@0/<2;3>/0'/*`, or an `h`-bearing origin step
//! `@0/48h/…/<0;1>/*`) leaves the lexer regex (`:55`) match ending at `>`,
//! the trailing fixed steps unconsumed, while the substitution regex (`:498`)
//! strips the same `@0/<…>` span and leaves the literal suffix — so the emitted
//! md1's recorded use-site path (the multipath) disagrees with its structural
//! tree (single-path, dropped fixed steps). That is a DIVERGENT card, silently,
//! exit 0. md1's `UseSitePath` cannot represent post-multipath fixed steps, so
//! the form is REJECTED (fail-closed) — an md1/`UseSitePath` representability
//! limit, NOT a BIP-389 prohibition (BIP-389 permits post-multipath steps).
//!
//! HARD CONSTRAINT (cycle-1 H13): the M5 reject must NOT regress H13's
//! hardened/malformed-multipath reject. For a fused body+suffix
//! (`<0'';1>/0'/*`), H13's group-3 validator fires FIRST (ordering guarantee),
//! so the error is the hardened/malformed message, NOT the M5 suffix message.
//!
//! See `mnemonic-toolkit/design/BRAINSTORM_cycle9_mdcli_parser.md` §3.1 (esp.
//! §3.1.4 H13-preservation) and `IMPLEMENTATION_PLAN_cycle9_mdcli_parser.md`
//! Phase 3 (M5 LAST). Decision D1/D2: REJECT, not canonicalize.

use assert_cmd::Command;
use predicates::prelude::*;

/// Abandon-mnemonic tpub at m/48'/1'/0'/2' (BIP 48 testnet account, depth 4).
const TPUB48: &str = "tpubDFH9dgzveyD8zTbPUFuLrGmCydNvxehyNdUXKJAQN8x4aZ4j6UZqGfnqFrD4NqyaTVGKbvEW54tsvPTK2UoSbCC1PJY8iCNiwTL3RWZEheQ";

/// (a) THE FUNDS CASE: a post-multipath fixed step `@0/<2;3>/0'/*` must reject
/// with exit 1 and a typed template-parse error naming the multipath-not-final
/// cause — NOT exit 0 with a silently-divergent md1 phrase on stdout. Keyless
/// `md encode` exercises `parse_template` (where M5 lives) directly, without
/// the per-key depth gate intercepting first.
#[test]
fn encode_post_multipath_fixed_step_rejects() {
    Command::cargo_bin("md")
        .unwrap()
        .args(["encode", "wpkh(@0/<2;3>/0'/*)"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("template parse error"))
        .stderr(predicate::str::contains("final"))
        // Must NOT silently emit an md1 phrase on stdout.
        .stdout(predicate::str::contains("md1").not());
}

/// (a') The `h`-bearing origin step variant `@0/48h/0h/0h/<0;1>/*` (unconsumed
/// `h…` residue) must also reject — same residue family.
#[test]
fn encode_h_in_origin_residue_rejects() {
    Command::cargo_bin("md")
        .unwrap()
        .args(["encode", "wpkh(@0/48h/0h/0h/<0;1>/*)"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("template parse error"))
        .stdout(predicate::str::contains("md1").not());
}

/// (b) FUSED ORDERING (the H13-preservation crux): a hardened/malformed body
/// `<0'';1>` followed by a post-multipath suffix `/0'/*` must error with H13's
/// hardened/malformed message (group-3 validator fires FIRST), NOT the M5
/// suffix message. H13's reject STAYS rejected.
#[test]
fn encode_fused_hardened_body_with_suffix_hits_h13_first() {
    Command::cargo_bin("md")
        .unwrap()
        .args([
            "encode",
            "wsh(multi(2,@0/<0'';1>/0'/*,@1/<0'';1>/0'/*))",
            "--key",
            &format!("@0={TPUB48}"),
            "--key",
            &format!("@1={TPUB48}"),
            "--network",
            "regtest",
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("template parse error"))
        // H13's message family (hardened / not-a-bare-unsigned-integer), NOT M5's.
        .stderr(
            predicate::str::contains("hardened")
                .or(predicate::str::contains("bare unsigned integer")),
        )
        .stderr(predicate::str::contains("final derivation step").not())
        .stdout(predicate::str::contains("md1").not());
}

/// (c) POSITIVE CONTROL: a canonical multipath-LAST template still encodes
/// (exit 0, emits an md1 phrase) — M5 narrows the grammar, it does not
/// over-reject valid multipath-last cards.
#[test]
fn encode_multipath_last_still_encodes() {
    Command::cargo_bin("md")
        .unwrap()
        .args([
            "encode",
            "--force-chunked",
            "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))",
            "--key",
            &format!("@0={TPUB48}"),
            "--key",
            &format!("@1={TPUB48}"),
            "--network",
            "regtest",
        ])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("md1"));
}
