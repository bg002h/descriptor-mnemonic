// The `--path` CLI flag is declared on `Encode` for forward-compat (BIP 388
// non-canonical wrappers will need it once the codec accepts an explicit
// override on encode). The plumbing is wired but not yet consumed; see
// follow-up `cli-path-arg-routing` once the codec API surfaces it.
#![allow(dead_code)]

use crate::error::CliError;
use bitcoin::bip32::DerivationPath;
use std::str::FromStr;

/// Parse a `--path <PATH>` argument: a name, a hex indicator, or a literal path.
pub fn parse_path(arg: &str) -> Result<DerivationPath, CliError> {
    if let Some(p) = parse_path_name(arg) {
        return Ok(p);
    }
    if let Some(p) = parse_path_hex(arg)? {
        return Ok(p);
    }
    DerivationPath::from_str(arg).map_err(|e| CliError::BadArg(
        format!("--path could not parse `{arg}` as name, hex, or literal path: {e}")
    ))
}

fn parse_path_name(s: &str) -> Option<DerivationPath> {
    match s {
        "bip44" => Some(DerivationPath::from_str("m/44'/0'/0'").unwrap()),
        "bip49" => Some(DerivationPath::from_str("m/49'/0'/0'").unwrap()),
        "bip84" => Some(DerivationPath::from_str("m/84'/0'/0'").unwrap()),
        "bip86" => Some(DerivationPath::from_str("m/86'/0'/0'").unwrap()),
        "bip48" => Some(DerivationPath::from_str("m/48'/0'/0'/2'").unwrap()),
        _ => None,
    }
}

fn parse_path_hex(s: &str) -> Result<Option<DerivationPath>, CliError> {
    let Some(rest) = s.strip_prefix("0x") else { return Ok(None) };
    let n = u32::from_str_radix(rest, 16).map_err(|_| CliError::BadArg(
        format!("--path hex value invalid: {s}")
    ))?;
    // Hex indicator selects a single hardened account-level path m/n'.
    let path = DerivationPath::from_str(&format!("m/{n}'")).map_err(|e| CliError::BadArg(
        format!("--path hex {s} → m/{n}': {e}")
    ))?;
    Ok(Some(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_name_bip48() {
        let p = parse_path("bip48").unwrap();
        assert_eq!(p.to_string(), "48'/0'/0'/2'");
    }

    #[test]
    fn parses_hex() {
        let p = parse_path("0x05").unwrap();
        assert_eq!(p.to_string(), "5'");
    }

    #[test]
    fn parses_literal() {
        let p = parse_path("m/48'/0'/0'/2'").unwrap();
        assert_eq!(p.to_string(), "48'/0'/0'/2'");
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_path("not-a-path").is_err());
    }
}
