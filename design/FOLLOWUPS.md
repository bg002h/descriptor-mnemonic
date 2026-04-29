# Follow-up tracker

Single source of truth for items that surfaced during a review or implementation pass but were not fixed in the same commit. Replaces the previous practice of scattering follow-ups across decision docs, commit messages, inline TODOs, and conversation history.

## How to use this file

**Format for each entry:**

```markdown
### `<short-id>` — <one-line title>

- **Surfaced:** Phase X.Y review of commit <SHA>, or "inline TODO at <file>:<line>"
- **Where:** `<file>:<line>` or "design — Cargo.toml `[patch]` block"
- **What:** 1–3 sentences describing the gap or improvement opportunity
- **Why deferred:** the reason it didn't ship in the original commit
- **Status:** `open` | `resolved <COMMIT>` | `wont-fix — <one-line reason>`
- **Tier:** `v0.1-blocker` | `v0.1-nice-to-have` | `v0.2` | `v1+` | `external`
```

The `<short-id>` is a stable handle (e.g., `5d-from-impl`, `5e-checksum-correction-fallback`, `p10-miniscript-dep-audit`). Reference this id from commit messages when you close the item: `closes FOLLOWUPS.md 5d-from-impl`.

## Conventions for adding items

**During a review subagent run:** the reviewer should append to this file (with a small entry per minor item) and reference it in their report. Reviewers in parallel batches must not write to this file simultaneously — the controller appends afterwards from the consolidated reports.

**During an implementer subagent run:** if the implementer notices a side concern they explicitly chose not to fix in their commit, they append an entry here in the same commit. This keeps the deferral visible.

**During controller (main-thread) work:** when wrapping a task, the controller verifies all minor items from that task's reviews are either resolved or recorded here.

**Persisting agent reports to disk (durable audit trail):** in addition to FOLLOWUPS.md, every implementer or reviewer subagent that produces a commit MUST also save its full final report (the verbatim text the agent returns to the controller) to `design/agent-reports/<filename>.md` per the file-naming convention in `design/agent-reports/README.md`. This protects against the controller losing minor items between conversation sessions: the raw report is durable on disk, and the post-batch FOLLOWUPS.md aggregation can re-read agent reports if the controller's working memory missed something. For parallel-batch dispatches, each agent saves to a distinct filename (no merge conflicts since filenames embed the bucket id).

**When closing an item:** change `Status:` to `resolved <COMMIT>` (where `<COMMIT>` is the short SHA of the fix). Do not delete the entry — closure history is informative for future reviewers. After 6+ months of resolved entries, a separate cleanup pass can archive them to `FOLLOWUPS_ARCHIVE.md`.

## Tiers (definitions)

- **`v0.1-blocker`**: must fix before tagging `wdm-codec-v0.1.0` (Phase 10). Failing to fix = ship blocked.
- **`v0.1-nice-to-have`**: should fix before v0.1 if time permits, but won't block release. Document the deferral in v0.1's CHANGELOG/README if shipped.
- **`v0.2`**: explicitly deferred to v0.2 by a phase decision or spec note. Tracked here for visibility; no v0.1 fix expected.
- **`v1+`**: deferred indefinitely. May be revisited only as part of a major version revision.
- **`external`**: depends on work outside this repo (e.g., upstream PR merging).

---

## Open items

### `v0-5-tap-terminal-name-and-tag-to-bip388-name-parallel-tables` — consolidate parallel hand-maintained operator-name tables

- **Surfaced:** Phase 2 code-quality reviewer (mid-execution, returned to controller; not persisted to `design/agent-reports/` at the time). Reviewer flagged: `tap_terminal_name(term: &Terminal) -> &'static str` (encode.rs ~572) and `tag_to_bip388_name(tag: Tag) -> &'static str` (decode.rs ~805) maintain parallel operator-name tables. A typo in either silently produces divergent diagnostics (encode-side rejection vs. decode-side rejection of the same operator surface different strings to the user).
- **Where:** `crates/md-codec/src/bytecode/encode.rs` (`tap_terminal_name`), `crates/md-codec/src/bytecode/decode.rs` (`tag_to_bip388_name`)
- **What:** Unify around the `Tag` form: have `tap_terminal_name` first map `Terminal` → `Tag` (via existing helpers), then call `tag_to_bip388_name`. Single source of truth; encode/decode-side diagnostics guaranteed byte-identical.
- **Why deferred:** Phase 2 reviewer marked nice-to-have-post-merge-OK; final cumulative reviewer (Phase 9) didn't independently flag.
- **Status:** open
- **Tier:** v0.5-nice-to-have (real maintenance hazard but not user-visible until a name typo lands)

### `v0-5-t7-chunking-boundary-misnomer` — T7 fixture doesn't actually cross chunking boundary

- **Surfaced:** Phase 6 reviewer (commit `7d6e278`). T7's 6-leaf right-spine fixture `tr_multi_leaf_chunking_boundary_md_v0_5` has a 35-byte bytecode that lands well under the 48-byte `ChunkCode::Regular` single-string capacity. The fixture does NOT exercise the chunked-plan path despite its name suggesting otherwise.
- **Where:** `crates/md-codec/src/vectors.rs` T7 entry; `crates/md-codec/tests/vectors/v0.2.json` `tr_multi_leaf_chunking_boundary_md_v0_5` fixture
- **What:** Either (a) rename to a shape-descriptive identifier (e.g., `tr_multi_leaf_right_spine_md_v0_5`) — T7 still adds value as a 6-leaf right-spine asymmetric regression anchor distinct from T3-T5 — or (b) tune the tree shape to 49+ bytes (need explicit derivation paths or a 32-leaf max-fan tree) so it actually crosses the Regular capacity boundary AND the 56-byte Long capacity, forcing chunking. (b) is the "true to original spec intent" path; (a) is the pragmatic ship-now path. T7 still increases coverage in either case.
- **Why deferred:** Not blocking. The fixture passes round-trip; the misnomer is documentation-only.
- **Status:** open
- **Tier:** v0.5-nice-to-have (resolve before v0.5.0 ship via rename, OR resolve in v0.5.x patch by tuning)

### `rust-miniscript-multi-a-in-curly-braces-parser-quirk` — concrete-key `multi_a(...)` inside `tr({...})` fails to parse

- **Surfaced:** Phase 6 implementer (commit `7d6e278`). T6 fixture's plan-prescribed concrete-key policy string failed to parse via rust-miniscript's wallet-policy parser; switched to the `@N`-template form which parses cleanly and matches existing `vectors.rs` convention.
- **Where:** rust-miniscript's wallet-policy parser; not a direct md-codec issue
- **What:** Concrete-key form `tr(<concrete>, {pk(<concrete>), multi_a(2, <concrete>, <concrete>)})` fails; `@N`-template form `tr(@0/**, {pk(@1/**), multi_a(2, @2/**, @3/**)})` works. Possibly an upstream parser bug or a documented limitation.
- **Why deferred:** Workaround is sound (use template form, which matches the rest of the corpus). Not blocking md-codec v0.5.
- **Status:** open
- **Tier:** v1+ (file as upstream issue if desired; not on md-codec critical path)

### `v0-5-spec-plan-encode-tap-subtree-entry-depth-bug` — spec + plan say `target_depth=1` at outer entry; should be `0`

- **Surfaced:** Phase 4 implementer (commit `bca2804`) caught this when the post-condition `debug_assert_eq!(cursor, leaves.len())` failed on a 2-leaf tree compiled per the literal spec text. Phase 4 reviewer (combined pass) confirmed independently with depth-trace analysis.
- **Where:**
  - `design/SPEC_v0_5_multi_leaf_taptree.md` §4 line 220: `encode_tap_subtree(&leaves, &mut cursor, 1, out, &placeholder_map)?;`
  - `design/IMPLEMENTATION_PLAN_v0_5_multi_leaf_taptree.md` Phase 4 Task 4.3 line 1325: same literal `1`
- **What:** The outer call to `encode_tap_subtree` must pass `target_depth=0` (the tree-root depth) so that for any `leaves[0].0 >= 1` the helper emits a `Tag::TapTree` framing before recursing into children with `target_depth=1`. Calling with `target_depth=1` short-circuits emission for symmetric depth-1 trees (matches the first leaf inline, drops the `0x08` framing, fails post-condition). Implementer's actual code at `encode.rs:166` correctly uses `0`; spec + plan text disagree with the working code.
- **Why deferred:** Documentation-only fix; working code is already correct. Spec + plan live on `main` (separate from the feature branch).
- **Status:** resolved (folded into release PR; see `chore(v0.5 m2): fix target_depth literal in spec + plan` commit on `feature/v0.5-multi-leaf-taptree`)
- **Tier:** v0.5-must-close-before-ship (closed)

### `p2-inline-key-tags` — Reserved tags 0x24–0x31 (descriptor-codec inline-key forms)

- **Surfaced:** Phase 2 D-2 (`design/PHASE_2_DECISIONS.md`)
- **Where:** `crates/wdm-codec/src/bytecode/{tag,encode,decode}.rs`
- **What:** Tags `0x24..=0x31` are reserved by descriptor-codec for inline-key forms (raw xpubs, key origins, wildcards). v0.1 rejects them per BIP-388 wallet-policy framing. v1+ may expose them for foreign-xpub support if/when WDM extends beyond pure BIP-388.
- **Why deferred:** v0.1 spec scope.
- **Status:** open
- **Tier:** v1+

### `external-pr-1-hash-terminals` — apoelstra/rust-miniscript PR #1

- **Surfaced:** Phase 5-B; submitted 2026-04-27
- **Where:** https://github.com/apoelstra/rust-miniscript/pull/1
- **What:** PR fixing `WalletPolicyTranslator` to support hash terminals (sha256/hash256/ripemd160/hash160). Until merged, our workspace `[patch]` redirects to a local clone of the patched fork.
- **Why deferred:** waiting for upstream maintainer review.
- **Status:** open
- **Tier:** external

### `decoded-string-data-memory-microopt` — drop `DecodedString.data`, replace with accessor backed by `data_with_checksum`

- **Surfaced:** Phase B bucket A reviewer (Opus 4.7) on commit `5f13812`
- **Where:** `crates/wdm-codec/src/encoding.rs::DecodedString`
- **What:** With `data_with_checksum: Vec<u8>` added in Phase B (so `corrected_char_at` works for checksum-region positions), `data` and `data_with_checksum` redundantly store the same symbol array (data + a 13/15-char suffix). Memory cost is ~26 bytes for Regular / ~30 for Long per `DecodedString`, plus `Vec` overhead — negligible at v0.1 scale. An obvious micro-opt: drop the `data: Vec<u8>` field; replace with a `pub fn data(&self) -> &[u8]` accessor that returns `&self.data_with_checksum[..self.data_with_checksum.len() - checksum_len]`.
- **Why deferred:** breaking API change (the `data` field is currently `pub`); v0.3 breaking-window candidate. Negligible at v0.1/v0.2 scale; not worth the breakage in v0.2.
- **Status:** open
- **Tier:** v0.3

### `phase-d-tap-leaf-wrapper-subset-clarification` — widen the tap-leaf wrapper subset if signers document broader safe support

- **Surfaced:** Phase D implementer (Opus 4.7) on commit `6f6eae9`
- **Where:** `crates/wdm-codec/src/bytecode/encode.rs::validate_tap_leaf_subset`
- **What:** Phase D allows only `c:` and `v:` wrapper terminals in tap leaves (BIP 388 parser emits both implicitly when expanding `pk(K)` and `and_v(v:..., ...)`). All other wrappers (`a:`/`s:`/`d:`/`j:`/`n:`/`u:`/`l:`/`t:`) are rejected. If hardware signers (Coldcard, others) document broader safe support for additional wrappers, widen the subset and update both encode-side and decode-side validators.
- **Why deferred:** v0.2 errs on the side of strict per the BIP MUST clause; widening requires evidence from real signers.
- **Status:** open
- **Tier:** v0.3

### `phase-d-tap-miniscript-type-check-parity` — full Tap-context type-check rules beyond the named subset

- **Surfaced:** Phase D implementer (Opus 4.7) on commit `6f6eae9`
- **Where:** `crates/wdm-codec/src/bytecode/encode.rs::validate_tap_leaf_subset` (and downstream — full type-check parity may need its own module)
- **What:** Phase D's subset filter accepts any `Terminal` from the named operator set (`PkK`/`PkH`/`MultiA`/`OrD`/`AndV`/`Older` plus `c:`/`v:` wrappers) without re-running miniscript's full Tap-context type-check. Coldcard and other signers may enforce more than just the operator-name set (e.g., satisfaction-cost bounds, dust-amount minimums). Full type-check parity with deployed signers is out of v0.2 scope; consider adding a `validate_tap_leaf_full()` wrapper that re-runs miniscript's Tap-context type-check + any signer-specific extras.
- **Why deferred:** the operator-name subset matches the BIP MUST clause and is sufficient for the v0.2 ship target; full parity is a tighter contract than the BIP requires.
- **Status:** open
- **Tier:** v0.3


### `cli-json-debug-formatted-enum-strings` — replace `format!("{:?}", enum_value)` with serde-typed enum mirrors in CLI JSON output

- **Surfaced:** Phase B bucket C reviewer (Opus 4.7) on commit `231574d`
- **Where:** `crates/wdm-codec/src/bin/wdm/json.rs` `confidence_debug` and `outcome_debug` helpers
- **What:** The CLI's `--json` output preserves v0.1.1 enum strings (`"Confirmed"`, `"AutoCorrected"`, etc.) by stringifying via `format!("{:?}", e)`. This works but couples the JSON contract to the library's `Debug` impl — if anyone ever changes a `Debug` derive (e.g., to add a field), the JSON output silently changes. Replacement: define bin-private serde-able enum mirrors with `#[serde(rename_all = "PascalCase")]` (or explicit `#[serde(rename = "...")]` per variant) so the JSON contract is anchored in the wrapper, not in `Debug`.
- **Why deferred:** v0.2's JSON contract is "byte-identical to v0.1.1" — the `Debug` shortcut achieves that. Decoupling the JSON contract from `Debug` is a v1.0 stabilization concern (v1.0 will pin the JSON shape as a contract, at which point the indirection through `Debug` becomes a real liability).
- **Status:** open
- **Tier:** v1+

### `legacy-pkh-permanent-exclusion` — `pkh(KEY)` is permanently excluded

- **Surfaced:** v0.4 spec brainstorming 2026-04-27 (user decision: "modern post-segwit only")
- **Where:** `bip/bip-mnemonic-descriptor.mediawiki` §"Top-level descriptor scope" reject-list + §FAQ "Why is MD narrower than BIP 388?"
- **What:** Top-level `pkh(KEY)` legacy P2PKH single-sig is permanently excluded from MD's accepted surface, even though BIP 388 admits it. Rationale: engravable steel backup is overkill for legacy single-sig (BIP 39 seed alone suffices); negligible new deployment.
- **Why deferred:** Permanent design exclusion, not a deferral.
- **Status:** wont-fix — modern post-segwit only per design.
- **Tier:** wont-fix

### `legacy-sh-multi-permanent-exclusion` — `sh(multi(K, ...))` is permanently excluded

- **Surfaced:** v0.4 spec brainstorming 2026-04-27 (user decision: address-prefix-ambiguity rationale)
- **Where:** `bip/bip-mnemonic-descriptor.mediawiki` §"Top-level descriptor scope" reject-list + §FAQ "Why is MD narrower than BIP 388?"
- **What:** Top-level `sh(multi(K, ...))` legacy P2SH unsorted multisig is permanently excluded from MD's accepted surface, even though BIP 388 admits it. Rationale: address-prefix-ambiguity with `sh(wsh(...))` (both produce 3... addresses) creates recovery footgun; negligible new deployment of pre-segwit multisig wallets.
- **Why deferred:** Permanent design exclusion, not a deferral.
- **Status:** wont-fix — modern post-segwit only per design.
- **Tier:** wont-fix

### `legacy-sh-sortedmulti-permanent-exclusion` — `sh(sortedmulti(K, ...))` is permanently excluded

- **Surfaced:** v0.4 spec brainstorming 2026-04-27 (user decision: address-prefix-ambiguity rationale)
- **Where:** `bip/bip-mnemonic-descriptor.mediawiki` §"Top-level descriptor scope" reject-list + §FAQ "Why is MD narrower than BIP 388?"
- **What:** Top-level `sh(sortedmulti(K, ...))` legacy P2SH sorted multisig is permanently excluded from MD's accepted surface, even though BIP 388 admits it. Rationale: address-prefix-ambiguity with `sh(wsh(...))` (both produce 3... addresses) creates recovery footgun; negligible new deployment of pre-segwit multisig wallets.
- **Why deferred:** Permanent design exclusion, not a deferral.
- **Status:** wont-fix — modern post-segwit only per design.
- **Tier:** wont-fix


### `cargo-toml-crates-io-metadata-fields` — add `keywords`, `categories`, `documentation`, `homepage` to crate manifest

- **Surfaced:** Phase 3 (Cargo rename) code-quality reviewer
- **Where:** `crates/md-codec/Cargo.toml`
- **What:** The crate manifest has no `keywords = [...]`, `categories = [...]`, `documentation = "..."`, or `homepage = "..."` fields. Not a regression introduced by Phase 3 (these were absent in `wdm-codec` pre-rename too). The `repository` field is workspace-inherited and correct.
- **Why deferred:** These fields are crates.io discoverability metadata. They're nice-to-have but not required for `cargo publish` to succeed. Adding them touches no code and can ship in any v0.3.x patch. Best done as part of a "publish to crates.io" prep pass, possibly in v1.0 stabilization.
- **Status:** open
- **Tier:** v1+ (crates.io publish prep)

### `rename-workflow-broad-sed-enumeration-lesson` — workflow doc should explicitly enumerate src/+tests/+bin/ for sed sweeps

- **Surfaced:** Phase 4 (identifier mass-rename) code-quality reviewer (Minor); learnable lesson from 2 oversight-fix commits
- **Where:** `design/RENAME_WORKFLOW.md` Phase 4 section
- **What:** Phase 4 implementer's broad sed sweep ran on `src/` only and missed `tests/`, `src/bin/`, and module-specific subdirectories. Required two follow-up commits (`6c303c0`, `2c9d720`) covering 12 additional files. Lesson: when documenting a future rename, the workflow doc's Phase 4 sub-batch instructions should explicitly enumerate `src/**/*.rs`, `tests/**/*.rs`, and `src/bin/**/*.rs` as separate targets — don't rely on a single glob.
- **Why deferred:** This is a meta-improvement to the workflow doc, not a current rename defect. Best applied next time `RENAME_WORKFLOW.md` is updated (e.g., during the next rename, or as a pre-emptive cleanup pass).
- **Status:** open
- **Tier:** v1+ (process improvement, not version-gating)

---

## Resolved items

(Closure log. Items move here from "Open items" when their `Status:` changes to `resolved <COMMIT>`. Useful for spec/audit reasons; not deleted to preserve provenance.)

### `v0-5-multi-leaf-taptree` — multi-leaf TapTree (`tr(KEY, TREE)`) admission

- **Surfaced:** v0.4 spec brainstorming 2026-04-27; named in BIP §FAQ "Why is multi-leaf TapTree deferred (vs excluded)?"
- **Where:** `crates/md-codec/src/bytecode/decode.rs` Tr handler; `bip/bip-mnemonic-descriptor.mediawiki` §"Top-level descriptor scope" + §"Taproot tree"
- **What:** Admit `tr(KEY, TREE)` with non-trivial multi-leaf TapTree (BIP 388 §"Taproot tree"). Required TapTree depth/balancing rules (BIP 341 depth-128 cap), per-leaf miniscript Tap-context validation, leaf-wrapper subset enforcement on every leaf, and recursive `[Tag::TapTree=0x08][LEFT][RIGHT]` framing. Delivered in v0.5.0 across 11 phases (spec ratification → type wiring → top-level dispatcher → encoder rewrite → tap_leaves population → 29 NEW + 1 RENAMED conformance vectors → BIP doc updates → CLI integration test → final cumulative review → CHANGELOG/MIGRATION → release prep).
- **Status:** resolved 865f889 (PR #1 merge commit; release tag md-codec-v0.5.0)
- **Tier:** v0.5 (planned admission, closed)

### `v0-5-stale-v0-4-message-strings-sweep` — sweep remaining "v0.4 does not support" / "reserved for v1+" stale strings

- **Surfaced:** Phase 3 review of v0.5 (commit `59797ef`). Phase 3 only updated the four `decode_descriptor` strings in scope; reviewer flagged additional stale "v0.4" / "v1+" strings in adjacent code that are now factually wrong at v0.5.
- **Where (all closed in Phase 4):**
  - `crates/md-codec/src/bytecode/encode.rs:116` — sh(legacy P2SH) error → "permanently rejected (legacy non-segwit out of scope per design)"
  - `crates/md-codec/src/bytecode/encode.rs:123,163` — top-level pkh/bare → split into separate Pkh and Bare messages, both with "permanently rejected" framing
  - `crates/md-codec/src/bytecode/encode.rs:13-17` — module doc rewritten: replaced "Multi-leaf TapTree is reserved for v1+" with v0.5 admission paragraph
  - `crates/md-codec/src/bytecode/decode.rs:167` — `decode_sh_inner` catch-all → "permanently rejected"
  - `crates/md-codec/src/bytecode/decode.rs:11-14` — module doc: replaced v1+ Tag::TapTree reservation with v0.5 admission paragraph + depth-128 mention
  - `crates/md-codec/src/bytecode/decode.rs:255-257` — `decode_tr_inner` doc: replaced "reserved for v1+" with v0.5 multi-leaf admission note
- **What:** Sweep all sites to version-agnostic / v0.5-correct text. Folded into Phase 4 encoder-rewrite commit; verified zero remaining "v0.4 does not support" / "reserved for v1+" strings in `encode.rs` and `decode.rs` post-commit.
- **Why deferred:** Phase 3 scope was only the top-level dispatcher; the implementer correctly kept scope tight. Reviewer recommended folding the encoder messages into Phase 4 (which already touches `encode.rs:126-158`).
- **Status:** resolved bca2804
- **Tier:** v0.5-nice-to-have (closed before v0.5.0 ship)

### `p10-bip-header-status-string` — align BIP draft header with the ref-impl-aware status

- **Surfaced:** Phase 10 Task 10.7 closure
- **Where:** `bip/bip-mnemonic-descriptor.mediawiki:8`
- **What:** The BIP draft preamble's `Status:` field still reads `Pre-Draft, AI only, not yet human reviewed`. The root README and project memory now use `Pre-Draft, AI + reference implementation, awaiting human review`. The BIP draft is its own artifact and could legitimately stay on the older string (the spec text itself hasn't been ref-impl-validated by a human), but for consistency the next BIP touch should consider aligning.
- **Why deferred:** stylistic; not a contract issue. The BIP draft predates the impl; the spec's status is independent.
- **Status:** resolved 270bf57
- **Tier:** v0.1-nice-to-have

### `bip-preliminary-hrp-disclaimer-tension` — reconcile "preliminary HRP" disclaimer with collision-vet claim

- **Surfaced:** Phase 2 (BIP rename) spec-compliance + code-quality reviewers, both flagged independently
- **Where:** `bip/bip-mnemonic-descriptor.mediawiki` — HRP §disclaimer vs §"Why a new HRP?" collision-vet claim
- **What:** Line saying the HRP "is preliminary and subject to change before this BIP is finalized" reads awkwardly alongside the collision-vet claim. Both reviewers classified this as "not a Phase 2 defect; flag for finalization."
- **Why deferred:** Reconciliation was deferred until SLIP-0173 PR (`slip-0173-register-md-hrp`) status was clearer. Fixed in v0.4.1 by upgrading the disclaimer to "subject to formal SLIP-0173 registration" which is consistent with the collision-vet claim.
- **Status:** resolved 270bf57
- **Tier:** v0.3-finalization (pre-1.0 BIP cleanup)

### `bch-known-vector-repin-with-md-hrp` — repin BCH known-vector tests with Python-computed checksums for HRP "md"

- **Surfaced:** Phase 5 (string literal sweep) spec compliance reviewer, judgment-call adjudication
- **Where:** `crates/md-codec/src/encoding.rs` — `bch_known_vector_regular` and `bch_known_vector_long` test functions
- **What:** Phase 5 implementer converted these from hardcoded-expected-checksum tests to round-trip tests. Both could go wrong together (wrong polynomial constant) and the test would still pass. Fix: compute Python-reference checksums for HRP `"md"` and add `assert_eq!(actual, &[…])` pin lines.
- **Why deferred:** Repinning required computing the new BCH-over-md-HRP checksums via an external Python reference.
- **Status:** resolved 270bf57
- **Tier:** v0.3-nice-to-have (downgraded from v0.3-blocker; redundant with SHA-pin layer but adds unit-level isolation)
- **Pins (v0.4.1):** regular `[25, 14, 21, 4, 26, 20, 18, 15, 5, 15, 23, 30, 15]`; long `[23, 8, 11, 10, 1, 2, 13, 8, 29, 0, 17, 11, 14, 25, 11]`. Script: `/tmp/compute_bch_md_pins.py`.

### `bip48-nested-name-table-entry` — CLI affordance for indicator 0x06

- **Surfaced:** v0.4 spec §3 / Phase 4 plan
- **Where:** `crates/md-codec/src/bin/md/main.rs` NAME_TABLE
- **What:** Add `("bip48-nested", 0x06)` and testnet variant to NAME_TABLE so users can write `--path bip48-nested` rather than `--path 0x06` or literal-path form.
- **Status:** resolved 45f6736 (Phase 4 of v0.4)
- **Tier:** v0.4-task

### `v0-4-bip-388-surface-completion` — extend top-level descriptor support to `wpkh` and `sh(wsh(...))`

- **Surfaced:** Design discussion 2026-04-27 prompted by user noticing the `PolicyScopeViolation` rejection of `wpkh()` at top level
- **Where:** `crates/md-codec/src/bytecode/encode.rs` (currently rejects `Descriptor::Wpkh`, `Descriptor::Sh`, `Descriptor::Pkh`, `Descriptor::Bare`); `bip/bip-mnemonic-descriptor.mediawiki` §"Top-level descriptor scope"; bytecode tag.rs (Wpkh tag 0x04 already reserved; Sh would need a new tag)
- **What:** v0.1 scoped to `wsh(...)` only; v0.2 added `tr(...)`. BIP 388 itself covers four top-level shapes: `wpkh`, `wsh`, `sh(wsh(...))`, `tr`. Expanding to the full BIP 388 surface lets BIP 84 single-sig wallets and BIP 48 P2SH-P2WSH multisig wallets (still emitted by Coldcard / Trezor / Ledger for backwards-compat) round-trip through MD without the awkward `wsh(pk(@0/**))` workaround. Scope: add `wpkh(@0/**)` and `sh(wsh(...))` encode/decode/round-trip + conformance vectors. Skip legacy `pkh(...)` and bare `sh(multi(...))` permanently — pre-segwit, no new wallets.
- **Why deferred:** v0.3 is the rename release (wdm→md); compounding it with new encode paths would muddy the wire-break audit. v0.4 is the natural home: "BIP 388 surface completion." Wire format additive: tags are allocated, wpkh likely reuses existing `Wpkh = 0x04` tag (already reserved per `bytecode/tag.rs:28`); `sh(wsh(...))` needs a new top-level tag in the unallocated range. Schema bump from 2 → 3 with `v0.3.json` carried forward unchanged + new `v0.4.json` adding wpkh + sh-wsh corpora.
- **Estimated effort:** 1 phase (~3 days). Encode path mechanical (single-key wpkh + composition for sh-wsh); decode path needs new tag dispatch + minor BIP §"Top-level descriptor scope" rewrite. Conformance vectors expand; family-stable promise carries to v0.4.x.
- **Status:** resolved 3ed3f2402bac712bcac86e49d36e7c931fbf1d55

**Closure note**: Stated scope (wpkh + sh(wsh)) addressed; v0.4 also adds
sh(wpkh) (BIP-388-required, omitted from original entry). Entry name
imprecise — v0.4 is the modern post-segwit SUBSET of BIP 388, narrower
than BIP 388 itself. Multi-leaf TapTree filed as new entry
`v0-5-multi-leaf-taptree`. Legacy exclusions filed as
`legacy-{pkh,sh-multi,sh-sortedmulti}-permanent-exclusion` (wont-fix).
See BIP §FAQ for rationale.

- **Tier:** v0.4 (post-rename, closed)

### `5a-from-inner-visibility` — `WalletPolicy::from_inner` should be `pub(crate)` not `pub`

- **Surfaced:** Phase 5-A code review of `56124c3`
- **Status:** resolved `2ec1d41` — function was removed entirely; no in-crate caller existed.
- **Tier:** v0.1-nice-to-have (closed)

### `5b-hash-byte-overcount` — `count_placeholder_indices` byte-scan over-counts on hash bytes ≡ 0x32

- **Surfaced:** Phase 5-B code review of `f0d9346`
- **Status:** resolved `48809b7` — Option A adopted; `count_placeholder_indices` deleted; `decode_template` now receives 32 dummy keys and `from_descriptor` re-derives `key_info` from actual descriptor structure.
- **Tier:** v0.1-blocker (closed)

### `5b-dummy-table-too-small` — DUMMY_KEYS table 8 entries; corpus C5 needs 11

- **Surfaced:** Phase 5-B code review of `f0d9346`
- **Status:** resolved `48809b7` — table grown to 32 entries (BIP 388 max placeholder count).
- **Tier:** v0.1-blocker (closed)

### `5c-walletid-words-display` — `WdmBackup::wallet_id()` hand-rolled space-join

- **Surfaced:** Phase 5-C code review of `62ae611`
- **Status:** resolved `8e00766` — uses `WalletIdWords::Display::to_string()`; also fixed an adjacent pre-existing `clippy::needless_borrows_for_generic_args` warning in `bip39::Mnemonic::parse` call.
- **Tier:** v0.1-nice-to-have (closed)

### `5d-chunk-index-from-header` — `EncodedChunk.chunk_index`/`total_chunks` should read from header, not loop counter

- **Surfaced:** Phase 5-D code review of `308b2e1`
- **Status:** resolved `9529ee8` — fields now destructured from `chunk.header`; loop is plain `for chunk` (no enumerate).
- **Tier:** v0.1-nice-to-have (closed)

### `5d-loop-invariant-bch-code` — BCH code lookup hoisted out of Stage 5 loop

- **Surfaced:** Phase 5-D code review of `308b2e1`
- **Status:** resolved `9529ee8` — match on `plan` to determine `bch_code` now happens once before the loop.
- **Tier:** v0.1-nice-to-have (closed)

### `5e-tests-13-14-merge` — `decode_report_outcome_clean` and `verifications_all_true_on_happy_path` were one combined test

- **Surfaced:** Phase 5-E code review of `7b7400b`
- **Status:** resolved `111f176` — split into two `#[test]` functions sharing a `happy_path_decode()` helper.
- **Tier:** v0.1-nice-to-have (closed)

### `5e-corrupted-hash-test-name` — `decode_rejects_corrupted_cross_chunk_hash` didn't exercise public API

- **Surfaced:** Phase 5-E code review of `7b7400b`
- **Status:** resolved `111f176` — test deleted; equivalent coverage already exists in `chunking.rs::tests::reassemble_cross_chunk_hash_mismatch_with_corrupted_hash_byte` (Phase 4-E followup) and `reassemble_cross_chunk_hash_mismatch`.
- **Tier:** v0.1-nice-to-have (closed)

### `5f-rustdoc-broken-links` — 5 rustdoc errors blocking the new `cargo doc` CI job

- **Surfaced:** Phase 5-F implementer's DONE_WITH_CONCERNS report on `571104b`
- **Status:** resolved across `111f176` (decode.rs:28 fix) + `4c73338` (4 fixes in key.rs/encode.rs/wallet_id.rs/encoding.rs); `RUSTDOCFLAGS="-D warnings" cargo doc` now finishes cleanly.
- **Tier:** v0.1-blocker (closed; doc CI green)

### `5b-from-exact-bytes-removed` — `Chunk::from_exact_bytes` and `Error::TrailingChunkBytes` were unreachable dead code

- **Surfaced:** Phase 4-E review of `f0d9346` (the Opus reviewer noticed the helper was structurally identical to `from_bytes` because chunk fragments have no length-bound)
- **Status:** resolved `e7a7a16` (Phase 4-E followup); rationale captured in `design/PHASE_7_DECISIONS.md` CF-1 (Phase 7 codex32 layer is the chunk byte-boundary source of truth).
- **Tier:** v0.1-nice-to-have (closed)

### `5a-test-7-tautology` — `shared_path_returns_none_for_template_only_policy` used `matches!(.., None | Some(_))`

- **Surfaced:** Phase 5-A code review of `56124c3`
- **Status:** resolved `22beba8` (Phase 5-B); test now uses `assert!(p.shared_path().is_none())` since the 5-B implementation correctly returns `None` for template-only policies.
- **Tier:** v0.1-nice-to-have (closed)

### `5a-key-count-cast` — `(m + 1) as usize` cast in `key_count`

- **Surfaced:** Phase 5-A code review of `56124c3`
- **Status:** resolved `2ec1d41` (Phase 5-A followup); `key_count` now uses `usize` throughout its scan, eliminating the cast entirely.
- **Tier:** v0.1-nice-to-have (closed)

### `5a-key-count-numeric-type` — `key_count` should use `usize` end-to-end (was `u32`-then-cast)

- **Surfaced:** Phase 5-A code review of `56124c3`
- **Status:** resolved `2ec1d41` (Phase 5-A followup); `Option<u32>` → `Option<usize>`, `parse::<u32>()` → `parse::<usize>()`.
- **Tier:** v0.1-nice-to-have (closed)

### `5a-key-count-rustdoc` — rustdoc clarification that `inner.to_string()` writes only the template

- **Surfaced:** Phase 5-A code review of `56124c3`
- **Status:** resolved `2ec1d41` (Phase 5-A followup); rustdoc explicitly notes BIP 388 template form (`@N`-only) and that origin xpubs appear only in full-descriptor display.
- **Tier:** v0.1-nice-to-have (closed)

### `5d-from-impl` — add `From<ChunkCode> for BchCode` impl

- **Surfaced:** Phase 5-D code review of `308b2e1`
- **Status:** resolved `430dbfc` (post-v0.1 followup batch 1, bucket A); `From<ChunkCode> for BchCode` impl added in `chunking.rs`; private `chunk_code_to_bch_code` helper in `encode.rs` removed and call sites switched to `BchCode::from(plan.code)`.
- **Tier:** v0.1-nice-to-have (closed)

### `5d-decision-cross-reference` — note force_long_code post-processor in chunking_decision rustdoc

- **Surfaced:** Phase 5-D code review of `308b2e1`
- **Status:** resolved `430dbfc` (post-v0.1 followup batch 1, bucket A); `chunking_decision` rustdoc now cross-references `EncodeOptions.force_long_code` and the `encode.rs` post-processor.
- **Tier:** v0.1-nice-to-have (closed)

### `6c-encode-options-builder` — `EncodeOptions` `#[non_exhaustive]` blocks struct-update syntax from external tests

- **Surfaced:** Phase 6 bucket C; Task 6.18 (`natural_long_code_boundary`)
- **Status:** resolved `a74e21b` (post-v0.1 followup batch 1, bucket B); fluent builder added — `EncodeOptions::default().with_force_chunking(true).with_force_long_code(true).with_seed(seed)` now works from external integration tests despite `#[non_exhaustive]`.
- **Tier:** v0.1-nice-to-have (closed)

### `5e-skip-silent` — tests with size-conditional assertions skip silently

- **Surfaced:** Phase 5-E code review of `7b7400b`
- **Status:** resolved `fa83737` (post-v0.1 followup batch 1, bucket C); tests at `decode.rs:270` and `decode.rs:530` now use `with_force_chunking(true)` so the chunked path is exercised deterministically regardless of bytecode length.
- **Tier:** v0.1-nice-to-have (closed)

### `5e-dead-branch` — `decode_rejects_chunks_with_duplicate_indices` has unreachable fallback

- **Surfaced:** Phase 5-E code review of `7b7400b`
- **Status:** resolved `fa83737` (post-v0.1 followup batch 1, bucket C); the unreachable `if backup.chunks.len() < 2` branch removed; test now goes straight to the multi-chunk assertion path on the 9-key multisig.
- **Tier:** v0.1-nice-to-have (closed)

### `5e-correction-position-doc` — rustdoc cross-reference for `Correction.char_position` coordinate system

- **Surfaced:** Phase 5-E code review of `7b7400b`
- **Status:** resolved `fa83737` (post-v0.1 followup batch 1, bucket C); `decode` rustdoc now cross-references the `Correction.char_position` coordinate system documented at `chunking.rs::Correction`.
- **Tier:** v0.1-nice-to-have (closed)

### `5e-five-bit-truncated-mapping` — `five_bit_to_bytes` failure error-variant choice

- **Surfaced:** Phase 5-E code review of `7b7400b`
- **Status:** resolved `fa83737` (post-v0.1 followup batch 1, bucket C); branch now `unreachable!()` with a justification comment that successful BCH validation guarantees a multiple-of-8 data part.
- **Tier:** v0.1-nice-to-have (closed)

### `6e-missing-children-unreachable` — `BytecodeErrorKind::MissingChildren` defined but never emitted

- **Surfaced:** Phase 6 bucket E; Task 6.21 — `rejects_invalid_bytecode_missing_children` was `#[ignore]`d
- **Status:** resolved `1ccc1d4` (post-v0.1 followup batch 1, bucket D); explicit arity check added in variable-arity decoder branches now emits `MissingChildren { expected, got }`; conformance test un-`#[ignore]`d (test count: 1 ignored → 0 ignored).
- **Tier:** v0.1-nice-to-have (closed)

### `7-cli-integration-tests` — CLI integration tests via `assert_cmd`

- **Surfaced:** Phase 7 implementation (Task 7 prompt, §Tests)
- **Status:** resolved `1ccc1d4` (post-v0.1 followup batch 1, bucket E); `tests/cli.rs` added with 12 `assert_cmd` tests (8 happy-path + 4 error-path) covering `encode`, `decode`, `verify`, `inspect`, `bytecode`; `assert_cmd = "2"` and `predicates = "3"` added as dev-deps. Closed early (was tier'd v0.2; accelerated to post-v0.1 nice-to-have).
- **Tier:** v0.2 (closed; accelerated)

### `p10-miniscript-dep-audit` — release-readiness audit of the miniscript git pin

- **Surfaced:** Phase 5 D-1 (`design/PHASE_5_DECISIONS.md`); Phase 7 carry-forward CF-1 documents adjacent context
- **Status:** resolved at tag `wdm-codec-v0.1.0` (`fef8dcb`) via option (b): git-dep pin documented in `crates/wdm-codec/Cargo.toml`, the workspace `[patch]` rationale captured in the root `Cargo.toml`, the BIP draft's reference-implementation section names the apoelstra fork dep, and the root README status notes the dep. Tag annotation message also contains the dep rationale. Forward work (flipping the `[patch]` block off when upstream PR merges) is tracked separately as `external-pr-1-hash-terminals`.
- **Tier:** v0.1-blocker (closed)

### `p4-chunking-rs-split` — split `chunking.rs` into a `chunking/` directory

- **Surfaced:** Phase 4-A and 4-D code reviews; Phase 4-E code review
- **Status:** wont-fix — every reviewer through Phase 7 confirmed the section-banner organization is navigable; no Phase 6/7/8/9/10 consumer found it unwieldy. Splitting now is pure churn (touches every test in the file, breaks any external pin to module path) for no reader-experience win. Revisit only if a future caller is genuinely impeded.
- **Tier:** v0.1-nice-to-have (closed)

### `6a-coldcard-corpus-shape` — Coldcard corpus entry uses representative shape (same as C2)

- **Surfaced:** Phase 6 bucket A; Task 6.11
- **Status:** wont-fix — v0.1 corpus is operator-shape based by design; the Coldcard entry is an existence-proof that real-world export shapes round-trip, not a coverage gap. Revisit if a future signer's BIP 388 export is structurally distinct from existing corpus shapes.
- **Tier:** v0.1-nice-to-have (closed)

### `6d-rand-gen-keyword` — `rng.r#gen()` raw-identifier workaround for Rust 2024 reserved keyword

- **Surfaced:** Phase 6 bucket D; Task 6.20 (`many_substitutions_always_rejected`)
- **Status:** resolved `ff7d1ea` — `rand` dev-dep bumped 0.8 → 0.9; all `r#gen()` and `gen_range` callsites switched to `random()` and `random_range()`.
- **Tier:** v0.1-nice-to-have (closed)

### `8-negative-fixture-placeholder-strings` — negative vector `input_strings` are placeholder-grade, not confirmed-correct WDM strings

- **Surfaced:** Phase 8 implementation (Task 8.3); implementer's own follow-up
- **Status:** resolved `c46f2c0` via option (b) — `vectors.rs` `NEGATIVE_FIXTURES` rustdoc rewritten to honestly document fixture provenance: `expected_error_variant` is the authoritative contract; `input_strings` are representative placeholders demonstrating the error class; n12, n29, n30 explicitly flagged as targeting lower-level APIs (`reassemble_chunks`, `policy.parse`, `chunking_decision`). The original misleading "all placeholder inputs are confirmed to trigger the correct variant" claim was deleted. Dynamic generation (option a) deferred as `8-negative-fixture-dynamic-generation` (open, v0.2).
- **Tier:** v0.1-nice-to-have (closed)

### `p10-cross-platform-ci-sanity` — confirm GitHub Actions green on Windows + macOS

- **Surfaced:** Phase 10 Task 10.2; deferred at controller closure
- **Status:** resolved `651c402` (post-push verification at run [25022150945](https://github.com/bg002h/descriptor-mnemonic/actions/runs/25022150945)) — full pipeline now green across `cargo test (ubuntu/windows/macos)` + `cargo clippy` + `cargo fmt` + `cargo doc`. Required four code/CI fixes that previous local-only validation never caught: `f4c8d3c` (workflow `git clone --depth` couldn't reach the SHA on a non-default branch), `06557a3` (matrix-ize the test job), `b12b814` (clippy 1.85.0 `precedence` lint in `polymod_step`), and `651c402` + `c46f2c0` (clippy 1.85.0 `format_collect` lint in `vectors.rs` and `bin/wdm.rs`). Lesson: pin a CI-equivalent toolchain locally if you need pre-push lint parity.
- **Tier:** v0.1-nice-to-have (closed)

### `p3-decode-declaration-from-bytes` — `decode_declaration_from_bytes(&[u8]) -> Result<(DerivationPath, usize), Error>` ergonomic alt

- **Surfaced:** Phase 3.5' code review of `bdeb639`
- **Status:** resolved (post-v0.1.1 v0.2 batch 1) — new `pub fn decode_declaration_from_bytes(&[u8]) -> Result<(DerivationPath, usize), Error>` added to `crates/wdm-codec/src/bytecode/path.rs`. Constructs an internal Cursor, calls the existing `pub(crate)` cursor-based decoder, returns `(path, cur.offset())`. Four new tests cover dictionary path round-trip, explicit path round-trip, trailing-bytes-not-consumed semantics, and error propagation. Purely additive; no existing API changed.
- **Tier:** v0.2 (closed)

### `p2-decoded-template-hybrid` — hybrid `DecodedTemplate` decoder shape

- **Surfaced:** Phase 2 D-5 (`design/PHASE_2_DECISIONS.md`)
- **Status:** wont-fix — Phase 2 D-5 chose option (A) (`decode_template` returns `Descriptor<DescriptorPublicKey>` directly via key substitution); through v0.1.1 no caller has surfaced needing lazy key substitution. The 2-arg `decode_template(bytes, &keys)` API is the natural inverse of `encode_template(d, &map)`. Revisit only if a real recovery-flow consumer needs to inspect the template before binding keys.
- **Tier:** v0.2 (closed)

### `4a-from-bytes-shape` — reconsider `Chunk::from_bytes` shape (slice+usize vs `&mut Cursor`)

- **Surfaced:** Phase 4-A code review of `aefdf3f` (deferred to "after 4-E"); 4-E used the slice+usize shape unchanged
- **Status:** wont-fix — through v0.1.1 no caller has surfaced needing the shape switched. Phase 7 CLI consumed `Chunk::from_bytes` via the slice+usize shape without friction; no Phase 5–10 consumer needed the Cursor shape. Both shapes do equivalent work; consolidating now is style-only churn. Revisit only if a non-test consumer surfaces a concrete need.
- **Tier:** v0.2 (closed)

### `p4-chunking-mode-enum` — `force_chunked: bool` → `ChunkingMode { Auto, ForceChunked }`

- **Surfaced:** Phase 4-D code review of `1fe9505`
- **Status:** resolved `fbbe6ec` (v0.2 Phase A bucket A) — pub enum `ChunkingMode { Auto, ForceChunked }` added to `chunking.rs`; `pub fn chunking_decision(usize, ChunkingMode)`; `EncodeOptions.force_chunking: bool` → `chunking_mode: ChunkingMode`. `with_force_chunking(self, bool)` builder preserved as a `bool → enum` shim for v0.1.1 source-compat. Wire format unchanged; vectors verify byte-identical. 2 new tests cover the bool↔enum shim and `Default = Auto`. Reviewer `APPROVE_WITH_FOLLOWUPS`; the `matches!` → exhaustive `match` nit applied inline by controller; 3 minor follow-ups filed (`p4-chunking-mode-stale-test-names`, `p4-with-chunking-mode-builder`).
- **Tier:** v0.2 (closed; breaking — see commit `fbbe6ec` body for full migration note)

### `6a-bytecode-roundtrip-path-mismatch` — encode→decode→encode is not byte-stable for template-only policies

- **Surfaced:** Phase 6 bucket A (corpus.rs idempotency test); Task 6.12 had to be reframed
- **Status:** resolved `86ca5df` (v0.2 Phase A bucket B) — `WalletPolicy` newtype gains `decoded_shared_path: Option<DerivationPath>` field; `from_bytecode` populates it from `decode_declaration`'s return value; `to_bytecode` consults it under the Phase A precedence rule (`decoded_shared_path > shared_path() > BIP 84 fallback`). Public signatures of `from_bytecode` / `to_bytecode` unchanged. `tests/corpus.rs` idempotency test tightened to assert FIRST-pass raw-byte equality (was second-pass-onward only). New inline test in `policy.rs` proves the round-trip for `m/48'/0'/0'/2'` (distinguishes from both BIP 84 fallback and dummy-key origin). Wire format unchanged; vectors verify byte-identical. Reviewer `APPROVE_WITH_FOLLOWUPS`; the field rustdoc note about `PartialEq` semantics applied inline by controller; MIGRATION.md follow-up filed (`wallet-policy-eq-migration-note`).
- **Tier:** v0.2 (closed; behavioral — see commit `86ca5df` body for full migration note)

### `5e-checksum-correction-fallback` — `Correction.corrected = 'q'` for checksum-region corrections

- **Surfaced:** Phase 5-E code review of `7b7400b`; `// TODO(post-v0.1)` added inline at `decode.rs:119` in `111f176`
- **Status:** resolved `5f13812` (v0.2 Phase B bucket A) — `DecodedString` extended with `pub fn corrected_char_at(char_position: usize) -> char` backed by a new `pub data_with_checksum: Vec<u8>` field (`#[non_exhaustive]` so additive). `decode.rs` Correction translator now uses `corrected_char_at(pos)` instead of the `'q'` placeholder; the `// TODO(post-v0.1)` comment is removed. Two new tests cover both checksum-region and data-region correction reporting. Wire format unchanged; vectors verify byte-identical. Reviewer `APPROVE_WITH_FOLLOWUPS`; rustdoc disambiguation on `corrected_char_at` Panics section applied inline by controller; v0.3 memory micro-opt filed (`decoded-string-data-memory-microopt`).
- **Tier:** v0.2 (closed)

### `7-encode-path-override` — `--path` override does not yet affect bytecode encoder

- **Surfaced:** Phase 7 implementation
- **Status:** resolved `0993dc0` (v0.2 Phase B bucket B) — `EncodeOptions::shared_path: Option<DerivationPath>` field added (additive on `#[non_exhaustive]`) along with a `with_shared_path(path)` builder method. `WalletPolicy::to_bytecode(&self)` signature changed to `to_bytecode(&self, opts: &EncodeOptions)` (breaking) so the encoder can consult the override. The 4-tier shared-path precedence is now: `EncodeOptions::shared_path > WalletPolicy.decoded_shared_path > WalletPolicy.shared_path() > BIP 84 mainnet fallback`. CLI `cmd_encode` no longer prints "warning: --path is parsed but not applied" — it actually applies the override. 22 `to_bytecode` call sites updated (1 pipeline, 1 wrapper, 1 wallet-id helper, 1 vector builder, 1 CLI handler, 16 tests). 5 new tests including a CLI integration test. Side-effect: `EncodeOptions` lost its derived `Copy` impl because `DerivationPath` isn't `Copy`. Wire format unchanged for default-path case; vectors verify. Reviewer `APPROVE_WITH_FOLLOWUPS`; the override-wins test strengthening (assert bytes != baseline) applied inline by controller; MIGRATION.md follow-up filed (`phase-b-encode-signature-and-copy-migration-note`).
- **Tier:** v0.2 (closed; breaking — see commit `0993dc0` body for full migration note)

### `7-serialize-derives` — manual JSON construction vs `#[derive(Serialize)]` on library types

- **Surfaced:** Phase 7 implementation
- **Status:** resolved `231574d` (v0.2 Phase B bucket C) — chosen strategy was option (A): bin-private serde-able wrapper types in a new `crates/wdm-codec/src/bin/wdm/json.rs` module. Library types unchanged (no serde derives sneaked into `WalletPolicy`, `WdmBackup`, etc.). Seven wrappers added (`EncodeJson`, `EncodedChunkJson`, `BchCodeJson`, `DecodeJson`, `DecodeReportJson`, `CorrectionJson`, `VerificationsJson`) with `From<&LibraryType>` impls and full `Serialize + Deserialize` round-trip. JSON output is byte-identical to v0.1.1's `serde_json::json!{}` literals — alphabetical wrapper-field ordering preserves `BTreeMap`-backed key order from `serde_json::Map`. File layout: `bin/wdm.rs` → `bin/wdm/main.rs` (Cargo bin-with-submodule convention) + new `bin/wdm/json.rs` (module rename from initial `wdm_json` per reviewer N-2). 10 new tests. Reviewer `APPROVE_WITH_FOLLOWUPS`; signature consistency (N-1) + module rename (N-2) applied inline by controller; v1.0 entry filed for the `Debug`-formatted enum strings (`cli-json-debug-formatted-enum-strings`).
- **Tier:** v0.2 (closed)

### `p1-bch-4-error-correction` — proper Berlekamp-Massey/Forney decoder for full 4-error correction

- **Surfaced:** inline `// TODO(v0.2)` at `crates/wdm-codec/src/encoding.rs:379` (since Phase 1)
- **Status:** resolved `3aabcf6` (v0.2 Phase C) — replaces brute-force 1-error correction with full syndrome-based BCH decoder: Berlekamp-Massey for the error-locator polynomial Λ(x), Chien search for the error positions, shifted Forney for the error magnitudes. Field representation `GF(1024) = GF(32)[ζ]/(ζ²-ζ-1)` per BIP 93. Primitive elements β = G·ζ (regular, order 93) and γ = E + X·ζ (long, order 1023). 8-consecutive-roots windows `{β^77..β^84}` and `{γ^1019..γ^1026}`. Defensive `bch_verify_*` re-check after applying corrections guards the >4-error edge case. Public API surface unchanged — only behavioral difference is that 2/3/4-error inputs now succeed instead of returning `BchUncorrectable`. Wire format unchanged; `gen_vectors --verify` PASS. New `crates/wdm-codec/src/encoding/bch_decode.rs` (~620 LOC) plus `crates/wdm-codec/tests/bch_correction.rs` (42 integration tests + 11 lib tests = 53 new tests, including 3,200 randomized round-trips at the t=4 capacity boundary). BIP §"Error-correction guarantees" gains a SHOULD-clause naming the canonical algorithm + field representation so cross-implementations report byte-identical `Correction.corrected` values. Reviewer (Opus 4.7) `APPROVE_WITH_FOLLOWUPS` with no algorithmic findings — explicitly cross-checked field, primitive orders, generator roots, BM, Chien, Forney, defensive verify ("no bugs found", "an unusually clean port"). 4 stylistic nits filed as cluster `phase-c-bch-decode-style-cleanups`.
- **Tier:** v0.2 (closed)

### `p2-taproot-tr-taptree` — taproot `Tr` / `TapTree` operator support

- **Surfaced:** Phase 2 (D-2, D-4, plan task 2.11 marked deferred)
- **Status:** resolved `6f6eae9` (v0.2 Phase D, cherry-picked from worktree commit `267036f`) — top-level `tr()` taproot descriptors now encode and decode end-to-end with the Coldcard per-leaf miniscript subset enforced at BOTH encode and decode time. Single-leaf only at depth 0 per BIP §"Taproot tree" v0 constraint; multi-leaf `Tag::TapTree` (`0x08`) reserved for v1+ and rejected with `PolicyScopeViolation("multi-leaf TapTree reserved for v1+")`. New `Error::TapLeafSubsetViolation { operator: String }` variant for the subset-violation case (registered in the conformance exhaustiveness gate). `Cursor::is_empty()` and `peek_byte()` helpers added for the optional-leaf delimiter detection. Wrapper terminals: only `c:` and `v:` allowed (BIP 388 parser emits implicitly); all others rejected. Phase 2's pre-shipped `multi_a` arms (`encode.rs:178`, `decode.rs:222`) now exercised in Tap context. New `tests/taproot.rs` (8 tests) + 1 conformance test for the exhaustiveness mirror. Wire format unchanged for the v0.1 corpus (`gen_vectors --verify v0.1.json` byte-stable); taproot corpus fixtures deferred to Phase F (filed as `phase-d-taproot-corpus-fixtures`). BIP draft updated: heading "Taproot tree (forward-defined)" → "Taproot tree", tag `0x08` clarified as v1+, concrete byte-layout examples added (regenerated from live encoder). Phase D decision log committed at `24a7a4b` resolved D-1..D-5 in advance. Reviewer (Opus 4.7) `APPROVE_WITH_FOLLOWUPS` with no spec deviations, no algorithmic findings; 3 nits + 4 v0.2/v0.3 follow-ups filed.
- **Tier:** v0.2 (closed; breaking — `Tr` rejection removed; new `Error::TapLeafSubsetViolation` variant)

### `p2-fingerprints-block` — v0.2 fingerprints block support

- **Surfaced:** Phase 5-B; documented at `crates/wdm-codec/src/policy.rs:316-317` and `:668`
- **Status:** resolved `6559c17` (v0.2 Phase E) — full fingerprints block end-to-end. `EncodeOptions::fingerprints: Option<Vec<bitcoin::bip32::Fingerprint>>` field (additive on `#[non_exhaustive]`) + `with_fingerprints(...)` builder. `Tag::Fingerprints = 0x35` added to the `#[non_exhaustive]` Tag enum + `from_byte` arm. Encoder default `None` → header `0x00` (preserves v0.1 wire output); `Some(fps)` → header `0x04` + emit block immediately after path declaration. Decoder validates count == `key_count()` per BIP MUST clause. New `Error::FingerprintsCountMismatch { expected, got }` variant (registered in conformance exhaustiveness gate). `from_bytecode_with_fingerprints` internal helper returns `(WalletPolicy, Option<Vec<Fingerprint>>)`; legacy `from_bytecode` preserved as thin wrapper. `DecodeResult.fingerprints: Option<Vec<Fingerprint>>` additive field surfaces the parsed block. v0.1 `PolicyScopeViolation` rejection at `policy.rs:416` REMOVED — header bit 2 = 1 is now valid (behavioral break tracked as `phase-e-fingerprints-behavioral-break-migration-note` for Phase G). New `tests/fingerprints.rs` (8 tests) + 1 conformance test + 1 unit test = 10 new. Wire format unchanged for no-fingerprints path; vectors verify byte-identical. BIP §"Fingerprints block" gains a normative Privacy paragraph + concrete byte-layout example (`0433033502deadbeefcafebabe0519020232003201` for `wsh(multi(2,@0/**,@1/**))` with two test fingerprints) pinned by `fingerprints_block_byte_layout_matches_bip_example` test. Phase E decision log committed + pushed at `0def1ec` resolved E-1..E-12 in advance. Reviewer (Opus 4.7) `APPROVE_WITH_FOLLOWUPS` with no spec deviations, no algorithmic findings — explicitly verified `key_count()` semantics match BIP MUST, encoder validation order, tag dispatch airtightness, and BIP byte-layout reproducibility.
- **Tier:** v0.2 (closed; breaking — header bit 2 rejection removed; new `Tag::Fingerprints` variant; new `Error::FingerprintsCountMismatch` variant; additive `EncodeOptions::fingerprints` and `DecodeResult.fingerprints` fields)

### `8-negative-fixture-dynamic-generation` — generate negative vectors dynamically by exercising actual error paths

- **Surfaced:** v0.2 carry-forward from `8-negative-fixture-placeholder-strings` closure
- **Status:** resolved `5348b12` (v0.2 Phase F) — schema bumped 1 → 2 (additive). `build_test_vectors_v2()` populates `input_strings` with byte-for-byte exact strings via ~30 per-variant generator functions; each asserts via `debug_assert!` that decode returns the expected variant. Variants that genuinely cannot be triggered via a WDM string (n12 `EmptyChunkList`, n17 `ChunkIndexOutOfRange`, n30 `PolicyTooLarge`, plus the 2 new encode-side rejections from Phase D/E) carry empty `input_strings` with honest `provenance` documenting the lower-level API or encode-side rejection that triggers them. `v0.1.json` LOCKED (SHA `1957b542...` byte-identical); `v0.2.json` NEW at SHA `92f0d5b2f365df38a6b22fcf24c3f0bc493883fd14f1db591f82418c001e0e42` (14 positive + 34 negative). Schema-2 additive fields: `Vector.expected_fingerprints_hex: Option<Vec<String>>` and `Vector.encode_options_fingerprints: Option<Vec<[u8; 4]>>` and `NegativeVector.provenance: Option<String>` — all `serde(default, skip_serializing_if = "Option::is_none")` so schema-1 readers parse v0.2.json cleanly. `gen_vectors` extended with `--schema <1|2>` (default 2 for output; inferred for verify). Reviewer (Opus 4.7) `APPROVE` (cleanest of any v0.2 phase; no FOLLOWUPS).
- **Tier:** v0.2 (closed; breaking-tagged because schema bump but additive enough that schema-1 consumers can still parse v0.2.json)

### `phase-d-taproot-corpus-fixtures` — add tr() positive + negative vectors to CORPUS_FIXTURES

- **Surfaced:** Phase D implementer (Opus 4.7) on commit `6f6eae9`
- **Status:** resolved `5348b12` (absorbed into v0.2 Phase F) — 3 positive taproot entries (`tr_keypath`, `tr_pk`, `tr_multia_2of3`) + 2 negative (`n_tap_leaf_subset`, `n_taptree_multi_leaf`) added to schema-2's `CORPUS_FIXTURES` / `NEGATIVE_FIXTURES`. The `tr_multia_2of3` policy uses `tr(@0/**, multi_a(2,@1/**,@2/**,@3/**))` (4 distinct placeholders) instead of the decisions-doc original (3-key reusing `@0`) because the original fails BIP 388's disjoint-paths constraint — sound in-flight correction by the agent, verified against the `tests/taproot.rs::taproot_single_leaf_multi_a_round_trips` precedent.
- **Tier:** v0.2 (closed)

### `p4-chunking-mode-stale-test-names` — sweep `force_chunked_*` test names + comments to new terminology

- **Surfaced:** Phase A bucket A reviewer (Opus 4.7) on commit `fbbe6ec`
- **Status:** resolved `0ef70f9` (Phase G polish sweep) — renamed 4 test functions (`force_chunked_skips_single_string` → `chunking_mode_force_chunked_skips_single_string` etc.) plus the `force_chunking_opts` test helper (3 call sites) plus inline comments. All sites in `chunking.rs::tests` and `decode.rs::tests`. Functionally no-op; vocabulary aligned with the `ChunkingMode` enum.
- **Tier:** v0.2-nice-to-have (closed)

### `phase-d-tap-decode-error-naming-parity` — encode/decode tap-leaf-subset rejection messages use different operator-name format

- **Surfaced:** Phase D reviewer (Opus 4.7) on commit `6f6eae9`
- **Status:** resolved `0ef70f9` (Phase G polish sweep) — added a new `tag_to_bip388_name(Tag) -> &'static str` helper in `bytecode/decode.rs` covering all 38 tag variants (operator tags + framing tags + reserved-for-v1+ inline-key tags get `<framing:0xNN>` / `<reserved:0xNN>` labels). Replaced `format!("{:?}", other)` (PascalCase: `"Sha256"`) with `tag_to_bip388_name(other).to_string()` (BIP 388 lowercase: `"sha256"`). Encode-side and decode-side rejections of the same out-of-subset operator now surface byte-identical user-facing diagnostics.
- **Tier:** v0.2-nice-to-have (closed)

### `phase-e-encoder-count-cast-hardening` — replace `fps.len() as u8` with `u8::try_from` for defense-in-depth

- **Surfaced:** Phase E reviewer (Opus 4.7) on commit `6559c17`
- **Status:** resolved `0ef70f9` (Phase G polish sweep) — replaced `fps.len() as u8` (gated only on `debug_assert!`) with `u8::try_from(fps.len()).map_err(|_| Error::FingerprintsCountMismatch { ... })?`. Returns a structured error in release mode if the validation funnel is ever bypassed, instead of silently truncating. Currently safe via the validation funnel but defense-in-depth for future refactors.
- **Tier:** v0.2-nice-to-have (closed)

### `phase-c-bch-decode-style-cleanups` — 4 stylistic / micro-opt nits in `encoding/bch_decode.rs`

- **Surfaced:** Phase C reviewer (Opus 4.7) on commit `3aabcf6`
- **Status:** resolved `0ef70f9` + `511e7a9` (Phase G polish sweep + N-3 follow-up) — all 4 nits applied. N-1: `lam.last().unwrap().is_zero()` → `lam.last().is_some_and(|x| x.is_zero())`. N-2: `k.wrapping_sub(i)` + `s_idx < n` guard → explicit `if i <= k && i < lam.len()` with direct `k - i` indexing. N-3 (initially skipped on cost/benefit, applied after user prompt): bumped `polymod_run` visibility in `encoding.rs` from private to `pub(in crate::encoding)`; replaced 15-line local copy in `bch_decode::tests` with `use super::super::polymod_run`; updated 4 call sites; dropped now-unused `POLYMOD_INIT` import. The dedup is **correctness coupling** not style — a future bug in `polymod_run` would be silently masked by a duplicate that agrees on the wrong answer. N-4: replaced `Vec<u8>` allocation in `compute_syndromes` with stack `[u8; 15]` + slice-the-active-prefix.
- **Tier:** v0.2-nice-to-have (closed)

### `wallet-policy-eq-migration-note` — document `WalletPolicy` `PartialEq` semantics around `decoded_shared_path` in MIGRATION.md

- **Surfaced:** Phase A bucket B reviewer (Opus 4.7) on commit `86ca5df`
- **Status:** resolved `548dc10` (Phase G `MIGRATION.md` write) — Phase G's `MIGRATION.md` §2 documents the breaking change with before/after code examples and recommends `.to_canonical_string()` for construction-path-agnostic equality.
- **Tier:** v0.2-nice-to-have (closed)

### `phase-b-encode-signature-and-copy-migration-note` — document Phase B breaking changes in MIGRATION.md

- **Surfaced:** Phase B bucket B reviewer (Opus 4.7) on commit `0993dc0`
- **Status:** resolved `548dc10` (Phase G `MIGRATION.md` write) — `MIGRATION.md` §1 documents both breaking changes (`to_bytecode` signature; `EncodeOptions: !Copy`) with before/after code examples and migration recipes (`&EncodeOptions::default()` for no-override callers; explicit `.clone()` for callers assuming `Copy`).
- **Tier:** v0.2-nice-to-have (closed)

### `phase-e-fingerprints-behavioral-break-migration-note` — document v0.1→v0.2 fingerprints rejection removal in MIGRATION.md

- **Surfaced:** Phase E decision log E-9 (deferred at dispatch)
- **Status:** resolved `548dc10` (Phase G `MIGRATION.md` write) — `MIGRATION.md` §3 documents the behavioral break (header bit 2 = 1 no longer fires `PolicyScopeViolation`) and recommends inspecting `WdmBackup.fingerprints` / `DecodeResult.fingerprints` directly for fingerprints-aware caller code.
- **Tier:** v0.2-nice-to-have (closed)

### `p4-with-chunking-mode-builder` — additive `EncodeOptions::with_chunking_mode(ChunkingMode)` builder

- **Surfaced:** Phase A bucket A dispatch (deferred per controller); reaffirmed by reviewer
- **Status:** resolved in v0.2.1 — `pub fn with_chunking_mode(mut self, mode: ChunkingMode) -> Self` added to `EncodeOptions` alongside the preserved `with_force_chunking(bool)` shim. Rustdoc cross-references both forms; new code prefers the typed enum, the bool shim stays for v0.1.1 source-compat.
- **Tier:** v0.2-nice-to-have (closed)

### `phase-e-cli-fingerprint-flag` — `wdm encode --fingerprint @i=<hex>` CLI flag

- **Surfaced:** Phase E decision log E-10 (deferred at dispatch)
- **Status:** resolved in v0.2.1 — `wdm encode --fingerprint @INDEX=HEX` repeatable flag added to `bin/wdm/main.rs::cmd_encode`. New `parse_fingerprint_arg` + `parse_fingerprints_args` helpers validate hex format (8 chars, optional `0x` prefix), index format (`@N` or `N`), and that supplied indices cover `0..N` with no gaps + no duplicates. CLI prints a stderr privacy warning whenever the flag is used per BIP §"Fingerprints block" Privacy paragraph (recovery tools MUST warn). 3 new CLI integration tests in `tests/cli.rs` (happy path + index-gap rejection + short-hex rejection).
- **Tier:** v0.2-nice-to-have (closed)

### `vectors-generator-string-patch-version-churn` — vector file SHA churns on every patch bump because `generator` field embeds full version

- **Surfaced:** v0.2.1 release prep (2026-04-28); v0.2.json regen produced a different SHA only because `generator: "wdm-codec 0.2.0"` → `"wdm-codec 0.2.1"`, despite byte-identical wire format and corpus.
- **Status:** resolved in v0.2.1 — `pub const GENERATOR_FAMILY: &str = "wdm-codec <major>.<minor>"` added to `vectors.rs` via `concat!` of `CARGO_PKG_VERSION_MAJOR` / `_MINOR`. Both v1 and v2 builders use this. `gen_vectors --output` logs the full crate version to stderr for traceability. v0.2.json regen now produces SHA `b403073b8a925bdda37adb92daa8521d527476aa7937450bd27fcbe0efdfd072` — stable across the entire 0.2.x patch line. (v0.2.0 SHA `3c208300...` remains correct for the v0.2.0 tag; consumers pinning it experience a one-time migration at v0.2.1 then no further churn.)
- **Tier:** v0.2-nice-to-have (closed; was originally filed as v0.3 but applied during v0.2.1 prep per user direction)

### `phase-5-cli-wdm1-assertion-sweep` — sweep `"wdm1"` string literals in tests/cli.rs

- **Surfaced:** Phase 4 (identifier mass-rename) code-quality reviewer (Important #1)
- **Status:** resolved `12da91f` (Phase 5 wire-format string literal sweep — HRP `wdm`→`md`); zero `wdm1` string literals remain in `crates/md-codec/tests/cli.rs` (verified post-rename).
- **Tier:** v0.3-blocker (closed)

### `slip-0173-register-md-hrp` — file SLIP-0173 PR registering `md` HRP

- **Surfaced:** Pre-flight Gate 1 of the wdm→md rename (HRP collision vet)
- **Status:** resolved 2026-04-28 — PR filed at https://github.com/satoshilabs/slips/pull/2011. The requested action (FILE the PR) is complete; merge state is now tracked externally on SatoshiLabs review cadence and is no longer an MD-side deferral.
- **Tier:** external (closed; awaiting upstream merge tracked separately)

### `v0-5-spec-section-3-helper-snippet-missing-per-leaf-gate` — spec §3 decoder-helper snippet omits the leaf-subset validation call

- **Surfaced:** Phase 2 spec compliance reviewer (mid-execution, returned to controller; not persisted at the time)
- **Status:** resolved `6aef662` (Pass-1 housekeeping batch) — added `validate_tap_leaf_subset(&leaf, Some(index))?;` call to the §3 `decode_tap_subtree` helper sketch in `design/SPEC_v0_5_multi_leaf_taptree.md` so the spec matches the working implementation at decode.rs:802.
- **Tier:** v0.5-nice-to-have (closed)

### `v0-5-decode-rs-comment-stale-task-number-references` — code comments reference plan-task numbers that will rot

- **Surfaced:** Phase 2 code-quality reviewer (mid-execution, not persisted at the time)
- **Status:** resolved `6aef662` (Pass-1 housekeeping batch) — replaced "see Task 2.3" / "see Task 2.6+2.8" plan refs at `encode.rs:529` and `decode.rs:728` with stable function-name anchors. Note: ~25 other `Task X.Y` references remain in the same files, but they are test-section organizational headers (e.g., `// --- Wsh inner / Terminal leaf round-trips and rejections (Task 2.13) ---`), a different category than the "see Task X for context" cross-refs the reviewer flagged. Broader sweep deliberately deferred — not in original scope.
- **Tier:** v0.5-nice-to-have (closed)

### `v0-5-decode-rs-module-doc-version-prefix-relax` — module-level rustdoc keeps "v0.5" prefixes that will read awkwardly post-release

- **Surfaced:** Final cumulative reviewer (Phase 9) M4 — explicitly marked optional
- **Status:** resolved `6aef662` (Pass-1 housekeeping batch) — replaced four chronologically-tangled "v0.X scope:" paragraphs (v0.1, v0.2, v0.5, v0.4) in `crates/md-codec/src/bytecode/decode.rs` module rustdoc with a single version-agnostic description of accepted top-level descriptors and TapTree decoding. Same approach as the earlier v0.4→v0.5 stale-strings sweep.
- **Tier:** v0.5-nice-to-have (closed)

---

## Convention notes for future agents

If you are an implementer or reviewer subagent dispatched on a task and you identify **minor items** (Important or Minor severity per the standard review rubric) that you are NOT fixing in your own commit, append an entry to this file in the same commit. Use a `<short-id>` like `<phase>-<keyword>` (e.g., `6c-corpus-fixture-helper`, `8a-vectors-schema-comment`).

If you are running in a **parallel batch** with sibling agents, do NOT write to this file directly — return your follow-up items in your final report and the controller will append them. Two parallel agents writing here cause merge conflicts.

If you are **closing** an item, edit its entry from `Status: open` → `Status: resolved <COMMIT>` and move the entry to the "Resolved items" section. Don't delete entries.
