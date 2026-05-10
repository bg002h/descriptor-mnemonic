use crate::error::CliError;

/// BIP-341 §"Constructing and spending Taproot outputs" NUMS H-point — the
/// canonical x-only public key with no known discrete log. Used as the
/// taproot internal key when a descriptor has no extractable single-key
/// spend. v0.18 encodes this via the `Body::Tr { key_index = n, .. }`
/// sentinel rule: when the descriptor's `tr()` internal key is exactly this
/// value, encoders MUST emit `key_index == n` (one past the highest
/// placeholder index). v0.17 used the now-removed `Tag::TrUnspendable`
/// extension code 0x05; v0.18 freed that code in favor of the sentinel for
/// engraving compactness (saves 3-4 bits per usage).
///
/// Lives in `parse/template.rs` (unconditional) rather than `compile.rs`
/// (feature-gated `cli-compiler`) so it is available to all consumers
/// (`format/text.rs` rendering, `walk_tr` recognition, plus `compile.rs`
/// when the feature is enabled).
pub(crate) const NUMS_H_POINT_X_ONLY_HEX: &str =
    "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0";
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
        Regex::new(r"@(\d+)((?:/\d+'?)*)(?:/<([0-9;]+)>)?(/\*(?:'|h)?)?")
            .expect("static regex compiles")
    });
    let mut out = Vec::new();
    for caps in re.captures_iter(template) {
        let i: u8 = caps[1].parse().map_err(|_| {
            CliError::TemplateParse(format!("@i index out of range: @{}", &caps[1]))
        })?;
        let origin_path = if let Some(m) = caps.get(2) {
            let s = m.as_str();
            if s.is_empty() {
                None
            } else {
                Some(
                    DerivationPath::from_str(s.trim_start_matches('/')).map_err(|e| {
                        CliError::TemplateParse(format!("@{i} origin path `{s}`: {e}"))
                    })?,
                )
            }
        } else {
            None
        };
        let multipath_alts = if let Some(m) = caps.get(3) {
            m.as_str()
                .split(';')
                .map(|n| {
                    n.parse::<u32>().map_err(|_| {
                        CliError::TemplateParse(format!("@{i} multipath alt `{n}` not u32"))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?
        } else {
            Vec::new()
        };
        let wildcard_hardened = caps
            .get(4)
            .map(|m| m.as_str().ends_with('\'') || m.as_str().ends_with('h'))
            .unwrap_or(false);
        out.push(PlaceholderOccurrence {
            i,
            origin_path,
            multipath_alts,
            wildcard_hardened,
        });
    }
    if out.is_empty() {
        return Err(CliError::TemplateParse(
            "template contains no @i placeholders".into(),
        ));
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
        assert_eq!(
            v[0].origin_path.as_ref().unwrap().to_string(),
            "48'/0'/0'/2'"
        );
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

use bitcoin::bip32::ChildNumber;
use md_codec::origin_path::{OriginPath, PathComponent, PathDecl, PathDeclPaths};
use md_codec::use_site_path::{Alternative, UseSitePath};

/// Resolved per-`@i` view after consistency checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPlaceholders {
    pub n: u8,
    pub path_decl: PathDecl,
    pub use_site_path: UseSitePath,
    pub use_site_path_overrides: Vec<(u8, UseSitePath)>,
}

pub fn resolve_placeholders(
    occs: &[PlaceholderOccurrence],
) -> Result<ResolvedPlaceholders, CliError> {
    // Collapse same-@i occurrences; reject if conflicting.
    let mut by_i: std::collections::BTreeMap<u8, &PlaceholderOccurrence> =
        std::collections::BTreeMap::new();
    for occ in occs {
        if let Some(prev) = by_i.get(&occ.i) {
            if prev.multipath_alts != occ.multipath_alts
                || prev.wildcard_hardened != occ.wildcard_hardened
                || prev.origin_path != occ.origin_path
            {
                return Err(CliError::TemplateParse(format!(
                    "@{} appears with inconsistent path/multipath/hardening",
                    occ.i
                )));
            }
        } else {
            by_i.insert(occ.i, occ);
        }
    }
    let n = (by_i
        .keys()
        .max()
        .copied()
        .ok_or_else(|| CliError::TemplateParse("no placeholders".into()))? as usize
        + 1) as u8;
    for i in 0..n {
        if !by_i.contains_key(&i) {
            return Err(CliError::TemplateParse(format!(
                "@{i} not present; placeholders must be dense 0..n"
            )));
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
    Ok(ResolvedPlaceholders {
        n,
        path_decl,
        use_site_path,
        use_site_path_overrides,
    })
}

fn make_use_site_path(occ: &PlaceholderOccurrence) -> Result<UseSitePath, CliError> {
    let alts: Vec<Alternative> = occ
        .multipath_alts
        .iter()
        .map(|v| Alternative {
            hardened: false,
            value: *v,
        })
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
        Some(dp) => dp
            .into_iter()
            .map(|c| match c {
                ChildNumber::Normal { index } => PathComponent {
                    hardened: false,
                    value: *index,
                },
                ChildNumber::Hardened { index } => PathComponent {
                    hardened: true,
                    value: *index,
                },
            })
            .collect(),
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
        let v: Vec<OriginPath> = (0..n)
            .map(|i| to_origin_path(by_i[&i].origin_path.as_ref()))
            .collect();
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
            PlaceholderOccurrence {
                i: 0,
                origin_path: None,
                multipath_alts: vec![0, 1],
                wildcard_hardened: false,
            },
            PlaceholderOccurrence {
                i: 0,
                origin_path: None,
                multipath_alts: vec![2, 3],
                wildcard_hardened: false,
            },
        ];
        let err = resolve_placeholders(&occs).unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("inconsistent"), "got: {msg}");
    }
}

use crate::parse::keys::{MAINNET_XPUB_VERSION, ScriptCtx};

/// A synthetic xpub keyed by placeholder index `i` and the outer script context.
/// Depth tracks BIP 388 expectation: depth 3 for single-sig (wpkh/pkh), depth
/// 4 for multisig/taproot. Deterministic, never emitted to wire.
///
/// The pubkey is a real secp256k1 point derived from a deterministic seed so
/// that miniscript's parser (which validates curve membership) accepts it.
fn synthetic_xpub_for(i: u8, ctx: ScriptCtx) -> String {
    use bitcoin::base58;
    use bitcoin::hashes::{Hash, sha256};
    use bitcoin::secp256k1::{Secp256k1, SecretKey};
    let depth = match ctx {
        ScriptCtx::SingleSig => 3u8,
        ScriptCtx::MultiSig => 4u8,
    };
    // Deterministic per (i, depth); domain-separated tag keeps test fixtures stable.
    let seed = sha256::Hash::hash(&[b'm', b'd', b'-', b'v', b'0', b'.', b'1', b'5', i, depth]);
    let chain_code = sha256::Hash::hash(&[b'c', b'c', i, depth]).to_byte_array();
    let secret = SecretKey::from_slice(&seed.to_byte_array()).expect("hash is valid scalar");
    let pubkey = secret.public_key(&Secp256k1::new()).serialize(); // 33-byte compressed
    let mut bytes = [0u8; 78];
    bytes[0..4].copy_from_slice(&MAINNET_XPUB_VERSION);
    bytes[4] = depth;
    // parent fp, child number left as zeros — bip32 has no cryptographic check on these.
    bytes[13..45].copy_from_slice(&chain_code);
    bytes[45..78].copy_from_slice(&pubkey);
    base58::encode_check(&bytes)
}

/// Substitute each `@i/...` with a synthetic xpub. Returns substituted template
/// + map (synthetic-xpub-string → placeholder index).
fn substitute_synthetic(
    template: &str,
    ctx: ScriptCtx,
) -> Result<(String, std::collections::BTreeMap<String, u8>), CliError> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r"@(\d+)((?:/\d+'?)*)(?:/<[0-9;]+>)?(?:/\*(?:'|h)?)?")
            .expect("static regex compiles")
    });
    let mut key_map = std::collections::BTreeMap::new();
    let mut keys_seen = std::collections::HashSet::new();
    let out = re
        .replace_all(template, |caps: &regex::Captures| {
            let i: u8 = caps[1].parse().unwrap_or(0);
            let xpub = synthetic_xpub_for(i, ctx);
            if keys_seen.insert(i) {
                key_map.insert(xpub.clone(), i);
            }
            xpub
        })
        .into_owned();
    Ok((out, key_map))
}

#[cfg(test)]
mod sub_tests {
    use super::*;

    #[test]
    fn synthetic_for_0_and_1_differ() {
        assert_ne!(
            synthetic_xpub_for(0, ScriptCtx::MultiSig),
            synthetic_xpub_for(1, ScriptCtx::MultiSig)
        );
    }

    #[test]
    fn synthetic_for_0_is_stable() {
        assert_eq!(
            synthetic_xpub_for(0, ScriptCtx::MultiSig),
            synthetic_xpub_for(0, ScriptCtx::MultiSig)
        );
    }

    #[test]
    fn singlesig_synthetic_uses_depth_3() {
        // Cross-context xpubs must differ — depth byte 4 is 3 vs 4.
        assert_ne!(
            synthetic_xpub_for(0, ScriptCtx::SingleSig),
            synthetic_xpub_for(0, ScriptCtx::MultiSig)
        );
    }

    #[test]
    fn substitution_strips_at_i_suffix() {
        let (s, _) =
            substitute_synthetic("wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))", ScriptCtx::MultiSig)
                .unwrap();
        assert!(!s.contains('@'));
        assert!(!s.contains('<'));
        assert!(!s.contains('*'));
    }

    #[test]
    fn substitution_emits_consistent_keys_per_index() {
        let (s, km) = substitute_synthetic(
            "wsh(or_d(pk(@0/<0;1>/*),pk(@0/<0;1>/*)))",
            ScriptCtx::MultiSig,
        )
        .unwrap();
        assert_eq!(km.len(), 1);
        let key = synthetic_xpub_for(0, ScriptCtx::MultiSig);
        assert_eq!(s.matches(&key).count(), 2);
    }
}

use md_codec::tag::Tag;
use md_codec::tree::{Body, Node};
use miniscript::{Descriptor as MsDescriptor, DescriptorPublicKey};

/// Walk the miniscript Descriptor's outermost wrapper and emit a `Node`.
fn walk_root(
    desc: &MsDescriptor<DescriptorPublicKey>,
    key_map: &std::collections::BTreeMap<String, u8>,
) -> Result<Node, CliError> {
    use miniscript::Descriptor::*;
    match desc {
        Wpkh(w) => Ok(Node {
            tag: Tag::Wpkh,
            body: Body::KeyArg {
                index: lookup_key(&w.as_inner().to_string(), key_map)?,
            },
        }),
        Pkh(p) => Ok(Node {
            tag: Tag::Pkh,
            body: Body::KeyArg {
                index: lookup_key(&p.as_inner().to_string(), key_map)?,
            },
        }),
        Wsh(w) => walk_wsh(w, key_map),
        Sh(s) => walk_sh(s, key_map),
        Tr(t) => walk_tr(t, key_map),
        _ => Err(CliError::TemplateParse(format!(
            "unsupported descriptor wrapper: {desc}"
        ))),
    }
}

fn lookup_key(
    key_str: &str,
    key_map: &std::collections::BTreeMap<String, u8>,
) -> Result<u8, CliError> {
    // miniscript may render the key with derivation suffix; strip suffix for lookup.
    let base = key_str.split('/').next().unwrap_or(key_str);
    key_map.get(base).copied().ok_or_else(|| {
        CliError::TemplateParse(format!(
            "internal: synthetic key {base} not found in key map (rendered: {key_str})"
        ))
    })
}

/// Wrap an inner `Node` under a single-arity wrapper tag like `Wsh`/`Sh`.
fn wrap_children(tag: Tag, inner: Node) -> Node {
    Node {
        tag,
        body: Body::Children(vec![inner]),
    }
}

/// Build `multi`/`sortedmulti` style: `Tag::Multi` (or `SortedMulti`) wrapping
/// each key as a `PkK` child. Use `MultiA`/`SortedMultiA` inside Tap context.
fn build_multi_node(
    tag: Tag,
    k: usize,
    keys: &[&DescriptorPublicKey],
    km: &std::collections::BTreeMap<String, u8>,
) -> Result<Node, CliError> {
    let children: Vec<Node> = keys
        .iter()
        .map(|kk| {
            let index = lookup_key(&kk.to_string(), km)?;
            Ok(Node {
                tag: Tag::PkK,
                body: Body::KeyArg { index },
            })
        })
        .collect::<Result<_, CliError>>()?;
    Ok(Node {
        tag,
        body: Body::Variable {
            k: k as u8,
            children,
        },
    })
}

fn walk_wsh(
    w: &miniscript::descriptor::Wsh<DescriptorPublicKey>,
    km: &std::collections::BTreeMap<String, u8>,
) -> Result<Node, CliError> {
    let inner = walk_wsh_inner(w, km)?;
    Ok(wrap_children(Tag::Wsh, inner))
}

fn walk_sh(
    s: &miniscript::descriptor::Sh<DescriptorPublicKey>,
    km: &std::collections::BTreeMap<String, u8>,
) -> Result<Node, CliError> {
    use miniscript::descriptor::ShInner;
    let inner = match s.as_inner() {
        ShInner::Wsh(w) => Node {
            tag: Tag::Wsh,
            body: Body::Children(vec![walk_wsh_inner(w, km)?]),
        },
        ShInner::Wpkh(wp) => Node {
            tag: Tag::Wpkh,
            body: Body::KeyArg {
                index: lookup_key(&wp.as_inner().to_string(), km)?,
            },
        },
        ShInner::Ms(ms) => walk_miniscript_node(ms, km, /*tap=*/ false)?,
        ShInner::SortedMulti(sm) => build_multi_node(
            Tag::SortedMulti,
            sm.k(),
            &sm.pks().iter().collect::<Vec<_>>(),
            km,
        )?,
    };
    Ok(wrap_children(Tag::Sh, inner))
}

fn walk_wsh_inner(
    w: &miniscript::descriptor::Wsh<DescriptorPublicKey>,
    km: &std::collections::BTreeMap<String, u8>,
) -> Result<Node, CliError> {
    use miniscript::descriptor::WshInner;
    match w.as_inner() {
        WshInner::Ms(ms) => walk_miniscript_node(ms, km, /*tap=*/ false),
        WshInner::SortedMulti(sm) => build_multi_node(
            Tag::SortedMulti,
            sm.k(),
            &sm.pks().iter().collect::<Vec<_>>(),
            km,
        ),
    }
}

fn walk_miniscript_node<C: miniscript::ScriptContext>(
    ms: &miniscript::Miniscript<DescriptorPublicKey, C>,
    km: &std::collections::BTreeMap<String, u8>,
    tap_context: bool,
) -> Result<Node, CliError> {
    use bitcoin::hashes::Hash;
    use miniscript::miniscript::decode::Terminal;
    match &ms.node {
        Terminal::PkK(k) => Ok(Node {
            tag: Tag::PkK,
            body: Body::KeyArg {
                index: lookup_key(&k.to_string(), km)?,
            },
        }),
        Terminal::PkH(k) => Ok(Node {
            tag: Tag::PkH,
            body: Body::KeyArg {
                index: lookup_key(&k.to_string(), km)?,
            },
        }),
        Terminal::Multi(thresh) => build_multi_node(
            Tag::Multi,
            thresh.k(),
            &thresh.data().iter().collect::<Vec<_>>(),
            km,
        ),
        Terminal::MultiA(thresh) => build_multi_node(
            Tag::MultiA,
            thresh.k(),
            &thresh.data().iter().collect::<Vec<_>>(),
            km,
        ),
        // `c:` wrapper. In Tapscript leaves, miniscript desugars `pk(K)` to
        // `c:pk_k(K)`; the BIP 388 wire form for that leaf is bare `PkK`, so
        // collapse the wrapper there. In segwitv0 we keep `Check` explicit.
        Terminal::Check(inner) => {
            if tap_context {
                if let Terminal::PkK(k) = &inner.node {
                    return Ok(Node {
                        tag: Tag::PkK,
                        body: Body::KeyArg {
                            index: lookup_key(&k.to_string(), km)?,
                        },
                    });
                }
                if let Terminal::PkH(k) = &inner.node {
                    return Ok(Node {
                        tag: Tag::PkH,
                        body: Body::KeyArg {
                            index: lookup_key(&k.to_string(), km)?,
                        },
                    });
                }
            }
            Ok(Node {
                tag: Tag::Check,
                body: Body::Children(vec![walk_miniscript_node(inner, km, tap_context)?]),
            })
        }
        // `v:` wrapper. Used inside and_v(v:pk(K), ...) shapes that
        // miniscript's policy compiler emits for and-conjunctions and
        // for any "must-also-sign" sub-expression.
        Terminal::Verify(inner) => Ok(Node {
            tag: Tag::Verify,
            body: Body::Children(vec![walk_miniscript_node(inner, km, tap_context)?]),
        }),
        // `and_v` — and-conjunction with verify-promotion semantics.
        Terminal::AndV(l, r) => Ok(Node {
            tag: Tag::AndV,
            body: Body::Children(vec![
                walk_miniscript_node(l, km, tap_context)?,
                walk_miniscript_node(r, km, tap_context)?,
            ]),
        }),
        // `older` — relative timelock. miniscript carries `Sequence`; we
        // unwrap to consensus u32 for the BIP 388 wire body.
        Terminal::Older(seq) => Ok(Node {
            tag: Tag::Older,
            body: Body::Timelock(seq.to_consensus_u32()),
        }),
        // `after` — absolute timelock (BIP-65 `OP_CHECKLOCKTIMEVERIFY`).
        // miniscript carries `AbsLockTime`; convert to consensus u32 (preserves
        // BIP-65's height-vs-timestamp boundary at 500_000_000).
        Terminal::After(abs) => Ok(Node {
            tag: Tag::After,
            body: Body::Timelock(abs.to_consensus_u32()),
        }),
        // `and_b` — and-conjunction via boolean stack (no verify-promotion).
        Terminal::AndB(l, r) => Ok(Node {
            tag: Tag::AndB,
            body: Body::Children(vec![
                walk_miniscript_node(l, km, tap_context)?,
                walk_miniscript_node(r, km, tap_context)?,
            ]),
        }),
        // `andor(a, b, c)` — ternary "if a then b else c" pattern. The only
        // ternary fragment in miniscript; emits md-codec's Tag::AndOr with
        // Body::Children of length 3.
        Terminal::AndOr(a, b, c) => Ok(Node {
            tag: Tag::AndOr,
            body: Body::Children(vec![
                walk_miniscript_node(a, km, tap_context)?,
                walk_miniscript_node(b, km, tap_context)?,
                walk_miniscript_node(c, km, tap_context)?,
            ]),
        }),
        // `or_b` — or-disjunction via BOOLOR. Both branches must produce a
        // canonical boolean stack value.
        Terminal::OrB(l, r) => Ok(Node {
            tag: Tag::OrB,
            body: Body::Children(vec![
                walk_miniscript_node(l, km, tap_context)?,
                walk_miniscript_node(r, km, tap_context)?,
            ]),
        }),
        // `or_c` — or-disjunction via NOTIF (right branch is verify-promoted).
        Terminal::OrC(l, r) => Ok(Node {
            tag: Tag::OrC,
            body: Body::Children(vec![
                walk_miniscript_node(l, km, tap_context)?,
                walk_miniscript_node(r, km, tap_context)?,
            ]),
        }),
        // `or_d` — or-disjunction via IFDUP. Common in recovery patterns
        // (e.g. or_d(pk(@0), and_v(v:pk(@1), older(N))) for BOLT-3-style
        // hot-cold recovery).
        Terminal::OrD(l, r) => Ok(Node {
            tag: Tag::OrD,
            body: Body::Children(vec![
                walk_miniscript_node(l, km, tap_context)?,
                walk_miniscript_node(r, km, tap_context)?,
            ]),
        }),
        // `or_i` — or-disjunction via IF/ELSE/ENDIF. Either branch may be a
        // full descriptor expression.
        Terminal::OrI(l, r) => Ok(Node {
            tag: Tag::OrI,
            body: Body::Children(vec![
                walk_miniscript_node(l, km, tap_context)?,
                walk_miniscript_node(r, km, tap_context)?,
            ]),
        }),
        // Hash preimage locks. miniscript carries arrays of bytes via newtypes
        // (Sha256/Hash256/Ripemd160/Hash160 wrappers); we project to raw byte
        // arrays for the BIP 388 wire body. SHA256 + HASH256 are 32-byte;
        // RIPEMD160 + HASH160 are 20-byte.
        Terminal::Sha256(h) => Ok(Node {
            tag: Tag::Sha256,
            body: Body::Hash256Body(h.to_byte_array()),
        }),
        Terminal::Hash256(h) => Ok(Node {
            tag: Tag::Hash256,
            body: Body::Hash256Body(h.to_byte_array()),
        }),
        Terminal::Ripemd160(h) => Ok(Node {
            tag: Tag::Ripemd160,
            body: Body::Hash160Body(h.to_byte_array()),
        }),
        Terminal::Hash160(h) => Ok(Node {
            tag: Tag::Hash160,
            body: Body::Hash160Body(h.to_byte_array()),
        }),
        // Single-arity stack-permutation / control-flow wrappers. Each takes
        // Body::Children([1]). These unblock the more interesting fragments
        // — Thresh / OrB / AndB position-2+ children all get s:-wrapped by
        // miniscript's typecheck.
        Terminal::Swap(inner) => Ok(Node {
            tag: Tag::Swap,
            body: Body::Children(vec![walk_miniscript_node(inner, km, tap_context)?]),
        }),
        Terminal::Alt(inner) => Ok(Node {
            tag: Tag::Alt,
            body: Body::Children(vec![walk_miniscript_node(inner, km, tap_context)?]),
        }),
        Terminal::DupIf(inner) => Ok(Node {
            tag: Tag::DupIf,
            body: Body::Children(vec![walk_miniscript_node(inner, km, tap_context)?]),
        }),
        Terminal::NonZero(inner) => Ok(Node {
            tag: Tag::NonZero,
            body: Body::Children(vec![walk_miniscript_node(inner, km, tap_context)?]),
        }),
        Terminal::ZeroNotEqual(inner) => Ok(Node {
            tag: Tag::ZeroNotEqual,
            body: Body::Children(vec![walk_miniscript_node(inner, km, tap_context)?]),
        }),
        // Boolean literals. Reachable inside compound fragments —
        // miniscript's `t:X` is sugar for `and_v(X, 1)`, so True appears
        // wherever a script consumer wants to promote V → T. False appears
        // less commonly but is structurally valid (e.g., as a never-spendable
        // branch of `or_i`). Both have empty bodies on the wire.
        Terminal::True => Ok(Node {
            tag: Tag::True,
            body: Body::Empty,
        }),
        Terminal::False => Ok(Node {
            tag: Tag::False,
            body: Body::Empty,
        }),
        // `thresh(k, c1, c2, ..., cn)` — k-of-n threshold over arbitrary
        // miniscript fragments (not just keys; distinct from Multi/MultiA).
        // Each child is recursively walked.
        Terminal::Thresh(thresh) => {
            // Bounds-check k BEFORE narrowing to u8: md-codec encodes k-1 in 5
            // bits (range 1..=32). Without this guard, a hypothetical k > 32
            // would silently truncate before reaching the codec's
            // ThresholdOutOfRange check, producing a corrupt encoding instead
            // of a clean error.
            let k = thresh.k();
            if !(1..=32).contains(&k) {
                return Err(CliError::TemplateParse(format!(
                    "thresh k={k} out of md1 wire-format range 1..=32"
                )));
            }
            let children: Vec<Node> = thresh
                .data()
                .iter()
                .map(|c| walk_miniscript_node(c, km, tap_context))
                .collect::<Result<_, CliError>>()?;
            Ok(Node {
                tag: Tag::Thresh,
                body: Body::Variable {
                    k: k as u8,
                    children,
                },
            })
        }
        // Other miniscript fragments — TemplateParse error until BIP 388 templates need them.
        _ => {
            let _ = tap_context;
            Err(CliError::TemplateParse(format!(
                "unsupported miniscript fragment: {ms}"
            )))
        }
    }
}

fn walk_tr(
    t: &miniscript::descriptor::Tr<DescriptorPublicKey>,
    km: &std::collections::BTreeMap<String, u8>,
) -> Result<Node, CliError> {
    let key_str = t.internal_key().to_string();
    let tree: Option<Box<Node>> = match t.tap_tree() {
        None => None,
        Some(tt) => Some(Box::new(walk_tap_tree(tt, km)?)),
    };
    // v0.18 NUMS sentinel: emit Tag::Tr with key_index = n (just past the
    // highest placeholder) iff the internal key is exactly the BIP-341 NUMS
    // H-point. Otherwise the internal key MUST be a placeholder-derived
    // synthetic xpub (i.e. an @N).
    if key_str == NUMS_H_POINT_X_ONLY_HEX {
        let n = km.len() as u8;
        return Ok(Node {
            tag: Tag::Tr,
            body: Body::Tr { key_index: n, tree },
        });
    }
    let key_index = lookup_key(&key_str, km).map_err(|orig_err| {
        // If the internal key isn't in the placeholder map AND looks like a
        // literal x-only hex, surface a clearer error than the generic
        // "synthetic key not found" message — md1 does not encode arbitrary
        // literal x-only keys in the tr() internal-key position.
        if is_x_only_hex(&key_str) {
            CliError::TemplateParse(format!(
                "unsupported internal-key form: literal hex `{key_str}` other than \
                 BIP-341 NUMS H-point. Use an @N placeholder (backed by an xpub via \
                 --keys) for the internal key, or the BIP-341 NUMS H-point \
                 ({NUMS_H_POINT_X_ONLY_HEX}) for the v0.18 NUMS sentinel encoding."
            ))
        } else {
            orig_err
        }
    })?;
    Ok(Node {
        tag: Tag::Tr,
        body: Body::Tr { key_index, tree },
    })
}

/// Returns `true` if `s` is exactly 64 ASCII hex digits — the shape of a
/// BIP-340 x-only public key. Used to distinguish "literal hex internal
/// key that md1 doesn't know how to encode" from generic key-map-miss
/// errors.
fn is_x_only_hex(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Walk miniscript 13's `TapTree` and produce an md1 wire-tree node.
///
/// miniscript represents a `TapTree` as `Vec<(depth, leaf-Miniscript)>` in
/// depth-first preorder. md1's wire format is a binary tree of `Tag::TapTree`
/// internal nodes (each carrying `Body::Children([left, right])`) with the
/// actual miniscript fragments at the leaves.
///
/// **Algorithm — stack-based depth-marked preorder reconstruction:** for
/// each leaf in iteration order, push `(depth, leaf-Node)` onto a stack;
/// while the top two entries share the same depth, pop them and wrap as a
/// new `Tag::TapTree` branch at depth-1, then push the parent back. After
/// all leaves are consumed, the stack holds exactly `[(0, root)]`.
///
/// **Single-leaf compatibility:** a single-leaf input `[(0, leaf)]` triggers
/// no merge, ends with `[(0, leaf)]`, and returns `leaf` directly with NO
/// `Tag::TapTree` wrap. This preserves the v0.18 single-leaf wire encoding
/// bit-identically — `tr(@0, pk(@1))` continues to round-trip.
///
/// **Defensive end-state checks:** miniscript's type system prevents
/// malformed `TapTree` values (`combine` is the only non-`leaf` constructor
/// and caps at depth 128 with a typed error), but if a future regression
/// breaks the invariant, surface a typed `CliError::TemplateParse` rather
/// than panic.
fn walk_tap_tree(
    tt: &miniscript::descriptor::TapTree<DescriptorPublicKey>,
    km: &std::collections::BTreeMap<String, u8>,
) -> Result<Node, CliError> {
    let mut stack: Vec<(u8, Node)> = Vec::new();
    for leaf in tt.leaves() {
        let leaf_node = walk_miniscript_node(leaf.miniscript(), km, /*tap=*/ true)?;
        stack.push((leaf.depth(), leaf_node));
        // Coalesce: while top two stack entries share the same depth, wrap
        // them as a Tag::TapTree branch at depth-1. miniscript's TapTree
        // always produces a complete binary tree in depth-first preorder,
        // so the merge sequence is deterministic.
        while stack.len() >= 2 && stack[stack.len() - 1].0 == stack[stack.len() - 2].0 {
            let (d, right) = stack.pop().unwrap();
            let (_, left) = stack.pop().unwrap();
            let parent_depth = d.checked_sub(1).ok_or_else(|| {
                CliError::TemplateParse(
                    "tap tree shape invariant violated: \
                     two adjacent leaves at depth 0"
                        .into(),
                )
            })?;
            stack.push((
                parent_depth,
                Node {
                    tag: Tag::TapTree,
                    body: Body::Children(vec![left, right]),
                },
            ));
        }
    }
    if stack.is_empty() {
        return Err(CliError::TemplateParse(
            "tap tree present but contains no leaves".into(),
        ));
    }
    if stack.len() != 1 || stack[0].0 != 0 {
        return Err(CliError::TemplateParse(format!(
            "tap tree shape invariant violated: stack ended with {} entries; \
             root entry depth was {}",
            stack.len(),
            stack[0].0,
        )));
    }
    Ok(stack.pop().unwrap().1)
}

#[cfg(test)]
mod root_tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn wpkh_root() {
        let (s, km) = substitute_synthetic("wpkh(@0/<0;1>/*)", ScriptCtx::SingleSig).unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        assert_eq!(root.tag, Tag::Wpkh);
        assert!(matches!(root.body, Body::KeyArg { index: 0 }));
    }

    #[test]
    fn pkh_root() {
        let (s, km) = substitute_synthetic("pkh(@0/<0;1>/*)", ScriptCtx::SingleSig).unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        assert_eq!(root.tag, Tag::Pkh);
    }
}

#[cfg(test)]
mod wsh_tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn wsh_multi_2of2() {
        let (s, km) =
            substitute_synthetic("wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))", ScriptCtx::MultiSig)
                .unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        assert_eq!(root.tag, Tag::Wsh);
        let inner = match root.body {
            Body::Children(ref v) if v.len() == 1 => &v[0],
            _ => panic!("expected Wsh.Children([inner])"),
        };
        assert_eq!(inner.tag, Tag::Multi);
        match &inner.body {
            Body::Variable { k, children } => {
                assert_eq!(*k, 2);
                assert_eq!(children.len(), 2);
                assert!(matches!(children[0].body, Body::KeyArg { index: 0 }));
                assert!(matches!(children[1].body, Body::KeyArg { index: 1 }));
            }
            _ => panic!("expected Body::Variable"),
        }
    }

    #[test]
    fn wsh_sortedmulti_2of3() {
        let (s, km) = substitute_synthetic(
            "wsh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))",
            ScriptCtx::MultiSig,
        )
        .unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        assert_eq!(root.tag, Tag::Wsh);
        let inner = match root.body {
            Body::Children(ref v) if v.len() == 1 => &v[0],
            _ => panic!("expected Wsh.Children([inner])"),
        };
        assert_eq!(inner.tag, Tag::SortedMulti);
        match &inner.body {
            Body::Variable { k, children } => {
                assert_eq!(*k, 2);
                assert_eq!(children.len(), 3);
            }
            _ => panic!("expected Body::Variable"),
        }
    }

    #[test]
    fn sh_wpkh_nested() {
        let (s, km) = substitute_synthetic("sh(wpkh(@0/<0;1>/*))", ScriptCtx::SingleSig).unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        assert_eq!(root.tag, Tag::Sh);
        let inner = match root.body {
            Body::Children(ref v) if v.len() == 1 => &v[0],
            _ => panic!("expected Sh.Children([inner])"),
        };
        assert_eq!(inner.tag, Tag::Wpkh);
        assert!(matches!(inner.body, Body::KeyArg { index: 0 }));
    }

    #[test]
    fn sh_wsh_multi_nested() {
        let (s, km) = substitute_synthetic(
            "sh(wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*)))",
            ScriptCtx::MultiSig,
        )
        .unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        assert_eq!(root.tag, Tag::Sh);
        let inner = match root.body {
            Body::Children(ref v) if v.len() == 1 => &v[0],
            _ => panic!("expected Sh.Children([inner])"),
        };
        assert_eq!(inner.tag, Tag::Wsh);
    }
}

#[cfg(test)]
mod tr_tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn tr_key_only() {
        let (s, km) = substitute_synthetic("tr(@0/<0;1>/*)", ScriptCtx::MultiSig).unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        assert_eq!(root.tag, Tag::Tr);
        match root.body {
            Body::Tr { key_index, tree } => {
                assert_eq!(key_index, 0);
                assert!(tree.is_none());
            }
            _ => panic!("expected Body::Tr"),
        }
    }

    #[test]
    fn tr_with_one_leaf() {
        let (s, km) =
            substitute_synthetic("tr(@0/<0;1>/*,pk(@1/<0;1>/*))", ScriptCtx::MultiSig).unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        assert_eq!(root.tag, Tag::Tr);
        match root.body {
            Body::Tr { key_index, tree } => {
                assert_eq!(key_index, 0);
                let leaf = tree.unwrap();
                assert_eq!(leaf.tag, Tag::PkK);
                assert!(matches!(leaf.body, Body::KeyArg { index: 1 }));
            }
            _ => panic!("expected Body::Tr"),
        }
    }

    /// Inheritance pattern: `tr(@0, and_v(v:pk(@1), older(144)))`.
    /// Exercises the v0.17 walker arms for Terminal::AndV, Terminal::Verify,
    /// and Terminal::Older. Spike-verified that `Policy::compile_tr(None)`
    /// emits this shape from `or(pk(@0), and(pk(@1), older(144)))`.
    #[test]
    fn tr_with_and_v_verify_older_inheritance() {
        let (s, km) = substitute_synthetic(
            "tr(@0/<0;1>/*,and_v(v:pk(@1/<0;1>/*),older(144)))",
            ScriptCtx::MultiSig,
        )
        .unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        assert_eq!(root.tag, Tag::Tr);
        let leaf = match root.body {
            Body::Tr { key_index, tree } => {
                assert_eq!(key_index, 0);
                tree.expect("tap tree must be present")
            }
            _ => panic!("expected Body::Tr"),
        };
        // Leaf is and_v(<verify-child>, <older-child>)
        assert_eq!(leaf.tag, Tag::AndV);
        let kids = match &leaf.body {
            Body::Children(v) if v.len() == 2 => v,
            _ => panic!("expected and_v.Children([verify, older])"),
        };
        // First child: v:pk(@1) → Tag::Verify wrapping bare Tag::PkK
        assert_eq!(kids[0].tag, Tag::Verify);
        let verify_inner = match &kids[0].body {
            Body::Children(v) if v.len() == 1 => &v[0],
            _ => panic!("expected Verify.Children([pkk])"),
        };
        assert_eq!(verify_inner.tag, Tag::PkK);
        assert!(matches!(verify_inner.body, Body::KeyArg { index: 1 }));
        // Second child: older(144) → Tag::Older with Body::Timelock(144)
        assert_eq!(kids[1].tag, Tag::Older);
        assert!(matches!(kids[1].body, Body::Timelock(144)));
    }

    /// v0.18 — NUMS H-point internal key emits Tag::Tr with the sentinel
    /// `key_index = n` (not the v0.17-era Tag::TrUnspendable). Confirms the
    /// canonicalization invariant in walk_tr post-Phase-3.
    /// Note: substitute_synthetic does NOT touch the literal NUMS hex (the
    /// regex matches `@N`-prefixed tokens only); the hex flows through into
    /// the parsed Descriptor's internal_key field as a literal x-only key.
    #[test]
    fn tr_with_nums_internal_key_emits_sentinel() {
        let template =
            format!("tr({NUMS_H_POINT_X_ONLY_HEX},multi_a(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))");
        let (s, km) = substitute_synthetic(&template, ScriptCtx::MultiSig).unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        assert_eq!(
            root.tag,
            Tag::Tr,
            "NUMS internal key MUST emit Tag::Tr (with sentinel key_index = n)"
        );
        let tree = match root.body {
            Body::Tr { key_index, tree } => {
                assert_eq!(
                    key_index, 3,
                    "n=3 placeholders → sentinel key_index = 3 (NUMS H-point)"
                );
                tree.expect("multi_a leaf must be present")
            }
            _ => panic!("expected Body::Tr"),
        };
        assert_eq!(tree.tag, Tag::MultiA);
        match &tree.body {
            Body::Variable { k, children } => {
                assert_eq!(*k, 2);
                assert_eq!(children.len(), 3);
                for (i, child) in children.iter().enumerate() {
                    assert_eq!(child.tag, Tag::PkK);
                    assert!(matches!(child.body, Body::KeyArg { index: ix } if ix as usize == i));
                }
            }
            _ => panic!("expected Body::Variable"),
        }
    }

    /// v0.18 — `tr(<NUMS>)` key-path-only (no tap tree) is a valid
    /// frozen/unspendable output. Confirms miniscript 13 accepts the no-tree
    /// shape and that walk_tr emits Tag::Tr with sentinel key_index = n and
    /// `tree: None` (the n=0 sentinel boundary case).
    #[test]
    fn tr_with_nums_key_only_no_tree_emits_sentinel_with_none_tree() {
        let template = format!("tr({NUMS_H_POINT_X_ONLY_HEX})");
        let (s, km) = substitute_synthetic(&template, ScriptCtx::MultiSig).unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        assert_eq!(root.tag, Tag::Tr);
        match root.body {
            Body::Tr { key_index, tree } => {
                assert_eq!(
                    key_index, 0,
                    "n=0 placeholders (no @N in template) → sentinel key_index = 0"
                );
                assert!(
                    tree.is_none(),
                    "tree must be None for tr(<NUMS>) with no script arg"
                );
            }
            _ => panic!("expected Body::Tr"),
        }
    }

    /// v0.17 Phase 3 — arbitrary x-only hex internal keys other than NUMS are
    /// rejected at the walker layer with a clear error message. md1 v0.17's
    /// wire format only supports @N placeholders or the BIP-341 NUMS sentinel
    /// in the tr() internal-key position. Non-NUMS literal hex would require
    /// a Tag::TrLiteralKey wire-format extension that v0.17 does not include.
    #[test]
    fn tr_with_non_nums_literal_hex_rejected_with_clear_message() {
        // secp256k1 generator point's x-coordinate — a guaranteed-valid
        // x-only public key that is NOT the BIP-341 NUMS H-point. miniscript
        // accepts it at parse time; walk_tr must reject it at the walker
        // layer with a clear error.
        let non_nums_hex = "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
        let template = format!("tr({non_nums_hex},pk(@0/<0;1>/*))");
        let (s, km) = substitute_synthetic(&template, ScriptCtx::MultiSig).unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let err = walk_root(&d, &km).unwrap_err();
        match err {
            CliError::TemplateParse(msg) => {
                assert!(
                    msg.contains("unsupported internal-key form: literal hex"),
                    "expected v0.17 non-NUMS-hex error, got: {msg}"
                );
                assert!(
                    msg.contains(non_nums_hex),
                    "error must surface the offending hex"
                );
                assert!(
                    msg.contains(NUMS_H_POINT_X_ONLY_HEX),
                    "error must show the NUMS alternative"
                );
            }
            other => panic!("expected TemplateParse, got {other:?}"),
        }
    }

    /// Multi-branch tap trees error with the new v0.17 message (no longer
    /// v0.19 — multi-branch tap tree, 2 leaves balanced.
    /// `tr(@0,{pk(@1),pk(@2)})` walks to `Body::Tr { tree:
    /// Some(TapTree { children: [PkK@1, PkK@2] }) }`. Replaces the v0.18-era
    /// `tr_multi_branch_rejected_with_v0_17_error_message` test using the
    /// SAME input string (unambiguous reject→accept transition in git diff).
    #[test]
    fn tr_multi_branch_two_leaf_balanced() {
        let (s, km) = substitute_synthetic(
            "tr(@0/<0;1>/*,{pk(@1/<0;1>/*),pk(@2/<0;1>/*)})",
            ScriptCtx::MultiSig,
        )
        .unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        assert_eq!(root.tag, Tag::Tr);
        let tree = match root.body {
            Body::Tr { key_index, tree } => {
                assert_eq!(key_index, 0, "internal key is @0");
                tree.expect("tap tree must be present")
            }
            _ => panic!("expected Body::Tr"),
        };
        assert_eq!(tree.tag, Tag::TapTree);
        let kids = match &tree.body {
            Body::Children(v) if v.len() == 2 => v,
            _ => panic!("expected TapTree.Children([2])"),
        };
        assert_eq!(kids[0].tag, Tag::PkK);
        assert!(matches!(kids[0].body, Body::KeyArg { index: 1 }));
        assert_eq!(kids[1].tag, Tag::PkK);
        assert!(matches!(kids[1].body, Body::KeyArg { index: 2 }));
    }

    /// v0.19 — 4-leaf balanced: `tr(@0,{{pk(@1),pk(@2)},{pk(@3),pk(@4)}})`
    /// → `TapTree { TapTree { PkK@1, PkK@2 }, TapTree { PkK@3, PkK@4 } }`.
    #[test]
    fn tr_multi_branch_four_leaf_balanced() {
        let (s, km) = substitute_synthetic(
            "tr(@0/<0;1>/*,{{pk(@1/<0;1>/*),pk(@2/<0;1>/*)},{pk(@3/<0;1>/*),pk(@4/<0;1>/*)}})",
            ScriptCtx::MultiSig,
        )
        .unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        let tree = match root.body {
            Body::Tr { tree, .. } => tree.expect("tap tree must be present"),
            _ => panic!("expected Body::Tr"),
        };
        assert_eq!(tree.tag, Tag::TapTree);
        let top_kids = match &tree.body {
            Body::Children(v) if v.len() == 2 => v,
            _ => panic!("expected TapTree.Children([2])"),
        };
        // Both top-level children are themselves TapTree branches.
        for (i, branch) in top_kids.iter().enumerate() {
            assert_eq!(branch.tag, Tag::TapTree, "top child {i} must be TapTree");
            let leaves = match &branch.body {
                Body::Children(v) if v.len() == 2 => v,
                _ => panic!("expected nested TapTree.Children([2])"),
            };
            assert_eq!(leaves[0].tag, Tag::PkK);
            assert_eq!(leaves[1].tag, Tag::PkK);
        }
        // Verify @N indices in document order: @1, @2, @3, @4.
        let expect_indices = [(0, 0, 1), (0, 1, 2), (1, 0, 3), (1, 1, 4)];
        for (top_i, leaf_i, expected_idx) in expect_indices {
            let branch = match &top_kids[top_i].body {
                Body::Children(v) => v,
                _ => unreachable!(),
            };
            assert!(
                matches!(branch[leaf_i].body, Body::KeyArg { index: i } if i == expected_idx),
                "top[{top_i}].leaf[{leaf_i}] must be @{expected_idx}",
            );
        }
    }

    /// v0.19 — 3-leaf left-unbalanced: depths `[1,2,2]`.
    /// `tr(@0,{pk(@1),{pk(@2),pk(@3)}})` →
    /// `TapTree { PkK@1, TapTree { PkK@2, PkK@3 } }`.
    #[test]
    fn tr_multi_branch_three_leaf_left_unbalanced() {
        let (s, km) = substitute_synthetic(
            "tr(@0/<0;1>/*,{pk(@1/<0;1>/*),{pk(@2/<0;1>/*),pk(@3/<0;1>/*)}})",
            ScriptCtx::MultiSig,
        )
        .unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        let tree = match root.body {
            Body::Tr { tree, .. } => tree.expect("tap tree must be present"),
            _ => panic!("expected Body::Tr"),
        };
        assert_eq!(tree.tag, Tag::TapTree);
        let top_kids = match &tree.body {
            Body::Children(v) if v.len() == 2 => v,
            _ => panic!("expected TapTree.Children([2])"),
        };
        // Left: leaf PkK@1.
        assert_eq!(top_kids[0].tag, Tag::PkK);
        assert!(matches!(top_kids[0].body, Body::KeyArg { index: 1 }));
        // Right: TapTree { PkK@2, PkK@3 }.
        assert_eq!(top_kids[1].tag, Tag::TapTree);
        let right_kids = match &top_kids[1].body {
            Body::Children(v) if v.len() == 2 => v,
            _ => panic!("expected nested TapTree.Children([2])"),
        };
        assert!(matches!(right_kids[0].body, Body::KeyArg { index: 2 }));
        assert!(matches!(right_kids[1].body, Body::KeyArg { index: 3 }));
    }

    /// v0.19 — 3-leaf right-unbalanced: depths `[2,2,1]`.
    /// `tr(@0,{{pk(@1),pk(@2)},pk(@3)})` →
    /// `TapTree { TapTree { PkK@1, PkK@2 }, PkK@3 }`.
    #[test]
    fn tr_multi_branch_three_leaf_right_unbalanced() {
        let (s, km) = substitute_synthetic(
            "tr(@0/<0;1>/*,{{pk(@1/<0;1>/*),pk(@2/<0;1>/*)},pk(@3/<0;1>/*)})",
            ScriptCtx::MultiSig,
        )
        .unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        let tree = match root.body {
            Body::Tr { tree, .. } => tree.expect("tap tree must be present"),
            _ => panic!("expected Body::Tr"),
        };
        let top_kids = match &tree.body {
            Body::Children(v) if v.len() == 2 => v,
            _ => panic!("expected TapTree.Children([2])"),
        };
        // Left: TapTree { PkK@1, PkK@2 }.
        assert_eq!(top_kids[0].tag, Tag::TapTree);
        let left_kids = match &top_kids[0].body {
            Body::Children(v) if v.len() == 2 => v,
            _ => panic!("expected nested TapTree.Children([2])"),
        };
        assert!(matches!(left_kids[0].body, Body::KeyArg { index: 1 }));
        assert!(matches!(left_kids[1].body, Body::KeyArg { index: 2 }));
        // Right: leaf PkK@3.
        assert_eq!(top_kids[1].tag, Tag::PkK);
        assert!(matches!(top_kids[1].body, Body::KeyArg { index: 3 }));
    }

    /// v0.19 — multi-branch with non-trivial leaf. Verifies the recursive
    /// `walk_miniscript_node` call inside the stack-reconstruction loop.
    /// `tr(@0,{pk(@1),and_v(v:pk(@2),older(144))})`.
    #[test]
    fn tr_multi_branch_with_complex_leaves() {
        let (s, km) = substitute_synthetic(
            "tr(@0/<0;1>/*,{pk(@1/<0;1>/*),and_v(v:pk(@2/<0;1>/*),older(144))})",
            ScriptCtx::MultiSig,
        )
        .unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        let tree = match root.body {
            Body::Tr { tree, .. } => tree.expect("tap tree must be present"),
            _ => panic!("expected Body::Tr"),
        };
        let kids = match &tree.body {
            Body::Children(v) if v.len() == 2 => v,
            _ => panic!("expected TapTree.Children([2])"),
        };
        assert_eq!(kids[0].tag, Tag::PkK);
        assert!(matches!(kids[0].body, Body::KeyArg { index: 1 }));
        // Right leaf is and_v(Verify(PkK@2), Older(144)).
        assert_eq!(kids[1].tag, Tag::AndV);
        let andv_kids = match &kids[1].body {
            Body::Children(v) if v.len() == 2 => v,
            _ => panic!("expected and_v.Children([2])"),
        };
        assert_eq!(andv_kids[0].tag, Tag::Verify);
        assert_eq!(andv_kids[1].tag, Tag::Older);
        assert!(matches!(andv_kids[1].body, Body::Timelock(144)));
    }

    /// v0.19 — NUMS sentinel + 2-leaf multi-branch. Verifies orthogonality
    /// of the NUMS sentinel `key_index = n` rule and the tree shape:
    /// `tr(<NUMS_HEX>,{pk(@0),pk(@1)})` with n=2 placeholders → sentinel
    /// is `key_index = 2`, plus a 2-leaf TapTree.
    #[test]
    fn tr_nums_sentinel_with_two_leaf_multi_branch() {
        let template = format!("tr({NUMS_H_POINT_X_ONLY_HEX},{{pk(@0/<0;1>/*),pk(@1/<0;1>/*)}})",);
        let (s, km) = substitute_synthetic(&template, ScriptCtx::MultiSig).unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        let tree = match root.body {
            Body::Tr { key_index, tree } => {
                assert_eq!(key_index, 2, "n=2 placeholders → sentinel key_index = 2");
                tree.expect("tap tree must be present")
            }
            _ => panic!("expected Body::Tr"),
        };
        assert_eq!(tree.tag, Tag::TapTree);
        let kids = match &tree.body {
            Body::Children(v) if v.len() == 2 => v,
            _ => panic!("expected TapTree.Children([2])"),
        };
        assert!(matches!(kids[0].body, Body::KeyArg { index: 0 }));
        assert!(matches!(kids[1].body, Body::KeyArg { index: 1 }));
    }

    /// v0.19 — NUMS sentinel + 3-leaf multi-branch. Verifies that
    /// `km.len() as u8` arithmetic at walk_tr correctly varies with
    /// placeholder count: n=3 here, so the sentinel must be `key_index = 3`.
    /// Without this test, only the n=2 sentinel case is exercised.
    #[test]
    fn tr_nums_sentinel_with_three_leaf_multi_branch() {
        let template = format!(
            "tr({NUMS_H_POINT_X_ONLY_HEX},{{pk(@0/<0;1>/*),{{pk(@1/<0;1>/*),pk(@2/<0;1>/*)}}}})",
        );
        let (s, km) = substitute_synthetic(&template, ScriptCtx::MultiSig).unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        let tree = match root.body {
            Body::Tr { key_index, tree } => {
                assert_eq!(key_index, 3, "n=3 placeholders → sentinel key_index = 3");
                tree.expect("tap tree must be present")
            }
            _ => panic!("expected Body::Tr"),
        };
        assert_eq!(tree.tag, Tag::TapTree);
    }

    /// v0.18 Phase 4a — andor ternary structural assertion. The only ternary
    /// fragment in miniscript; verify Body::Children has exactly 3 elements.
    /// Round-trip in format/text.rs covers the rendering side; this test
    /// pins the wire-level Body shape.
    #[test]
    fn tr_with_and_or_ternary_emits_three_children() {
        let (s, km) = substitute_synthetic(
            "tr(@0/<0;1>/*,andor(pk(@1/<0;1>/*),pk(@2/<0;1>/*),pk(@3/<0;1>/*)))",
            ScriptCtx::MultiSig,
        )
        .unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        let leaf = match root.body {
            Body::Tr { tree, .. } => tree.expect("tap tree must be present"),
            _ => panic!("expected Body::Tr"),
        };
        assert_eq!(leaf.tag, Tag::AndOr);
        match &leaf.body {
            Body::Children(v) => {
                assert_eq!(v.len(), 3, "andor must have exactly 3 children");
                for (i, child) in v.iter().enumerate() {
                    assert_eq!(child.tag, Tag::PkK);
                    assert!(
                        matches!(child.body, Body::KeyArg { index: ix } if ix as usize == i + 1)
                    );
                }
            }
            _ => panic!("expected Body::Children"),
        }
    }

    /// v0.18 Phase 4a — `after()` (BIP-65 absolute timelock) walker
    /// emits Body::Timelock with the consensus u32 value. Distinct
    /// from `older()` (BIP-112 relative timelock) which already shipped
    /// in v0.17 Phase 2.
    #[test]
    fn tr_with_after_absolute_timelock_emits_timelock_body() {
        let (s, km) = substitute_synthetic(
            "tr(@0/<0;1>/*,and_v(v:pk(@1/<0;1>/*),after(700000)))",
            ScriptCtx::MultiSig,
        )
        .unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        let leaf = match root.body {
            Body::Tr { tree, .. } => tree.expect("tap tree must be present"),
            _ => panic!("expected Body::Tr"),
        };
        assert_eq!(leaf.tag, Tag::AndV);
        let kids = match &leaf.body {
            Body::Children(v) if v.len() == 2 => v,
            _ => panic!("expected and_v.Children([2])"),
        };
        assert_eq!(kids[1].tag, Tag::After);
        assert!(
            matches!(kids[1].body, Body::Timelock(700000)),
            "after(700000) must emit Body::Timelock(700000); got {:?}",
            kids[1].body
        );
    }

    /// v0.18 Phase 4a — `thresh(k, ...)` walker emits Body::Variable (distinct
    /// from Body::Children that binary fragments use). 1-of-1 form chosen
    /// because miniscript's typecheck demands W-typed children for thresh
    /// position 2+, which require Swap (Phase 4b). 1-of-1 is the structurally
    /// simplest case that does not need wrappers; multi-child Thresh tests
    /// land in Phase 4b alongside the wrapper coverage.
    #[test]
    fn tr_with_thresh_1_of_1_emits_variable_body() {
        let (s, km) = substitute_synthetic(
            "tr(@0/<0;1>/*,thresh(1,pk(@1/<0;1>/*)))",
            ScriptCtx::MultiSig,
        )
        .unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        let leaf = match root.body {
            Body::Tr { tree, .. } => tree.expect("tap tree must be present"),
            _ => panic!("expected Body::Tr"),
        };
        assert_eq!(leaf.tag, Tag::Thresh);
        match &leaf.body {
            Body::Variable { k, children } => {
                assert_eq!(*k, 1);
                assert_eq!(children.len(), 1);
                assert_eq!(children[0].tag, Tag::PkK);
                assert!(matches!(children[0].body, Body::KeyArg { index: 1 }));
            }
            _ => panic!("expected Body::Variable, got {:?}", leaf.body),
        }
    }

    /// v0.18 Phase 4b — sha256 walker emits Body::Hash256Body(32 bytes).
    /// Pinned hash is the canonical "trivial" 32-byte value.
    #[test]
    fn tr_with_sha256_emits_hash256_body() {
        let (s, km) = substitute_synthetic(
            "tr(@0/<0;1>/*,and_v(v:pk(@1/<0;1>/*),sha256(0000000000000000000000000000000000000000000000000000000000000001)))",
            ScriptCtx::MultiSig,
        )
        .unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        let leaf = match root.body {
            Body::Tr { tree, .. } => tree.expect("tap tree must be present"),
            _ => panic!("expected Body::Tr"),
        };
        assert_eq!(leaf.tag, Tag::AndV);
        let kids = match &leaf.body {
            Body::Children(v) if v.len() == 2 => v,
            _ => panic!("expected and_v.Children([2])"),
        };
        assert_eq!(kids[1].tag, Tag::Sha256);
        match &kids[1].body {
            Body::Hash256Body(h) => {
                let mut expected = [0u8; 32];
                expected[31] = 1;
                assert_eq!(*h, expected);
            }
            _ => panic!("expected Body::Hash256Body"),
        }
    }

    /// v0.18 Phase 4b — `s:` (Swap) wrapper walker emits Body::Children([1]).
    /// Tests one of the 5 prefix wrappers; the others (a:, d:, j:, n:)
    /// follow the same code path.
    #[test]
    fn wsh_thresh_with_swap_wrapper_emits_children_one() {
        let (s, km) = substitute_synthetic(
            "wsh(and_b(pk(@0/<0;1>/*),s:pk(@1/<0;1>/*)))",
            ScriptCtx::MultiSig,
        )
        .unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        // Wsh wraps the and_b which has a c:pk_k (B-typed) and s:c:pk_k (W-typed)
        let inner = match root.body {
            Body::Children(ref v) if v.len() == 1 => &v[0],
            _ => panic!("expected Wsh.Children([inner])"),
        };
        assert_eq!(inner.tag, Tag::AndB);
        let kids = match &inner.body {
            Body::Children(v) if v.len() == 2 => v,
            _ => panic!("expected and_b.Children([2])"),
        };
        // Second child has the Swap wrapper.
        assert_eq!(kids[1].tag, Tag::Swap);
        match &kids[1].body {
            Body::Children(v) if v.len() == 1 => {
                // Inside the Swap is c:pk_k(@1), which collapses to Check(PkK).
                assert_eq!(v[0].tag, Tag::Check);
            }
            _ => panic!("expected Swap.Children([1])"),
        }
    }

    /// v0.18 Phase 4b — Terminal::True (reachable via miniscript's `t:` sugar
    /// = and_v(X, 1)) emits Tag::True with Body::Empty. Confirms that True
    /// is NOT rejected by the walker; the originally-planned negative test
    /// "walker_rejects_true_in_tap_top_level_context" was wrong.
    #[test]
    fn tr_t_or_c_walker_emits_true_in_and_v_subtree() {
        let (s, km) = substitute_synthetic(
            "tr(@0/<0;1>/*,t:or_c(pk(@1/<0;1>/*),v:pk(@2/<0;1>/*)))",
            ScriptCtx::MultiSig,
        )
        .unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        let leaf = match root.body {
            Body::Tr { tree, .. } => tree.expect("tap tree must be present"),
            _ => panic!("expected Body::Tr"),
        };
        // t:or_c(...) desugars to and_v(or_c(...), 1)
        assert_eq!(leaf.tag, Tag::AndV);
        let kids = match &leaf.body {
            Body::Children(v) if v.len() == 2 => v,
            _ => panic!("expected and_v.Children([2])"),
        };
        assert_eq!(kids[0].tag, Tag::OrC);
        assert_eq!(kids[1].tag, Tag::True);
        assert!(matches!(kids[1].body, Body::Empty));
    }

    /// v0.18 Phase 4a — or_d recovery pattern walker structural assertion.
    /// Body shape: or_d(pk(@N), and_v(v:pk(@M), older(K))).
    #[test]
    fn tr_with_or_d_recovery_pattern_walker_shape() {
        let (s, km) = substitute_synthetic(
            "tr(@0/<0;1>/*,or_d(pk(@1/<0;1>/*),and_v(v:pk(@2/<0;1>/*),older(144))))",
            ScriptCtx::MultiSig,
        )
        .unwrap();
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(&s).unwrap();
        let root = walk_root(&d, &km).unwrap();
        let leaf = match root.body {
            Body::Tr { tree, .. } => tree.expect("tap tree must be present"),
            _ => panic!("expected Body::Tr"),
        };
        assert_eq!(leaf.tag, Tag::OrD);
        let kids = match &leaf.body {
            Body::Children(v) if v.len() == 2 => v,
            _ => panic!("expected or_d.Children([2])"),
        };
        assert_eq!(kids[0].tag, Tag::PkK);
        assert_eq!(kids[1].tag, Tag::AndV);
    }
}

use crate::parse::keys::{ParsedFingerprint, ParsedKey};
use md_codec::encode::Descriptor;
use md_codec::tlv::TlvSection;

pub fn parse_template(
    template: &str,
    keys: &[ParsedKey],
    fingerprints: &[ParsedFingerprint],
) -> Result<Descriptor, CliError> {
    let ctx = ctx_for_template(template);
    let occs = lex_placeholders(template)?;
    let resolved = resolve_placeholders(&occs)?;

    let (substituted, key_map) = substitute_synthetic(template, ctx)?;
    let ms_desc = MsDescriptor::<DescriptorPublicKey>::from_str(&substituted)
        .map_err(|e| CliError::TemplateParse(format!("miniscript parse failed: {e}")))?;
    let tree = walk_root(&ms_desc, &key_map)?;

    // TLV encoder (md_codec::tlv) requires strict ascending @i; sort before populating.
    let pubkeys = if keys.is_empty() {
        None
    } else {
        let mut v: Vec<_> = keys.iter().map(|k| (k.i, k.payload)).collect();
        v.sort_by_key(|(i, _)| *i);
        Some(v)
    };
    let fp_vec = if fingerprints.is_empty() {
        None
    } else {
        let mut v: Vec<_> = fingerprints.iter().map(|f| (f.i, f.fp)).collect();
        v.sort_by_key(|(i, _)| *i);
        Some(v)
    };
    let use_site_path_overrides = if resolved.use_site_path_overrides.is_empty() {
        None
    } else {
        Some(resolved.use_site_path_overrides)
    };

    // TlvSection has no Default derive; populate via new_empty + field assignment.
    let mut tlv = TlvSection::new_empty();
    tlv.use_site_path_overrides = use_site_path_overrides;
    tlv.fingerprints = fp_vec;
    tlv.pubkeys = pubkeys;

    Ok(Descriptor {
        n: resolved.n,
        path_decl: resolved.path_decl,
        use_site_path: resolved.use_site_path,
        tree,
        tlv,
    })
}

/// Convenience: derive script-context expectation from the template's outer wrapper.
pub fn ctx_for_template(template: &str) -> ScriptCtx {
    let head = template.trim_start();
    if head.starts_with("wpkh(") || head.starts_with("pkh(") || head.starts_with("sh(wpkh(") {
        ScriptCtx::SingleSig
    } else {
        ScriptCtx::MultiSig
    }
}

#[cfg(test)]
mod entry_tests {
    use super::*;

    #[test]
    fn end_to_end_wsh_multi_template_only() {
        let d = parse_template("wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))", &[], &[]).unwrap();
        assert_eq!(d.n, 2);
        assert_eq!(d.tree.tag, Tag::Wsh);
        assert!(d.tlv.pubkeys.is_none());
    }

    #[test]
    fn end_to_end_with_fingerprints() {
        let fps = vec![
            ParsedFingerprint {
                i: 0,
                fp: [0xDE, 0xAD, 0xBE, 0xEF],
            },
            ParsedFingerprint {
                i: 1,
                fp: [0xCA, 0xFE, 0xBA, 0xBE],
            },
        ];
        let d = parse_template("wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))", &[], &fps).unwrap();
        let v = d.tlv.fingerprints.unwrap();
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn ctx_for_wpkh_is_singlesig() {
        assert_eq!(ctx_for_template("wpkh(@0/<0;1>/*)"), ScriptCtx::SingleSig);
    }

    #[test]
    fn ctx_for_wsh_is_multisig() {
        assert_eq!(ctx_for_template("wsh(multi(2,...))"), ScriptCtx::MultiSig);
    }
}
