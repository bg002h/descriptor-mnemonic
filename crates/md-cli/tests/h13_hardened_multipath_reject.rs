#![allow(missing_docs)]

//! cycle-1 H13 (md-cli leg) — a hardened multipath alternative
//! (`<0';1'>` / `<0h;1h>`) must be DETECTED at template lex and rejected with a
//! typed `CliError::TemplateParse` (exit 1), NEVER silently collapsed to a bare
//! `/*` single-path key (which mis-encodes to a wrong-address card the
//! constellation can never restore — hardened derivation is impossible on an
//! xpub / watch-only key, BIP-32).
//!
//! See `mnemonic-toolkit/design/BRAINSTORM_cycle1_critical_fixes.md` §3 and
//! `IMPLEMENTATION_PLAN_cycle1_critical_fixes.md` Phase P1. Decision: REJECT
//! (R0 round-1 C1), not faithful-represent and not silent-collapse.

use assert_cmd::Command;
use predicates::prelude::*;

/// Abandon-mnemonic tpub at m/48'/1'/0'/2' (BIP 48 testnet account, depth 4) —
/// the natural cosigner depth for a `wsh(multi(...))` 2-of-2. Same value as
/// `parse::keys::ABANDON_TPUB_DEPTH4_BIP48` in the bin crate (integration tests
/// can't reach pub(crate) items there). Accepted under regtest (same BIP-32
/// testnet version bytes).
const TPUB48: &str = "tpubDFH9dgzveyD8zTbPUFuLrGmCydNvxehyNdUXKJAQN8x4aZ4j6UZqGfnqFrD4NqyaTVGKbvEW54tsvPTK2UoSbCC1PJY8iCNiwTL3RWZEheQ";

/// (a) A well-formed hardened multipath alternative `<0';1'>` must be rejected
/// with exit 1 and a typed template-parse error naming the hardened alt — NOT
/// exit 0 with a silently-collapsed bare-`/*` md1 phrase.
#[test]
fn encode_hardened_multipath_apostrophe_rejects() {
    Command::cargo_bin("md")
        .unwrap()
        .args([
            "encode",
            "wsh(multi(2,@0/<0';1'>/*,@1/<0';1'>/*))",
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
        .stderr(predicate::str::contains("hardened"))
        // Must NOT silently collapse and emit an md1 phrase on stdout.
        .stdout(predicate::str::contains("md1").not());
}

/// (b) The `h`-marker form `<0h;1h>` must reject identically.
#[test]
fn encode_hardened_multipath_h_form_rejects() {
    Command::cargo_bin("md")
        .unwrap()
        .args([
            "encode",
            "wsh(multi(2,@0/<0h;1h>/*,@1/<0h;1h>/*))",
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
        .stderr(predicate::str::contains("hardened"))
        .stdout(predicate::str::contains("md1").not());
}

/// (b') A mixed body where only one alt is hardened must also reject — the
/// reject fires if ANY alt carries a hardened marker.
#[test]
fn encode_hardened_multipath_mixed_rejects() {
    Command::cargo_bin("md")
        .unwrap()
        .args([
            "encode",
            "wsh(multi(2,@0/<0;1'>/*,@1/<0;1'>/*))",
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
        .stdout(predicate::str::contains("md1").not());
}

/// (b'') MALFORMED double-marker bodies (`<0'';1>`, `<0'h;1>`, `<0h';1>`) must
/// ALSO reject with exit 1 / typed `TemplateParse`, NOT silently collapse to a
/// bare-`/*` single-path card (`wsh(multi(2,@0/*,@1/*))`). This is the C1
/// regression-guard (H13 impl-review round-1): pre-fix the strict-alternation
/// capture skip-matched these empty and the widened strip class silently
/// stripped the residue → exit 0 with the multipath dropped (wrong-address,
/// un-restorable). The fix captures the body permissively and validates it
/// strictly, so any non-`[0-9;]` body (double markers, single markers, stray
/// residue) fails closed and loud.
#[test]
fn encode_malformed_hardened_multipath_rejects() {
    for body in ["<0'';1>", "<0'h;1>", "<0h';1>"] {
        let template = format!("wsh(multi(2,@0/{body}/*,@1/{body}/*))");
        let out = Command::cargo_bin("md")
            .unwrap()
            .args([
                "encode",
                &template,
                "--key",
                &format!("@0={TPUB48}"),
                "--key",
                &format!("@1={TPUB48}"),
                "--network",
                "regtest",
            ])
            .assert()
            .failure() // non-zero exit — NEVER exit 0 / silent collapse
            // Must NOT silently emit a (collapsed) md1 phrase on stdout.
            .stdout(predicate::str::contains("md1").not())
            .get_output()
            .to_owned();
        let code = out.status.code().unwrap_or(-1);
        assert_eq!(
            code,
            1,
            "malformed double-marker body `{body}` must reject with exit 1 \
             (typed TemplateParse), got exit {code}; stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            String::from_utf8_lossy(&out.stderr).contains("template parse error"),
            "malformed body `{body}` must surface a typed template parse error; stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

/// (c) CLEAN-NEGATIVE / regression: a non-hardened `<0;1>` multipath MUST still
/// encode successfully (exit 0) and decode back to the same multipath template
/// byte-for-byte. The reject must NOT over-fire on legitimate non-hardened
/// multipaths.
#[test]
fn encode_nonhardened_multipath_roundtrips() {
    let template = "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))";

    // cycle-4 H6: this 2-of-2 wallet-policy (two 65-byte xpub TLVs) exceeds the
    // 80-data-symbol single-string cap, so encode via the chunked path
    // (`--force-chunked --group-size 0`); H13's non-hardened-multipath accept is
    // orthogonal to the length cap and must still hold through chunked encode.
    let encode_out = Command::cargo_bin("md")
        .unwrap()
        .args([
            "encode",
            template,
            "--key",
            &format!("@0={TPUB48}"),
            "--key",
            &format!("@1={TPUB48}"),
            "--network",
            "regtest",
            "--force-chunked",
            "--group-size",
            "0",
        ])
        .output()
        .unwrap();
    assert!(
        encode_out.status.success(),
        "non-hardened <0;1> multipath must encode (exit 0); stderr: {}",
        String::from_utf8_lossy(&encode_out.stderr)
    );
    let chunks: Vec<String> = String::from_utf8(encode_out.stdout)
        .unwrap()
        .lines()
        .filter(|l| l.starts_with("md1"))
        .map(|l| l.to_string())
        .collect();
    assert!(
        !chunks.is_empty() && chunks.iter().all(|c| c.starts_with("md1")),
        "expected md1 chunk phrases, got: {chunks:?}"
    );

    let mut decode_args: Vec<String> = vec!["decode".to_string()];
    decode_args.extend(chunks);
    let decode_out = Command::cargo_bin("md")
        .unwrap()
        .args(&decode_args)
        .output()
        .unwrap();
    assert!(
        decode_out.status.success(),
        "decode of non-hardened phrase failed"
    );
    let decoded = String::from_utf8(decode_out.stdout)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .to_string();
    assert_eq!(
        decoded, template,
        "non-hardened <0;1> multipath must round-trip to the same template (NOT collapse to a bare /*)"
    );
}
