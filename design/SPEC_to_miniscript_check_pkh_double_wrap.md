# SPEC — md-codec `to_miniscript` `Check(PkH)`/`Check(PkK)` double-wrap fix (PART 2)

**Cycle:** md-codec **PATCH 0.35.0 → 0.35.1** (renderer-tolerance; no wire/API change) + md-cli exact-pin lockstep · **Source SHA:** `b93674f` · **Recon:** the toolkit C1 deep-dive (agent B), `mnemonic-toolkit/design/RECON_faithful_general_policy_restore.md` §PART 2.
**Resolves:** `to-miniscript-check-pkh-double-wrap` (companion to the toolkit's v0.54.0 restore C1 fix — unblocks faithful restore of `pk(@N)`/`pkh(@N)`-keyed policies).

## Root cause (empirically confirmed)
`md-codec/src/to_miniscript.rs::node_to_miniscript` renders a key-check node twice:
- `Tag::PkK`/`Tag::PkH` arms (`:290-303`) build `Terminal::Check(from_ast(Terminal::PkK/PkH(pk)))` — they RE-APPLY `Check` to the bare key (the wire strips the outer `c:`; this re-adds it). Correct in isolation: `Check(PkK)` = `pk()` (type B).
- `Tag::Check` arm (`:304-307`) UNCONDITIONALLY wraps `Terminal::Check(node_to_miniscript(children[0]))`. When `children[0]` is `Tag::PkH`/`PkK`, the child arm already produced `Check(PkH)` (type B) → the outer `Check` is `c:` over type-B → `from_ast`/type-check fails: **"fragment «c:pkh(…)» cannot wrap a fragment of type B"**.

So a wire `Tag::Check(Tag::PkH)` (exactly what the toolkit walker emits for `pkh(@N)` in non-tap context — `parse_descriptor.rs:601-624`, tap_context-gated; pre-v0.30 md-cli cards carry the same shape) renders `Check(Check(PkH))` and **errors**. Multi-keyed policies avoid it entirely (`multi()` keys go through `Body::MultiKeys` → `build_multi_threshold`, never the PkK/PkH/Check arms). Verified: `pk_k`/`pk_h` are `Base::K`, `cast_check` maps `K→B` else `ChildBase1` — identical in crates.io 13.0.0 and the toolkit's patched rev `95fdd1c` (no version-dependent behavior).

This is MANDATORY regardless of any encoder fix: **affected md1 cards already exist on steel** (toolkit-emitted `wsh(andor(pkh(@0),…))` bundles since v0.19.0; pre-v0.30 md-cli cards) — they cannot be re-engraved, so the renderer must accept the shape.

## FIX (Tier A1 — Check-idempotence collapse; minimal, strictly error→success)
In the `Tag::Check` arm, when the single child is a bare `PkK`/`PkH` key node (`Body::KeyArg`), return the child's render DIRECTLY (it already applies `Check`) instead of wrapping a second `Check`. Mirrors md-cli's text-renderer trailing-`c` collapse (`format/text.rs:363-385`).
```rust
(Tag::Check, Body::Children(children)) => {
    arity_eq(node.tag, children.len(), 1)?;
    // Check-idempotence: `Tag::Check` over a bare key tag denotes the SAME
    // fragment as the bare key tag — both mean `c:pk_k`/`c:pk_h` (type B), and
    // the PkK/PkH arms already re-apply `Check`. Wrapping a second `Check`
    // produces `Check(Check(PkH))` = `c:` over type-B → a type error. (The wire
    // shape `Tag::Check(Tag::PkK/PkH)` is emitted by the toolkit walker in
    // non-tap context and by pre-v0.30 md-cli cards — both already on steel.)
    if matches!(
        (&children[0].tag, &children[0].body),
        (Tag::PkK | Tag::PkH, Body::KeyArg { .. })
    ) {
        return node_to_miniscript::<Ctx>(&children[0], keys);
    }
    Terminal::Check(Arc::new(node_to_miniscript::<Ctx>(&children[0], keys)?))
}
```
A genuinely ill-typed wire (`Check(Check(PkK))` — a `Tag::Check` whose child is itself a `Tag::Check`) still correctly errors (the inner non-key child renders type-B, the outer `Check` rejects it). No currently-SUCCEEDING input changes output (every affected shape errors today) → strictly error→success.

**Tier A2 (R0 DECISION — optional, defer or take):** thread a `want_k: bool` through `node_to_miniscript` (default `false`): `Tag::PkK/PkH` emit bare `Terminal::PkK/PkH` (type K) when `want_k`, else `Check(...)`; `Tag::Check` renders its child with `want_k=true` then wraps one `Check`; `OrI`→both children inherit, `AndV`→right child, `AndOr`→children 2&3 inherit, all else reset to `false`. ~25 LOC; closes the deeper `Check(or_i(pk_k,pk_k))` shape C too. `want_k=false` everywhere reproduces today's behavior bit-for-bit. **Recommendation: ship A1 (unblocks the flagship + all on-steel cards with minimal blast radius); file shape C as a follow-up** unless R0 prefers A2's completeness.

## Tests (md-codec, RED-first)
- `wsh_check_pkh_renders_same_as_bare_pkh`: a wire `Wsh(Check(PkH@0))` renders the SAME descriptor string as `Wsh(PkH@0)` (the md-cli-canonical bare form). RED-proven (pre-fix the `Check(PkH)` shape errors).
- `flagship_pkh_keyed_policy_renders`: the flagship tree `wsh(andor(pkh(@0),after(N),or_i(and_v(v:pkh(@1),older(M)),and_v(v:pkh(@2),older(K)))))` in the toolkit `Check(PkH)` wire dialect renders to the expected descriptor (RED pre-fix).
- `ill_typed_double_check_still_errors`: a hand-built `Check(Check(PkK))` still errors (guards the collapse is key-shape-specific).
- (A2 only) `check_or_i_pk_k renders c:or_i(pk_k,pk_k)`.

## md-cli lockstep
md-cli pins `md-codec = { path, version = "=0.35.0" }` (`crates/md-cli/Cargo.toml:28`) → bump the exact-pin to `=0.35.1` (md-cli version unchanged unless its own surface changes). md-cli's text renderer already handles the direct `Check(PkK/PkH)` case (`format/text.rs:360-385`); A2 (if taken) additionally needs md-cli `format/text.rs:64-85` to emit `pk_k`/`pk_h` at K positions for shape C — A1 needs no md-cli code change.

## Ritual / SemVer / cross-repo
- md-codec **PATCH 0.35.1** (renderer accepts a previously-erroring wire shape; no wire/encode/API change). CHANGELOG (md-codec + md-cli if it has one). No `[patch]`/sibling-pin gates in this repo for md-codec.
- **crates.io publish of md-codec 0.35.1 is IRREVERSIBLE → confirm with the user before `cargo publish`.** Toolkit then `cargo update -p md-codec` (it declares `md-codec = "0.35"`) → the pk-keyed flagship reconstructs through the toolkit's existing v0.54.0 general arm with ZERO toolkit code changes (new test cells only).
- FOLLOWUPS: resolve `to-miniscript-check-pkh-double-wrap` (file the companion entry in THIS repo cross-citing the toolkit entry); optionally file the toolkit walker-canonicity companion (drop the `tap_context` gate in `parse_descriptor.rs` so toolkit & md-cli agree on `wallet_policy_id` — a separate MINOR wire-content change, NOT required for this fix). Mandatory R0 gate to 0C/0I; persist reviews to `design/agent-reports/`.

## Non-goals
The toolkit walker `tap_context` normalization (separate MINOR, optional); shape C if R0 picks A1; the `SortedMultiA` stale-message reword (separate tracked item, cheap rider if touching the file).
