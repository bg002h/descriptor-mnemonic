use crate::error::CliError;
use bitcoin::bip32::DerivationPath;
use regex::Regex;
use std::str::FromStr;
use std::sync::OnceLock;

/// One occurrence of a `@i/...` placeholder in the raw template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceholderOccurrence {
    pub i: u8,
    pub origin_path: Option<DerivationPath>,
    pub multipath_alts: Vec<u32>,
    pub wildcard_hardened: bool,
}

/// Pass A: extract every `@i/...` placeholder from the raw template string.
pub fn lex_placeholders(template: &str) -> Result<Vec<PlaceholderOccurrence>, CliError> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        // Captures:
        //   1: @i index digits
        //   2: optional origin path (e.g. "/48'/0'/0'/2'")
        //   3: optional multipath body (e.g. "0;1")
        //   4: wildcard with optional hardening (e.g. "*", "*'", "*h")
        Regex::new(
            r"@(\d+)((?:/\d+'?)*)(?:/<([0-9;]+)>)?(/\*(?:'|h)?)?"
        ).expect("static regex compiles")
    });
    let mut out = Vec::new();
    for caps in re.captures_iter(template) {
        let i: u8 = caps[1].parse().map_err(|_| CliError::TemplateParse(
            format!("@i index out of range: @{}", &caps[1])
        ))?;
        let origin_path = if let Some(m) = caps.get(2) {
            let s = m.as_str();
            if s.is_empty() { None } else {
                Some(DerivationPath::from_str(s.trim_start_matches('/'))
                    .map_err(|e| CliError::TemplateParse(format!("@{i} origin path `{s}`: {e}")))?)
            }
        } else { None };
        let multipath_alts = if let Some(m) = caps.get(3) {
            m.as_str().split(';').map(|n| n.parse::<u32>()
                .map_err(|_| CliError::TemplateParse(format!("@{i} multipath alt `{n}` not u32"))))
                .collect::<Result<Vec<_>, _>>()?
        } else { Vec::new() };
        let wildcard_hardened = caps.get(4).map(|m| m.as_str().ends_with('\'') || m.as_str().ends_with('h')).unwrap_or(false);
        out.push(PlaceholderOccurrence { i, origin_path, multipath_alts, wildcard_hardened });
    }
    if out.is_empty() {
        return Err(CliError::TemplateParse("template contains no @i placeholders".into()));
    }
    Ok(out)
}

#[cfg(test)]
mod lex_tests {
    use super::*;

    #[test]
    fn single_at0_no_multipath() {
        let v = lex_placeholders("wpkh(@0/*)").unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].i, 0);
        assert_eq!(v[0].multipath_alts, Vec::<u32>::new());
        assert!(!v[0].wildcard_hardened);
    }

    #[test]
    fn at0_hardened_wildcard() {
        let v = lex_placeholders("wpkh(@0/*')").unwrap();
        assert!(v[0].wildcard_hardened);
    }

    #[test]
    fn at0_hardened_wildcard_h_form() {
        let v = lex_placeholders("wpkh(@0/*h)").unwrap();
        assert!(v[0].wildcard_hardened);
    }

    #[test]
    fn multipath_arity_2() {
        let v = lex_placeholders("wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))").unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].multipath_alts, vec![0, 1]);
        assert_eq!(v[1].multipath_alts, vec![0, 1]);
    }

    #[test]
    fn multipath_arity_3() {
        let v = lex_placeholders("wpkh(@0/<0;1;2>/*)").unwrap();
        assert_eq!(v[0].multipath_alts, vec![0, 1, 2]);
    }

    #[test]
    fn origin_path_extracted() {
        let v = lex_placeholders("wpkh(@0/48'/0'/0'/2'/<0;1>/*)").unwrap();
        assert_eq!(v[0].origin_path.as_ref().unwrap().to_string(), "48'/0'/0'/2'");
    }

    #[test]
    fn multiple_at_i_collected() {
        let v = lex_placeholders("wsh(multi(3,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))").unwrap();
        assert_eq!(v.len(), 3);
        assert_eq!(v.iter().map(|p| p.i).collect::<Vec<_>>(), vec![0, 1, 2]);
    }

    #[test]
    fn rejects_template_with_no_placeholders() {
        assert!(lex_placeholders("wpkh(xpubAAAAA)").is_err());
    }
}

use md_codec::origin_path::{OriginPath, PathComponent, PathDecl, PathDeclPaths};
use md_codec::use_site_path::{Alternative, UseSitePath};
use bitcoin::bip32::ChildNumber;

/// Resolved per-`@i` view after consistency checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPlaceholders {
    pub n: u8,
    pub path_decl: PathDecl,
    pub use_site_path: UseSitePath,
    pub use_site_path_overrides: Vec<(u8, UseSitePath)>,
}

pub fn resolve_placeholders(occs: &[PlaceholderOccurrence]) -> Result<ResolvedPlaceholders, CliError> {
    // Collapse same-@i occurrences; reject if conflicting.
    let mut by_i: std::collections::BTreeMap<u8, &PlaceholderOccurrence> = std::collections::BTreeMap::new();
    for occ in occs {
        if let Some(prev) = by_i.get(&occ.i) {
            if prev.multipath_alts != occ.multipath_alts
                || prev.wildcard_hardened != occ.wildcard_hardened
                || prev.origin_path != occ.origin_path
            {
                return Err(CliError::TemplateParse(format!(
                    "@{} appears with inconsistent path/multipath/hardening", occ.i
                )));
            }
        } else {
            by_i.insert(occ.i, occ);
        }
    }
    let n = (by_i.keys().max().copied().ok_or_else(|| CliError::TemplateParse("no placeholders".into()))? as usize + 1) as u8;
    for i in 0..n {
        if !by_i.contains_key(&i) {
            return Err(CliError::TemplateParse(format!("@{i} not present; placeholders must be dense 0..n")));
        }
    }
    let at0 = by_i[&0];
    let use_site_path = make_use_site_path(at0)?;
    let mut use_site_path_overrides = Vec::new();
    for i in 1..n {
        let occ = by_i[&i];
        let usp_i = make_use_site_path(occ)?;
        if usp_i != use_site_path {
            use_site_path_overrides.push((i, usp_i));
        }
    }
    let path_decl = make_path_decl(&by_i, n, at0)?;
    Ok(ResolvedPlaceholders { n, path_decl, use_site_path, use_site_path_overrides })
}

fn make_use_site_path(occ: &PlaceholderOccurrence) -> Result<UseSitePath, CliError> {
    let alts: Vec<Alternative> = occ.multipath_alts.iter()
        .map(|v| Alternative { hardened: false, value: *v })
        .collect();
    Ok(UseSitePath {
        multipath: if alts.is_empty() { None } else { Some(alts) },
        wildcard_hardened: occ.wildcard_hardened,
    })
}

/// Convert a `bitcoin::DerivationPath` (or absence-of-path) into an `OriginPath`.
/// `None` becomes the empty origin (depth 0); otherwise each child becomes a
/// `PathComponent { hardened, value }`.
pub(crate) fn to_origin_path(p: Option<&bitcoin::bip32::DerivationPath>) -> OriginPath {
    let components = match p {
        None => Vec::new(),
        Some(dp) => dp.into_iter().map(|c| match c {
            ChildNumber::Normal { index }   => PathComponent { hardened: false, value: *index },
            ChildNumber::Hardened { index } => PathComponent { hardened: true,  value: *index },
        }).collect(),
    };
    OriginPath { components }
}

fn make_path_decl(
    by_i: &std::collections::BTreeMap<u8, &PlaceholderOccurrence>,
    n: u8,
    at0: &PlaceholderOccurrence,
) -> Result<PathDecl, CliError> {
    let all_same = (0..n).all(|i| by_i[&i].origin_path == at0.origin_path);
    let paths = if all_same {
        PathDeclPaths::Shared(to_origin_path(at0.origin_path.as_ref()))
    } else {
        let v: Vec<OriginPath> = (0..n).map(|i| to_origin_path(by_i[&i].origin_path.as_ref())).collect();
        PathDeclPaths::Divergent(v)
    };
    Ok(PathDecl { n, paths })
}

#[cfg(test)]
mod resolve_tests {
    use super::*;

    #[test]
    fn shared_use_site_path_when_all_match() {
        let occs = lex_placeholders("wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))").unwrap();
        let r = resolve_placeholders(&occs).unwrap();
        assert_eq!(r.n, 2);
        assert!(r.use_site_path_overrides.is_empty());
    }

    #[test]
    fn divergent_use_site_path_when_at1_differs() {
        let occs = lex_placeholders("wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))").unwrap();
        let r = resolve_placeholders(&occs).unwrap();
        assert_eq!(r.n, 2);
        assert_eq!(r.use_site_path_overrides.len(), 1);
        assert_eq!(r.use_site_path_overrides[0].0, 1);
    }

    #[test]
    fn rejects_nondense_placeholders() {
        let occs = lex_placeholders("wsh(multi(2,@0/<0;1>/*,@2/<0;1>/*))").unwrap();
        let err = resolve_placeholders(&occs).unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("dense"), "got: {msg}");
    }

    #[test]
    fn rejects_same_at_i_conflicting() {
        // Synthesize directly, lexer would also accept these as separate occurrences.
        let occs = vec![
            PlaceholderOccurrence { i: 0, origin_path: None, multipath_alts: vec![0,1], wildcard_hardened: false },
            PlaceholderOccurrence { i: 0, origin_path: None, multipath_alts: vec![2,3], wildcard_hardened: false },
        ];
        let err = resolve_placeholders(&occs).unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("inconsistent"), "got: {msg}");
    }
}

use crate::parse::keys::{ScriptCtx, MAINNET_XPUB_VERSION};

/// A synthetic xpub keyed by placeholder index `i` and the outer script context.
/// Depth tracks BIP 388 expectation: depth 3 for single-sig (wpkh/pkh), depth
/// 4 for multisig/taproot. Deterministic, never emitted to wire.
fn synthetic_xpub_for(i: u8, ctx: ScriptCtx) -> String {
    use bitcoin::base58;
    let mut bytes = [0u8; 78];
    bytes[0..4].copy_from_slice(&MAINNET_XPUB_VERSION);
    bytes[4] = match ctx { ScriptCtx::SingleSig => 3, ScriptCtx::MultiSig => 4 };
    bytes[5..9].copy_from_slice(&[0;4]);   // parent fp (zeros)
    bytes[9..13].copy_from_slice(&[0;4]);  // child number (zeros)
    bytes[13] = i;                         // first chain-code byte = i (uniqueness)
    bytes[45] = 0x02;                      // compressed pubkey prefix (even)
    bytes[46..78].copy_from_slice(&[i; 32]); // pubkey body = 0x{ii} * 32
    base58::encode_check(&bytes)
}

/// Substitute each `@i/...` with a synthetic xpub. Returns substituted template
/// + map (synthetic-xpub-string → placeholder index).
fn substitute_synthetic(template: &str, ctx: ScriptCtx)
    -> Result<(String, std::collections::BTreeMap<String, u8>), CliError>
{
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(
        r"@(\d+)((?:/\d+'?)*)(?:/<[0-9;]+>)?(?:/\*(?:'|h)?)?"
    ).expect("static regex compiles"));
    let mut key_map = std::collections::BTreeMap::new();
    let mut keys_seen = std::collections::HashSet::new();
    let out = re.replace_all(template, |caps: &regex::Captures| {
        let i: u8 = caps[1].parse().unwrap_or(0);
        let xpub = synthetic_xpub_for(i, ctx);
        if keys_seen.insert(i) {
            key_map.insert(xpub.clone(), i);
        }
        xpub
    }).into_owned();
    Ok((out, key_map))
}

#[cfg(test)]
mod sub_tests {
    use super::*;

    #[test]
    fn synthetic_for_0_and_1_differ() {
        assert_ne!(synthetic_xpub_for(0, ScriptCtx::MultiSig), synthetic_xpub_for(1, ScriptCtx::MultiSig));
    }

    #[test]
    fn synthetic_for_0_is_stable() {
        assert_eq!(synthetic_xpub_for(0, ScriptCtx::MultiSig), synthetic_xpub_for(0, ScriptCtx::MultiSig));
    }

    #[test]
    fn singlesig_synthetic_uses_depth_3() {
        // Cross-context xpubs must differ — depth byte 4 is 3 vs 4.
        assert_ne!(synthetic_xpub_for(0, ScriptCtx::SingleSig), synthetic_xpub_for(0, ScriptCtx::MultiSig));
    }

    #[test]
    fn substitution_strips_at_i_suffix() {
        let (s, _) = substitute_synthetic("wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))", ScriptCtx::MultiSig).unwrap();
        assert!(!s.contains('@'));
        assert!(!s.contains('<'));
        assert!(!s.contains('*'));
    }

    #[test]
    fn substitution_emits_consistent_keys_per_index() {
        let (s, km) = substitute_synthetic("wsh(or_d(pk(@0/<0;1>/*),pk(@0/<0;1>/*)))", ScriptCtx::MultiSig).unwrap();
        assert_eq!(km.len(), 1);
        let key = synthetic_xpub_for(0, ScriptCtx::MultiSig);
        assert_eq!(s.matches(&key).count(), 2);
    }
}
