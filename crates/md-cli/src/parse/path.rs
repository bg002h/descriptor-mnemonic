// `--path` parsing, shared by `md encode`, `md address` and `md verify`. Accepts named forms
// (`bip44`, `bip48`, `bip49`, `bip84`, `bip86`), hex form (`0xNN`), or
// a literal `m/...` path. Routed through `cmd::encode::run` to override
// `descriptor.path_decl.paths` with a Shared origin path.

use crate::error::CliError;
use crate::parse::template::to_origin_path;
use bitcoin::bip32::DerivationPath;
use md_codec::Descriptor;
use md_codec::origin_path::PathDeclPaths;
use std::str::FromStr;

/// Apply a `--path` override to a parsed descriptor, if one was supplied.
///
/// ONE IMPLEMENTATION, THREE CALLERS. `encode` carried this inline and
/// `address`/`verify` had no `--path` at all, which meant the shapes that
/// REQUIRE an explicit origin -- taproot with a script tree, and non-canonical
/// wrappers generally -- could be encoded but never had an address derived or a
/// backup verified from their template:
///
///     md: codec error: non-canonical wrapper requires explicit origin for @0,
///         but none provided
///
/// Three copies of a rule about how a descriptor's origin is decided is exactly
/// the shape that drifts, and origin drift moves funds to the wrong addresses.
pub fn apply_path_override(
    descriptor: &mut Descriptor,
    path: Option<&str>,
) -> Result<(), CliError> {
    if let Some(arg) = path {
        let dp = parse_path(arg)?;
        descriptor.path_decl.paths = PathDeclPaths::Shared(to_origin_path(Some(&dp)));
    }
    Ok(())
}

/// Parse a `--path <PATH>` argument: a name, a hex indicator, or a literal path.
pub fn parse_path(arg: &str) -> Result<DerivationPath, CliError> {
    if let Some(p) = parse_path_name(arg) {
        return Ok(p);
    }
    if let Some(p) = parse_path_hex(arg)? {
        return Ok(p);
    }
    DerivationPath::from_str(arg).map_err(|e| {
        CliError::BadArg(format!(
            "--path could not parse `{arg}` as name, hex, or literal path: {e}"
        ))
    })
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
    let Some(rest) = s.strip_prefix("0x") else {
        return Ok(None);
    };
    let n = u32::from_str_radix(rest, 16)
        .map_err(|_| CliError::BadArg(format!("--path hex value invalid: {s}")))?;
    // Hex indicator selects a single hardened account-level path m/n'.
    let path = DerivationPath::from_str(&format!("m/{n}'"))
        .map_err(|e| CliError::BadArg(format!("--path hex {s} → m/{n}': {e}")))?;
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
