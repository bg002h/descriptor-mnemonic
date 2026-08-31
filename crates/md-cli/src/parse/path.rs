// `--path` parsing, shared by `md encode`, `md address` and `md verify`. Accepts named forms
// (`bip44`, `bip48`, `bip49`, `bip84`, `bip86`), hex form (`0xNN`), or
// a literal `m/...` path. Routed through `cmd::encode::run` to override
// `descriptor.path_decl.paths` with a Shared origin path.

use crate::error::CliError;
use crate::parse::template::to_origin_path;
use bitcoin::bip32::DerivationPath;
use md_codec::Descriptor;
use md_codec::origin_path::{OriginPath, PathDeclPaths};
use std::collections::{BTreeMap, BTreeSet};
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

/// Apply a `--path` override PER SLOT (SPEC P1's per-datum precedence,
/// widened by N3's bracket-as-last-resort-source): an inline template origin
/// WINS for the slots that declared one; `--path` fills in the slots that
/// did not; an origin-notated `--key` bracket path fills in whatever is
/// STILL left — never the whole-descriptor overwrite [`apply_path_override`]
/// above does. "Agreement is not an override" (P1) — a slot with an inline
/// origin keeps it verbatim even when `path` disagrees with it; the caller
/// (`build_descriptor`) is responsible for refusing that disagreement BEFORE
/// this runs, per V-PATHAGREE/V-PATHEFF — this function only decides who
/// WINS, never who is right.
///
/// `inline_declared` names which `@i` slots the TEMPLATE itself gave an
/// origin to. `descriptor.path_decl.paths` alone cannot answer that
/// question once folded to `Shared`/`Divergent` by
/// `template::make_path_decl` — a slot with no inline origin and a slot
/// explicitly declaring the empty/root origin both fold to the same empty
/// `OriginPath` (there is no template syntax for the latter, so this is
/// not a real ambiguity in practice, but the fold has already discarded
/// the distinction). The caller computes the set from
/// `parse::template::lex_placeholders` BEFORE the fold and passes it in.
///
/// `bracket_sourced` names the per-slot paths N3 admits: SPEC N3's
/// precedence is "inline template origin > --path > --key bracket", so by
/// construction `resolve_keys_fingerprints_and_precedence` only ever
/// populates this map for a slot when BOTH the first two sources were
/// silent for it — a slot present here can never also be in
/// `inline_declared`, and whenever `path` is `Some` every entry here has
/// already been checked to AGREE with the shared path (V-PATHEFF), so it is
/// redundant with `shared` rather than in tension with it. `--path` still
/// wins the slot outright in that case; this map only matters when `path`
/// is `None`.
///
/// Used only by `cmd::build::build_descriptor` (`descriptor`/`address`,
/// SPEC P1/N3) — `encode`/`verify`/`vectors` keep [`apply_path_override`]'s
/// existing whole-descriptor-overwrite behaviour, unchanged and out of
/// C1's scope (their templates have no analogous per-slot precedence rule
/// defined yet).
pub fn apply_path_override_per_slot(
    descriptor: &mut Descriptor,
    path: Option<&str>,
    inline_declared: &BTreeSet<u8>,
    bracket_sourced: &BTreeMap<u8, DerivationPath>,
) -> Result<(), CliError> {
    if path.is_none() && bracket_sourced.is_empty() {
        // Nothing left to fill in from any of the two sources this function
        // owns; leave path_decl exactly as parse_template built it.
        return Ok(());
    }
    let n = descriptor.path_decl.n;
    if (0..n).all(|i| inline_declared.contains(&i)) {
        // Every slot already has an inline origin, which by construction
        // means bracket_sourced is empty too (see the doc comment above) —
        // neither remaining source has anything left to fill in.
        return Ok(());
    }
    let shared: Option<OriginPath> = match path {
        Some(arg) => Some(to_origin_path(Some(&parse_path(arg)?))),
        None => None,
    };
    let current = |i: u8| -> OriginPath {
        match &descriptor.path_decl.paths {
            PathDeclPaths::Shared(p) => p.clone(),
            PathDeclPaths::Divergent(v) => v[i as usize].clone(),
        }
    };
    let merged: Vec<OriginPath> = (0..n)
        .map(|i| {
            if inline_declared.contains(&i) {
                current(i)
            } else if let Some(shared) = &shared {
                shared.clone()
            } else if let Some(bracket) = bracket_sourced.get(&i) {
                to_origin_path(Some(bracket))
            } else {
                current(i)
            }
        })
        .collect();
    descriptor.path_decl.paths = if merged.windows(2).all(|w| w[0] == w[1]) {
        PathDeclPaths::Shared(
            merged
                .into_iter()
                .next()
                .unwrap_or(OriginPath { components: vec![] }),
        )
    } else {
        PathDeclPaths::Divergent(merged)
    };
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
