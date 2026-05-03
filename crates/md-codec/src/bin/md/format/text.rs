use crate::error::CliError;
use md_codec::encode::Descriptor;
use md_codec::tag::Tag;
use md_codec::tree::{Body, Node};
use md_codec::use_site_path::UseSitePath;
use std::fmt::Write as _;

/// Render a `Descriptor` back to a BIP 388 template string with `@i` placeholders.
pub fn descriptor_to_template(d: &Descriptor) -> Result<String, CliError> {
    let mut out = String::new();
    render_node(&d.tree, &d.use_site_path,
                d.tlv.use_site_path_overrides.as_deref(), &mut out)?;
    Ok(out)
}

fn render_node(
    node: &Node,
    default_usp: &UseSitePath,
    overrides: Option<&[(u8, UseSitePath)]>,
    out: &mut String,
) -> Result<(), CliError> {
    match node.tag {
        Tag::Wpkh => render_wrapper("wpkh", node, default_usp, overrides, out),
        Tag::Pkh  => render_wrapper("pkh",  node, default_usp, overrides, out),
        Tag::Wsh  => render_wrapper("wsh",  node, default_usp, overrides, out),
        Tag::Sh   => render_wrapper("sh",   node, default_usp, overrides, out),
        Tag::Tr => {
            out.push_str("tr(");
            match &node.body {
                Body::Tr { key_index, tree } => {
                    render_key(*key_index, default_usp, overrides, out)?;
                    if let Some(t) = tree {
                        out.push(',');
                        render_tap_node(t, default_usp, overrides, out)?;
                    }
                }
                _ => return Err(CliError::TemplateParse("Tag::Tr without Body::Tr".into())),
            }
            out.push(')');
            Ok(())
        }
        Tag::Multi       => render_multi("multi",       node, default_usp, overrides, out),
        Tag::SortedMulti => render_multi("sortedmulti", node, default_usp, overrides, out),
        Tag::MultiA       => render_multi("multi_a",       node, default_usp, overrides, out),
        Tag::SortedMultiA => render_multi("sortedmulti_a", node, default_usp, overrides, out),
        Tag::PkK | Tag::PkH => match node.body {
            Body::KeyArg { index } => {
                if matches!(node.tag, Tag::PkH) { out.push_str("pk_h("); } else { out.push_str("pk("); }
                render_key(index, default_usp, overrides, out)?;
                out.push(')');
                Ok(())
            }
            _ => Err(CliError::TemplateParse("PkK/PkH without KeyArg body".into())),
        },
        other => Err(CliError::TemplateParse(format!("unsupported tag in render: {other:?}"))),
    }
}

/// Render a single-arity wrapper (wsh, sh, wpkh, pkh) — both `Children([inner])`
/// and `KeyArg{index}` (Wpkh/Pkh leaf form) work.
fn render_wrapper(
    name: &str, node: &Node, default_usp: &UseSitePath,
    overrides: Option<&[(u8, UseSitePath)]>, out: &mut String,
) -> Result<(), CliError> {
    out.push_str(name);
    out.push('(');
    match &node.body {
        Body::KeyArg { index } => render_key(*index, default_usp, overrides, out)?,
        Body::Children(v) if v.len() == 1 => render_node(&v[0], default_usp, overrides, out)?,
        _ => return Err(CliError::TemplateParse(format!("{name} body must be KeyArg or Children([1])"))),
    }
    out.push(')');
    Ok(())
}

fn render_multi(
    name: &str, node: &Node, default_usp: &UseSitePath,
    overrides: Option<&[(u8, UseSitePath)]>, out: &mut String,
) -> Result<(), CliError> {
    let (k, children) = match &node.body {
        Body::Variable { k, children } => (*k, children),
        _ => return Err(CliError::TemplateParse(format!("{name} body must be Variable"))),
    };
    write!(out, "{name}({k}").unwrap();
    for child in children {
        let idx = match child.body {
            Body::KeyArg { index } => index,
            _ => return Err(CliError::TemplateParse(format!("{name} child must be KeyArg"))),
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
    node: &Node, default_usp: &UseSitePath,
    overrides: Option<&[(u8, UseSitePath)]>, out: &mut String,
) -> Result<(), CliError> {
    if matches!(node.tag, Tag::TapTree) {
        let children = match &node.body {
            Body::Children(v) if v.len() == 2 => v,
            _ => return Err(CliError::TemplateParse("TapTree must have Children([2])".into())),
        };
        out.push('{');
        render_tap_node(&children[0], default_usp, overrides, out)?;
        out.push(',');
        render_tap_node(&children[1], default_usp, overrides, out)?;
        out.push('}');
        Ok(())
    } else {
        render_node(node, default_usp, overrides, out)
    }
}

fn render_key(idx: u8, default_usp: &UseSitePath, overrides: Option<&[(u8, UseSitePath)]>, out: &mut String) -> Result<(), CliError> {
    let usp = overrides.and_then(|v| v.iter().find(|(i, _)| *i == idx).map(|(_, u)| u)).unwrap_or(default_usp);
    write!(out, "@{idx}").unwrap();
    if let Some(alts) = &usp.multipath {
        out.push_str("/<");
        for (n, alt) in alts.iter().enumerate() {
            if n > 0 { out.push(';'); }
            write!(out, "{}", alt.value).unwrap();
            if alt.hardened { out.push('\''); }
        }
        out.push_str(">/*");
    } else {
        out.push_str("/*");
    }
    if usp.wildcard_hardened { out.push('\''); }
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
}
