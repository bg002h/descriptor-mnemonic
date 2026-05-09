use crate::error::CliError;
use std::str::FromStr;

#[derive(Debug)]
pub enum CompileError {
    Parse(String),
    Compile(String),
    BadContext(String),
}
impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::Parse(m) => write!(f, "parse: {m}"),
            CompileError::Compile(m) => write!(f, "compile: {m}"),
            CompileError::BadContext(m) => write!(f, "bad-context: {m}"),
        }
    }
}
impl From<CompileError> for CliError {
    fn from(e: CompileError) -> Self { CliError::Compile(e.to_string()) }
}

#[derive(Debug, Clone, Copy)]
pub enum ScriptContext { Tap, SegwitV0 }
impl FromStr for ScriptContext {
    type Err = CompileError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s { "tap" => Ok(Self::Tap), "segwitv0" => Ok(Self::SegwitV0),
                  other => Err(CompileError::BadContext(other.into())) }
    }
}

/// Compile a high-level Concrete-Policy expression to a BIP-388 wallet-policy
/// template string.
///
/// `expr` is a Concrete-Policy expression with `@N` placeholders (e.g.
/// `or(pk(@0), and(pk(@1), older(144)))`, `thresh(2,pk(@0),pk(@1),pk(@2))`).
/// `ctx` selects the script context — `Tap` for `tr()`, `SegwitV0` for `wsh()`.
///
/// `unspendable_key` is a Tap-context-only fallback hint passed to miniscript's
/// `compile_tr(unspendable_key)`. Three accepted forms (per SPEC v0.17):
/// `Some("<xpub-style descriptor key>")` for advanced script-path-only spending
/// through a user-supplied key; `Some("<BIP-341 NUMS H-point hex>")` for
/// explicit NUMS (rendered as `Tag::TrUnspendable`); or `None` to let md-cli
/// auto-supply the BIP-341 NUMS H-point. The auto-NUMS default is strictly
/// additive — miniscript's extract-first behavior preserves single-key
/// extraction when possible; auto-NUMS only kicks in when no extraction is
/// available (e.g. threshold-multisig). For `SegwitV0`, the parameter is
/// ignored (`wsh()` has no internal-key concept).
pub fn compile_policy_to_template(
    expr: &str,
    ctx: ScriptContext,
    unspendable_key: Option<&str>,
) -> Result<String, CompileError> {
    use miniscript::policy::concrete::Policy;
    let policy: Policy<String> = expr.parse().map_err(|e| CompileError::Parse(format!("{e}")))?;
    match ctx {
        ScriptContext::SegwitV0 => {
            // wsh() has no internal-key concept; unspendable_key is silently
            // ignored. The CLI layer rejects the flag for --context segwitv0
            // before reaching here, but the assertion guards programmatic
            // callers in debug builds.
            debug_assert!(unspendable_key.is_none(),
                "unspendable_key must be None for SegwitV0; CLI should reject upstream");
            let ms = policy.compile::<miniscript::Segwitv0>()
                .map_err(|e| CompileError::Compile(format!("{e}")))?;
            Ok(format!("wsh({ms})"))
        }
        ScriptContext::Tap => {
            // Auto-NUMS default: when the caller doesn't supply an
            // unspendable_key, fall back to the canonical BIP-341 NUMS
            // H-point. miniscript's compile_tr is "extract-first; fallback
            // second" — so this default is strictly additive: it preserves
            // single-key extraction for policies like pk(@0) and only kicks
            // in when no extraction is possible (e.g. thresh(2,pk,pk,pk)).
            // See SPEC v0.17 § "--unspendable-key accepted forms".
            let unspendable = unspendable_key
                .map(String::from)
                .or_else(|| Some(crate::parse::template::NUMS_H_POINT_X_ONLY_HEX.to_string()));
            let desc = policy.compile_tr(unspendable)
                .map_err(|e| CompileError::Compile(format!("{e}")))?;
            // Descriptor::to_string() includes a trailing #<8-char-checksum>
            // (rust-miniscript's BIP-380 descriptor checksum); md1's encode
            // pipeline does not consume this. Strip via split_once('#').
            let rendered = desc.to_string();
            let template = rendered
                .split_once('#')
                .map(|(t, _)| t.to_string())
                .unwrap_or(rendered);
            Ok(template)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::template::NUMS_H_POINT_X_ONLY_HEX;

    #[test]
    fn compile_segwitv0_pk() {
        let s = compile_policy_to_template("pk(@0)", ScriptContext::SegwitV0, None).unwrap();
        assert!(s.starts_with("wsh("));
        assert!(s.contains("@0"));
    }

    #[test]
    fn bad_context() {
        assert!("xpub".parse::<ScriptContext>().is_err());
    }

    /// Spike-verified: `pk(@0)` with auto-NUMS default → extract wins → `tr(@0)`.
    #[test]
    fn compile_pk_tap_keypath_only() {
        let s = compile_policy_to_template("pk(@0)", ScriptContext::Tap, None).unwrap();
        assert_eq!(s, "tr(@0)", "single-key tap should extract @0 as internal key");
    }

    /// Spike-verified: `or(pk(@0),pk(@1))` → miniscript extracts @1 as internal,
    /// puts @0 as a script-path leaf. Auto-NUMS default ignored (extract wins).
    #[test]
    fn compile_or_two_keys_tap() {
        let s = compile_policy_to_template("or(pk(@0),pk(@1))", ScriptContext::Tap, None).unwrap();
        assert_eq!(s, "tr(@1,pk(@0))");
    }

    /// Spike-verified: `or(pk(@0),and(pk(@1),older(144)))` → inheritance pattern.
    /// Miniscript extracts @0; the and-branch becomes the script-path leaf.
    #[test]
    fn compile_or_pk_and_pk_older_tap() {
        let s = compile_policy_to_template(
            "or(pk(@0),and(pk(@1),older(144)))",
            ScriptContext::Tap, None,
        ).unwrap();
        assert_eq!(s, "tr(@0,and_v(v:pk(@1),older(144)))");
    }

    /// Spike-verified: `thresh(2,pk(@0),pk(@1),pk(@2))` → no key extractable;
    /// auto-NUMS default kicks in → `tr(<NUMS>, multi_a(2,@0,@1,@2))`.
    #[test]
    fn compile_thresh_2_of_3_tap_auto_nums() {
        let s = compile_policy_to_template(
            "thresh(2,pk(@0),pk(@1),pk(@2))",
            ScriptContext::Tap, None,
        ).unwrap();
        assert_eq!(s, format!("tr({NUMS_H_POINT_X_ONLY_HEX},multi_a(2,@0,@1,@2))"));
    }

    /// Spike-verified: `and(pk(@0),pk(@1))` → no single-key extractable;
    /// auto-NUMS default → `tr(<NUMS>, and_v(v:pk(@0),pk(@1)))`.
    #[test]
    fn compile_and_pk_pk_tap_auto_nums() {
        let s = compile_policy_to_template(
            "and(pk(@0),pk(@1))",
            ScriptContext::Tap, None,
        ).unwrap();
        assert_eq!(s, format!("tr({NUMS_H_POINT_X_ONLY_HEX},and_v(v:pk(@0),pk(@1)))"));
    }

    /// Explicit NUMS override: `pk(@0)` with `--unspendable-key <NUMS>` forces
    /// script-path-only by passing NUMS as the fallback hint. miniscript's
    /// compile_tr extract-first still picks @0 here (the policy has an
    /// extractable key); explicit NUMS is therefore silently ignored.
    /// (Spike pass 2 verified: `pk(@0)` with `Some("NUMS")` → `tr(@0)`.)
    #[test]
    fn compile_pk_tap_explicit_nums_extract_still_wins() {
        let s = compile_policy_to_template(
            "pk(@0)",
            ScriptContext::Tap,
            Some(NUMS_H_POINT_X_ONLY_HEX),
        ).unwrap();
        // Same as auto-NUMS default — extract-first behavior holds.
        assert_eq!(s, "tr(@0)");
    }

    /// Checksum strip: rust-miniscript's Descriptor::to_string() emits a
    /// trailing #<8-char-checksum> that md1 doesn't consume. The split_once
    /// strip MUST drop it.
    #[test]
    fn compile_strips_descriptor_checksum() {
        let s = compile_policy_to_template("pk(@0)", ScriptContext::Tap, None).unwrap();
        assert!(!s.contains('#'),
            "compile_policy_to_template output must not include #<checksum>; got {s:?}");
    }
}
