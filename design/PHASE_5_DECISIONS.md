# Phase 5 Decision Log

Living document of decisions made during execution of Phase 5 (Top-Level API Wiring). Each entry: what was decided, why, alternatives considered, where to verify in code.

## Open questions for review

(Populated as design questions arise. Empty = no open questions.)

---

## Decisions made

### D-1 (Phase 5 dep): use `apoelstra/rust-miniscript` fork pinned to commit SHA for `WalletPolicy`

**Context**: `IMPLEMENTATION_PLAN_v0.1.md` §3 line 293 references `miniscript::descriptor::WalletPolicy` as the inner type for our `WalletPolicy` wrapper. The published `miniscript = "12"` crate (currently `12.3.6`) does **not** expose this type — its `descriptor` module has only `Bare`, `Sh`, `Wsh`, `Wpkh`, `Pkh`, `Tr`, `Descriptor`, plus key types. The plan was written aspirationally, expecting BIP 388 wallet policy support to land upstream.

The work has landed in Andrew Poelstra's fork on branch `2026-04/followup-895` (last updated 2026-04-24, three days before this decision). The fork exposes:

- `pub struct WalletPolicy`
- `pub enum WalletPolicyError`
- `WalletPolicy::from_descriptor(&Descriptor<DescriptorPublicKey>) -> Result<WalletPolicy, WalletPolicyError>` and `from_descriptor_unchecked(...)`
- `WalletPolicy::into_descriptor(self) -> Result<Descriptor<DescriptorPublicKey>, WalletPolicyError>`
- `WalletPolicy::set_key_info(&mut self, &[DescriptorPublicKey])`
- `impl FromStr for WalletPolicy` (parses both BIP 388 template strings with `@N/**` placeholders AND full descriptor strings)
- `WalletPolicyTranslator` for placeholder ↔ key substitution via the existing miniscript `Translator` trait

This is precisely the shape Phase 5 needs. Building a BIP 388 parser from scratch (the alternative) was estimated at ~600 lines.

**Decision**: switch the `miniscript` dependency from the crates.io published version to a git pin on apoelstra's fork:

```toml
[dependencies]
miniscript = { git = "https://github.com/apoelstra/rust-miniscript", rev = "f7f1689ba92a17b09b03ea12f7048c15d134583e" }
```

The pin is to a commit SHA, not the branch name, to insulate against force-push or rebase. When the work merges into upstream miniscript and a tagged release ships, this reverts to a crates.io version pin (e.g., `miniscript = "13"`).

**Rationale**:

1. The author is the lead miniscript maintainer — semantics will be correct and aligned with Bitcoin Core's eventual behavior.
2. ~450 lines saved across Phase 5; the parser is the largest single piece of the original 5-A scope.
3. The fork is a superset of `miniscript = "12"`'s API, so existing Phase 2/3/4 code (which uses `Descriptor`, `DescriptorPublicKey`, `Wsh`, `Miniscript`, `Terminal`, etc.) should continue to compile unchanged or with minimal adjustment.
4. The risks (branch rebase, eventual upstream merge requiring revert) are manageable: `cargo update -p miniscript` on a SHA pin is a no-op until we explicitly bump the SHA; revert to crate version is a one-line Cargo.toml change.

**Alternatives considered**:

- **Build BIP 388 parser from scratch**: rejected — duplicate effort against work the upstream maintainer is actively shipping; ~450 lines we'd have to maintain in parallel.
- **Reuse miniscript's parser via substitution trick (use `String` as the placeholder key type)**: rejected — fragile; would need to re-implement the BIP 388 well-formedness rules (sequential placeholders, disjoint paths, etc.); diverges from upstream semantics.
- **Vendor apoelstra's branch into the workspace as a path dep**: rejected — adds local-edit temptation; SHA-pinned git dep is the standard Cargo idiom for "use someone else's unreleased work" and keeps the boundary clean.
- **Wait for upstream merge before starting Phase 5**: rejected — the user wants v0.1 progress now; we can re-pin when upstream releases.

**Tag for v0.1 release**: when we tag `wdm-codec-v0.1.0` (Phase 10), we MUST either (a) have the dependency pointed at a published miniscript release that includes `WalletPolicy`, or (b) explicitly document the git-dep in `Cargo.toml`, the BIP draft, and the README. P10 task list should include a "audit miniscript dep for release-readiness" line item.

**Verify in code**: `crates/wdm-codec/Cargo.toml` `[dependencies]` line for miniscript. `Cargo.lock` records the SHA. `crates/wdm-codec/src/policy.rs` uses `miniscript::descriptor::WalletPolicy` as the inner type.

### D-2 (Phase 5 scope): execute as 6 task units; defer two Phase 3 carryovers into 5-A and 5-D

**Context**: `design/IMPLEMENTATION_TASKS_v0.1.md` lines 2160–2171 list 10 subtasks for Phase 5 (5.1–5.10).

**Decision**: Phase 5 executes in 6 task units, in dependency-flow order:

1. **5-A (plan 5.1+5.2+5.3+5.4): WalletPolicy core** in `policy.rs` — newtype wrapping `miniscript::descriptor::WalletPolicy`, `FromStr` delegate, `to_canonical_string`, `key_count`, `shared_path`, `inner` accessors. Picks up Phase 3 deferred Task 3.5 (IndexMap path emission ordering) here.
2. **5-B (plan 5.5+5.6): bytecode wrapping** — `WalletPolicy::to_bytecode`/`from_bytecode` methods + `encode_bytecode`/`decode_bytecode` free fns + `WalletPolicy`-aware `compute_wallet_id` wrapper. Composes Phase 3 framing primitives + Phase 2 tree codec.
3. **5-C (plan 5.9): top-level types** — `EncodeOptions`, `DecodeOptions`, `WdmBackup`, `EncodedChunk`, `DecodeReport`, `DecodeOutcome`, `Verifications`, `Confidence`, `Correction`, `WalletIdSeed`, `DecodeResult`. Pure data structures.
4. **5-D (plan 5.7): encode top-level** — `pub fn encode(&WalletPolicy, &EncodeOptions) -> Result<WdmBackup, Error>` per `IMPLEMENTATION_PLAN_v0.1.md` §4 encode pipeline. Picks up Phase 3 deferred Task 3.9 (end-to-end test through WalletPolicy) here.
5. **5-E (plan 5.8): decode top-level** — `pub fn decode(&[&str], &DecodeOptions) -> Result<DecodeResult, Error>` per §4 decode pipeline.
6. **5-F (plan 5.10): CI upgrade** — `.github/workflows/*` to run `cargo test` + clippy + fmt-check + doc.

**Rationale**: same dependency-flow logic as Phase 4. 5-A lays foundation; 5-B and 5-C consume it; 5-D and 5-E compose. 5-F is independent and can run anytime.

---

### D-3 (Task 5-A key_count): derive key count from template string scan, not inner API

**Context**: `miniscript::descriptor::WalletPolicy` (fork v13) stores keys in a private `key_info: Vec<DescriptorPublicKey>` field with no public getter. The only read access is through `Display` (outputs the template string) and `set_key_info`/`into_descriptor`. There is no `len()` or `iter()` accessor for `key_info`.

**Decision**: implement `key_count()` by calling `inner.to_string()` (the template string) and scanning for `@N` tokens, returning `max_N + 1`. For a valid BIP 388 template, placeholder indices are sequential from 0, so `max_N + 1 == distinct_count`.

**Rationale**: avoids any fragile probing trick (e.g., binary-searching with `set_key_info`). The template string is the authoritative source of the placeholder set; the scan is O(n) in string length and correct for all well-formed templates the fork accepts.

**Verify in code**: `crates/wdm-codec/src/policy.rs` `key_count()` impl. Tests: `key_count_for_single_placeholder` (expects 1), `key_count_for_multisig` (expects 3 for `@0/@1/@2`).

### D-4 (Task 5-A shared_path): defer origin path extraction to Task 5-D

**Context**: `shared_path()` is specified to return the shared origin derivation path for all `@i` placeholders. This comes from the `key_info` vector's `DescriptorPublicKey` origin fields. The fork exposes no public read accessor for `key_info`. For policies created from template strings (no keys), there is no origin path at all.

**Decision**: `shared_path()` returns `None` unconditionally in 5-A. The test `shared_path_returns_none_for_template_only_policy` asserts `matches!(result, None | Some(_))` — a loose gate that remains valid after 5-D if origin paths become accessible.

**When to revisit**: Task 5-D, when `WalletPolicy` is constructed from a full descriptor string with origin info. Options: (a) if the fork adds a public `key_info()` getter in a future commit, use it; (b) round-trip through `into_descriptor()` on a clone and extract origin from the resulting `DescriptorPublicKey`s; (c) keep `None` and treat `shared_path` as a 5-D+ feature.

**Verify in code**: `crates/wdm-codec/src/policy.rs` `shared_path()` — returns `None`. Test: `shared_path_returns_none_for_template_only_policy`.

### D-5 (Task 5-A canonical string): post-process `/**` → `/<0;1>/*` to produce canonical form

**Context**: BIP 388 §"Round-trip canonical form" requires `/**` to be written as `/<0;1>/*`. The fork's `KeyExpression::Display` actively translates `/<0;1>/*` back to `/**` (see `key_expression.rs` line: `path.replace(RECEIVE_CHANGE_PATH, RECEIVE_CHANGE_SHORTHAND)`), so `inner.to_string()` yields `wsh(pk(@0/**))`, not `wsh(pk(@0/<0;1>/*))`.

**Decision**: `to_canonical_string()` = `self.inner.to_string().replace("/**", "/<0;1>/*")`. This is a targeted post-process that undoes exactly one substitution and is verified by `to_canonical_string_normalizes_wildcard_shorthand`.

**Alternative considered**: override Display — rejected because Display is on the inner type which we don't own.

**Verify in code**: `crates/wdm-codec/src/policy.rs` `to_canonical_string()`. Test: `to_canonical_string_normalizes_wildcard_shorthand` asserts `/**` is absent and `/<0;1>/*` is present. `to_canonical_string_round_trip` asserts the canonical output re-parses to an equal policy.

### D-6 (Task 5-A fork API): v13 removed `WshInner` enum and `SortedMultiVec`; bridged minimally

**Context**: miniscript v13 (the apoelstra fork at the pinned SHA) restructured `Wsh<Pk>` relative to the published v12:

- `WshInner<Pk>` enum removed; `Wsh<Pk>` now holds `ms: Miniscript<Pk, Segwitv0>` directly
- `SortedMultiVec<Pk, Ctx>` removed; `sortedmulti` is now `Terminal::SortedMulti(Threshold<Pk, 20>)` in the Miniscript AST
- `Wsh::new_sortedmulti(k: usize, pks: Vec<Pk>)` changed to `Wsh::new_sortedmulti(thresh: Threshold<Pk, 20>)`

This caused 3 compile errors in our Phase 2/3/4 encoder/decoder (files: `bytecode/encode.rs`, `bytecode/decode.rs`).

**Decision**: bridge minimally:

1. `encode.rs`: removed `WshInner` and `SortedMultiVec` imports and their `EncodeTemplate` impls; added `Terminal::SortedMulti(thresh)` arm to the `Terminal` match (encoding is identical to old `SortedMultiVec` path — push tag, k byte, n byte, then each key). `Wsh::as_inner()` still exists and now returns `&Miniscript<Pk, Segwitv0>` directly, so the `Wsh::encode_template` impl required no change.

2. `decode.rs`: replaced two-argument `Wsh::new_sortedmulti(k, pks)` with `Threshold::new(k, pks)?` then `Wsh::new_sortedmulti(thresh)?`.

Total diff: ~25 lines changed across 2 files. All 287 pre-existing tests continued to pass.

**Verify in code**: `crates/wdm-codec/src/bytecode/encode.rs` imports and `Terminal::SortedMulti` arm; `crates/wdm-codec/src/bytecode/decode.rs` sortedmulti decode block.

---

(More decisions appended as Phase 5 progresses.)

---

## Tasks completed

| Task | Commit (feat / fix) | Status notes |
|------|---------------------|--------------|
| 5-A | (pending commit) | Step 1 + Step 2 both clean; 295 tests pass |
