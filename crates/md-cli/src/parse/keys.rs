use crate::error::CliError;
use bitcoin::base58;

const XPUB_LEN: usize = 78;
pub(crate) const MAINNET_XPUB_VERSION: [u8; 4] = [0x04, 0x88, 0xB2, 0x1E];
pub(crate) const TESTNET_XPUB_VERSION: [u8; 4] = [0x04, 0x35, 0x87, 0xCF];

/// Script context of the key being parsed.
///
/// Since 2026-08-19 this no longer *gates* depth — `parse_key` admits depth 3
/// or 4 in either context (see the admission comment there). It records which
/// convention the context follows, so a rejection can name it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptCtx {
    /// Single-sig (e.g. wpkh, pkh). Conventionally depth 3 — BIP-44/49/84/86.
    SingleSig,
    /// Multisig / taproot (e.g. wsh, sh-wsh, tr). Conventionally depth 4 —
    /// BIP-48. But BIP-87 multisig accounts are depth 3 and are also valid.
    MultiSig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedKey {
    pub i: u8,
    /// BIP-32 depth of the seated xpub — byte 4 of the 78-byte
    /// serialization, the same field `bitcoin::bip32::Xpub::depth` exposes,
    /// read out of the buffer this function has already decoded rather than
    /// parsing the key a second time.
    ///
    /// NOT part of the encoded payload (`payload` starts at byte 13), so
    /// carrying it here can move no wire byte, no address and no wallet id.
    /// It exists for ADVISORIES that must compare a template's DECLARED origin
    /// against the key actually seated in that slot — see
    /// `emit_unhardened_origin_note` (F-411), which cannot ask the question
    /// without it.
    ///
    /// The admitted set is `{3, 4}` (see the depth-admission comment in
    /// `parse_key`), so a `0` here is unreachable through the CLI. That is why
    /// the `depth >= 1` guard downstream is exercised by a unit test rather
    /// than an end-to-end one.
    pub depth: u8,
    /// chain code (32) ‖ compressed pubkey (33).
    pub payload: [u8; 65],
}

pub fn parse_key(
    arg: &str,
    ctx: ScriptCtx,
    network: bitcoin::Network,
) -> Result<ParsedKey, CliError> {
    let (i_str, xpub_str) = arg
        .split_once('=')
        .ok_or_else(|| CliError::BadArg(format!("--key expects @i=XPUB, got: {arg}")))?;
    let i = parse_index(i_str)?;
    let bytes = base58::decode_check(xpub_str).map_err(|e| CliError::BadXpub {
        i,
        why: format!("base58check decode: {e}"),
    })?;
    if bytes.len() != XPUB_LEN {
        return Err(CliError::BadXpub {
            i,
            why: format!("expected 78 bytes, got {}", bytes.len()),
        });
    }
    let (expected_version, network_label) = match network {
        bitcoin::Network::Bitcoin => (MAINNET_XPUB_VERSION, "mainnet"),
        // BIP 32 testnet bytes (0x043587CF) cover all testnet flavors.
        bitcoin::Network::Testnet
        | bitcoin::Network::Testnet4
        | bitcoin::Network::Signet
        | bitcoin::Network::Regtest => (TESTNET_XPUB_VERSION, "testnet"),
    };
    if bytes[0..4] != expected_version {
        return Err(CliError::BadXpub {
            i,
            why: format!(
                "expected {network_label} xpub version {:02X}{:02X}{:02X}{:02X}, got {:02X}{:02X}{:02X}{:02X}",
                expected_version[0],
                expected_version[1],
                expected_version[2],
                expected_version[3],
                bytes[0],
                bytes[1],
                bytes[2],
                bytes[3]
            ),
        });
    }
    // DEPTH ADMISSION — both account-level depths, either context.
    //
    // OPERATOR STOPGAP 2026-08-19, widening a previous exact match (SingleSig
    // == 3, MultiSig == 4) that was wrong in BOTH directions:
    //
    //   TOO STRICT — BIP-87 (Complete, *Hierarchy for Deterministic Multisig
    //   Wallets*) publishes `wsh(sortedmulti(2,[xfpForA/87'/0'/0']XpubA/0/*,…))`,
    //   a DEPTH-3 xpub in multisig, which the old rule rejected outright.
    //   BIP-388's own vector uses a depth-5 xpub inside `sortedmulti_a`.
    //
    //   TOO LOOSE — depth 4 is not evidence of BIP-48. `44'/0'/0'/100'` passed,
    //   and so did `m/1/2/3/4`, so the rule failed at the very thing it existed
    //   to do while rejecting valid keys.
    //
    // The old rule's stated rationale was "Depth tracks BIP 388 expectation".
    // BIP-388 does not contain the word "depth"; neither does BIP-87. Full
    // working, with source quotations, in the constellation recon report
    // `mnemonic-engrave/design/agent-reports/recon-protocol-multisig-xpub-depth.md`.
    //
    // WHAT THIS STILL CATCHES: a master key (depth 0), and any leaf or
    // address-level key (depth ≥ 5) — the gross paste-the-wrong-xpub errors.
    // WHAT IT NO LONGER CATCHES, deliberately: an account key at the *other*
    // standard's depth, e.g. a BIP-84 depth-3 key handed to a multisig slot.
    // The invariant that would catch that without rejecting BIP-87 is
    // `depth == origin_path.len()`, which is NOT reachable here — `parse_key`
    // receives only `@i=XPUB` and never sees an origin. Reaching it means
    // plumbing origin through the CLI, which is a cycle, not a stopgap.
    //
    // Widening admission cannot move a wallet id or change an encoded md1
    // string: the payload below is `bytes[13..78]`, so depth never reaches it.
    // `depth_does_not_affect_the_encoded_payload` pins that.
    let depth = bytes[4];
    // Used for the error message only — the admitted set is the same either
    // way. Kept per-context so a rejection still names the convention the
    // operator was probably reaching for.
    let conventional = match ctx {
        ScriptCtx::SingleSig => 3, // BIP-44/49/84/86: m/purpose'/coin'/account'
        ScriptCtx::MultiSig => 4,  // BIP-48: m/48'/coin'/account'/script'
    };
    if !matches!(depth, 3 | 4) {
        return Err(CliError::BadXpub {
            i,
            why: format!(
                "expected an account-level xpub at depth 3 or 4 \
                 (this script context conventionally uses {conventional}), got {depth}"
            ),
        });
    }
    // M11 (cycle-9): reject an off-curve compressed pubkey at parse. The xpub
    // layout is version(4) ‖ depth(1) ‖ parent_fp(4) ‖ child(4) ‖ chaincode(32)
    // ‖ pubkey(33), so the compressed point is `bytes[45..78]`. A valid BIP-32
    // serialized xpub carries an on-curve point; a corrupt / all-zero point
    // previously slipped through intake and only failed later at derivation.
    bitcoin::secp256k1::PublicKey::from_slice(&bytes[45..78]).map_err(|e| CliError::BadXpub {
        i,
        why: format!("xpub public key is not a valid secp256k1 point: {e}"),
    })?;
    let mut payload = [0u8; 65];
    payload.copy_from_slice(&bytes[13..78]);
    Ok(ParsedKey { i, depth, payload })
}

fn parse_index(s: &str) -> Result<u8, CliError> {
    let stripped = s.strip_prefix('@').unwrap_or(s);
    stripped
        .parse::<u8>()
        .map_err(|_| CliError::BadArg(format!("--key index must be 0..255, got: {s}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real xpub at depth 4 (m/48'/0'/0'/2') from the abandon-mnemonic, mainnet.
    const XPUB_DEPTH4: &str = "xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf";

    #[test]
    fn rejects_no_equals() {
        let err =
            parse_key("@0xpub6...", ScriptCtx::MultiSig, bitcoin::Network::Bitcoin).unwrap_err();
        assert!(matches!(err, CliError::BadArg(_)));
    }

    #[test]
    fn rejects_bad_index() {
        let err = parse_key(
            format!("@notnum={XPUB_DEPTH4}").as_str(),
            ScriptCtx::MultiSig,
            bitcoin::Network::Bitcoin,
        )
        .unwrap_err();
        assert!(matches!(err, CliError::BadArg(_)));
    }

    #[test]
    fn rejects_bad_checksum() {
        let err = parse_key("@0=xpubBADCHECKSUMxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", ScriptCtx::MultiSig, bitcoin::Network::Bitcoin).unwrap_err();
        assert!(
            matches!(err, CliError::BadXpub { i: 0, .. }),
            "got: {err:?}"
        );
    }

    #[test]
    fn accepts_valid_depth4_xpub() {
        let parsed = parse_key(
            format!("@2={XPUB_DEPTH4}").as_str(),
            ScriptCtx::MultiSig,
            bitcoin::Network::Bitcoin,
        )
        .unwrap();
        assert_eq!(parsed.i, 2);
        assert_eq!(parsed.payload.len(), 65);
    }

    /// Re-stamp an xpub's depth byte, keeping base58check valid.
    ///
    /// Depth is byte 4 of the 78-byte payload; chaincode and the on-curve
    /// pubkey are untouched, so this isolates the depth check from every other
    /// gate in `parse_key` and lets a test reach depths no fixture provides.
    fn with_depth(xpub: &str, depth: u8) -> String {
        let mut bytes = base58::decode_check(xpub).expect("fixture must decode");
        bytes[4] = depth;
        base58::encode_check(&bytes)
    }

    /// WAS `rejects_depth4_xpub_in_singlesig_context`, inverted deliberately by
    /// the 2026-08-19 stopgap. A depth-4 key in a single-sig context is no
    /// longer an error — see the admission comment in `parse_key`.
    #[test]
    fn accepts_depth4_xpub_in_singlesig_context() {
        let p = parse_key(
            format!("@0={XPUB_DEPTH4}").as_str(),
            ScriptCtx::SingleSig,
            bitcoin::Network::Bitcoin,
        )
        .expect("depth 4 is admitted in either context since the stopgap");
        assert_eq!(p.i, 0);
    }

    /// The BIP-87 case the old rule rejected: a depth-3 account key used for
    /// multisig. This is the whole reason the rule was widened.
    #[test]
    fn accepts_depth3_xpub_in_multisig_context() {
        let p = parse_key(
            format!("@0={ABANDON_TPUB_DEPTH3_BIP84}").as_str(),
            ScriptCtx::MultiSig,
            bitcoin::Network::Testnet,
        )
        .expect("BIP-87 multisig accounts are depth 3 and must be admitted");
        assert_eq!(p.i, 0);
    }

    /// Both admitted depths, both contexts — the full 2×2, so a regression that
    /// re-narrows either arm fails here rather than in a journey.
    #[test]
    fn accepts_both_account_depths_in_both_contexts() {
        for ctx in [ScriptCtx::SingleSig, ScriptCtx::MultiSig] {
            for d in [3u8, 4] {
                parse_key(
                    format!("@0={}", with_depth(XPUB_DEPTH4, d)).as_str(),
                    ctx,
                    bitcoin::Network::Bitcoin,
                )
                .unwrap_or_else(|e| panic!("ctx {ctx:?} depth {d} must be admitted: {e:?}"));
            }
        }
    }

    /// The stopgap is a WIDENING, not a removal. A master key and any
    /// leaf/address-level key are still refused — those are the gross
    /// paste-the-wrong-xpub errors the check exists for.
    #[test]
    fn still_rejects_depths_outside_3_and_4() {
        for d in [0u8, 1, 2, 5, 6, 255] {
            let err = parse_key(
                format!("@0={}", with_depth(XPUB_DEPTH4, d)).as_str(),
                ScriptCtx::MultiSig,
                bitcoin::Network::Bitcoin,
            )
            .unwrap_err();
            let msg = format!("{err:?}");
            assert!(msg.contains("depth 3 or 4"), "depth {d} gave: {msg}");
            assert!(msg.contains(&format!("got {d}")), "depth {d} gave: {msg}");
        }
    }

    /// THE SAFETY PROPERTY that makes this a stopgap rather than a wire change.
    ///
    /// `payload` is `bytes[13..78]` — chaincode ‖ pubkey — so depth, parent
    /// fingerprint and child number never reach it. Admitting more keys
    /// therefore cannot move a wallet id, change an encoded md1 string, or
    /// alter a policy-id stub: the same xpub yields the same 65 bytes whatever
    /// its depth byte says.
    #[test]
    fn depth_does_not_affect_the_encoded_payload() {
        let d3 = parse_key(
            format!("@0={}", with_depth(XPUB_DEPTH4, 3)).as_str(),
            ScriptCtx::MultiSig,
            bitcoin::Network::Bitcoin,
        )
        .unwrap();
        let d4 = parse_key(
            format!("@0={}", with_depth(XPUB_DEPTH4, 4)).as_str(),
            ScriptCtx::MultiSig,
            bitcoin::Network::Bitcoin,
        )
        .unwrap();
        assert_eq!(
            d3.payload, d4.payload,
            "depth must not reach the payload; if this fails the stopgap is a wire change"
        );
    }

    /// Abandon-mnemonic tpub at m/84'/1'/0' (BIP 84 testnet account, depth 3).
    pub(crate) const ABANDON_TPUB_DEPTH3_BIP84: &str = "tpubDC8msFGeGuwnKG9Upg7DM2b4DaRqg3CUZa5g8v2SRQ6K4NSkxUgd7HsL2XVWbVm39yBA4LAxysQAm397zwQSQoQgewGiYZqrA9DsP4zbQ1M";
    /// Abandon-mnemonic tpub at m/48'/1'/0'/2' (BIP 48 testnet account, depth 4).
    ///
    /// Was `#[allow(dead_code)]`, "reserved for a future wsh-multi testnet test
    /// fixture (currently unused)". The 2026-08-19 depth stopgap is that test:
    /// `accepts_real_bip48_depth4_tpub_in_multisig` below reads it, so the lint
    /// suppression is no longer needed and has been removed rather than left to
    /// hide a genuinely unused fixture later.
    pub(crate) const ABANDON_TPUB_DEPTH4_BIP48: &str = "tpubDFH9dgzveyD8zTbPUFuLrGmCydNvxehyNdUXKJAQN8x4aZ4j6UZqGfnqFrD4NqyaTVGKbvEW54tsvPTK2UoSbCC1PJY8iCNiwTL3RWZEheQ";

    /// A REAL BIP-48 depth-4 testnet account key, not a depth-restamped one —
    /// so the 2×2 above is backed by at least one genuine fixture per depth.
    #[test]
    fn accepts_real_bip48_depth4_tpub_in_multisig() {
        let p = parse_key(
            format!("@0={ABANDON_TPUB_DEPTH4_BIP48}").as_str(),
            ScriptCtx::MultiSig,
            bitcoin::Network::Testnet,
        )
        .expect("a real BIP-48 depth-4 account key must be admitted");
        assert_eq!(p.i, 0);
        assert_eq!(p.payload.len(), 65);
    }

    #[test]
    fn strips_optional_at_prefix() {
        // Both forms accepted.
        let a = parse_key(
            format!("@1={XPUB_DEPTH4}").as_str(),
            ScriptCtx::MultiSig,
            bitcoin::Network::Bitcoin,
        )
        .unwrap();
        let b = parse_key(
            format!("1={XPUB_DEPTH4}").as_str(),
            ScriptCtx::MultiSig,
            bitcoin::Network::Bitcoin,
        )
        .unwrap();
        assert_eq!(a.i, b.i);
        assert_eq!(a.payload, b.payload);
    }

    // ─── Network-routing tests (Phase 1) ──────────────────────────────────

    #[test]
    fn accepts_tpub_under_testnet() {
        let p = parse_key(
            format!("@0={ABANDON_TPUB_DEPTH3_BIP84}").as_str(),
            ScriptCtx::SingleSig,
            bitcoin::Network::Testnet,
        )
        .unwrap();
        assert_eq!(p.i, 0);
        assert_eq!(p.payload.len(), 65);
    }

    #[test]
    fn accepts_tpub_under_signet() {
        // Signet uses the same testnet version bytes per BIP 32.
        let p = parse_key(
            format!("@0={ABANDON_TPUB_DEPTH3_BIP84}").as_str(),
            ScriptCtx::SingleSig,
            bitcoin::Network::Signet,
        )
        .unwrap();
        assert_eq!(p.i, 0);
    }

    #[test]
    fn accepts_tpub_under_regtest() {
        let p = parse_key(
            format!("@0={ABANDON_TPUB_DEPTH3_BIP84}").as_str(),
            ScriptCtx::SingleSig,
            bitcoin::Network::Regtest,
        )
        .unwrap();
        assert_eq!(p.i, 0);
    }

    #[test]
    fn rejects_xpub_under_testnet() {
        let err = parse_key(
            format!("@0={XPUB_DEPTH4}").as_str(),
            ScriptCtx::MultiSig,
            bitcoin::Network::Testnet,
        )
        .unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("expected testnet"), "got: {msg}");
    }

    #[test]
    fn rejects_tpub_under_mainnet() {
        let err = parse_key(
            format!("@0={ABANDON_TPUB_DEPTH3_BIP84}").as_str(),
            ScriptCtx::SingleSig,
            bitcoin::Network::Bitcoin,
        )
        .unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("expected mainnet"), "got: {msg}");
    }

    // ─── M11 (cycle-9): off-curve xpub point check at parse ───────────────
    // `parse_key` validates base58check / length / version / depth but never
    // checked that the embedded compressed pubkey (`bytes[45..78]`) is a valid
    // secp256k1 point. An off-curve / all-zero pubkey passed intake and only
    // failed later at derive_address. Fix: reject at parse with `BadXpub`.

    /// Re-encode a real xpub with its compressed-pubkey bytes (`bytes[45..78]`)
    /// replaced by `new_key`, preserving version / depth / chaincode so ONLY
    /// the point check can reject it.
    fn xpub_with_pubkey(xpub_str: &str, new_key: [u8; 33]) -> String {
        let mut bytes = base58::decode_check(xpub_str).unwrap();
        assert_eq!(bytes.len(), XPUB_LEN);
        bytes[45..78].copy_from_slice(&new_key);
        base58::encode_check(&bytes)
    }

    #[test]
    fn rejects_off_curve_all_zero_pubkey() {
        // 0x02 || 32 zero bytes — well-formed compressed encoding, but the
        // x-coordinate 0 is NOT on secp256k1, so it must reject at parse.
        let mut bad = [0u8; 33];
        bad[0] = 0x02;
        let bad_xpub = xpub_with_pubkey(XPUB_DEPTH4, bad);
        let err = parse_key(
            format!("@2={bad_xpub}").as_str(),
            ScriptCtx::MultiSig,
            bitcoin::Network::Bitcoin,
        )
        .unwrap_err();
        assert!(
            matches!(err, CliError::BadXpub { i: 2, .. }),
            "got: {err:?}"
        );
        let msg = format!("{err}");
        assert!(
            msg.contains("secp256k1 point"),
            "error must name the point check; got: {msg}"
        );
    }

    #[test]
    fn accepts_real_depth4_xpub_point_check_positive_control() {
        // The real fixture's pubkey IS on-curve → still parses after the check.
        let p = parse_key(
            format!("@2={XPUB_DEPTH4}").as_str(),
            ScriptCtx::MultiSig,
            bitcoin::Network::Bitcoin,
        )
        .unwrap();
        assert_eq!(p.i, 2);
    }

    #[test]
    fn accepts_real_depth3_tpub_point_check_positive_control() {
        // A real depth-3 BIP-84 tpub (on-curve) still parses.
        let p = parse_key(
            format!("@0={ABANDON_TPUB_DEPTH3_BIP84}").as_str(),
            ScriptCtx::SingleSig,
            bitcoin::Network::Testnet,
        )
        .unwrap();
        assert_eq!(p.i, 0);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedFingerprint {
    pub i: u8,
    pub fp: [u8; 4],
}

pub fn parse_fingerprint(arg: &str) -> Result<ParsedFingerprint, CliError> {
    let (i_str, hex_str) = arg
        .split_once('=')
        .ok_or_else(|| CliError::BadArg(format!("--fingerprint expects @i=HEX, got: {arg}")))?;
    let i = parse_index(i_str)?;
    let hex = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    if hex.len() != 8 {
        return Err(CliError::BadFingerprint {
            i,
            why: format!("expected 8 hex chars (4 bytes), got {}", hex.len()),
        });
    }
    let mut fp = [0u8; 4];
    for (n, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let s = std::str::from_utf8(chunk).map_err(|_| CliError::BadFingerprint {
            i,
            why: "non-utf8 hex".into(),
        })?;
        fp[n] = u8::from_str_radix(s, 16).map_err(|_| CliError::BadFingerprint {
            i,
            why: format!("invalid hex byte: {s}"),
        })?;
    }
    Ok(ParsedFingerprint { i, fp })
}

#[cfg(test)]
mod fp_tests {
    use super::*;

    #[test]
    fn accepts_8_hex_chars() {
        let p = parse_fingerprint("@0=deadbeef").unwrap();
        assert_eq!(p.i, 0);
        assert_eq!(p.fp, [0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn accepts_0x_prefix() {
        let p = parse_fingerprint("@1=0xCAFEBABE").unwrap();
        assert_eq!(p.fp, [0xCA, 0xFE, 0xBA, 0xBE]);
    }

    #[test]
    fn rejects_wrong_length() {
        let err = parse_fingerprint("@0=dead").unwrap_err();
        assert!(matches!(err, CliError::BadFingerprint { i: 0, .. }));
    }

    #[test]
    fn rejects_non_hex() {
        let err = parse_fingerprint("@0=zzzzzzzz").unwrap_err();
        assert!(matches!(err, CliError::BadFingerprint { i: 0, .. }));
    }
}
