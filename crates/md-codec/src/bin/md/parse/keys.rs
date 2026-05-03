use crate::error::CliError;
use bitcoin::base58;

const XPUB_LEN: usize = 78;
pub(crate) const MAINNET_XPUB_VERSION: [u8; 4] = [0x04, 0x88, 0xB2, 0x1E];

/// Script-context expectation for depth validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptCtx {
    /// Single-sig: depth 3 expected (e.g. wpkh, pkh).
    SingleSig,
    /// Multisig / taproot: depth 4 expected (e.g. wsh, sh-wsh, tr).
    MultiSig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedKey {
    pub i: u8,
    /// chain code (32) ‖ compressed pubkey (33).
    pub payload: [u8; 65],
}

pub fn parse_key(arg: &str, ctx: ScriptCtx) -> Result<ParsedKey, CliError> {
    let (i_str, xpub_str) = arg.split_once('=').ok_or_else(|| CliError::BadArg(
        format!("--key expects @i=XPUB, got: {arg}")
    ))?;
    let i = parse_index(i_str)?;
    let bytes = base58::decode_check(xpub_str)
        .map_err(|e| CliError::BadXpub { i, why: format!("base58check decode: {e}") })?;
    if bytes.len() != XPUB_LEN {
        return Err(CliError::BadXpub { i, why: format!("expected 78 bytes, got {}", bytes.len()) });
    }
    if bytes[0..4] != MAINNET_XPUB_VERSION {
        return Err(CliError::BadXpub { i, why: format!(
            "expected mainnet xpub version 0488B21E, got {:02X}{:02X}{:02X}{:02X}",
            bytes[0], bytes[1], bytes[2], bytes[3]
        )});
    }
    let depth = bytes[4];
    let expected_depth = match ctx { ScriptCtx::SingleSig => 3, ScriptCtx::MultiSig => 4 };
    if depth != expected_depth {
        return Err(CliError::BadXpub { i, why: format!(
            "expected depth {expected_depth} for this script context, got {depth}"
        )});
    }
    let mut payload = [0u8; 65];
    payload.copy_from_slice(&bytes[13..78]);
    Ok(ParsedKey { i, payload })
}

fn parse_index(s: &str) -> Result<u8, CliError> {
    let stripped = s.strip_prefix('@').unwrap_or(s);
    stripped.parse::<u8>().map_err(|_| CliError::BadArg(
        format!("--key index must be 0..255, got: {s}")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real xpub at depth 4 (m/48'/0'/0'/2') from the abandon-mnemonic, mainnet.
    const XPUB_DEPTH4: &str = "xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf";

    #[test]
    fn rejects_no_equals() {
        let err = parse_key("@0xpub6...", ScriptCtx::MultiSig).unwrap_err();
        assert!(matches!(err, CliError::BadArg(_)));
    }

    #[test]
    fn rejects_bad_index() {
        let err = parse_key(format!("@notnum={XPUB_DEPTH4}").as_str(), ScriptCtx::MultiSig).unwrap_err();
        assert!(matches!(err, CliError::BadArg(_)));
    }

    #[test]
    fn rejects_bad_checksum() {
        let err = parse_key("@0=xpubBADCHECKSUMxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", ScriptCtx::MultiSig).unwrap_err();
        assert!(matches!(err, CliError::BadXpub { i: 0, .. }), "got: {err:?}");
    }

    #[test]
    fn accepts_valid_depth4_xpub() {
        let parsed = parse_key(format!("@2={XPUB_DEPTH4}").as_str(), ScriptCtx::MultiSig).unwrap();
        assert_eq!(parsed.i, 2);
        assert_eq!(parsed.payload.len(), 65);
    }

    #[test]
    fn rejects_depth4_xpub_in_singlesig_context() {
        let err = parse_key(format!("@0={XPUB_DEPTH4}").as_str(), ScriptCtx::SingleSig).unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("depth 3"), "got: {msg}");
    }

    /// Abandon-mnemonic tpub at m/84'/1'/0' (BIP 84 testnet account, depth 3).
    pub(crate) const ABANDON_TPUB_DEPTH3_BIP84: &str = "tpubDC8msFGeGuwnKG9Upg7DM2b4DaRqg3CUZa5g8v2SRQ6K4NSkxUgd7HsL2XVWbVm39yBA4LAxysQAm397zwQSQoQgewGiYZqrA9DsP4zbQ1M";
    /// Abandon-mnemonic tpub at m/48'/1'/0'/2' (BIP 48 testnet account, depth 4).
    #[allow(dead_code)] // referenced by future Phase 4 wsh-multi testnet test if added
    pub(crate) const ABANDON_TPUB_DEPTH4_BIP48: &str = "tpubDFH9dgzveyD8zTbPUFuLrGmCydNvxehyNdUXKJAQN8x4aZ4j6UZqGfnqFrD4NqyaTVGKbvEW54tsvPTK2UoSbCC1PJY8iCNiwTL3RWZEheQ";

    #[test]
    fn strips_optional_at_prefix() {
        // Both forms accepted.
        let a = parse_key(format!("@1={XPUB_DEPTH4}").as_str(), ScriptCtx::MultiSig).unwrap();
        let b = parse_key(format!("1={XPUB_DEPTH4}").as_str(), ScriptCtx::MultiSig).unwrap();
        assert_eq!(a.i, b.i);
        assert_eq!(a.payload, b.payload);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedFingerprint {
    pub i: u8,
    pub fp: [u8; 4],
}

pub fn parse_fingerprint(arg: &str) -> Result<ParsedFingerprint, CliError> {
    let (i_str, hex_str) = arg.split_once('=').ok_or_else(|| CliError::BadArg(
        format!("--fingerprint expects @i=HEX, got: {arg}")
    ))?;
    let i = parse_index(i_str)?;
    let hex = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    if hex.len() != 8 {
        return Err(CliError::BadFingerprint { i, why: format!(
            "expected 8 hex chars (4 bytes), got {}", hex.len()
        )});
    }
    let mut fp = [0u8; 4];
    for (n, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let s = std::str::from_utf8(chunk).map_err(|_| CliError::BadFingerprint {
            i, why: "non-utf8 hex".into()
        })?;
        fp[n] = u8::from_str_radix(s, 16).map_err(|_| CliError::BadFingerprint {
            i, why: format!("invalid hex byte: {s}")
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
