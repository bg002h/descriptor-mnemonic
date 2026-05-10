use crate::error::CliError;
use crate::parse::template::NUMS_H_POINT_X_ONLY_HEX;
use md_codec::encode::Descriptor;
use md_codec::tag::Tag;
use md_codec::tree::{Body, Node};
use md_codec::use_site_path::UseSitePath;
use std::fmt::Write as _;

/// Render a `Descriptor` back to a BIP 388 template string with `@i` placeholders.
pub fn descriptor_to_template(d: &Descriptor) -> Result<String, CliError> {
    let mut out = String::new();
    render_node(
        &d.tree,
        d.n,
        &d.use_site_path,
        d.tlv.use_site_path_overrides.as_deref(),
        &mut out,
    )?;
    Ok(out)
}

fn render_node(
    node: &Node,
    n: u8,
    default_usp: &UseSitePath,
    overrides: Option<&[(u8, UseSitePath)]>,
    out: &mut String,
) -> Result<(), CliError> {
    match node.tag {
        Tag::Wpkh => render_wrapper("wpkh", node, n, default_usp, overrides, out),
        Tag::Pkh => render_wrapper("pkh", node, n, default_usp, overrides, out),
        Tag::Wsh => render_wrapper("wsh", node, n, default_usp, overrides, out),
        Tag::Sh => render_wrapper("sh", node, n, default_usp, overrides, out),
        Tag::Tr => {
            out.push_str("tr(");
            match &node.body {
                Body::Tr { key_index, tree } => {
                    // v0.18 NUMS sentinel: key_index == n encodes the BIP-341
                    // NUMS H-point as the implicit internal key. Render as the
                    // literal x-only hex string. Other values reference @N.
                    if *key_index == n {
                        out.push_str(NUMS_H_POINT_X_ONLY_HEX);
                    } else {
                        render_key(*key_index, default_usp, overrides, out)?;
                    }
                    if let Some(t) = tree {
                        out.push(',');
                        render_tap_node(t, n, default_usp, overrides, out)?;
                    }
                }
                _ => return Err(CliError::TemplateParse("Tag::Tr without Body::Tr".into())),
            }
            out.push(')');
            Ok(())
        }
        Tag::Multi => render_multi("multi", node, default_usp, overrides, out),
        Tag::SortedMulti => render_multi("sortedmulti", node, default_usp, overrides, out),
        Tag::MultiA => render_multi("multi_a", node, default_usp, overrides, out),
        Tag::SortedMultiA => render_multi("sortedmulti_a", node, default_usp, overrides, out),
        Tag::PkK | Tag::PkH => match node.body {
            Body::KeyArg { index } => {
                if matches!(node.tag, Tag::PkH) {
                    out.push_str("pk_h(");
                } else {
                    out.push_str("pk(");
                }
                render_key(index, default_usp, overrides, out)?;
                out.push(')');
                Ok(())
            }
            _ => Err(CliError::TemplateParse(
                "PkK/PkH without KeyArg body".into(),
            )),
        },
        Tag::AndV => {
            // and_v(left, right) — function-call syntax. Used inside tap-script
            // leaves for and-conjunction / inheritance patterns.
            let kids = match &node.body {
                Body::Children(v) if v.len() == 2 => v,
                _ => {
                    return Err(CliError::TemplateParse(
                        "AndV body must be Children([2])".into(),
                    ));
                }
            };
            out.push_str("and_v(");
            render_node(&kids[0], n, default_usp, overrides, out)?;
            out.push(',');
            render_node(&kids[1], n, default_usp, overrides, out)?;
            out.push(')');
            Ok(())
        }
        Tag::Verify => {
            // `v:` wrapper — prefix syntax (no parens). The wrapped child is
            // rendered inline; e.g. `v:pk(@1)`.
            let inner = match &node.body {
                Body::Children(v) if v.len() == 1 => &v[0],
                _ => {
                    return Err(CliError::TemplateParse(
                        "Verify body must be Children([1])".into(),
                    ));
                }
            };
            out.push_str("v:");
            render_node(inner, n, default_usp, overrides, out)
        }
        Tag::Older => {
            let v = match node.body {
                Body::Timelock(v) => v,
                _ => {
                    return Err(CliError::TemplateParse(
                        "Older body must be Timelock".into(),
                    ));
                }
            };
            write!(out, "older({v})").unwrap();
            Ok(())
        }
        Tag::After => {
            let v = match node.body {
                Body::Timelock(v) => v,
                _ => {
                    return Err(CliError::TemplateParse(
                        "After body must be Timelock".into(),
                    ));
                }
            };
            write!(out, "after({v})").unwrap();
            Ok(())
        }
        Tag::AndB => render_binary("and_b", node, n, default_usp, overrides, out),
        Tag::OrB => render_binary("or_b", node, n, default_usp, overrides, out),
        Tag::OrC => render_binary("or_c", node, n, default_usp, overrides, out),
        Tag::OrD => render_binary("or_d", node, n, default_usp, overrides, out),
        Tag::OrI => render_binary("or_i", node, n, default_usp, overrides, out),
        Tag::AndOr => {
            // andor(a, b, c) — ternary "if a then b else c". Only ternary
            // fragment in miniscript; Body::Children must have length 3.
            let kids = match &node.body {
                Body::Children(v) if v.len() == 3 => v,
                _ => {
                    return Err(CliError::TemplateParse(
                        "AndOr body must be Children([3])".into(),
                    ));
                }
            };
            out.push_str("andor(");
            render_node(&kids[0], n, default_usp, overrides, out)?;
            out.push(',');
            render_node(&kids[1], n, default_usp, overrides, out)?;
            out.push(',');
            render_node(&kids[2], n, default_usp, overrides, out)?;
            out.push(')');
            Ok(())
        }
        Tag::Thresh => {
            // thresh(k, c1, c2, ..., cn) — k-of-n threshold over arbitrary
            // miniscript fragments (distinct from Multi/MultiA which take only
            // keys). Each child is rendered recursively.
            let (k, children) = match &node.body {
                Body::Variable { k, children } => (*k, children),
                _ => {
                    return Err(CliError::TemplateParse(
                        "Thresh body must be Variable".into(),
                    ));
                }
            };
            write!(out, "thresh({k}").unwrap();
            for child in children {
                out.push(',');
                render_node(child, n, default_usp, overrides, out)?;
            }
            out.push(')');
            Ok(())
        }
        other => Err(CliError::TemplateParse(format!(
            "unsupported tag in render: {other:?}"
        ))),
    }
}

/// Render a binary fragment `name(left, right)` — used for and_b, or_b, or_c,
/// or_d, or_i. Body::Children must have exactly 2 elements.
fn render_binary(
    name: &str,
    node: &Node,
    n: u8,
    default_usp: &UseSitePath,
    overrides: Option<&[(u8, UseSitePath)]>,
    out: &mut String,
) -> Result<(), CliError> {
    let kids = match &node.body {
        Body::Children(v) if v.len() == 2 => v,
        _ => {
            return Err(CliError::TemplateParse(format!(
                "{name} body must be Children([2])"
            )));
        }
    };
    out.push_str(name);
    out.push('(');
    render_node(&kids[0], n, default_usp, overrides, out)?;
    out.push(',');
    render_node(&kids[1], n, default_usp, overrides, out)?;
    out.push(')');
    Ok(())
}

/// Render a single-arity wrapper (wsh, sh, wpkh, pkh) — both `Children([inner])`
/// and `KeyArg{index}` (Wpkh/Pkh leaf form) work.
fn render_wrapper(
    name: &str,
    node: &Node,
    n: u8,
    default_usp: &UseSitePath,
    overrides: Option<&[(u8, UseSitePath)]>,
    out: &mut String,
) -> Result<(), CliError> {
    out.push_str(name);
    out.push('(');
    match &node.body {
        Body::KeyArg { index } => render_key(*index, default_usp, overrides, out)?,
        Body::Children(v) if v.len() == 1 => render_node(&v[0], n, default_usp, overrides, out)?,
        _ => {
            return Err(CliError::TemplateParse(format!(
                "{name} body must be KeyArg or Children([1])"
            )));
        }
    }
    out.push(')');
    Ok(())
}

fn render_multi(
    name: &str,
    node: &Node,
    default_usp: &UseSitePath,
    overrides: Option<&[(u8, UseSitePath)]>,
    out: &mut String,
) -> Result<(), CliError> {
    let (k, children) = match &node.body {
        Body::Variable { k, children } => (*k, children),
        _ => {
            return Err(CliError::TemplateParse(format!(
                "{name} body must be Variable"
            )));
        }
    };
    write!(out, "{name}({k}").unwrap();
    for child in children {
        let idx = match child.body {
            Body::KeyArg { index } => index,
            _ => {
                return Err(CliError::TemplateParse(format!(
                    "{name} child must be KeyArg"
                )));
            }
        };
        out.push(',');
        render_key(idx, default_usp, overrides, out)?;
    }
    out.push(')');
    Ok(())
}

/// Render a tap-tree node. Branches → `{left,right}`; leaves → render their body
/// directly (no wrapper around the leaf).
fn render_tap_node(
    node: &Node,
    n: u8,
    default_usp: &UseSitePath,
    overrides: Option<&[(u8, UseSitePath)]>,
    out: &mut String,
) -> Result<(), CliError> {
    if matches!(node.tag, Tag::TapTree) {
        let children = match &node.body {
            Body::Children(v) if v.len() == 2 => v,
            _ => {
                return Err(CliError::TemplateParse(
                    "TapTree must have Children([2])".into(),
                ));
            }
        };
        out.push('{');
        render_tap_node(&children[0], n, default_usp, overrides, out)?;
        out.push(',');
        render_tap_node(&children[1], n, default_usp, overrides, out)?;
        out.push('}');
        Ok(())
    } else {
        render_node(node, n, default_usp, overrides, out)
    }
}

fn render_key(
    idx: u8,
    default_usp: &UseSitePath,
    overrides: Option<&[(u8, UseSitePath)]>,
    out: &mut String,
) -> Result<(), CliError> {
    let usp = overrides
        .and_then(|v| v.iter().find(|(i, _)| *i == idx).map(|(_, u)| u))
        .unwrap_or(default_usp);
    write!(out, "@{idx}").unwrap();
    if let Some(alts) = &usp.multipath {
        out.push_str("/<");
        for (n, alt) in alts.iter().enumerate() {
            if n > 0 {
                out.push(';');
            }
            write!(out, "{}", alt.value).unwrap();
            if alt.hardened {
                out.push('\'');
            }
        }
        out.push_str(">/*");
    } else {
        out.push_str("/*");
    }
    if usp.wildcard_hardened {
        out.push('\'');
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::template::parse_template;

    #[test]
    fn roundtrip_wpkh_singlepath() {
        let t = "wpkh(@0/*)";
        let d = parse_template(t, &[], &[]).unwrap();
        assert_eq!(descriptor_to_template(&d).unwrap(), t);
    }

    #[test]
    fn roundtrip_wsh_multi_2of2() {
        let t = "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))";
        let d = parse_template(t, &[], &[]).unwrap();
        assert_eq!(descriptor_to_template(&d).unwrap(), t);
    }

    #[test]
    fn roundtrip_sh_wpkh() {
        let t = "sh(wpkh(@0/<0;1>/*))";
        let d = parse_template(t, &[], &[]).unwrap();
        assert_eq!(descriptor_to_template(&d).unwrap(), t);
    }

    #[test]
    fn roundtrip_tr_keyonly() {
        let t = "tr(@0/<0;1>/*)";
        let d = parse_template(t, &[], &[]).unwrap();
        assert_eq!(descriptor_to_template(&d).unwrap(), t);
    }

    /// v0.17 Phase 2 — round-trip of the inheritance pattern through the
    /// text renderer (decode-side path). Ensures `Tag::AndV`, `Tag::Verify`,
    /// `Tag::Older` render correctly when descriptors are reconstructed from
    /// md1 bytecode.
    #[test]
    fn roundtrip_tr_and_v_verify_older_inheritance() {
        let t = "tr(@0/<0;1>/*,and_v(v:pk(@1/<0;1>/*),older(144)))";
        let d = parse_template(t, &[], &[]).unwrap();
        assert_eq!(descriptor_to_template(&d).unwrap(), t);
    }

    /// v0.18 Phase 4a — or_d recovery pattern: `or_d(pk(K1), and_v(v:pk(K2),
    /// older(N)))`. Common BOLT-3-style hot-cold split: hot key spends
    /// immediately; cold key spends after timelock.
    #[test]
    fn roundtrip_tr_or_d_recovery_pattern() {
        let t = "tr(@0/<0;1>/*,or_d(pk(@1/<0;1>/*),and_v(v:pk(@2/<0;1>/*),older(144))))";
        let d = parse_template(t, &[], &[]).unwrap();
        assert_eq!(descriptor_to_template(&d).unwrap(), t);
    }

    /// v0.18 Phase 4a — or_i disjunction round-trip in tap-leaf context.
    #[test]
    fn roundtrip_tr_or_i_disjunction() {
        let t = "tr(@0/<0;1>/*,or_i(pk(@1/<0;1>/*),pk(@2/<0;1>/*)))";
        let d = parse_template(t, &[], &[]).unwrap();
        assert_eq!(descriptor_to_template(&d).unwrap(), t);
    }

    // or_c top-level test deferred to Phase 4b: or_c is V-typed at the
    // top level which miniscript rejects as a non-T fragment. Phase 4b's
    // wrapper coverage will enable wrapping or_c with `t:` (or using it
    // inside another fragment) so a valid testable expression exists.

    /// v0.18 Phase 4a — andor ternary: `andor(a, b, c)` is "if a then b else
    /// c". The only ternary fragment in miniscript; exercises Body::Children
    /// with length 3.
    #[test]
    fn roundtrip_tr_and_or_ternary() {
        let t = "tr(@0/<0;1>/*,andor(pk(@1/<0;1>/*),pk(@2/<0;1>/*),pk(@3/<0;1>/*)))";
        let d = parse_template(t, &[], &[]).unwrap();
        assert_eq!(descriptor_to_template(&d).unwrap(), t);
    }

    /// v0.18 Phase 4a — `after()` absolute timelock (BIP-65). Distinct from
    /// `older()` (relative timelock, BIP-112). Pinned at a value > 500_000_000
    /// would be a Unix-timestamp; here the height-form is exercised.
    #[test]
    fn roundtrip_tr_and_v_after_absolute_timelock() {
        let t = "tr(@0/<0;1>/*,and_v(v:pk(@1/<0;1>/*),after(700000)))";
        let d = parse_template(t, &[], &[]).unwrap();
        assert_eq!(descriptor_to_template(&d).unwrap(), t);
    }
}

use md_codec::chunk::ChunkHeader;
use md_codec::identity::{Md1EncodingId, WalletDescriptorTemplateId, WalletPolicyId};

pub fn fmt_md1_id(id: &Md1EncodingId) -> String {
    let bytes = id.as_bytes();
    let mut s = String::with_capacity(64);
    for b in bytes {
        write!(s, "{b:02x}").unwrap();
    }
    s
}
pub fn fmt_template_id(id: &WalletDescriptorTemplateId) -> String {
    let bytes = id.as_bytes();
    let mut s = String::with_capacity(64);
    for b in bytes {
        write!(s, "{b:02x}").unwrap();
    }
    s
}
pub fn fmt_policy_id(id: &WalletPolicyId) -> String {
    let bytes = id.as_bytes();
    let mut s = String::with_capacity(32);
    for b in bytes {
        write!(s, "{b:02x}").unwrap();
    }
    s
}
/// 4-byte fingerprint of a `WalletPolicyId`. v0.14's `WalletPolicyId` has
/// no `fingerprint()` method; we slice the first 4 bytes directly.
pub fn fmt_policy_id_fingerprint(id: &WalletPolicyId) -> String {
    let b = id.as_bytes();
    format!("0x{:02x}{:02x}{:02x}{:02x}", b[0], b[1], b[2], b[3])
}
#[allow(dead_code)] // declared for future chunked-display callers; no current usage
pub fn fmt_chunk_header(h: &ChunkHeader) -> String {
    format!(
        "chunk-set-id=0x{:05x}, count={}, index={}",
        h.chunk_set_id, h.count, h.index
    )
}

#[cfg(test)]
mod hash_tests {
    use super::*;

    #[test]
    fn policy_id_fingerprint_format() {
        let bytes = [
            0x9E, 0x1D, 0x72, 0xB6, 0x00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let id = WalletPolicyId::new(bytes);
        assert_eq!(fmt_policy_id_fingerprint(&id), "0x9e1d72b6");
    }
}
