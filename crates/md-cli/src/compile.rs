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
            render_tr_template(&desc)
        }
    }
}

/// Render a compiled `tr(...)` descriptor to a BIP-388 template string.
///
/// **Why this exists instead of `Descriptor::to_string()`.** rust-miniscript
/// 13.0.0's `TapTree` `Display` (`descriptor/tr/taptree.rs`) tracks only the
/// depth *change between adjacent leaves*, which loses which leaves are
/// siblings whenever the leaf-depth sequence decreases or stays flat across a
/// subtree boundary. It then emits a run of leaves under a single brace pair,
/// and the result does not re-parse. Measured before this function existed:
///
/// ```text
/// thresh(1,pk(@0)..pk(@4))  ->  tr(@4,{{pk(@3),pk(@2),pk(@1),pk(@0)}})
/// or(pk(@0),or(pk(@1),or(pk(@2),pk(@3))))
///                           ->  tr(@0,{{pk(@3),pk(@2),pk(@1)}})
/// ```
///
/// Both are malformed, so a plain 1-of-5 taproot wallet could not be compiled
/// at all. Upstream fixed this in PR #953 (merged 2026-05-25), which is in **no
/// released version through 13.1.0** — and the pin cannot simply be advanced,
/// because `md-cli` does not compile against those revisions (two PR #915 API
/// breaks). So the corrected algorithm lives here.
///
/// It is #953's approach: an explicit child-count stack that closes a subtree
/// exactly when it has emitted both children. Note that #953 drops the old
/// trailing "close whatever is still open" loop and relies on the tree being a
/// proper binary tree; rather than inherit that assumption silently, an unclosed
/// stack is reported as an error below. `compile_tr` cannot produce such a tree,
/// which is precisely why a violation would mean something is wrong.
///
/// When a release carrying #953 lands and the pin moves, this becomes redundant
/// rather than wrong, and can be deleted in favour of `to_string()`.
///
/// Only the *tree* formatter was ever broken — the internal key and each leaf's
/// miniscript render correctly — so leaves are still rendered by their own
/// `Display`. Building the string here also means no BIP-380 `#checksum` is ever
/// appended, so there is nothing to strip.
fn render_tr_template(desc: &miniscript::Descriptor<String>) -> Result<String, CompileError> {
    let tr = match desc {
        miniscript::Descriptor::Tr(tr) => tr,
        // N2: this branch is **proven unreachable**, and is kept as defensive
        // cover rather than because anything depends on it. The proof chain,
        // recorded so a future reader need not re-derive it: this function has
        // exactly one call site (the `compile_tr` result above); `compile_tr`
        // returns `Descriptor::new_tr(...)`
        // (`vendor/miniscript/src/policy/concrete.rs:246`); and `new_tr` is
        // `Ok(Descriptor::Tr(Tr::new(key, script)?))`
        // (`vendor/miniscript/src/descriptor/mod.rs:246-248`). So the variant
        // can only ever be `Tr`.
        //
        // The body is the verbatim pre-fix checksum-stripping code, so it would
        // still be correct if some future call site made it reachable.
        other => {
            let rendered = other.to_string();
            return Ok(rendered
                .split_once('#')
                .map(|(t, _)| t.to_string())
                .unwrap_or(rendered));
        }
    };

    let internal = tr.internal_key();
    let Some(tree) = tr.tap_tree() else {
        return Ok(format!("tr({internal})"));
    };

    let mut out = String::new();
    let mut child_counts: Vec<u8> = Vec::new();
    for leaf in tree.leaves() {
        let depth = usize::from(leaf.depth());
        if !child_counts.is_empty() {
            out.push(',');
        }
        while child_counts.len() < depth {
            out.push('{');
            child_counts.push(0);
        }
        out.push_str(&leaf.miniscript().to_string());
        if let Some(c) = child_counts.last_mut() {
            *c += 1;
        }
        while let Some(2) = child_counts.last() {
            out.push('}');
            child_counts.pop();
            if let Some(c) = child_counts.last_mut() {
                *c += 1;
            }
        }
    }

    // N4: this branch is asserted by NO test, deliberately and unavoidably.
    // Mutation confirmed it: deleting the whole branch leaves every test green.
    // It cannot be exercised through the public surface either — `TapTree`'s
    // `depths_leaves` field is private
    // (`vendor/miniscript/src/descriptor/tr/taptree.rs:30`) and every
    // constructor preserves properness, so no caller can hand us an improper
    // tree. It is recorded as untested rather than quietly assumed sound: it is
    // this function's one deliberate departure from upstream PR #953, which
    // drops the equivalent guard and simply trusts the invariant.
    if !child_counts.is_empty() {
        // N1: the partial render is deliberately shown *without* balancing
        // parens — it is a truncated prefix, and pretending otherwise by
        // appending a `)` would misrepresent what was actually built. The
        // `[truncated: ...]` framing says so, rather than leaving the reader
        // to wonder whether the missing `)` is the bug being reported.
        return Err(CompileError::Compile(format!(
            "internal error: taptree left {} node(s) unclosed; refusing to emit a malformed template [truncated render: tr({internal},{out} ...]",
            child_counts.len()
        )));
    }

    Ok(format!("tr({internal},{out})"))
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
        crate::parse::template::parse_template(&t, &[], &[])
            .unwrap_or_else(|e| panic!("compile emitted an unparseable template for {expr}\n  emitted: {t}\n  error:   {e}"));
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
    /// The classes are chosen from the leaf-depth sequence, which is what the
    /// old formatter got wrong:
    ///
    /// | policy | leaf depths | class |
    /// |---|---|---|
    /// | `or(pk,pk)` | `[0]` | single bare leaf |
    /// | `thresh(1,pk,pk,pk)` | `[1,1]` | flat pair |
    /// | `or(pk,or(pk,or(pk,pk)))` | `[2,2,1]` | **decreasing** |
    /// | `thresh(1,pk×5)` | `[2,2,2,2]` | **balanced** |
    ///
    /// The last two are the ones that were broken.
    #[test]
    fn render_tr_template_pins_every_topology_class() {
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

    /// **Tripwire, not a bug report.** Asserts that upstream's `Display` is
    /// still broken for a non-caterpillar tree — which is the entire reason
    /// `render_tr_template` exists.
    ///
    /// Per this project's "pin the gap, don't fail forever" rule, this asserts
    /// the gap's *exact shape* rather than tolerating it. **When this test
    /// starts failing, that is good news:** it means the miniscript pin has
    /// moved to a release carrying PR #953, and `render_tr_template` can be
    /// deleted in favour of `Descriptor::to_string()` (minus its `#checksum`).
    ///
    /// **N5 — read the failure carefully before acting on it.** The exact
    /// string pins *three* independent upstream behaviours, not just the
    /// `Display` bug: which key `extract_key` promotes to the internal
    /// position, the `BinaryHeap` tie-break inside `with_huffman_tree`
    /// (`vendor/miniscript/src/policy/concrete.rs:955-978`), and the
    /// `fmt_helper` flattening defect. So a release that changed leaf
    /// *ordering* without fixing #953 would fire this test with a misleading
    /// message.
    ///
    /// Disambiguate with its sibling: `render_tr_template_pins_every_topology_class`
    /// drives the **same** policy through the **same** compiler, so an ordering
    /// change fires *both* tests, while a genuine #953 landing fires only this
    /// one. Measured at the time of writing: under miniscript 13.1.0 — what a
    /// build *without* `--locked` resolves to — all tests here still pass, so
    /// the hazard is hypothetical at the current pin.
    #[test]
    fn upstream_display_is_still_broken_delete_local_renderer_when_this_fails() {
        use miniscript::policy::concrete::Policy;
        let policy: Policy<String> = "thresh(1,pk(@0),pk(@1),pk(@2),pk(@3),pk(@4))"
            .parse()
            .unwrap();
        let desc = policy
            .compile_tr(Some(NUMS_H_POINT_X_ONLY_HEX.to_string()))
            .unwrap();
        let upstream = desc.to_string();
        let upstream = upstream.split_once('#').map(|(t, _)| t).unwrap_or(&upstream);

        assert_eq!(
            upstream, "tr(@4,{{pk(@3),pk(@2),pk(@1),pk(@0)}})",
            "upstream Display no longer flattens this taptree — check whether the \
             miniscript pin moved past PR #953; if so, delete render_tr_template"
        );
        // ...and confirm the flattened form is genuinely unusable, so this test
        // is about a real defect and not a cosmetic difference.
        assert!(
            crate::parse::template::parse_template(upstream, &[], &[]).is_err(),
            "upstream's flattened taptree unexpectedly re-parsed"
        );
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
    /// **N3 — what this test guards changed, and the old comment lied about
    /// it.** It used to say "the `split_once` strip MUST drop it". On the tap
    /// path there is no longer a `split_once` strip: `render_tr_template`
    /// builds the string with `format!`, so a `#` is never produced in the
    /// first place. The test is **not** vacuous — mutating the keypath-only
    /// return to `desc.to_string()` turns it red — but the claim it now proves
    /// is *"the keypath-only branch does not fall back to `to_string()`"*.
    ///
    /// Scope note, measured by mutation: this exercises only the
    /// `tap_tree() == None` branch. Appending a bogus `#deadbeef` to the
    /// **tree** output leaves this test green while killing 8 others — the
    /// tree path is covered, just not here.
    #[test]
    fn compile_strips_descriptor_checksum() {
        let s = compile_policy_to_template("pk(@0)", ScriptContext::Tap, None).unwrap();
        assert!(
            !s.contains('#'),
            "compile_policy_to_template output must not include #<checksum>; got {s:?}"
        );
    }
}
