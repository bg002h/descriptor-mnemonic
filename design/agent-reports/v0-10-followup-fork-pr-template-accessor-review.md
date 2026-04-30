# Upstream PR Review — `descriptor: expose WalletPolicy template + key_info accessors`

## 1. Verdict

**INLINE-FIX** — the patch achieves its stated goal and tests pass, but it
introduces two `missing_docs` warnings that under `#![warn(missing_docs)]` +
`-D warnings` (used by upstream `cargo rbmt lint`) become hard CI failures.
The commit message also contains a factually inaccurate parenthetical that
will mislead a maintainer reviewing the rationale. Both are easy fixes; the
substantive API design is fine.

## 2. Scope reviewed

- **Commit:** `765aa14 descriptor: expose WalletPolicy template + key_info accessors`
- **Worktree:** `/scratch/code/shibboleth/rust-miniscript-template-accessor`
- **Base branch:** `origin/2026-04/followup-895` (correct base — wallet_policy/ is not on master)
- **Files:**
  - `src/descriptor/mod.rs` — re-export line widened to include `KeyExpression`, `KeyIndex`
  - `src/descriptor/wallet_policy/mod.rs` — `pub use` of nested module symbols + two accessors + two unit tests
- **Commands run (cwd = worktree):**
  - `cargo build` — succeeds, but with **2 new `missing_docs` warnings** vs. baseline (`HEAD~1`) which builds clean
  - `cargo test wallet_policy` — 6 passed, 0 failed
  - `cargo fmt --check` — clean
  - `cargo clippy --all-targets -- -D warnings` (via stable toolchain) — **fails with 2 errors** caused by this PR (the `missing_docs` warnings get promoted)
  - Verified `iter_pk()` parse-vs-lex order claim by writing a throwaway test in `tests/iter_pk_order_test.rs` (since deleted) — confirmed `iter_pk()` on `Descriptor<DescriptorPublicKey>` returns keys in **parse order, NOT lex byte order** for `sortedmulti(...)`

## 3. Findings

### 3.1 — Two `missing_docs` warnings introduced (CI-blocking under `-D warnings`)

- **Severity:** Critical / blocking
- **Disposition:** Must fix before opening PR.
- **Description:** The crate carries `#![warn(missing_docs)]` (`src/lib.rs:80`).
  Pre-patch (`HEAD~1`), `cargo build` is clean. Post-patch:

  ```
  warning: missing documentation for a struct
    --> src/descriptor/wallet_policy/key_expression.rs:27:1
     |
  27 | pub struct KeyIndex(pub u32);

  warning: missing documentation for a method
    --> src/descriptor/wallet_policy/key_expression.rs:30:5
     |
  30 |     pub fn is_disjoint(&self, other: &KeyExpression) -> bool {
  ```

  These were latent because `KeyExpression` and `KeyIndex` were module-private
  re-exports inside `wallet_policy`. Promoting them to `pub use` makes them
  reachable from the crate root, and the lint catches them.
  `KeyIndex.0` (a `pub u32` tuple field), `KeyExpression.{index, derivation_paths, wildcard}` (all `pub` per `key_expression.rs:18-23`) compile clean today only because `KeyExpression` itself has a doc comment, which by lint convention covers its public fields. But `KeyIndex` does not, and `is_disjoint` does not.
  Upstream `cargo rbmt lint` runs nightly clippy with `-D warnings` (per `.github/workflows/rust.yml:64-78`) — these warnings will be hard failures.
- **Suggested fix:** Add doc comments to `KeyIndex` and `is_disjoint` in
  `src/descriptor/wallet_policy/key_expression.rs`:

  ```rust
  /// The numeric index of a BIP-388 key placeholder (the `N` in `@N`).
  #[derive(Debug, Clone, Copy, Hash, PartialOrd, Ord, PartialEq, Eq)]
  pub struct KeyIndex(pub u32);
  ```

  ```rust
  impl KeyExpression {
      /// Returns `true` if the multipath derivation suffixes of `self` and
      /// `other` share no concrete child-derivation step.
      pub fn is_disjoint(&self, other: &KeyExpression) -> bool {
  ```

  Probably also worth touching up `KeyExpression`'s public fields with `///`
  doc comments while we're here (current ones are short single-liners, which
  is fine, but the BIP-388 vocabulary "KI" / "KP" might confuse a non-BIP-388
  reader), but that is a style nit, not a CI gate.

### 3.2 — Commit message claim about `iter_pk()` order is factually wrong

- **Severity:** Important
- **Disposition:** Reword before opening PR.
- **Description:** The commit message says:

  > `iter_pk()` on the materialized descriptor would traverse in AST order
  > (lex-sorted-by-pubkey-bytes for sortedmulti), not placeholder-index order.

  The parenthetical is incorrect for the rust-miniscript types. `iter_pk()`
  for `Terminal::SortedMulti` reads `thresh.data().get(n)`
  (`src/miniscript/iter.rs:105`), which is **parse-time order**, not BIP67
  byte order. The BIP67 byte-sort (`into_sorted_bip67()`) is applied only at
  Bitcoin script encoding time in
  `src/miniscript/astelem.rs:155-167`. I verified this empirically by
  parsing `wsh(sortedmulti(2, KEY_F.../<0;1>/*, KEY_E.../<0;1>/*))` and the
  same descriptor with the keys swapped — `iter_pk()` returned different
  first keys for each, proving parse order.
- **Why it matters for the motivation:** The real reason `iter_pk()` on the
  materialized `Descriptor<DescriptorPublicKey>` is unsuitable for placeholder-
  indexed traversal is that **multipath placeholders allow the same `@N` to
  appear at multiple AST positions** (e.g.
  `sh(multi(1,@0/**,@0/<2;3>/*))` — the very test vector at
  `wallet_policy/mod.rs:324-326`). After `into_descriptor()`, both AST
  positions hold the same `DescriptorPublicKey` (with different multipath
  branches selected at derivation time), and from the materialized
  descriptor alone you cannot recover that they were `@0` and `@0` rather
  than `@0` and `@1`. The template accessor preserves the `@N` labels via
  `KeyExpression::index`. **That is the real motivation; the lex-byte-sort
  framing is a red herring.**
- **Suggested fix:** Replace the offending sentence with something like:

  > … for example, encoders that emit per-`@N` divergent origin paths in
  > BIP-388 multipath templates, where the same key placeholder `@N` may
  > appear at multiple AST positions (e.g.
  > `sh(multi(1,@0/**,@0/<2;3>/*))`). Once the wallet policy is materialized
  > to `Descriptor<DescriptorPublicKey>`, the `@N` labels are erased — the
  > template accessor preserves them via `KeyExpression::index`.

### 3.3 — Tests do not exercise the AST≠placeholder-index mismatch case

- **Severity:** Important
- **Disposition:** Add one more test before opening PR.
- **Description:** Both new tests use templates where AST position equals
  placeholder index (`(0,0), (1,1)` etc.). They would still pass even if the
  accessor returned a buggy template that lost the `index` field, because
  the only way `template().iter_pk().nth(i).index.0 != i` is when the same
  `@N` appears at multiple AST positions. The wallet-policy validator
  (`wallet_policy/mod.rs:151-167`) actually forbids out-of-order placeholders
  entirely — the only legal way for AST position to differ from placeholder
  index is **multipath-on-the-same-`@N`** (the validator's
  `prev.index.0 == curr.index.0` arm).
- **Suggested fix:** Add a test using the existing in-tree test-vector
  `sh(multi(1,@0/**,@0/<2;3>/*))`. Expected pairs: `(0, 0), (1, 0)` —
  i.e. AST positions 0 and 1 both have placeholder index 0. Sketch:

  ```rust
  #[test]
  fn template_accessor_distinguishes_ast_position_from_placeholder_index() {
      // Multipath with shared @0 placeholder at two AST positions —
      // the only case where AST position can differ from placeholder index.
      let policy =
          WalletPolicy::from_str("sh(multi(1,@0/**,@0/<2;3>/*))").expect("parse template");
      let pairs: Vec<(usize, u32)> = policy
          .template()
          .iter_pk()
          .enumerate()
          .map(|(ast_pos, ke)| (ast_pos, ke.index.0))
          .collect();
      assert_eq!(pairs, vec![(0, 0), (1, 0)]);
  }
  ```

  This is the test that justifies why the accessor is needed in the first
  place; without it, a maintainer can reasonably ask "why not just use
  `iter_pk()` on the materialized descriptor?" and the existing tests don't
  give them an answer.

### 3.4 — `key_info()` is broader than the stated motivation requires

- **Severity:** Minor / Important (pick one based on upstream taste)
- **Disposition:** Defensible to keep; could split into a separate commit.
- **Description:** The stated motivation only requires `template()` —
  callers can already get the concrete keys via `into_descriptor()` (which
  consumes `self`) or by translating `template().iter_pk().map(|ke|
  &policy.key_info()[ast_pos])` ... wait, that requires `key_info()`. So
  actually, `key_info()` is needed **if** the caller wants to walk
  template + concrete-key in lockstep without consuming the policy. That
  is the natural pattern. Symmetric with the existing `set_key_info()`
  setter, which makes a strong case to ship the getter. **My recommendation:
  keep both, leave as a single commit.** But be aware a strict upstream
  maintainer may ask for either:
  (a) splitting into two commits ("expose template", "expose key_info"), or
  (b) dropping `key_info()` since `into_descriptor()` already exposes the
      keys and the caller should re-parse if they need both views.
- **Suggested action:** Be ready to defend "kept symmetric with
  `set_key_info()`" if the maintainer raises it. No code change.

### 3.5 — `pub use` exposes more of `KeyExpression` than callers strictly need

- **Severity:** Minor
- **Disposition:** Acceptable as-is, document the rationale if asked.
- **Description:** `KeyExpression` has three `pub` fields:
  `index: KeyIndex`, `derivation_paths: DerivPaths`, `wildcard: Wildcard`.
  Downstream callers only need `index` for the stated motivation. Exposing
  the full struct means `derivation_paths` and `wildcard` become part of the
  public API surface forever. Alternative: add a narrow accessor like
  `KeyExpression::placeholder_index() -> u32` that returns just `self.index.0`,
  and keep the struct private. But `KeyExpression` is the `Pk` parameter on
  `Descriptor<KeyExpression>`, so the type itself must be `pub` (or
  `template()` cannot be called from outside the crate). The fields are a
  fair trade — they are simple data, the struct already implements
  `MiniscriptKey`, and exposing the multipath suffix and wildcard is
  arguably useful (md-codec specifically wants to know if the suffix is
  `/<0;1>/*` or `/<2;3>/*`!). I'd leave it. Consider adding a brief
  rustdoc note on the struct that "this is a public type but its fields
  are stable surface" so future refactors don't accidentally break it.

### 3.6 — `template()` doc references `Self::into_descriptor` but not `set_key_info`

- **Severity:** Minor (nit)
- **Disposition:** Optional polish.
- **Description:** The `template()` rustdoc points readers to
  `Self::into_descriptor` for "the resolved descriptor with concrete keys".
  That is correct. The `key_info()` rustdoc references the AST positions
  but does not mention that callers can populate it via
  `Self::set_key_info`. A `[Self::set_key_info]` link in `key_info()`'s doc
  would close the loop.

### 3.7 — Methods are not `#[inline]`

- **Severity:** Minor
- **Disposition:** Optional.
- **Description:** They are one-liner trivial getters. `#[inline]` would
  match the pattern of trivial accessors in many other crates, but
  rust-miniscript does not consistently use `#[inline]` on getters
  (spot-check: `WalletPolicy::set_key_info` is not inline; the existing
  `Display::fmt` impls are not). I would not add it — be consistent with
  the file. Skip.

### 3.8 — No doctest on the new methods

- **Severity:** Minor
- **Disposition:** Optional.
- **Description:** The struct-level doc on `WalletPolicy` already has a
  comprehensive doctest (`wallet_policy/mod.rs:14-39`). Method-level
  doctests are not required by file convention (`from_descriptor_unchecked`,
  `from_descriptor`, `into_descriptor`, `set_key_info` all lack them).
  I would not add one.

## 4. API design assessment

The shapes are correct.

- `template(&self) -> &Descriptor<KeyExpression>` — returns by reference,
  zero-copy, mirrors how `Descriptor` is normally observed. Forces no
  allocation; caller calls `.iter_pk()` to walk. Right shape.
- `key_info(&self) -> &[DescriptorPublicKey]` — slice reference, also
  zero-copy. Mirrors `set_key_info(&mut self, keys: &[DescriptorPublicKey])`.
  Right shape.
- Naming: `template` and `key_info` match the field names and the BIP-388
  vocabulary. Symmetric with `set_key_info`. Consistent with
  `from_descriptor`, `into_descriptor` (those are conversions, not
  accessors, so different naming category — irrelevant). Names are good.
- Returning `&KeyIndex` collections instead would force allocation and
  complicate the contract — `iter_pk()` is the canonical traversal and the
  AST-position correlation is a feature, not a bug. The chosen API is
  correct.

The single substantive design question is whether to expose `KeyExpression`
+ `KeyIndex` as crate-root re-exports. The answer is yes, because
`Descriptor<KeyExpression>` is unnamable from outside the crate without it.

## 5. Test coverage assessment

- ✅ `template_accessor_yields_keyexpressions_with_placeholder_indices`
  covers: template-only parse, indices are read, key_info empty.
- ✅ `key_info_accessor_yields_concrete_keys_in_ast_order` covers:
  full-parse population, key_info length, AST-position iteration.
- ❌ **No test exercises the AST≠placeholder-index mismatch case**, which
  is the exact scenario the new accessor is needed for. The validator
  forbids out-of-order placeholders, so the only way they differ is
  multipath-on-`@0` (e.g. `sh(multi(1,@0/**,@0/<2;3>/*))`). Without this
  test, the patch's tests would still pass even on a hypothetical buggy
  implementation that returned `KeyExpression { index: KeyIndex(ast_pos), .. }`
  (i.e. erased the placeholder mapping). See finding 3.3 for fix.

## 6. Upstream-friendliness assessment

- **Patch is minimal and additive.** No public-API removals, no signature
  changes. The two re-exports are purely additive. This is exactly the
  kind of small, defensible accessor patch that lands cleanly upstream.
- **Commit is single-purpose.** Title is clear. Body explains motivation,
  surface, and the use case. The factual error in the parenthetical
  (finding 3.2) is the only thing that would make a maintainer pause.
- **No breaking-change risk.** Confirmed by reading the diff.
- **Base-branch choice.** Cut from `origin/2026-04/followup-895`, which is
  correct (the `wallet_policy/` module has not landed on master yet). The
  PR will need to be re-targeted or rebased depending on whether 895
  lands first; that's a routine operational concern, not a review issue.
- **CI:** As-is, `cargo rbmt lint` will fail (finding 3.1). Fix required
  before pushing.

## 7. Recommended action

**Inline-fix and re-review (lightweight)**, then push. Specifically:

1. Add doc comments to `KeyIndex` and `KeyExpression::is_disjoint` in
   `src/descriptor/wallet_policy/key_expression.rs` — fixes finding 3.1.
2. Reword the commit-message paragraph that claims sortedmulti
   `iter_pk()` is lex-byte-sorted — fixes finding 3.2.
3. Add the `template_accessor_distinguishes_ast_position_from_placeholder_index`
   test sketched in finding 3.3 — addresses the strongest substantive gap.
4. Re-run `cargo build`, `cargo test wallet_policy`, and `cargo clippy
   --all-targets -- -D warnings`. All three should be clean.
5. Push and open the PR.

After steps 1–3, this is **READY-TO-PR**. The remaining minor items
(3.4–3.8) are stylistic and can be addressed in PR review if a maintainer
asks; none warrant blocking the initial submission.

## Pass-2 verification

Re-ran on amended commit `a95a61a` (was `765aa14`). Stats: 3 files,
+77/-2 (now also touches `wallet_policy/key_expression.rs`).

### Verdict: **READY-TO-PR**

### Fix 1 — `missing_docs` cleared

Doc comments added in `key_expression.rs:26` (`KeyIndex`) and
`key_expression.rs:31-32` (`is_disjoint`). Wording is accurate and
non-trivial:
- `KeyIndex`: "The numeric index of a BIP-388 key placeholder (the `N`
  in `@N`)." — correctly cites BIP-388 vocabulary; non-trivial.
- `is_disjoint`: "Returns `true` if the multipath derivation suffixes of
  `self` and `other` share no concrete child-derivation step." —
  accurate per the impl (computes set-disjointness on flattened child
  numbers across multipath branches).

`cargo build` produces zero warnings. Lint gate (`cargo +nightly clippy
--all-targets -- -D warnings`) passes clean.

### Fix 2 — Commit message corrected

The lex-sorted-by-pubkey-bytes claim is gone. The new motivation
paragraph correctly cites multipath-shared-`@N`:

> The motivating case is multipath-shared `@N` placeholders — e.g.
> `sh(multi(1,@0/**,@0/<2;3>/*))`, where the same placeholder appears
> at two distinct AST positions. After `into_descriptor()` the
> placeholder labels are erased and the materialized descriptor
> surfaces two separate keys; the template preserves placeholder
> identity via the public `KeyExpression::index` field.

The example `sh(multi(1,@0/**,@0/<2;3>/*))` is mentioned explicitly,
matching the in-tree test vector. This is a straight upgrade from the
prior framing.

### Fix 3 — AST-vs-placeholder test added

`template_accessor_distinguishes_ast_position_from_placeholder_index`
exists at `wallet_policy/mod.rs:413-429`. It parses
`sh(multi(1,@0/**,@0/<2;3>/*))` and asserts
`pairs == vec![(0, 0), (1, 0)]`. A buggy impl returning
`KeyIndex(ast_pos)` would yield `[(0, 0), (1, 1)]`, failing on the
second tuple — confirms the test discriminates the bug class. Test
runs and passes.

### Gates

All four gates clean:

- `cargo build 2>&1 | grep -E '^(warning|error)'` — empty (clean)
- `cargo test wallet_policy` — 7 passed, 0 failed (was 6 pre-fix; the
  new `template_accessor_distinguishes_ast_position_from_placeholder_index`
  test is the +1)
- `cargo +nightly fmt --check` — exit 0
- `cargo +nightly clippy --all-targets -- -D warnings` — exit 0

### Meta-question: was the original Tier-2-stub deferral justified?

The implementer's specific rationale for deferring Tier 2 — that
`iter_pk()` on the materialized `Descriptor<DescriptorPublicKey>` would
yield lex-sorted-by-pubkey-bytes order for `sortedmulti` — was
**incorrect**. `iter_pk()` walks parse-time order via
`thresh.data().get(n)` (`miniscript/iter.rs:105`); BIP67 byte-sort is
applied only at script-encoding time
(`miniscript/astelem.rs:155-167`).

However, the **fork accessor is still useful** independent of that
mistake: the real distinguishing case is multipath-shared-`@N` (e.g.
`sh(multi(1,@0/**,@0/<2;3>/*))`), where the materialized descriptor
surfaces two `DescriptorPublicKey` entries that are equal-or-multipath-
sibling but cannot be reliably mapped back to "they were both `@0`"
without inspecting the original wallet-policy template. The new
`template()` accessor preserves that placeholder identity directly via
`KeyExpression::index`, which is the cleanest in-tree solution. A
purely-`iter_pk()`-based approach would have to heuristically detect
multipath-pair adjacency and assume identity-of-`@N`, which is fragile
(BIP-388 forbids multiple `@N` referring to the same key, but the
identity-of-multipath-pair-bytes invariant is not enforced anywhere
the encoder can rely on it). Tier 2 in-tree using `template()` is the
right path.

### Recommended next action

Push the worktree branch and open the upstream PR against
`apoelstra/rust-miniscript`. (Reminder: re-target or rebase as the
underlying `2026-04/followup-895` base branch evolves upstream — that
is a routine operational concern, not a review-blocking issue.)
