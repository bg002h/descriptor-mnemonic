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
    fn from(e: CompileError) -> Self {
        CliError::Compile(e.to_string())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ScriptContext {
    Tap,
    SegwitV0,
}
impl FromStr for ScriptContext {
    type Err = CompileError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "tap" => Ok(Self::Tap),
            "segwitv0" => Ok(Self::SegwitV0),
            other => Err(CompileError::BadContext(other.into())),
        }
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
/// `compile_tr(unspendable_key)`. Two accepted forms (v0.18 Item G CLI guard):
/// `Some("<BIP-341 NUMS H-point hex>")` (the literal x-only hex
/// `50929b...e803ac0`) for explicit NUMS, or `None` to let md-cli auto-supply
/// the BIP-341 NUMS H-point. The CLI dispatch layer rejects any other value
/// (xpub-style descriptor keys, arbitrary x-only hex) with a BadArg noting
/// that those forms are deferred to a future version. The auto-NUMS default
/// is strictly additive — miniscript's extract-first behavior preserves
/// single-key extraction when possible; auto-NUMS only kicks in when no
/// extraction is available (e.g. threshold-multisig). For `SegwitV0`, the
/// parameter is ignored (`wsh()` has no internal-key concept).
pub fn compile_policy_to_template(
    expr: &str,
    ctx: ScriptContext,
    unspendable_key: Option<&str>,
) -> Result<String, CompileError> {
    use miniscript::policy::concrete::Policy;
    let policy: Policy<String> = expr
        .parse()
        .map_err(|e| CompileError::Parse(format!("{e}")))?;
    match ctx {
        ScriptContext::SegwitV0 => {
            // wsh() has no internal-key concept; unspendable_key is silently
            // ignored. The CLI layer rejects the flag for --context segwitv0
            // before reaching here, but the assertion guards programmatic
            // callers in debug builds.
            debug_assert!(
                unspendable_key.is_none(),
                "unspendable_key must be None for SegwitV0; CLI should reject upstream"
            );
            let ms = policy
                .compile::<miniscript::Segwitv0>()
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
            let desc = policy
                .compile_tr(unspendable)
                .map_err(|e| CompileError::Compile(format!("{e}")))?;
            // Upstream `Descriptor::to_string()` (rust-miniscript PR #953,
            // pinned via the workspace `[patch.crates-io]` git rev) now
            // renders non-caterpillar taptrees correctly; the local
            // child-count-stack renderer this replaced is redundant. Upstream
            // still appends a BIP-380 `#checksum` that the old `format!`
            // build never produced, so it is stripped here to keep
            // md1's encode pipeline checksum-free (compile_strips_descriptor_checksum).
            let rendered = desc.to_string();
            Ok(rendered
                .split_once('#')
                .map(|(t, _)| t.to_string())
                .unwrap_or(rendered))
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

    /// Assert that what `compile_policy_to_template` emits actually re-parses.
    ///
    /// This is the property that was silently missing: the function returned
    /// `Ok(<malformed string>)` and the failure only surfaced downstream, in
    /// `parse_template`, as `miniscript parse failed: taptree branch must have
    /// 2 children, but found 1`.
    fn compiles_and_reparses(expr: &str) -> String {
        let t = compile_policy_to_template(expr, ScriptContext::Tap, None)
            .unwrap_or_else(|e| panic!("compile failed for {expr}: {e}"));
        crate::parse::template::parse_template(&t, &[], &[]).unwrap_or_else(|e| {
            panic!(
                "compile emitted an unparseable template for {expr}\n  emitted: {t}\n  error:   {e}"
            )
        });
        t
    }

    /// A plain **1-of-5 taproot wallet** — the smallest realistic trigger.
    ///
    /// `compile_tr` builds a taptree whose leaf-depth sequence DECREASES, and
    /// rust-miniscript 13.0.0's `Display` (which this module used to
    /// round-trip through) tracks only the depth change between ADJACENT
    /// leaves, so it emits a flat run of leaves under one brace pair. Fixed
    /// upstream by PR #953, which is merged but in no release through 13.1.0.
    #[test]
    fn compile_thresh_1_of_5_tap_reparses() {
        compiles_and_reparses("thresh(1,pk(@0),pk(@1),pk(@2),pk(@3),pk(@4))");
    }

    /// Right-skewed `or` chain over four keys — the other measured trigger.
    #[test]
    fn compile_or_chain_four_keys_tap_reparses() {
        compiles_and_reparses("or(pk(@0),or(pk(@1),or(pk(@2),pk(@3))))");
    }

    /// The shapes that already worked must keep working — these are
    /// caterpillar trees (leaf depths never decrease), which the buggy
    /// formatter happened to render correctly.
    #[test]
    fn compile_caterpillar_shapes_still_reparse() {
        assert_eq!(compiles_and_reparses("or(pk(@0),pk(@1))"), "tr(@1,pk(@0))");
        compiles_and_reparses("thresh(1,pk(@0),pk(@1),pk(@2))");
        compiles_and_reparses("or(4@pk(@0),1@or(pk(@1),pk(@2)))");
        compiles_and_reparses("or(1@or(pk(@0),pk(@1)),1@or(pk(@2),pk(@3)))");
    }

    /// Pin the EXACT rendering across the four topology classes, so a
    /// regression shows up as a wrong tree rather than merely a parse failure.
    ///
    /// The classes are chosen from the leaf-depth sequence, which is what
    /// upstream's pre-PR-#953 `Display` used to get wrong (see the workspace
    /// `Cargo.toml` `[patch.crates-io]` comment for that history):
    ///
    /// | policy | leaf depths | class |
    /// |---|---|---|
    /// | `or(pk,pk)` | `[0]` | single bare leaf |
    /// | `thresh(1,pk,pk,pk)` | `[1,1]` | flat pair |
    /// | `or(pk,or(pk,or(pk,pk)))` | `[2,2,1]` | **decreasing** |
    /// | `thresh(1,pk×5)` | `[2,2,2,2]` | **balanced** |
    ///
    /// The last two are the ones that were broken. Expected strings are
    /// checksum-free by construction (no `#`), which doubles as the gate for
    /// `compile_policy_to_template`'s Tap-branch checksum strip: if that strip
    /// were ever dropped, upstream's `Descriptor::to_string()` would append a
    /// BIP-380 `#checksum` and every case here would fail.
    #[test]
    fn tap_template_pins_every_topology_class() {
        let cases = [
            ("or(pk(@0),pk(@1))", "tr(@1,pk(@0))"),
            ("thresh(1,pk(@0),pk(@1),pk(@2))", "tr(@2,{pk(@1),pk(@0)})"),
            (
                "or(pk(@0),or(pk(@1),or(pk(@2),pk(@3))))",
                "tr(@0,{{pk(@3),pk(@2)},pk(@1)})",
            ),
            (
                "thresh(1,pk(@0),pk(@1),pk(@2),pk(@3),pk(@4))",
                "tr(@4,{{pk(@3),pk(@2)},{pk(@1),pk(@0)}})",
            ),
        ];
        for (policy, expected) in cases {
            let got = compile_policy_to_template(policy, ScriptContext::Tap, None)
                .unwrap_or_else(|e| panic!("compile failed for {policy}: {e}"));
            assert_eq!(got, expected, "wrong taptree rendering for {policy}");
        }
    }

    /// Spike-verified: `pk(@0)` with auto-NUMS default → extract wins → `tr(@0)`.
    #[test]
    fn compile_pk_tap_keypath_only() {
        let s = compile_policy_to_template("pk(@0)", ScriptContext::Tap, None).unwrap();
        assert_eq!(
            s, "tr(@0)",
            "single-key tap should extract @0 as internal key"
        );
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
            ScriptContext::Tap,
            None,
        )
        .unwrap();
        assert_eq!(s, "tr(@0,and_v(v:pk(@1),older(144)))");
    }

    /// Spike-verified: `thresh(2,pk(@0),pk(@1),pk(@2))` → no key extractable;
    /// auto-NUMS default kicks in → `tr(<NUMS>, multi_a(2,@0,@1,@2))`.
    #[test]
    fn compile_thresh_2_of_3_tap_auto_nums() {
        let s =
            compile_policy_to_template("thresh(2,pk(@0),pk(@1),pk(@2))", ScriptContext::Tap, None)
                .unwrap();
        assert_eq!(
            s,
            format!("tr({NUMS_H_POINT_X_ONLY_HEX},multi_a(2,@0,@1,@2))")
        );
    }

    /// Spike-verified: `and(pk(@0),pk(@1))` → no single-key extractable;
    /// auto-NUMS default → `tr(<NUMS>, and_v(v:pk(@0),pk(@1)))`.
    #[test]
    fn compile_and_pk_pk_tap_auto_nums() {
        let s = compile_policy_to_template("and(pk(@0),pk(@1))", ScriptContext::Tap, None).unwrap();
        assert_eq!(
            s,
            format!("tr({NUMS_H_POINT_X_ONLY_HEX},and_v(v:pk(@0),pk(@1)))")
        );
    }

    /// Explicit NUMS override: `pk(@0)` with `--unspendable-key <NUMS>` forces
    /// script-path-only by passing NUMS as the fallback hint. miniscript's
    /// compile_tr extract-first still picks @0 here (the policy has an
    /// extractable key); explicit NUMS is therefore silently ignored.
    /// (Spike pass 2 verified: `pk(@0)` with `Some("NUMS")` → `tr(@0)`.)
    #[test]
    fn compile_pk_tap_explicit_nums_extract_still_wins() {
        let s =
            compile_policy_to_template("pk(@0)", ScriptContext::Tap, Some(NUMS_H_POINT_X_ONLY_HEX))
                .unwrap();
        // Same as auto-NUMS default — extract-first behavior holds.
        assert_eq!(s, "tr(@0)");
    }

    /// No `#<8-char-checksum>` may reach md1's encode pipeline.
    ///
    /// Since P1 deleted the local `render_tr_template` renderer, the Tap
    /// branch of `compile_policy_to_template` renders via upstream
    /// `Descriptor::to_string()` for every shape — keypath-only (`tr(@0)`)
    /// and taptree alike, both through the same code path — and upstream
    /// DOES append a BIP-380 `#checksum`. This test guards the
    /// `split_once('#')` strip that follows it; `tap_template_pins_every_topology_class`
    /// guards the same strip for taptree shapes via its checksum-free
    /// expected strings.
    #[test]
    fn compile_strips_descriptor_checksum() {
        let s = compile_policy_to_template("pk(@0)", ScriptContext::Tap, None).unwrap();
        assert!(
            !s.contains('#'),
            "compile_policy_to_template output must not include #<checksum>; got {s:?}"
        );
    }
}
