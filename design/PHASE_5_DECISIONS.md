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

### D-7 (Task 5-B approach): Approach B (dummy-key materialization) for bytecode encoding

**Context**: Task 5-B must produce a `Descriptor<DescriptorPublicKey>` from a `WalletPolicy` to call `encode_template`. Two approaches were considered:
- **Approach A (template AST)**: Walk the fork's `WalletPolicy::template` field directly. The field is private (`template: Descriptor<KeyExpression>`), and there is no public method exposing the template AST.
- **Approach B (dummy keys)**: Set dummy `DescriptorPublicKey` values via `set_key_info()`, call `into_descriptor()`, and encode the resulting descriptor.

**Decision**: Approach B. The fork does not expose the `template` field or any equivalent method. Approach A would require a fork modification.

**Dummy key details**:
- 8 hardcoded entries from the fork's own test fixtures (proven valid xpubs).
- Each entry has `/<0;1>/*` derivation suffix (required because `from_descriptor()` calls `pk.wildcard()` when reconstructing the template; `/**` is not valid in `DescriptorPublicKey::from_str`).
- Fingerprints and origin paths differ per entry so that `DescriptorPublicKey`'s `PartialEq` treats them as distinct (needed for `HashMap<DescriptorPublicKey, u8>` in `encode_template`).
- 8 entries covers single-sig, 2-of-3, 3-of-5, 5-of-8 multisig. Extend the table if more are needed.

**Verify in code**: `crates/wdm-codec/src/policy.rs` `DUMMY_KEYS` constant and `dummy_keys()` function.

### D-8 (Task 5-B from_bytecode): two-pass scan to determine key_count before decode

**Context**: `from_bytecode` needs to call `decode_template(tree_bytes, &keys)` but doesn't know `key_count` until it's parsed the bytecode. Options:
1. Supply maximum dummy keys (8); let unused entries be ignored.
2. Pre-scan the tree bytes for `Tag::Placeholder` (0x32) to find `max_index`, then decode with `max_index + 1` dummies.

**Decision**: Option 2 (pre-scan). Supplying 8 dummy keys when a policy has only 1 key would cause `from_descriptor()` to produce a policy with 8 keys in `key_info`, all but the first unused. The resulting `WalletPolicy` would have mismatched key count. Option 2 gives an accurate count with minimal code (simple linear scan for `0x32` bytes — no full decode needed).

**Verify in code**: `crates/wdm-codec/src/policy.rs` `count_placeholder_indices()` function.

### D-9 (Task 5-B naming): `compute_wallet_id_for_policy` instead of overloaded `compute_wallet_id`

**Context**: `IMPLEMENTATION_PLAN_v0.1.md` line 276 specifies the same `compute_wallet_id` name for both the bytes-level and the policy-level variants, written as if Rust supports overloading. Rust does not have function overloading.

**Decision**: Name the policy-aware wrapper `compute_wallet_id_for_policy`. The existing `compute_wallet_id(&[u8])` is unchanged. Both are re-exported from `lib.rs`.

**Verify in code**: `crates/wdm-codec/src/wallet_id.rs` `compute_wallet_id_for_policy`; `crates/wdm-codec/src/lib.rs` re-export.

### D-10 (Task 5-B shared_path): materialization via into_descriptor()

**Context**: 5-A deferred `shared_path()` to return `None` always (D-4). For 5-B, the encode path needs the shared path to write the path declaration. Two routes existed: (a) keep returning None and default to a hardcoded path; (b) materialize the descriptor by cloning and calling `into_descriptor()`, then extract origin from the first key.

**Decision**: Route (b): `shared_path()` now clones the inner policy and calls `into_descriptor()`. If materialization fails (template-only policy with no key_info), returns `None`. For policies with real keys, returns the first key's origin derivation path. If None, `to_bytecode()` falls back to BIP 84 mainnet (`m/84'/0'/0'`) as the default path.

**Test update**: 5-A's `shared_path_returns_none_for_template_only_policy` now uses `is_none()` (the tautological `matches!(result, None | Some(_))` from D-4 is fixed). New test `shared_path_returns_some_for_policy_with_keys` asserts `Some(m/84'/0'/0')` for a policy parsed from a full descriptor string.

**Verify in code**: `crates/wdm-codec/src/policy.rs` `shared_path()` impl and tests.

---

(More decisions appended as Phase 5 progresses.)

---

## Tasks completed

| Task | Commit (feat / fix) | Status notes |
|------|---------------------|--------------|
| 5-A WalletPolicy core | `56124c3` (no fix needed; approve-with-followup x2) | Step 1: dep switch + 3 trivial v13 bridges (`WshInner` removal, `SortedMultiVec` → `Terminal::SortedMulti`, `Wsh::new_sortedmulti` signature). Step 2: `WalletPolicy` newtype, `FromStr` → `Error::PolicyParse`, `to_canonical_string` post-processes `/**` → `/<0;1>/*` (D-5), `key_count` scans template for `@N` (D-3), `shared_path` returns `None` (D-4 — defer to 5-D), `inner` is `#[doc(hidden)]`; 8 new tests (287 → 295). Code-review minor follow-ups (defer to 5-B/5-D opportunistic): (a) `from_inner` could be `pub(crate)` not `pub` (both consumers in-crate); (b) test #7 `matches!(.., None | Some(_))` is tautological — switch to `is_none()` once D-4 is known stable; (c) `(m + 1) as usize` cast cosmetic — prefer `m as usize + 1`; (d) `key_count` could use `usize` throughout; (e) add comment in `key_count` rustdoc that `inner.to_string()` writes only the template (no `@` outside `@N` placeholders). |
| 5-B bytecode wrapping | (pending commit) | `WalletPolicy::to_bytecode` + `from_bytecode` + `encode_bytecode`/`decode_bytecode` free fns + `compute_wallet_id_for_policy`. Approach B (dummy keys). `decode_declaration` `#[allow(dead_code)]` removed. `shared_path()` now extracts real path (D-10). 12 new tests (295 → 307). |
