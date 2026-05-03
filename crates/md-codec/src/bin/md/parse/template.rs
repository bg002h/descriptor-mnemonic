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
