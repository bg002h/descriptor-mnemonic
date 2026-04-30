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

### `wallet-id-is-really-template-id` — current `WalletId` identifies a policy template, not a wallet instance

- **Surfaced:** 2026-04-29 mk1 design discussion. While drafting `design/mk/SPEC_mk_v0_1.md` §5 (recovery-flow linkage between policy card and key cards), the user observed that md-encoded wallet descriptors are themselves reusable across wallets and may have zero-many actual wallet instances associated with them. The `WalletId = SHA-256(canonical_bytecode)[0..16]` is computed over the BIP 388 *template* (with `@N` placeholders), not over the assembled descriptor with concrete cosigner xpubs. Therefore two distinct wallets that share an identical policy template (same multisig shape + same shared path, different cosigner sets) produce identical md1 wallet IDs. The "wallet ID" name is misleading; what we have is really a "policy ID" or "template ID."

- **Implications:**
  - **md1 cards** have a many-to-many relationship with actual wallets: zero (engraved speculatively for "I might use this template"), one (instantiated for a single wallet), or many (re-used across multiple wallets that happen to share the template). The wallet-ID-as-anchor framing in the BIP draft and the `wallet_id` rustdoc treats this as one-to-one, which is wrong.
  - **mk1 wallet-ID stubs** (per `design/mk/SPEC_mk_v0_1.md` §3.3) inherit the same naming. The stub on a key card identifies the *template* the xpub serves, not a unique wallet instance. The mk SPEC's §5 recovery flow already does the right thing (cryptographic match happens at descriptor reassembly, not at stub match), but the WORDING calls the stub a "wallet ID" which is misleading.
  - **The "wallet ID anchor"** described in the MD BIP draft (the optional 12-word BIP 39 mnemonic engraved on a separate medium for cross-verifying a digital backup) is anchoring the *template*, not the wallet. Users who hold two wallets at the same template would have two physical anchor cards with identical 12-word phrases — confusing at best.

- **Where:**
  - `crates/md-codec/src/wallet_id.rs` (canonical computation: `compute_wallet_id`, `WalletId` type)
  - `bip/bip-mnemonic-descriptor.mediawiki` §"Wallet identifier (Tier 3)" (line ~679 in current draft)
  - `crates/md-codec/src/chunking/...` (chunk-header `wallet_id` stub field)
  - `design/mk/SPEC_mk_v0_1.md` §3.3, §5 (mk1 wallet-ID stubs and recovery flow)
  - `bip/bip-mnemonic-key.mediawiki` (TBD; mk1 BIP draft will inherit the rename)

- **Proposed v0.8 shape** (agreed in 2026-04-29 design session):

  1. **Rename** the existing 16-byte template-only hash from `WalletId` → `PolicyId` (or `TemplateId`; bikeshed at v0.8 implementation time). Pure naming change in code, rustdoc, spec text, and BIP draft. The 16-byte value, the 4-byte chunk-header stub, and the 12-word BIP 39 anchor encoding are all **unchanged**; only the name and the framing shift. No wire-format break.

  2. **Define** a new derived quantity:

     ```text
     WalletInstanceId = SHA-256(canonical_bytecode || canonical_xpub_serialization)
     ```

     where `canonical_xpub_serialization` is the concatenation of each `@N`-placeholder's resolved xpub in placeholder-index order, in BIP 32 78-byte serialization form. The `WalletInstanceId` is **defined in the BIP** (so every tool computes it the same way) but is NOT carried by any physical card or wire structure. It's a recovery-time derivation: tools that have the policy card + the cosigners' xpubs (whether from md1 + mk1 cards, from a digital descriptor backup, or from the wallet itself) can compute it on demand.

     No new physical Tier-3 anchor for `WalletInstanceId` is introduced. The existing BIP 39 12-word phrase anchor remains a `PolicyId` anchor, with renamed framing.

  3. **Use `WalletInstanceId` in mk SPEC §5** recovery flow. The current mk1 spec recomputes the policy ID at step 4 to verify reassembly; the v0.8 update changes step 4 to compute and verify the `WalletInstanceId` — which is the actual cryptographic check that distinguishes "this xpub set, plugged into this template, produces this exact wallet" from "some other xpub set in the same template family." The stub-match in step 2 stays exactly as is (template-level filter, fast).

- **Why deferred:** Naming change is mechanical but touches many surfaces (code identifiers, rustdoc, spec text, BIP draft, in-flight mk1 design). The new `WalletInstanceId` definition is small (one paragraph in the BIP, ~20 lines of derived helper in md-codec) but should land alongside the rename so the conceptual split is introduced atomically rather than across two releases.

- **Status:** resolved md-codec-v0.8.0. All three steps of the proposed v0.8 shape landed atomically: (1) `WalletId` → `PolicyId` rename across ~720 references and ~40 files (code, rustdoc, spec, BIP draft, README, CHANGELOG, MIGRATION); (2) new `WalletInstanceId` 16-byte derived identifier with `pub fn compute_wallet_instance_id(canonical_bytecode, xpubs)` helper, three unit tests, and a new `===Wallet Instance ID===` BIP draft section; (3) the mk1 SPEC §5 recovery-flow step-4 update lands in the sibling `bg002h/mnemonic-key` repo (not in this commit; doc-only follow-up over there). Wire format byte-identical to v0.7.x; vector files regenerated under `"md-codec 0.8"` family token.
- **Tier:** v0.8 (closed)

### `chunk-set-id-rename` — rename "wallet identifier" → `chunk_set_id` in md1

- **Surfaced:** 2026-04-29 mk1 v0.1 closure-design pass (Q-5 / D-15). Companion entry: `chunk-set-id-rename` in `bg002h/mnemonic-key` `design/FOLLOWUPS.md`.
- **Where:**
  - `bip/bip-mnemonic-descriptor.mediawiki` §"Header" line ~188 ("Wallet identifier (4 chars, 20 bits)…")
  - `crates/md-codec/src/chunking/...` (chunked-header field name, getters, error messages)
  - `crates/md-codec/src/...` (any other "wallet identifier" / `wallet_identifier` symbol references)
  - `MIGRATION.md` and CHANGELOG (rename note for the docs-only release)
- **What:** md-codec v0.8.x ships the chunked-string-header 20-bit per-encoding random tag under the name "wallet identifier." Per the closure-design naming review the name conflicts with `Policy ID` and `Wallet Instance ID` and means neither — it identifies the chunk-set assembly, nothing more. Rename to `chunk_set_id` across the BIP, code, and docs. Wire format unchanged; this is purely a documentation and code-symbol rename, suitable for a docs-and-symbols-only release (proposed: md-codec v0.9.0).
- **Why deferred:** Surfaced after v0.8.0 ship; needs its own docs-only release. The mk1 BIP submission is gated on this rename landing first (mk1 cannot publish referencing a name md1 itself does not use).
- **Sequencing requirement:** This rename is a hard precondition for mk1's formal BIP submission. Should be released before mk1's `bip-cross-reference-completeness` audit closes.
- **Status:** resolved md-codec-v0.9.0. Renamed `ChunkPolicyId → ChunkSetId`, `PolicyIdSeed → ChunkSetIdSeed`, `EncodeOptions::policy_id_seed → chunk_set_id_seed`, error variants `PolicyIdMismatch → ChunkSetIdMismatch` / `ReservedPolicyIdBitsSet → ReservedChunkSetIdBitsSet`, struct fields `ChunkHeader::Chunked.policy_id → chunk_set_id` / `Chunk.policy_id → chunk_set_id` / `Verifications.policy_id_consistent → chunk_set_id_consistent`, plus all related test helpers and prose (~85 sites; ~150 references). BIP §"Wallet identifier" → §"Chunk-set identifier" with v0.8→v0.9 naming-note. Wire format byte-identical to v0.8.x; test-vector corpora regenerate due to rename of `expected_error_variant` strings and family-token roll. mk1's BIP-submission gate is now cleared.
- **Tier:** v0.9 (closed)

### `md-path-dictionary-0x16-gap` — add missing testnet 0x16 entry to path dictionary

- **Surfaced:** 2026-04-29 mk1 v0.1 Phase 2 BIP review (commit `4728230` in `bg002h/mnemonic-key`). Companion: `md-path-dictionary-0x16-gap` in `bg002h/mnemonic-key` `design/FOLLOWUPS.md`.
- **Where:**
  - `bip/bip-mnemonic-descriptor.mediawiki` §"Path dictionary" lines ~339-349 — testnet rows list 0x11, 0x12, 0x13, 0x14, 0x15, 0x17 with no 0x16.
  - `crates/md-codec/src/bytecode/path/` — `Tag::SharedPath` indicator dictionary in code (mirror).
- **What:** Mainnet has indicator 0x06 (`m/48'/0'/0'/1'`, BIP 48 nested-segwit multisig) but the testnet companion 0x16 (`m/48'/1'/0'/1'`) is absent from md1's published BIP table and code. This is plausibly an oversight rather than a deliberate omission — every other mainnet path family has a testnet pair, and BIP 48 nested-segwit is the only one missing on the testnet side. mk1 v0.1 inherits the gap by its byte-for-byte mirror clause and currently documents `0x16` as reserved-pending-md1-update; closing the gap on the md1 side lets mk1 inherit cleanly.
- **Why deferred:** Single-row dictionary addition, low complexity, but is technically a wire-format extension (existing decoders treat 0x16 as reserved). Suitable for the next md-codec release that touches the path dictionary.
- **Status:** resolved md-codec-v0.9.0. Added `0x16 = m/48'/1'/0'/1'` to dictionary in `crates/md-codec/src/bytecode/path.rs` (DICT array, FIXTURE table, both negative-test arrays adjusted, dictionary array size 13 → 14). BIP §"Path dictionary" gains the new row between `0x15` and `0x17`. New TDD-style positive test `indicator_0x16_decodes_to_bip48_testnet_p2sh_p2wsh` plus corpus vector `t1_sh_wsh_testnet_0x16` (schema 2) exercising the indicator via `EncodeOptions::with_shared_path("m/48'/1'/0'/1'")`. Verified single-byte `0x16` encoding (not explicit-path fallback) per opus P2 review Q7.
- **Tier:** v0.9 (closed)

### `path-dictionary-mirror-stewardship` — formalize the md1↔mk1 path-dictionary inheritance contract

- **Surfaced:** 2026-04-29 mk1 v0.1 Phase 2 BIP review open observation (commit `4728230` in `bg002h/mnemonic-key`). Companion: `path-dictionary-mirror-stewardship` in `bg002h/mnemonic-key` `design/FOLLOWUPS.md`.
- **Where:** Process / cross-repo coordination. Likely lands as a one-paragraph note in `bip/bip-mnemonic-descriptor.mediawiki` §"Path dictionary" plus a parallel note in mk1's spec, plus a CHANGELOG / RELEASE_PROCESS checklist item.
- **What:** mk1's path dictionary is contractually identical to md1's `Tag::SharedPath` table ("byte-for-byte mirror"). Today this is a prose statement in mk1's spec/BIP, not a tracked invariant. If md1 allocates new dictionary entries (e.g., closing the 0x16 gap or adding new BIP-style accounts in a future md1 release), mk1 inherits the allocation by the mirror clause — but a future md1 release could land a path-dictionary change without an mk1 spec amendment, producing silent drift. Formalize the inheritance contract: bidirectional release-checklist item that "if path dictionary changes, both repos must update in lockstep before either ships."
- **Why deferred:** Process / stewardship concern, not blocking any specific release. Becomes load-bearing the next time either repo touches the path dictionary.
- **Status:** resolved md-codec-v0.9.0. Created `design/RELEASE_PROCESS.md` documenting the lockstep-checklist invariant (any path-dictionary change requires a coordinated mk1 spec amendment in the same release window), CLAUDE.md crosspointer maintenance rules, SHA pin / family-generator practice, and a 16-step standard release checklist. BIP §"Path dictionary" gains a "Cross-format inheritance" paragraph pointing readers to the release-process doc.
- **Tier:** external (closed — invariant now tracked)

### `tag-sharedpath-rustdoc-stale-0x33` — `path.rs` rustdoc says `Tag::SharedPath` is `0x33`; actual is `0x34`

- **Surfaced:** opus P2 review of v0.9.0 (commit `e622540`, see `design/agent-reports/v0-9-phase-2-review.md` finding F2).
- **Where:** `crates/md-codec/src/bytecode/path.rs` lines 63, 168, 170, 171, 188, 199, 260 — rustdoc references claim `Tag::SharedPath` is `0x33`.
- **What:** Actual value is `0x34` (per `crates/md-codec/src/bytecode/tag.rs:122`). The v0.5→v0.6 renumber bumped `Placeholder → 0x33` and `SharedPath → 0x33 → 0x34`, but `path.rs` rustdoc was not swept. Cosmetic but misleading for anyone reading the rustdoc to understand wire-format byte positions. No functional impact; the actual encoder/decoder uses the correct `Tag::SharedPath.as_byte()`.
- **Why deferred:** Pre-existing in v0.6+; surfaced during P2 review but P2-orthogonal. Trivial sed `s/SharedPath\` (`0x33`)/SharedPath\` (`0x34`)/g` plus a few prose forms. Suitable for any v0.9.x housekeeping window.
- **Status:** resolved md-codec-v0.9.1. Eight rustdoc sites updated; `cargo doc` clean.
- **Tier:** v0.9.x (closed)

### `policy-compiler-rustdoc-broken-link` — `Concrete::compile_tr(unspendable_key)` intra-doc-link warning

- **Surfaced:** Observed during cargo doc runs in v0.7.2+ and again during v0.9.0 P1 work. Pre-existing rustdoc warning unrelated to v0.9 changes.
- **Where:** `crates/md-codec/src/policy_compiler.rs:19` — `[\`Concrete::compile_tr(unspendable_key)\`]` rustdoc link.
- **What:** rustdoc emits `warning: unresolved link to \`Concrete::compile_tr(unspendable_key)\`` because the `(unspendable_key)` parameter notation isn't valid intra-doc syntax (parens are interpreted as a method-disambiguator, which fails to resolve). Lines 78 and `bin/md/main.rs:143` use the same prose but without bracket-link syntax, so they don't warn.
- **Fix:** drop the bracket form on line 19 (keep just backticks: `` \`Concrete::compile_tr(unspendable_key)\` ``), or rewrite as a proper link to `[\`miniscript::policy::Concrete::compile_tr\`]` and move the `(unspendable_key)` clarification into surrounding prose.
- **Why deferred:** Pre-existing pre-v0.9; doesn't block CI (warning, not error). Catches whenever the implementer next runs `cargo doc`.
- **Status:** resolved md-codec-v0.9.1. Dropped bracket-link form on `policy_compiler.rs:19`; kept code-formatting backticks. `cargo doc` now warning-free.
- **Tier:** v0.9.x (closed)

### `reproducible-builds` — bit-for-bit reproducible builds

- **Surfaced:** 2026-04-29 conversation post-md-codec-v0.9.0 ship. User asked whether reproducible builds are achievable and when to implement.
- **Where:**
  - `rust-toolchain.toml` (new, repo root) — pin exact rustc version + components
  - `.cargo/config.toml` (new, repo root) — `--remap-path-prefix`, deterministic codegen flags
  - `flake.nix` or `Dockerfile.repro` (new, v1.0 milestone) — hermetic build environment
  - `.github/workflows/repro-build.yml` (new, v1.0) — CI job that builds twice and diffs
  - README + RELEASE_PROCESS — verification recipe for end users
- **What:** Two-phase plan.
  1. **Cheap wins (any time, ~30 min).** Pin the rust toolchain via `rust-toolchain.toml` so all builds use the exact same compiler. Add `.cargo/config.toml` with `--remap-path-prefix=$(pwd)=.` and `-C codegen-units=1` so binaries don't bake in the build path or vary by parallelism. These are no-ops for normal contributors, modest improvement for any auditor diffing two builds. Cargo.lock is already committed; SHA-pinned test vectors already demonstrate reproducibility-aware design at the test-data level.
  2. **Full hermetic build (v1.0 milestone).** Add a Nix flake (or pinned Docker image) that fixes every input — toolchain, system libraries, build env — so any auditor can produce the same binary as the published release. Add a CI job that builds twice on different runners and asserts byte-identical output (Linux only initially; macOS/Windows linker reproducibility is a long tail). Document a verification recipe in README so users can verify a release tag against a published binary. Pairs naturally with v1.0 API+wire stability — once those settle, it's worth the trust statement.
- **Why deferred:** Pre-v1.0, the wire format and public API are still moving (cf. v0.7→v0.8→v0.9 rename cascade). Building hermetic-build infrastructure now means re-doing it each time a major release breaks something. Auditors don't typically care about pre-1.0 reproducibility either — there are too many other moving parts. The cheap pins are different: they're free now and accumulate value, so they can land in any housekeeping window before v1.0.
- **Status:** phase 1 resolved md-codec-v0.9.1 (added `rust-toolchain.toml` pinning rustc 1.85.0 to match CI; added `.cargo/config.toml` with `[profile.release]` `codegen-units = 1` and `strip = "symbols"`). Phase 2 (hermetic Nix/Docker + repro-CI + verification recipe) remains open as a v1.0 milestone item.
- **Tier:** phase 1 closed (v0.9.1); phase 2 open (v1.0 milestone)

### `walletinstanceid-rendering-parity` — should `WalletInstanceId` (Type 0) get a BIP-39 word rendering parallel to `PolicyId`?

- **Surfaced:** 2026-04-29 v0.10 brainstorm conversation (post-Q11). Asked while exploring the Type 0 / Type 1 PolicyId typology framing.
- **Where:** `crates/md-codec/src/policy_id.rs` — `WalletInstanceId` currently has only `Display` (32 hex chars), no `to_words()`. `PolicyId` has both.
- **What:** v0.8.0 added `WalletInstanceId` as a recovery-time derivation, deliberately omitting BIP-39 rendering because Type 0 has no engraving use case (it's derivable from inputs the user already has — bytecode from md1 + xpubs from mk1/seeds — so engraving the output is redundant). Adding `WalletInstanceId::to_words()` is technically trivial (~5 lines; same BIP-39 input shape as `PolicyId`) but would imply an engraving use case that doesn't currently exist, inviting users to engrave Type 0 phrases under false impressions.
- **When this becomes load-bearing:** if a real workflow surfaces that wants Type 0 in 12-word form (e.g., "engrave the assembled-wallet instance fingerprint as a stronger anchor than just template fingerprint" — useful for foreign-xpub-multisig recovery where you want to verify the *complete* wallet, not just the template). At that point also reopen the type-tagging question (HRP-prefix codex32 vs BIP-39, since rendering Type 0 alongside Type 1 in the same encoding scheme would create user-confusion risk).
- **Why deferred:** No current workflow requires it. Adding it speculatively reinforces the wrong mental model (suggesting Type 0 is engrave-worthy). YAGNI.
- **Status:** open
- **Tier:** v1+ (or wont-fix if no Type 0 engraving workflow ever surfaces)

### `v2-design-questions` — clean-slate questions to revisit at a major redesign

- **Surfaced:** 2026-04-29 v0.10 brainstorm conversation. While locking Q6 (Tag::Fingerprints vs Tag::OriginPaths separation), the question came up: "if we were starting the format from scratch today, would we pick something different?" Yes — and the same question generalizes to other accumulated design choices.
- **Where:** Conceptual / cross-cutting. Lives as a v2+ scoping document if/when major-redesign is on the table. Not tied to any specific code path.
- **What:** This entry catalogs design choices that v0.x ships with for good reason (path dependence, accumulated constraints, organic evolution) but that a clean-slate v2.0 redesign would likely re-evaluate. Capturing them now prevents future sessions from reinventing the analysis.

  Specific questions to revisit:

  1. **Unified per-`@N` metadata block.** Today md1 has three different ways to express path/fingerprint metadata (`Tag::SharedPath`, `Tag::Fingerprints`, `Tag::OriginPaths` from v0.10). A fresh design would likely use one block carrying `(fingerprint, path, ...)` tuples per `@N`, with optional shared-path / shared-fingerprint optimizations layered on top. Wire format closer to BIP 380's `[fp/path]xpub` origin-block structure.

  2. **Path dictionary.** v0.x has a 14-entry dictionary of well-known BIP paths (`0x01`–`0x17`) plus explicit-path encoding via `0xFE`. A fresh design might choose: always-explicit (simpler decoder, bigger wire), or a more aggressive dictionary (more compact for common cases, but more allocation churn).

  3. **md1 + mk1 unification.** v0.x ships md1 and mk1 as twin formats with shared BCH plumbing (codex32 + HRP-mixing). A fresh design might unify them into a single "MC" format with one HRP and a top-level type discriminator, eliminating the cross-format coordination overhead. Or — going the other way — split further, with separate HRPs per use case.

  4. **Bytecode encoding uniformity.** v0.x mixes varints, LEB128, and fixed-width u8 fields somewhat ad-hoc. A fresh design might pick one (probably LEB128 throughout for unsigned integers; fixed-width for byte arrays) and apply uniformly.

  5. **String-layer vs bytecode-layer metadata split.** Today most metadata lives in the bytecode layer; the string-layer header carries only chunking metadata. A fresh design might move some per-`@N` metadata directly into the string-layer header for streaming-decoder friendliness.

  6. **BCH polynomial domain separation.** v0.x reuses BIP 93 polynomials with HRP-mixing for cross-format domain separation (md1, mk1, ms1 all share polynomials). A fresh design might use distinct polynomials per format (more rigorous separation; complicates the shared `mc-codex32` extraction targeted at v1.0+).

  7. **Tag space layout.** v0.x has tag bytes scattered across `0x00`–`0x35` (now `0x36+` post-v0.10) due to organic accumulation. A fresh design might reserve byte ranges semantically — operators 0x00-0x3F, framing 0x40-0x4F, etc.

  8. **Header version field width.** v0.x uses 4 bits for version (allowing 16 future versions). A fresh design might use 8 bits or none (using the format-name itself for major-version discrimination).

- **Why deferred:** None of these affect v0.10 (or v0.11, v0.12, v1.0). They're meta-design questions worth capturing once and revisiting if a major redesign is ever contemplated. Pursuing any one of them now would mean re-doing wire-format work each iteration, which the project has explicitly avoided pre-v1.0.
- **Status:** open
- **Tier:** v2+ (or wont-fix if v2.0 is never contemplated; the analysis here remains useful as design rationale either way)

### `rust-miniscript-multi-a-in-curly-braces-parser-quirk` — concrete-key `multi_a(...)` inside `tr({...})` fails to parse

- **Surfaced:** Phase 6 implementer (commit `7d6e278`). T6 fixture's plan-prescribed concrete-key policy string failed to parse via rust-miniscript's wallet-policy parser; switched to the `@N`-template form which parses cleanly and matches existing `vectors.rs` convention.
- **Where:** rust-miniscript's wallet-policy parser; not a direct md-codec issue
- **What:** Concrete-key form `tr(<concrete>, {pk(<concrete>), multi_a(2, <concrete>, <concrete>)})` fails; `@N`-template form `tr(@0/**, {pk(@1/**), multi_a(2, @2/**, @3/**)})` works. Possibly an upstream parser bug or a documented limitation.
- **Why deferred:** Workaround is sound (use template form, which matches the rest of the corpus). Not blocking md-codec v0.5.
- **Status:** open
- **Tier:** v1+ (file as upstream issue if desired; not on md-codec critical path)

### `p2-inline-key-tags` — Reserved tags 0x24–0x31 (descriptor-codec inline-key forms)

- **Surfaced:** Phase 2 D-2 (`design/PHASE_2_DECISIONS.md`)
- **Where:** `crates/wdm-codec/src/bytecode/{tag,encode,decode}.rs`
- **What:** Tags `0x24..=0x31` are reserved by descriptor-codec for inline-key forms (raw xpubs, key origins, wildcards). v0.1 rejects them per BIP-388 wallet-policy framing. v1+ may expose them for foreign-xpub support if/when WDM extends beyond pure BIP-388.
- **Why deferred:** v0.1 spec scope.
- **Status:** wont-fix — out of scope per design (2026-04-28 decision: drop the Reserved\* range entirely as part of `md-tag-space-rework`; MD's BIP 388 wallet-policy framing explicitly forbids inline keys, and the descriptor-codec inline-key-form vendoring is dead weight relative to MD's stated scope. Engravable steel backup is incompatible with raw-xpub or full-pubkey encoding by size alone — a separate format with its own HRP would be the right home if anyone ever wanted that, not a v1+ extension of MD)
- **Tier:** v1+ → wont-fix

### `external-pr-1-hash-terminals` — apoelstra/rust-miniscript PR #1

- **Surfaced:** Phase 5-B; submitted 2026-04-27
- **Where:** https://github.com/apoelstra/rust-miniscript/pull/1
- **What:** PR fixing `WalletPolicyTranslator` to support hash terminals (sha256/hash256/ripemd160/hash160). Until merged, our workspace `[patch]` redirects to a local clone of the patched fork.
- **Why deferred:** waiting for upstream maintainer review.
- **Status:** open
- **Tier:** external

### `external-pr-2-template-accessor` — apoelstra/rust-miniscript PR #2

- **Surfaced:** v0.10 follow-up 2026-04-29; submitted same day
- **Where:** https://github.com/apoelstra/rust-miniscript/pull/2
- **What:** PR adding `WalletPolicy::template() -> &Descriptor<KeyExpression>` and `WalletPolicy::key_info() -> &[DescriptorPublicKey]` accessors so external consumers (specifically md-codec's per-`@N` divergent-path encoder) can walk a policy in placeholder-index order without consuming it via `into_descriptor()`. Re-exports `KeyExpression`/`KeyIndex` at `miniscript::descriptor::*`. The motivating case is multipath-shared `@N` placeholders (e.g. `sh(multi(1,@0/**,@0/<2;3>/*))`) where `into_descriptor()` erases placeholder identity.
- **Why deferred:** waiting for upstream maintainer review. Until merged, the workspace `[patch]` redirects to the local `md-codec-local-stack` branch in the `rust-miniscript-fork` sibling clone, which stacks this commit on top of `fix/wallet-policy-hash-terminals` (PR #1).
- **Coordination:** PR #1 (`external-pr-1-hash-terminals`) and PR #2 are independent — either can land first. The `md-codec-local-stack` branch is local-only by design (pushing would pollute the per-PR branches). When PR #2 merges, strip its commit from `md-codec-local-stack` and update the workspace `[patch]` rationale; when both PRs merge, the `[patch]` block can be removed and the SHA pin bumped to upstream.
- **Downstream value:** unblocks `v010-p3-tier-2-kiv-walk-deferred` (the v0.10.0 Tier 2 KIV walk stub), targeted for v0.10.1 or v0.11.
- **Status:** open
- **Tier:** external

### `phase-d-tap-leaf-wrapper-subset-clarification` — widen the tap-leaf wrapper subset if signers document broader safe support

- **Surfaced:** Phase D implementer (Opus 4.7) on commit `6f6eae9`
- **Where:** `crates/wdm-codec/src/bytecode/encode.rs::validate_tap_leaf_subset`
- **What:** Phase D allows only `c:` and `v:` wrapper terminals in tap leaves (BIP 388 parser emits both implicitly when expanding `pk(K)` and `and_v(v:..., ...)`). All other wrappers (`a:`/`s:`/`d:`/`j:`/`n:`/`u:`/`l:`/`t:`) are rejected. If hardware signers (Coldcard, others) document broader safe support for additional wrappers, widen the subset and update both encode-side and decode-side validators.
- **Why deferred:** v0.2 errs on the side of strict per the BIP MUST clause; widening requires evidence from real signers.
- **Status:** wont-fix — superseded by `md-scope-strip-layer-3-signer-curation` (the broader meta-question dissolves: MD no longer curates a signer-specific admit set, so per-wrapper widening decisions are no longer MD's concern; named signer subsets move to `md-signer-compat-checker-separate-library`)
- **Tier:** v0.3 → wont-fix

### `phase-d-tap-miniscript-type-check-parity` — full Tap-context type-check rules beyond the named subset

- **Surfaced:** Phase D implementer (Opus 4.7) on commit `6f6eae9`
- **Where:** `crates/wdm-codec/src/bytecode/encode.rs::validate_tap_leaf_subset` (and downstream — full type-check parity may need its own module)
- **What:** Phase D's subset filter accepts any `Terminal` from the named operator set (`PkK`/`PkH`/`MultiA`/`OrD`/`AndV`/`Older` plus `c:`/`v:` wrappers) without re-running miniscript's full Tap-context type-check. Coldcard and other signers may enforce more than just the operator-name set (e.g., satisfaction-cost bounds, dust-amount minimums). Full type-check parity with deployed signers is out of v0.2 scope; consider adding a `validate_tap_leaf_full()` wrapper that re-runs miniscript's Tap-context type-check + any signer-specific extras.
- **Why deferred:** the operator-name subset matches the BIP MUST clause and is sufficient for the v0.2 ship target; full parity is a tighter contract than the BIP requires.
- **Status:** wont-fix — superseded by `md-scope-strip-layer-3-signer-curation` (signer-specific type-check parity is no longer MD's concern; if implemented at all it lives in `md-signer-compat-checker-separate-library` as part of a named signer subset's validation logic)
- **Tier:** v0.3 → wont-fix

### `tap-leaf-admit-sortedmulti-a` — admit `sortedmulti_a` in tap leaves (signer evidence available)

- **Surfaced:** 2026-04-28 evidence-gathering session in response to `phase-d-tap-leaf-wrapper-subset-clarification`. Two independent hardware-signer vendors document admittance:
  - **Coldcard** (firmware/edge branch): `docs/taproot.md` §"Allowed descriptors" lists `tr(internal_key, sortedmulti_a(2,@0,@1))` and `tr(internal_key, {sortedmulti_a(2,@0,@1),pk(@2)})` as admitted shapes.
  - **Ledger** (LedgerHQ/vanadium): `apps/bitcoin/common/src/bip388/cleartext.rs` has first-class `SortedMultisig` variant in the BIP 388 wallet-policy validator.
  Plus rust-miniscript's `VALID_TEMPLATES` test fixture (`src/descriptor/wallet_policy/mod.rs:351`) and a working address-invariance example (`examples/xpub_descriptors.rs::p2tr_sortedmulti_a`).
- **Where:** `crates/md-codec/src/bytecode/tag.rs` (allocate new Tag — currently no wire-format slot exists; `Terminal::SortedMultiA` falls through to a literal `"sortedmulti_a"` string in `tap_terminal_name`); `crates/md-codec/src/bytecode/{encode,decode}.rs` (loosen `validate_tap_leaf_subset` + add round-trip path); `crates/md-codec/src/vectors.rs` (positive corpus fixture); `bip/bip-mnemonic-descriptor.mediawiki` §"Taproot tree" admit-list update with vendor citations.
- **What:** Allocate a Tag byte for `sortedmulti_a`. Candidates: `0x34` (currently "reserved-invalid" — would change `tag.rs:225-232` test gate semantics) or somewhere in `0x36+` (cleaner — further from existing operator block). Wire-format question covered separately: tag space has ~203 free bytes. Add encode/decode dispatch; loosen validators on both sides; add positive vector(s) covering bare `sortedmulti_a` in single-leaf form and inside a multi-leaf TapTree; update BIP draft to document admission with citations to Coldcard `docs/taproot.md` (edge) and Ledger `vanadium/apps/bitcoin/common/src/bip388/cleartext.rs`.
- **Why deferred:** Wire-format-additive change (new Tag allocation) — should land in a labelled release for clean CHANGELOG/MIGRATION coverage. Could land in 0.6.0 alongside the `decoded-string-data-memory-microopt` API break and `tap-leaf-admit-after`.
- **Status:** wont-fix — superseded by `md-tag-space-rework` (Tag allocation absorbed into the broader v0.6 reorganization) + `md-strip-validator-default-and-corpus` (the validator widening is moot once the gate is removed by default; `sortedmulti_a` is admitted along with everything else)
- **Tier:** v0.6 → wont-fix

### `tap-leaf-admit-after` — admit `after` (absolute timelock) in tap leaves

- **Surfaced:** 2026-04-28 evidence-gathering session in response to `phase-d-tap-leaf-wrapper-subset-clarification`. Ledger's vanadium bitcoin app (`apps/bitcoin/common/src/bip388/cleartext.rs`) admits 4 timelocked-multisig compound shapes as first-class wallet-policy variants. 2 of them use `after(n)`: `and_v(v:multi_a(...), after(n))` with `n < 500000000` (absolute height) and with `n >= 500000000 && n < 4194304` (absolute time-based, second range derived from BIP 65). MD's current admit set covers all other parts of these shapes (`and_v`, `v:`, `multi_a`); only `after` is missing.
- **Where:** `crates/md-codec/src/bytecode/encode.rs::validate_tap_leaf_subset` (and decode-side counterpart); `crates/md-codec/src/vectors.rs` (positive corpus fixture for absolute-timelock multisig shape); `bip/bip-mnemonic-descriptor.mediawiki` §"Taproot tree" admit-list update.
- **What:** Add `Terminal::After` to the tap-leaf admit set in `validate_tap_leaf_subset` (and matching `Tag::After` arm on decode side). The Tag is already allocated (`Tag::After = 0x1E`); no wire-format change needed. Add at least one positive corpus vector capturing a `tr(KEY, and_v(v:multi_a(2,@1,@2), after(700000)))`-style shape so the round-trip is exercised by conformance tests. Update BIP draft.
- **Why deferred:** Validator-only change but technically expands MD's admitted surface — should ship in a labelled release rather than a silent v0.5.x patch. v0.6 candidate (alongside `tap-leaf-admit-sortedmulti-a`) so signer-evidence-driven widenings land in one breaking release with a coherent CHANGELOG entry.
- **Status:** wont-fix — superseded by `md-strip-validator-default-and-corpus` (`after` is admitted by default once the gate is removed; positive corpus vector for the absolute-timelock multisig shape is part of the corpus expansion in that entry)
- **Tier:** v0.6 → wont-fix

### `tap-leaf-corpus-timelocked-multisig-shapes` — add positive corpus vectors for signer-canonical timelocked-multisig compound shapes

- **Surfaced:** 2026-04-28 evidence-gathering session. Ledger's vanadium bitcoin app (`apps/bitcoin/common/src/bip388/cleartext.rs`) classifies 4 timelocked-multisig compound shapes as first-class wallet-policy variants for UX display: `RelativeHeightlockMultiSig` (`and_v(v:multi_a(...), older(n))` with `n < 65536`), `RelativeTimelockMultiSig` (`older` with `n >= 4194305 && n < 4259840` per BIP 112's relative-time encoding), `AbsoluteHeightlockMultiSig` (`after(n)` with `n < 500000000`), `AbsoluteTimelockMultiSig` (`after` with `n >= 500000000`). MD doesn't need first-class detection (we're a wire format, not a UI), but currently has zero positive corpus vectors exercising these compound shapes, so round-trip behaviour is not pinned by the conformance suite.
- **Where:** `crates/md-codec/src/vectors.rs` (positive corpus fixtures); `bip/bip-mnemonic-descriptor.mediawiki` §"Taproot tree" example list.
- **What:** Add 2–4 positive corpus vectors capturing canonical timelocked-multisig shapes — e.g. `tr(@0/**, and_v(v:multi_a(2,@1/**,@2/**), older(144)))` for relative height, plus `after`-using variants once `tap-leaf-admit-after` lands. The relative-height/time variants (using `older`) work today with our current admit set; only the corpus coverage is missing. Absolute variants (using `after`) are blocked on `tap-leaf-admit-after`. Update BIP draft examples to mirror the Ledger compound-shape vocabulary so signer-aware tooling can recognize the patterns even though MD itself is shape-agnostic.
- **Why deferred:** Pure corpus expansion + spec example growth. No validator changes; not blocking. Family-stable SHA promise means new corpus vectors cannot land in a v0.5.x patch — they'd change the v0.5.json SHA. v0.6 is the natural home alongside the other admit-set widenings.
- **Status:** wont-fix — superseded by `md-strip-validator-default-and-corpus` (the corpus expansion in that entry covers all 4 timelocked-multisig compound shapes alongside the broader strip-driven corpus growth)
- **Tier:** v0.6 → wont-fix

### `tap-leaf-corpus-pkh-shape` — verify and lock in `pkh()` round-trip in tap leaves

- **Surfaced:** 2026-04-28 evidence-gathering session. Coldcard's `docs/taproot.md` (edge branch) lists `tr(internal_key, {or_d(pk(@0), and_v(v:pkh(@1), older(1000))), pk(@2)})` as an admitted descriptor — uses `pkh()` inside a tap leaf. MD's current admit set already admits `Terminal::PkH` (`Tag::PkH = 0x1C`) and `Terminal::Check` (`Tag::Check = 0x0C`), so `pkh(K)` (BIP 379 sugar for `c:pk_h(K)`) is expected to round-trip via rust-miniscript's parse-time desugaring — but no positive corpus vector locks this in.
- **Where:** `crates/md-codec/src/vectors.rs` (positive corpus fixture); `bip/bip-mnemonic-descriptor.mediawiki` §"Taproot tree" examples (mention `pkh()` as part of Coldcard's documented shape vocabulary).
- **What:** First, verify `tr(@0/**, and_v(v:pkh(@1/**), older(144)))` round-trips through MD today. Expected: rust-miniscript desugars `pkh(K)` → `c:pk_h(K)` at parse time, both terminals already admitted, so round-trip succeeds. If verified, add a positive corpus vector capturing the shape so the round-trip is locked by conformance tests. If the round-trip unexpectedly fails (e.g., rust-miniscript preserves a distinct `Pkh` AST node we don't handle), reframe as an admit-set widening rather than a corpus addition.
- **Why deferred:** Verification + corpus expansion. No validator changes expected; sub-day work. Family-stable SHA constraint pushes this to v0.6.
- **Status:** wont-fix — superseded by `md-strip-validator-default-and-corpus` (`pkh()` round-trip is part of the corpus expansion; the verification step still runs but now within the broader strip work)
- **Tier:** v0.6 → wont-fix

### `v0-6-release-prep` — coordinate the cross-cutting v0.6 release work

- **Surfaced:** 2026-04-28. Several v0.6-tagged FOLLOWUPS entries have landed (`decoded-string-data-memory-microopt` already done in `d79125d`; `tap-leaf-admit-sortedmulti-a`, `tap-leaf-admit-after`, `tap-leaf-corpus-timelocked-multisig-shapes`, `tap-leaf-corpus-pkh-shape` queued). They share cross-cutting concerns that don't fit any single entry: a Tag-byte-allocation decision for `sortedmulti_a`, a coordinated CHANGELOG / MIGRATION pass at release time, the family-stable SHA reset (`"md-codec 0.5"` → `"md-codec 0.6"`), and the version bump itself. This entry coordinates the meta-work.
- **Where:** `crates/md-codec/Cargo.toml` (version bump); `crates/md-codec/src/bytecode/tag.rs` (`Tag::SortedMultiA` allocation); `crates/md-codec/src/vectors.rs` (`GENERATOR_FAMILY` major.minor token); `crates/md-codec/tests/vectors_schema.rs` (v0.1.json + v0.2.json SHA pin updates after regen); `CHANGELOG.md` (`[Unreleased]` → `[0.6.0]` rename + consolidated entries); `MIGRATION.md` (rewrite or extend `v0.5.x → v0.6.0` section to cover all the breaking/widening changes); `bip/bip-mnemonic-descriptor.mediawiki` (admit-list updates).
- **What:** Three coordinated tasks beyond the per-entry work:
  1. **Tag-byte allocation decision for `sortedmulti_a`.** Candidates per the wire-format audit: `0x34` (currently the only "reserved-invalid" byte in the 0x00–0x33 range — would change `tag.rs:225-232` test gate semantics) or anywhere in `0x36+` (further from the existing operator block but cleaner). Decision affects `tap-leaf-admit-sortedmulti-a` only.
  2. **Coordinated validator + corpus + BIP-draft pass.** Land the `validate_tap_leaf_subset` widening (covers `tap-leaf-admit-sortedmulti-a` and `tap-leaf-admit-after` together), regenerate corpus with the new positive vectors (covers `tap-leaf-corpus-timelocked-multisig-shapes` and `tap-leaf-corpus-pkh-shape`), update BIP draft §"Taproot tree" admit-list with vendor citations (Coldcard `docs/taproot.md` edge + Ledger `vanadium/apps/bitcoin/common/src/bip388/cleartext.rs`). Single PR or stacked PRs at controller's discretion.
  3. **Release plumbing.** `Cargo.toml` version 0.5.0 → 0.6.0; `GENERATOR_FAMILY` token rolls `"md-codec 0.5"` → `"md-codec 0.6"`; v0.1.json and v0.2.json regenerated with the new family token (SHAs change once at the v0.5.x → v0.6.0 boundary, then stable across the v0.6.x patch line per the family-stable promise); `vectors_schema.rs` SHA pins updated; CHANGELOG `[Unreleased]` section renamed to `[0.6.0] — <date>` with entries from all v0.6 work consolidated; MIGRATION's `v0.5.x → v0.6.0` section extended to cover the admit-set widenings (currently only documents the `DecodedString.data` field removal).
- **Why deferred:** Coordination only — no per-entry blocker. Lands at the v0.6 release cut, after the per-entry implementation work is done.
- **Status:** wont-fix — superseded by `v0-6-release-prep-revised` (the original framing assumed admit-set widening; the strip-Layer-3 design pivot replaces that approach, and the release-prep-revised entry covers the new plumbing requirements including Tag-space reorganization rather than per-operator admit decisions)
- **Tier:** v0.6 → wont-fix


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


### `rename-workflow-broad-sed-enumeration-lesson` — workflow doc should explicitly enumerate src/+tests/+bin/ for sed sweeps

- **Surfaced:** Phase 4 (identifier mass-rename) code-quality reviewer (Minor); learnable lesson from 2 oversight-fix commits
- **Where:** `design/RENAME_WORKFLOW.md` Phase 4 section
- **What:** Phase 4 implementer's broad sed sweep ran on `src/` only and missed `tests/`, `src/bin/`, and module-specific subdirectories. Required two follow-up commits (`6c303c0`, `2c9d720`) covering 12 additional files. Lesson: when documenting a future rename, the workflow doc's Phase 4 sub-batch instructions should explicitly enumerate `src/**/*.rs`, `tests/**/*.rs`, and `src/bin/**/*.rs` as separate targets — don't rely on a single glob.
- **Why deferred:** This is a meta-improvement to the workflow doc, not a current rename defect. Best applied next time `RENAME_WORKFLOW.md` is updated (e.g., during the next rename, or as a pre-emptive cleanup pass).
- **Status:** open
- **Tier:** v1+ (process improvement, not version-gating)

### `md-scope-strip-layer-3-signer-curation` — strip MD's signer-compatibility curation layer

- **Surfaced:** 2026-04-28 design discussion. The premise that MD must enforce hardware-signer-subset compatibility was challenged: MD is a wire format for BIP 388 wallet policies; whether a given policy is signable on a given signer is a layered concern handled by tools above and below MD. BIP 388 §"Implementation guidelines" (`bip-0388.mediawiki:216`) explicitly permits subsets but doesn't direct implementations to mirror signers — "It is acceptable to implement only a subset of the possible wallet policies defined by this standard." The MUST clause in MD's own BIP draft (`bip/bip-mnemonic-descriptor.mediawiki:547`) was a Phase D / Phase 2 design choice, not a spec inheritance. Recovery-footgun argument reconsidered: the responsibility chain is wallet software → MD → signer, not "MD curates for the signer."
- **Where:** Cross-cutting design pivot. See child entries for component-level work: `md-strip-validator-default-and-corpus`, `md-strip-spec-and-docs`, `md-tag-space-rework`, `md-signer-compat-checker-separate-library`, `md-policy-compiler-feature`, `v0-6-release-prep-revised`.
- **What:** Reframe MD's scope to encoding-only (BIP 388 wallet-policy serialization with BCH error correction). Drop the implicit "MD-encoded backups are guaranteed signable on Coldcard" promise; replace with explicit responsibility-chain framing in BIP draft and READMEs. The Phase D `validate_tap_leaf_subset` infrastructure is retained as `pub fn` for explicit-call use but no longer gates encoding/decoding by default. Named signer subsets become a separately-versioned layer (`md-signer-compat-checker-separate-library`).
- **Why deferred:** Master principle entry; no commits close it directly. Closes when all child entries close at the v0.6.0 tag.
- **Status:** resolved md-codec-v0.6.0. All child entries closed. MD scope is now encoding-only; signer-compatibility moves to the layered `md-signer-compat` crate (shipped v0.7.0). BIP draft, READMEs, and rustdocs updated to reflect the responsibility-chain framing.
- **Tier:** v0.6 (closed)

### `md-strip-validator-default-and-corpus` — flip encoder/decoder defaults; expand corpus

- **Surfaced:** 2026-04-28; child of `md-scope-strip-layer-3-signer-curation`.
- **Where:** `crates/md-codec/src/bytecode/encode.rs` (encoder's `EncodeTemplate for Miniscript<_, Tap>` impl + `validate_tap_leaf_subset` infrastructure); `crates/md-codec/src/bytecode/decode.rs` (`decode_tap_terminal` rejection paths + the `validate_tap_leaf_subset` calls at `decode.rs:295` and `decode.rs:802`); `crates/md-codec/src/error.rs` (`Error::TapLeafSubsetViolation` variant — retained); `crates/md-codec/src/vectors.rs` (positive corpus vectors for newly-admitted shapes); negative-fixture generators that asserted rejection of out-of-subset operators (flip or remove).
- **What:** Three coupled changes:
  (a) **Encoder default**: tap-leaf encode path no longer calls `validate_tap_leaf_subset`. The function and `validate_tap_leaf_terminal` helper stay as `pub fn` so callers can invoke them explicitly. Decoder mirrors: drop the catch-all rejection arms in `decode_tap_terminal`; drop the `validate_tap_leaf_subset` calls at `decode.rs:295` (single-leaf path) and `decode.rs:802` (multi-leaf path).
  (b) **Corpus**: add positive vectors for previously-rejected-but-now-admitted shapes — `sortedmulti_a` (once Tag allocated by `md-tag-space-rework`), `after` in tap context, `thresh`, `or_b`, hash terminals (`sha256`/`hash256`/`ripemd160`/`hash160`) in tap leaves, timelocked-multisig compounds (`and_v(v:multi_a(...), older(n))` and `after(n)` variants), `pkh()` round-trip via desugaring, and a representative wrapper-richer fixture (`s:`/`a:`/`d:`/`j:`/`n:` wrappers in legitimate compositions). Negative vectors that asserted rejection of these get flipped to positive or removed.
  (c) **`Error::TapLeafSubsetViolation` retained**: the variant stays in `error.rs` for use by the explicit-call validator path. Optional opt-in API design (`EncodeOptions::with_signer_subset(...)`) is deferred to `md-signer-compat-checker-separate-library` — not part of this v0.6 entry.
- **Why deferred:** Wire-format-affecting (corpus changes regenerate v0.1.json + v0.2.json). v0.6 breaking release.
- **Status:** resolved md-codec-v0.6.0. Encoder/decoder default validator gate removed; `validate_tap_leaf_subset` retained as `pub fn` (and refactored in v0.7.0 to accept a caller-supplied allowlist). Corpus expanded with 17 new positive fixtures for previously-rejected shapes; negative fixtures asserting rejection were flipped or removed. `Error::TapLeafSubsetViolation` renamed to `Error::SubsetViolation`.
- **Tier:** v0.6 (closed)

### `md-strip-spec-and-docs` — rewrite BIP draft + README + CLI help for the new framing

- **Surfaced:** 2026-04-28; child of `md-scope-strip-layer-3-signer-curation`.
- **Where:** `bip/bip-mnemonic-descriptor.mediawiki` §"Taproot tree" (line 547 MUST clause) + new informational §"Signer compatibility"; `README.md` (top-level scope framing); `crates/md-codec/README.md`; CLI help text in `crates/md-codec/src/bin/md/main.rs`; rustdoc on relevant public API.
- **What:** Two coupled doc changes:
  (a) **BIP draft**: rewrite §"Taproot tree" subset paragraph from MUST to MAY-informational. Cite BIP 388 §"Implementation guidelines" (line 216) allowing subsets. Add a §"Signer compatibility (informational)" section that (1) explains MD's scope is encoding/decoding, not signer curation; (2) frames the responsibility chain (wallet software → MD → signer); (3) provides vendor-citation pattern as an example (Coldcard `docs/taproot.md` edge, Ledger vanadium `apps/bitcoin/common/src/bip388/cleartext.rs`) without endorsing a specific subset; (4) points readers at the layered checker (`md-signer-compat-checker-separate-library`) once it exists.
  (b) **READMEs + CLI help**: add a "you are responsible for ensuring your policy is signable on your target signer" warning. Link to the BIP §"Signer compatibility" section. CLI help on `md encode`: brief one-liner pointer.
- **Why deferred:** Spec text changes paired with the v0.6 code release.
- **Status:** resolved md-codec-v0.6.0. BIP draft §"Taproot tree" rewrote MUST → MAY-informational; new §"Signer compatibility (informational)" section frames the responsibility chain. README and CLI help updated for the new framing. Note: a v0.7.x doc-sync pass (this commit) adds pointers to the v0.7.0 layered checker (`md-signer-compat`) and the policy-compiler wrapper.
- **Tier:** v0.6 (closed)

### `md-tag-space-rework` — allocate `Tag::SortedMultiA`, reorganize Tag enum, drop Reserved\* range

- **Surfaced:** 2026-04-28; child of `md-scope-strip-layer-3-signer-curation`. User confirmed pre-1.0 backwards compatibility is not a constraint ("nobody has used the software yet"), opening the full Tag enum to reshuffling. User chose Option B for the Reserved\* tags: drop entirely, since MD's BIP 388 wallet-policy framing explicitly forbids inline keys and the descriptor-codec inline-key-form vendoring is dead weight.
- **Where:** `crates/md-codec/src/bytecode/tag.rs` (Tag enum + `from_byte` match + tests + module rustdoc); `crates/md-codec/src/bytecode/{encode,decode}.rs` (any code matching Tag values explicitly); `bip/bip-mnemonic-descriptor.mediawiki` Tag table (§"Bytecode operators" or wherever the tag list lives); `crates/md-codec/src/vectors.rs` (every existing fixture's `expected_bytecode_hex` changes); `crates/md-codec/tests/vectors/v0.1.json` + `v0.2.json` (fully regenerated); `crates/md-codec/tests/vectors_schema.rs` (SHA pin updates).
- **What:** Coordinated wire-format-breaking reorganization, lands once for v0.6:
  (a) **Allocate `Tag::SortedMultiA`** — adjacent to the rest of the multisig family. `Terminal::SortedMultiA` exists in miniscript (used by Coldcard / Ledger / rust-miniscript wallet-policy fixture); MD currently has no Tag for it.
  (b) **Reorganize the Tag enum** from descriptor-codec-vendored layout to a coherent grouping. Move `SortedMulti = 0x09` (descriptor-codec heritage, out-of-place) adjacent to `Multi`. Group all multisig adjacent (`Multi`, `SortedMulti`, `MultiA`, `SortedMultiA`).
  (c) **Drop `Reserved*` variants entirely (Option B)** — remove all 14 variants (`ReservedOrigin`/`ReservedNoOrigin`/`Reserved*FullKey`/`ReservedXOnly`/`Reserved*XPub`/`Reserved*Priv*`/`Reserved*Wildcard`) at 0x24–0x31 from the Tag enum. The bytes 0x24–0x31 become unallocated (return `None` from `from_byte`); MD's BIP 388 wallet-policy scope explicitly forbids inline keys, so the descriptor-codec inline-key-form vendoring is dead weight. Reclaims a contiguous 14-byte block adjacent to the operator block. Update `tag.rs:225-232` test gates accordingly. Document the design rationale in module rustdoc.
  (d) **Reshuffle remaining tags** for clean blocks (constants / top-level descriptors / framing / wrappers / logical / multisig / keys / timelocks / hashes). Final layout documented in `tag.rs` and the BIP draft Tag table.
  (e) **Corpus regen**: every existing positive-vector `expected_bytecode_hex` changes. v0.1.json + v0.2.json fully regenerated; SHA pins in `tests/vectors_schema.rs` updated. `GENERATOR_FAMILY` token roll covered separately by `v0-6-release-prep-revised`.
- **Why deferred:** Once-and-done opportunity — pre-1.0 + no users yet means we can reshape; after v0.6 ships and gets used, this freedom evaporates. v0.6 is the moment.
- **Status:** resolved md-codec-v0.6.0. `Tag::SortedMultiA = 0x0B` allocated; multisig family contiguous (Multi 0x08, SortedMulti 0x09, MultiA 0x0A, SortedMultiA 0x0B); 14 `Reserved*` variants dropped (0x24–0x31 unallocated); wrappers/logical operators shifted by 2 for clean blocks; `Tag::Bare` dropped (byte 0x07 reused for `TapTree`); `Placeholder` 0x32→0x33 (byte 0x32 left unallocated to surface v0.5→v0.6 transcoder mistakes). Spec §2.2 documents the final layout; v0.7.1 added §2.2.1 alphabetical index.
- **Tier:** v0.6 (closed)

### `md-signer-compat-checker-separate-library` — named signer subsets + opt-in validation API (aspirational)

- **Surfaced:** 2026-04-28; child of `md-scope-strip-layer-3-signer-curation`. Phase D's `validate_tap_leaf_subset` infrastructure is preserved by `md-strip-validator-default-and-corpus`; this entry covers (a) the *named signer subset* registry that should live separately from md-codec, and (b) the opt-in validation API design that lets callers wire the checker into the encoder/decoder.
- **Where:** New crate, e.g. `crates/md-signer-compat/` (or a separate repo if MD ever spins out). md-codec stays neutral on signer specifics.
- **What:** Two coupled deliverables:
  (a) **Named signer subsets**: `md_signer_compat::COLDCARD_TAP`, `md_signer_compat::LEDGER_TAP`, etc. Each is a `SignerSubset` value (operator allowlist) populated from the vendor's documented admit list, with a citation comment pointing at the source (e.g., Coldcard `docs/taproot.md` edge SHA, Ledger `vanadium/apps/bitcoin/common/src/bip388/cleartext.rs`). Update cadence: vendor doc revision → subset bump → patch release of the layered crate.
  (b) **Opt-in validation API**: design and ship the `EncodeOptions::with_signer_subset(subset: SignerSubset)` / `DecodeOptions::with_signer_subset(...)` mechanism. Recommended shape per the 2026-04-28 design discussion: `SignerSubset` is a public struct (operator allowlist) defined in md-codec, *populated by the caller*. md-codec ships only the validation mechanism; named subsets ship in the separate crate so vendor-tracking concerns don't bleed into md-codec.
  (c) Optional CLI surface: `md validate --signer coldcard <bytecode>` decodes a bytecode and runs the named subset check, reporting any out-of-subset operators.
- **Why deferred:** Aspirational — does not block the strip. Provides an opt-in safety net for users who want signer-aware validation without committing md-codec itself to tracking signer firmware. Maintenance burden concentrates in this crate, where it belongs.
- **Status:** resolved md-codec-v0.7.0 (Phase 4). NEW workspace crate `crates/md-signer-compat/` ships `pub const COLDCARD_TAP`, `pub const LEDGER_TAP`, `pub fn validate(subset, ms, leaf_index)`, and `pub fn validate_tap_tree(subset, tap_tree)` for DFS-pre-order leaf-index threading. Vendor-citation discipline established: each subset rustdoc cites source URL, repo SHA, last-checked date.
- **Tier:** v0.6+ (closed)

### `md-policy-compiler-feature` — expose policy-to-bytecode compilation (future release)

- **Surfaced:** 2026-04-28; child of `md-scope-strip-layer-3-signer-curation`. Now-clean feature add since the admit-set gate is gone.
- **Where:** `crates/md-codec/Cargo.toml` (enable rust-miniscript `compiler` feature); new public API surface in md-codec; CLI tool exposure.
- **What:** Enable the `compiler` feature on the `miniscript` git-pinned dep. Expose a `pub fn policy_to_bytecode(policy: &str, options: &EncodeOptions) -> Result<Vec<u8>, Error>` (or similar shape) that parses a high-level Concrete-Policy string, runs miniscript's policy compiler to produce optimal miniscript, and encodes the result. CLI tool gains a `md encode --from-policy <expr>` mode. With Layer 3 stripped, the compiler can pick any miniscript shape and md-codec will encode it; signer compatibility is the caller's concern. Optional pairing with the layered checker (`md-signer-compat-checker-separate-library`) for "compile then validate against signer X" workflows.
- **Why deferred:** Independent feature. Future release post-strip; the strip itself doesn't require the compiler.
- **Status:** resolved md-codec-v0.7.0 (Phase 5). Added `compiler` and `cli-compiler` cargo features (default-off). New `pub fn policy_to_bytecode(policy, options, ScriptContext, internal_key)` with caller-supplied Tap internal key per Plan reviewer #1 Concern 2 (None → upstream NUMS unspendable). New `md from-policy <expr> --context <tap|segwitv0> [--internal-key <KEY>]` CLI subcommand.
- **Tier:** v0.7+ (closed)

### `v06-plan-targeted-decoder-arm-tests` — per-arm decoder unit tests for Phase 3 (defensive)

- **Surfaced:** v0.6 plan round-1 review (`design/agent-reports/v0-6-plan-review-1.md` TDD audit); flagged as nice-to-have, not blocking.
- **Where:** `crates/md-codec/tests/taproot.rs` or a new `tests/decoder_arms.rs`.
- **What:** Phase 3 of the strip plan adds ~20 new arms to `decode_tap_terminal`. The plan relies on corpus round-trip (Phase 5 fixtures) to catch decoder bugs. This is adequate but defensive: a decoder arm that consumes the wrong number of payload bytes AND a symmetrically-wrong encoder arm would both round-trip but produce malformed wire output. Add 5-7 targeted unit tests that synthesize a known bytecode (Tag byte + payload), feed to `decode_tap_terminal` directly, and assert the resulting Terminal matches the expected AST shape and consumed-byte-count. ~30-minute effort.
- **Why deferred:** corpus round-trip is sufficient for normal regression catching; this is purely defensive against a class of bug that hasn't actually occurred in the v0.5 codebase. Can land as a v0.6.x patch or v0.7+ when convenient.
- **Status:** resolved md-codec-v0.7.0 (Phase 2). Added 6 per-arm decoder unit tests in `crates/md-codec/src/bytecode/hand_ast_coverage.rs`: multi_a, andor, thresh, after, sortedmulti_a, hash256.
- **Tier:** v0.6+ (closed)

### `v06-test-byte-literal-rebaseline` — rebaseline 38 unit tests pinning v0.5 byte literals

- **Surfaced:** v0.6 Phase 10 release plumbing. After Tag enum reorganization (Phase 1), the wire format changed for many operators (e.g., `Tag::Multi` 0x19 → 0x08; `Tag::Placeholder` 0x32 → 0x33; logical operators shifted by 2). 38 unit tests in `bytecode::{encode,decode,path}::tests` and `policy::tests` and `vectors::tests` use literal byte values like `vec![0x05, 0x16, 0x00, 0x01]` that encode v0.5 byte sequences. These tests now fail because the decoder interprets the bytes per v0.6 semantics.
- **Where:** Tests in `crates/md-codec/src/bytecode/decode.rs` (~16 tests), `crates/md-codec/src/bytecode/encode.rs` (~10 tests), `crates/md-codec/src/bytecode/path/...` (~6 tests), `crates/md-codec/src/policy.rs` (~5 tests), `crates/md-codec/src/vectors.rs` (~1 test).
- **What:** Walk each failing test and update literal byte values per the v0.5→v0.6 byte-shift table in `design/SPEC_v0_6_strip_layer_3.md` §2.3. Pattern: replace literal v0.5 bytes (e.g., `0x16` for OrD) with v0.6 bytes (`0x18` for OrD). Where possible, replace literals with symbolic `Tag::Foo.as_byte()` references so future Tag changes don't re-break. The test SEMANTICS are correct; only the BYTE LITERALS need updating.
- **Why deferred:** v0.6.0 release ships with these tests temporarily failing because the wire format change is a coordinated single-commit operation; rebaseline is mechanical follow-up work suitable for a v0.6.0.1 patch. The release cuts with v0.6.0 release-prep complete; rebaseline lands in v0.6.0.1.
- **Status:** resolved md-codec-v0.7.0 (Phase 1, commits 35caa24/d7de42d/de63db3). All 38 unit tests + ~11 integration tests rebaselined to v0.6 byte codes using symbolic `Tag::Foo.as_byte()` references where helpful. Three subset-violation tests rewrote to use the v0.6 opt-in `validate_tap_leaf_subset` API; corpus-count assertions updated for the regenerated v0.6 corpus (43 positive vectors).
- **Tier:** v0.7.0 (closed)

### `v06-corpus-d-wrapper-coverage` — add d: wrapper tap-leaf round-trip vector

- **Surfaced:** v0.6 Phase 10 corpus regen. Initial fixture `tr_d_wrapper_in_tap_leaf_md_v0_6` with form `tr(@0/**, andor(pk(@1/**), pk(@2/**), d:older(144)))` failed parser typing: `d:` requires Vz-type child, but `older(n)` is B-type. Removed from corpus; filed for follow-up.
- **Where:** `crates/md-codec/src/vectors.rs` TAPROOT_FIXTURES.
- **What:** Add a d: wrapper round-trip fixture using a Vz-type child (e.g., `d:v:older(144)` if v:older is V and z; or hand-construct the AST in `tests/taproot.rs`). Exercises `Tag::DupIf = 0x0F`. The wrapper byte is wire-format-supported and exercised by encoder/decoder symmetric arms; only the corpus pin is missing.
- **Why deferred:** Same as `v06-corpus-or-c-coverage` and `v06-corpus-j-n-wrapper-coverage`. Not blocking ship; defensive corpus growth.
- **Status:** resolved md-codec-v0.7.0 (Phase 2). Added `d_wrapper_tap_leaf_byte_form` hand-AST test in `crates/md-codec/src/bytecode/hand_ast_coverage.rs`: pins wire bytes for `d:v:older(144)` (= `Terminal::DupIf(Verify(Older))`) including LEB128(144) = `[0x90, 0x01]`.
- **Tier:** v0.6.x (closed)

### `v06-corpus-or-c-coverage` — add or_c tap-leaf round-trip vector

- **Surfaced:** v0.6 Phase 5 execution. The plan listed `tr_or_c_in_tap_leaf_md_v0_6` as a per-Terminal coverage vector with the form `tr(@0/**, or_c(pk(@1/**), v:pk(@2/**)))`. Parser rejected it: `or_c` returns V-type, but BIP 388 / rust-miniscript wallet-policy parser requires top-level tap leaves to be B-type.
- **Where:** `crates/md-codec/src/vectors.rs` TAPROOT_FIXTURES.
- **What:** Add an or_c fixture using a B-typed wrapping like `tr(@0/**, t:or_c(pk(@1/**), v:pk(@2/**)))` (where `t:` desugars to `and_v(X, 1)` = B-type) OR construct the AST hand-coded in a unit test rather than via Descriptor::from_str. The Tag::OrC byte form needs corpus coverage; the parser reject is a typing constraint, not a wire-format issue.
- **Why deferred:** Plan's per-Terminal coverage rule; not blocking v0.6.0 ship since OrC byte is wire-format-supported and exercised by encoder/decoder symmetric arms, just not pinned in a fixture.
- **Status:** resolved md-codec-v0.7.0 (Phase 2). Added `or_c_unwrapped_tap_leaf_byte_form` (encoder wire-byte pin) and `t_or_c_tap_leaf_round_trips` (full encode→decode→re-encode round-trip via `t:or_c` wrap) hand-AST tests.
- **Tier:** v0.6+ (closed)

### `v06-corpus-j-n-wrapper-coverage` — add j: and n: wrapper tap-leaf round-trip vectors

- **Surfaced:** v0.6 spec round-1 review (IMP-5) + Phase 5 execution. The j: (NonZero) and n: (ZeroNotEqual) wrappers have typing constraints (j: requires Bn-type child; n: requires B-type child) that make them awkward to spell in BIP 388 source form. Plan flagged as TBD; deferred to FOLLOWUPS.
- **Where:** `crates/md-codec/src/vectors.rs` TAPROOT_FIXTURES (or `tests/taproot.rs` hand-AST tests).
- **What:** Add round-trip fixtures for Tag::NonZero (0x11) and Tag::ZeroNotEqual (0x12). If the BIP 388 source-form policies don't naturally produce these wrappers, hand-construct the AST via `Terminal::NonZero(Arc::new(child))` / `Terminal::ZeroNotEqual(Arc::new(child))` in unit tests. Encoder + decoder arms exist and are byte-symmetric; only the corpus pin is missing.
- **Why deferred:** Same as `v06-corpus-or-c-coverage`. Not blocking ship; defensive corpus growth.
- **Status:** resolved md-codec-v0.7.0 (Phase 2). Added `j_wrapper_tap_leaf_byte_form` (`j:pk_k(a)` = `Terminal::NonZero(PkK)`) and `n_wrapper_tap_leaf_byte_form` (`n:c:pk_k(a)` = `Terminal::ZeroNotEqual(Check(PkK))`) hand-AST tests pinning wire-byte form for both wrappers.
- **Tier:** v0.6+ (closed)

### `v07-cli-validate-signer-subset` — `md validate --signer <name> <bytecode>` CLI mode

- **Surfaced:** v0.7.0 spec round-1 review (Q5). Spec §9 deferred this CLI surface to v0.7.x patch.
- **Where:** `crates/md-codec/src/bin/md/main.rs` (CLI subcommand for validate-against-signer); depends on `crates/md-signer-compat` (NEW in v0.7.0).
- **What:** Add a `validate` subcommand to the `md` CLI that takes a bytecode (or string) input and a `--signer <NAME>` arg, decodes the bytecode, and runs the named SignerSubset's `validate()` against each tap leaf. Output: pretty-print pass/fail per leaf with operator name + leaf_index on rejection. Considerations: machine-readable mode (`--json`), exit code on subset violation, handling of decoded but partial-validation cases.
- **Why deferred:** v0.7.0 release adds three new things (md-signer-compat crate, compiler feature, CLI policy mode); a fourth would dilute focus. CLI UX questions (output format, exit code, machine-readable) deserve their own design pass.
- **Status:** resolved md-codec-v0.7.1 (`md-signer-compat` 0.1.1). Shipped as a NEW binary `md-signer-compat` in the md-signer-compat crate with subcommands `validate --signer <coldcard|ledger> {--bytecode-hex HEX | --string MD-STRING...}` and `list-signers`. Architectural divergence from the original "extend `md` binary" framing: md-signer-compat already depends on md-codec, so adding the reverse dep for the CLI would cycle. The new binary delivers the equivalent functionality.
- **Tier:** v0.7.x (closed)

### `v06-corpus-byte-order-defensive-test` — defensive hand-pinned hash byte-order test

- **Surfaced:** v0.6 spec round-1 review (§6.3 spec-coverage concern). Plan Step 5.1.6 specified adding a defensive byte-pin test in `tests/taproot.rs` that takes a known input hash, encodes via the Hash256/Sha256/Ripemd160/Hash160 path, and asserts the bytecode contains the input bytes in **internal byte order** (NOT reversed-display-order). The corpus round-trip alone cannot catch a regression where encoder + decoder both flip to display-order (would be round-trip-stable but format-changed).
- **Where:** `crates/md-codec/tests/taproot.rs` (or new `tests/hash_byte_order.rs`).
- **What:** Hand-coded byte-pin assertion: construct a known 32-byte hash (e.g., all-0xAA), invoke encoder via `encode_template` or similar, assert the bytecode bytes immediately after the Tag byte equal the input bytes UNREVERSED. Repeat for all 4 hash terminals.
- **Why deferred:** Plan's defensive-test step deferred to v0.6+ during overnight autonomous execution to focus on shipping. Round-trip via the corpus fixtures provides indirect coverage; the dedicated byte-pin would catch the very specific encoder+decoder symmetric regression.
- **Status:** resolved md-codec-v0.7.0 (Phase 2). Added `hash_terminals_encode_internal_byte_order_with_decode_round_trip` covering Sha256, Hash256, Ripemd160, Hash160 with **asymmetric** input patterns (`[0x00..0x1F]` and `[0x80..0x93]` via `std::array::from_fn`) per Phase 2 reviewer IMP-1 — palindromic constant-fill defeats the asymmetric encode/decode reversal bug class. Decode-direction round-trip per Plan reviewer #1 Concern 5.
- **Tier:** v0.6+ (closed)

### `v06-spec-tag-byte-display-table` — alphabetical Tag→byte index for spec audit convenience

- **Surfaced:** v0.6 spec round-1 review (agent report `v0-6-spec-review-1.md`); flagged as nice-to-have, not blocking.
- **Where:** `design/SPEC_v0_6_strip_layer_3.md` §2.2.
- **What:** §2.2 lists the Tag enum grouped-by-purpose (constants, top-level, framing, multisig, wrappers, logical, keys, timelocks, hashes). For audit-by-name (e.g., "where is `Hash160`?"), an alphabetical secondary listing `Tag → byte` would make spot-checks fast. Add a small subsection §2.2.1 with the alphabetical index after §2.2's grouped listing.
- **Why deferred:** Cosmetic spec readability; doesn't affect implementation. Easy to add at any time.
- **Status:** resolved md-codec-v0.7.1. Added §2.2.1 "Alphabetical Tag → byte index (audit convenience)" listing all 39 Tag variants in alphabetical order with their v0.6 bytes.
- **Tier:** v0.6 (closed)

### `v0-6-release-prep-revised` — coordinate the v0.6 release plumbing under the strip framing

- **Surfaced:** 2026-04-28; replaces `v0-6-release-prep` (which was framed around the now-superseded admit-set widening approach).
- **Where:** `crates/md-codec/Cargo.toml` (version bump); `crates/md-codec/src/vectors.rs` (`GENERATOR_FAMILY` token); `crates/md-codec/tests/vectors_schema.rs` (SHA pin updates after regen); `CHANGELOG.md`; `MIGRATION.md`.
- **What:** Release plumbing for v0.6 once the per-entry implementation work lands:
  - Cargo.toml: 0.5.0 → 0.6.0
  - GENERATOR_FAMILY token: `"md-codec 0.5"` → `"md-codec 0.6"`
  - v0.1.json + v0.2.json fully regenerated (Tag-space rework changes every `expected_bytecode_hex`; corpus expansion adds new positive vectors)
  - vectors_schema.rs SHA pins updated
  - CHANGELOG `[Unreleased]` → `[0.6.0] — <date>` with consolidated entries from all v0.6 strip work + the `decoded-string-data-memory-microopt` change already landed in `d79125d`
  - MIGRATION's `v0.5.x → v0.6.0` section extended to cover:
    - The `DecodedString.data` field removal (already documented; reaffirm)
    - The strip (no longer-validated tap-leaf shapes — `Error::TapLeafSubsetViolation` no longer fired by default; explicit-call path documented for callers who want it)
    - Tag-space reorganization (every Tag value changes; consumers depending on specific bytes break)
    - Spec changes (BIP MUST → MAY clause; new §"Signer compatibility (informational)")
- **Why deferred:** Coordination only — no per-entry blocker. Lands at the v0.6 release cut.
- **Status:** resolved md-codec-v0.6.0 (commit `e8243fd`, tag `md-codec-v0.6.0`). Cargo.toml bumped 0.5.0 → 0.6.0; `GENERATOR_FAMILY` rolled to `"md-codec 0.6"`; vector files regenerated; SHA pins updated; CHANGELOG `[0.6.0]` and MIGRATION `v0.5.x → v0.6.0` sections shipped.
- **Tier:** v0.6 (closed)

### `v07-tap-leaf-iterator-with-index-coverage` — Phase 4 must include a multi-leaf DFS-pre-order leaf-index attribution test

- **Surfaced:** v0.7.0 Phase 1 reviewer (Opus). The Phase 1 commit `de63db3` deleted MD-codec test `tap_leaf_subset_violation_carries_leaf_index` (LI2) on the rationale that "leaf-index attribution moves to md-signer-compat." LI2 was the only test exercising **multi-leaf DFS-pre-order** `leaf_index` correctness.
- **Where:** `crates/md-signer-compat/src/tests.rs`.
- **What:** at least one multi-leaf test where the offending operator's `leaf_index` in the resulting `Error::SubsetViolation` is *derived* from a tap-tree walker (the v0.7+ "iterate tap leaves and call `validate(...)` per leaf" primitive), **not** supplied as a constant.
- **Status:** resolved (this commit). md-signer-compat exposes `pub fn validate_tap_tree(subset, tap_tree)` that walks `TapTree::leaves()` in DFS pre-order, threading an enumerated `leaf_index` per call. New test `tests::validate_tap_tree_attributes_violation_to_dfs_pre_order_index` constructs a 3-leaf `{leaf_0, {leaf_1_violator, leaf_2}}` shape, places `sha256(...)` (out of COLDCARD_TAP) at leaf 1, and asserts the resulting `SubsetViolation.leaf_index == Some(1)`.
- **Tier:** v0.7-blocker (closed)

### `v07-phase2-asymmetric-byte-order-test-inputs` — palindromic byte-fill defeats decode-direction asymmetric-reversal check

- **Surfaced:** v0.7.0 Phase 2 reviewer (Opus). `hash_terminals_encode_internal_byte_order_with_decode_round_trip` originally used palindromic `[0xAA; 32]` and `[0xBB; 20]`; reversing a constant-fill array is a no-op, so a symmetric encode/decode reversal bug (the bug class Plan reviewer #1 Concern 5 motivated) would not be caught.
- **Where:** `crates/md-codec/src/bytecode/hand_ast_coverage.rs::hash_terminals_encode_internal_byte_order_with_decode_round_trip`.
- **What:** replace constant fills with strictly increasing patterns: `known_32 = [0x00..0x1F]` via `std::array::from_fn(|i| i as u8)`; `known_20 = [0x80..0x93]` via `std::array::from_fn(|i| 0x80 + i as u8)`.
- **Status:** resolved (this commit). Inputs are now asymmetric so any reversal is observable.
- **Tier:** v0.7-blocker (closed)

### `v07-decode-rejects-sh-bare-rename` — rename or delete `decode_rejects_sh_bare` (mislabelled in v0.6)

- **Surfaced:** v0.7.0 Phase 1 reviewer (Opus). `crates/md-codec/src/bytecode/decode.rs:2413-2423` defines `decode_rejects_sh_bare` whose inline comment says `// [Sh=0x03, Bare=0x07, ...]`, but in v0.6 byte 0x07 is `Tag::TapTree`. The test passes (assertion is `msg.contains("sh(")` — generic) but is misleadingly named.
- **Where:** `crates/md-codec/src/bytecode/decode.rs::tests::decode_rejects_sh_bare`.
- **What:** rename to `decode_rejects_sh_taptree` and update the inline byte comment, or delete as redundant with `decode_rejects_sh_inner_script_andv` + `decode_rejects_sh_key_slot`.
- **Status:** resolved md-codec-v0.7.1. Renamed `decode_rejects_sh_bare` → `decode_rejects_sh_with_disallowed_inner_tag`; rewrote with symbolic `Tag::Sh.as_byte()` / `Tag::TapTree.as_byte()` and updated docstring/inline comment to match v0.6 reality.
- **Tier:** v0.7.x (closed)

### `v07-stale-byte-annotation-comments` — sweep stale v0.5 byte comments in integration tests

- **Surfaced:** v0.7.0 Phase 1 reviewer (Opus). Several integration tests use symbolic `Tag::Foo.as_byte()` (correct) but trail with stale `// 0x32`, `// 0x19`, `// 0x33`, `// 0x05` annotations naming **v0.5** byte values. Tests function correctly; comments mislead future readers.
- **Where:** `crates/md-codec/tests/fingerprints.rs:175`; `crates/md-codec/tests/conformance.rs:765, 768, 771, 773, 1083, 650`.
- **What:** mechanical sed sweep — either remove the byte-value comment entirely or update to the v0.6 byte.
- **Status:** resolved md-codec-v0.7.1. Removed stale v0.5 byte comments at `tests/fingerprints.rs:175` and 6 sites in `tests/conformance.rs` (lines 650, 765, 768, 771, 773, 1083). Symbolic `Tag::Foo.as_byte()` refs now stand on their own.
- **Tier:** v0.7.x (closed)

### `v07-taptree-diagnostic-runtime-byte` — refactor TapTree-at-top diagnostic to format byte at runtime

- **Surfaced:** v0.7.0 Phase 1 reviewer (Opus). Phase 1 added the literal `(0x07)` to `decode_descriptor`'s `Tag::TapTree` arm error message at `decode.rs:78-82` (test-driven; see `taptree_at_top_level_produces_specific_diagnostic`). Hardcoding the byte creates a Tag-byte-rolling drift liability — a future major release re-numbering TapTree must update both the production string AND test pin in lockstep.
- **Where:** `crates/md-codec/src/bytecode/decode.rs:78-82` and the test at `decode.rs:2429-2447`.
- **What:** refactor to `format!("TapTree (0x{:02X}) is not a valid top-level...", Tag::TapTree.as_byte())` so the byte tracks the enum.
- **Status:** resolved md-codec-v0.7.1. Diagnostic message now formats the byte at runtime via `Tag::TapTree.as_byte()`; future Tag-byte rolls won't desync the production string from the enum.
- **Tier:** v0.7.x (closed)

### `v07-phase2-decoder-arm-cursor-sentinel-pattern` — strengthen decoder-arm cursor-consumption assertions with trailing sentinel

- **Surfaced:** v0.7.0 Phase 2 reviewer (Opus). All six `decoder_arm_*` tests in `crates/md-codec/src/bytecode/hand_ast_coverage.rs` assert `cur.is_empty()` after decode, but the wire bytes contain no trailing sentinel — over-consumption surfaces as `UnexpectedEnd` rather than as a cursor-position drift, and under-consumption can't surface at all.
- **Where:** `crates/md-codec/src/bytecode/hand_ast_coverage.rs` (six `decoder_arm_*` tests); `crates/md-codec/src/bytecode/cursor.rs` (add `pub(crate) fn remaining(&self) -> &[u8]`).
- **What:** add a `0xFF` sentinel byte to each test's wire form and assert remaining cursor contents equal `[0xFF]`. Requires adding `Cursor::remaining()` (~3 lines).
- **Why deferred:** decoder primitives are themselves tightly bounded; bug class is unlikely in practice. Defensive nice-to-have.
- **Status:** resolved md-codec-v0.7.1. Added `pub(crate) Cursor::remaining()` (test-only via `#[cfg(test)]`) and converted all six `decoder_arm_*` tests in `hand_ast_coverage.rs` to the trailing-sentinel pattern.
- **Tier:** v0.7.x (closed)

### `v07-phase2-or-c-unwrapped-test-docstring-drift` — `or_c_unwrapped_tap_leaf_byte_form` docstring promises decoder-branch assertion the test body doesn't perform

- **Surfaced:** v0.7.0 Phase 2 reviewer (Opus). Docstring describes a two-branch decoder-behavior policy ("If decoder accepts... If decoder rejects, this test asserts the rejection diagnostic"), but the test only asserts encoder wire bytes — the decoder is never run on `out`.
- **Where:** `crates/md-codec/src/bytecode/hand_ast_coverage.rs::or_c_unwrapped_tap_leaf_byte_form`.
- **What:** either tighten the docstring to "encoder wire-form pin only" or extend the test to run the decoder and assert the actual outcome.
- **Why deferred:** test passes; coverage is provided by the companion `t_or_c_tap_leaf_round_trips`. Future-reader-confusion issue, not functional.
- **Status:** resolved md-codec-v0.7.1. Tightened docstring to "encoder wire-byte pin only" with a one-line pointer to `t_or_c_tap_leaf_round_trips` for the round-trip variant.
- **Tier:** v0.7.x (closed)

### `v07-coldcard-multi-a-citation-gap` — `COLDCARD_TAP` initially included `multi_a` not cited in vendor source

- **Surfaced:** v0.7.0 Phase 4 reviewer (Opus). The cited Coldcard `docs/taproot.md` allowed-descriptors list documents only `sortedmulti_a` for tap-leaf multisig — bare `multi_a` was not cited. The initial Phase 4 commit included `multi_a` in `COLDCARD_TAP.allowed_operators`.
- **Where:** `crates/md-signer-compat/src/coldcard.rs::COLDCARD_TAP`.
- **What:** removed `multi_a` from the allowlist; rustdoc explicitly notes "multi_a deliberately omitted" with rationale tying back to the cited source. If a future Coldcard revision admits `multi_a`, add back with a citation note.
- **Status:** resolved (folded inline post-Phase-4)
- **Tier:** v0.7-blocker (closed)

### `v07-tap-tree-leaves-docstring-iterator-shape` — `validate_tap_tree` docstring drift on iterator-yield shape

- **Surfaced:** v0.7.0 Phase 4 reviewer (Opus). `lib.rs::validate_tap_tree` originally claimed `TapTree::leaves()` "yields `(depth, leaf_ms)` tuples"; reality is a `TapTreeIterItem` struct with `.miniscript()` / `.depth()` accessors.
- **Where:** `crates/md-signer-compat/src/lib.rs::validate_tap_tree`.
- **What:** updated docstring to describe the actual `TapTreeIterItem` struct API.
- **Status:** resolved (folded inline post-Phase-4)
- **Tier:** v0.7-blocker (closed)

### `v07-ledger-rustdoc-variant-enumeration-incomplete` — `LEDGER_TAP` rustdoc enumerates 7/16 vanadium variants

- **Surfaced:** v0.7.0 Phase 4 reviewer (Opus). `ledger.rs` rustdoc lists 7 variants from `cleartext.rs` but the vanadium enum has 16. Operator-set is still sound (the omitted variants use already-listed operators) but doc enumeration is incomplete.
- **Where:** `crates/md-signer-compat/src/ledger.rs::LEDGER_TAP` rustdoc.
- **What:** expand to full 16-variant list, OR change framing to "representative subset" with note that the operator union covers all variants.
- **Why deferred:** allowlist itself is correct; doc-enumeration completeness is cosmetic.
- **Status:** resolved md-codec-v0.7.1. Reframed the variant list as "representative subset" with explicit note that the operator union remains complete; added pointers to the additional single-sig + timelock and multisig + locktime shapes.
- **Tier:** v0.7.x (closed)

### `v07-md-signer-compat-shared-test-key-helpers` — `dummy_key_a` / `dummy_key_b` duplicated across crates

- **Surfaced:** v0.7.0 Phase 4 reviewer (Opus). Same two test pubkey strings appear in both `crates/md-codec/src/bytecode/hand_ast_coverage.rs` and `crates/md-signer-compat/src/tests.rs`.
- **Where:** both files.
- **What:** consider a shared test-only helper module (e.g., a `pub(crate)` module in md-codec exported under `#[cfg(feature = "test-helpers")]`).
- **Why deferred:** small duplication; cross-crate test utility is over-engineering at this scale.
- **Status:** resolved md-codec-v0.7.1. Added `pub mod test_helpers` in md-codec gated on `#[cfg(any(test, feature = "test-helpers"))]` exposing `dummy_key_a/b/c()`. md-signer-compat enables the `test-helpers` feature in `[dev-dependencies]` and consumes them; `hand_ast_coverage.rs` follows the same pattern.
- **Tier:** v0.7.x (closed)

### `v07-historical-coldcard-const-visibility` — tighten `pub const HISTORICAL_COLDCARD_TAP_OPERATORS` to `pub(crate)`

- **Surfaced:** v0.7.0 Phase 3 reviewer (Opus). Plan §3.3 specified `const` (private); implementation used `pub const`. md-signer-compat (Phase 4) defines its own `COLDCARD_TAP.allowed_operators` array and does not reference this constant; the only in-tree consumer is the same-module back-compat shim.
- **Where:** `crates/md-codec/src/bytecode/encode.rs` `pub const HISTORICAL_COLDCARD_TAP_OPERATORS`.
- **What:** change `pub const` → `pub(crate) const`.
- **Why deferred:** harmless as `pub`; modest auditor-facing value; not breaking.
- **Status:** resolved md-codec-v0.7.3. Tightened to `pub(crate) const`. Added rustdoc note explaining the visibility choice (md-signer-compat defines its own `COLDCARD_TAP.allowed_operators` and does not reference the historical constant).
- **Tier:** v0.7.x (closed)

### `v07-walker-deepest-violation-pin-test` — add regression test for depth-first leaf-first walker semantics

- **Surfaced:** v0.7.0 Phase 3 reviewer (Opus). The new walker's "depth-first leaf-first reporting → deepest violation surfaced" contract is observable in `taproot_rejects_wrapper_alt_outside_subset` (which defensively allows both old "thresh" and new "s:" outcomes) but no test pins it. A future regression flipping back to top-down rejection would pass existing tests.
- **Where:** `crates/md-codec/src/bytecode/hand_ast_coverage.rs` (suggested location).
- **What:** ≤20-line hand-AST test on `thresh(1, sha256(H))` with empty allowlist — assert `operator == "sha256"`, not `"thresh"`.
- **Why deferred:** test passes today (semantics match the docstring); regression-pin coverage is defensive.
- **Status:** resolved md-codec-v0.7.1. Added `walker_reports_deepest_violation_first` in `hand_ast_coverage.rs` exercising `thresh(1, sha256(H))` against an empty allowlist; asserts the reported operator is `"sha256"`.
- **Tier:** v0.7.x (closed)

### `v07-phase5-tap-none-test` — add unit test for `ScriptContext::Tap` with `internal_key=None` NUMS path

- **Surfaced:** v0.7.0 Phase 5 reviewer (Opus). Initial Phase 5 unit-test set covered `Some(internal)` for Tap; the `None` (NUMS-unspendable internal-key) path — the very behaviour Plan reviewer #1 Concern 2 motivated — was untested.
- **Where:** `crates/md-codec/src/policy_compiler.rs::tests`.
- **What:** new test `tap_pk_with_nums_internal_key_compiles_and_encodes` exercising `internal_key = None`.
- **Status:** resolved (folded inline post-Phase-5)
- **Tier:** v0.7-blocker (closed)

### `v07-phase5-cli-test-gate` — tighten CLI test gate from `compiler` to `all(compiler, cli)`

- **Surfaced:** v0.7.0 Phase 5 reviewer (Opus). `tests/cli.rs::md_from_policy_segwitv0_pk_emits_bytecode_hex` was gated `#[cfg(feature = "compiler")]`, but the `md` binary requires `cli`. A `compiler`-only feature combo would compile the test and then panic at `cargo_bin("md")`.
- **Where:** `crates/md-codec/tests/cli.rs::md_from_policy_segwitv0_pk_emits_bytecode_hex`.
- **What:** changed gate to `#[cfg(all(feature = "compiler", feature = "cli"))]`.
- **Status:** resolved (folded inline post-Phase-5)
- **Tier:** v0.7-blocker (closed)

### `v07-phase5-policyscopeviolation-rustdoc` — `Error::PolicyScopeViolation` rustdoc still says "v0.1 scope"

- **Surfaced:** v0.7.0 Phase 5 reviewer (Opus). `error.rs::Error::PolicyScopeViolation` rustdoc opens with "Policy violates the v0.1 implementation scope" — pre-strip language. Phase 5's `policy_to_bytecode` wrapper widens the variant's semantic load by also returning it when the compiler emits a top-level shape MD can't encode.
- **Where:** `crates/md-codec/src/error.rs::Error::PolicyScopeViolation` rustdoc.
- **What:** add a one-line note: "Also returned by `policy_to_bytecode` when the compiler emits a top-level shape MD does not encode."
- **Why deferred:** doc-only refresh; low-priority.
- **Status:** resolved md-codec-v0.7.1. Reframed the rustdoc to drop the v0.1-only language ("Policy violates MD encoding scope" instead of "v0.1 implementation scope"), added the `policy_to_bytecode` use-site note, and updated the Display message accordingly.
- **Tier:** v0.7.x (closed)

### `v07-phase5-cli-context-error-msg` — `--context` error message omits `wsh`/`tr` aliases

- **Surfaced:** v0.7.0 Phase 5 reviewer (Opus). `cmd_from_policy` accepts `segwitv0`/`wsh`/`tap`/`tr` (case-insensitive) but the bail message says "must be one of: segwitv0, tap; got X".
- **Where:** `crates/md-codec/src/bin/md/main.rs::cmd_from_policy`.
- **What:** update error message to enumerate all four accepted forms.
- **Why deferred:** user-facing UX nit; not blocking.
- **Status:** resolved md-codec-v0.7.1. Bail message now reads `"--context must be one of: segwitv0, wsh, tap, tr; got X"`.
- **Tier:** v0.7.x (closed)

### `v07-phase2-decode-helpers-pub-super-tightening` — tighten `decode_tap_miniscript` / `decode_tap_terminal` to `pub(super)`

- **Surfaced:** v0.7.0 Phase 2 reviewer (Opus). Both functions are currently `pub(crate)` solely for `bytecode::hand_ast_coverage` (sibling test module). `pub(super)` would suffice and constrain visibility tighter.
- **Where:** `crates/md-codec/src/bytecode/decode.rs:583` (`decode_tap_miniscript`), `crates/md-codec/src/bytecode/decode.rs:608` (`decode_tap_terminal`).
- **What:** change `pub(crate)` → `pub(super)` for both.
- **Why deferred:** `pub(crate)` is already well-scoped (no public API leakage); tightening defensively guards against unintended future cross-module use, but no concrete risk today.
- **Status:** resolved md-codec-v0.7.3. Both `decode_tap_miniscript` and `decode_tap_terminal` tightened to `pub(super)`. The only sibling consumer (`bytecode::hand_ast_coverage`) compiles unchanged.
- **Tier:** v0.8 housekeeping (closed early; pulled into v0.7.3 cleanup pass)

### `v07-n_taptree_at_top_level-description-stale-v05-byte` — `n_taptree_at_top_level` description still says "0x08"

- **Surfaced:** v0.7.0 Phase 1 reviewer (Opus). `vectors.rs:1867-1895` — the in-source comment AND the public-facing `description` field of the negative vector say "Tag::TapTree (0x08)" (v0.5). The vector itself is correct (built via `Tag::TapTree.as_byte()` symbolic refs). The `description` ships in `tests/vectors/v0.2.json` and is part of the v0.2 schema-2 SHA pin; updating requires regenerating the SHA.
- **Where:** `crates/md-codec/src/vectors.rs:1867-1895`.
- **What:** update both the in-source comment and the `description` field to "(0x07)" when v0.7 regenerates `v0.2.json`.
- **Status:** resolved md-codec-v0.7.3. Description string updated to "(0x07)" in both the in-source comment and the negative-vector `description` field. Vector files regenerated; v0.2.json SHA pin updated `014006ea…f99628` → `4f8afba0…dbb8b9`.
- **Tier:** v0.7-Phase-6 (closed)

### `v07-from-policy-internal-key-semantic-clarification` — `--internal-key` is upstream `unspendable_key`, not "force this internal key"

- **Surfaced:** v0.7.x docs-sync smoke test (2026-04-29). The `md from-policy --context tap --internal-key K1` CLI flag is plumbed through to rust-miniscript's `Concrete::compile_tr(unspendable_key=Some(K1))`. Upstream `compile_tr` calls `extract_key(unspendable_key)` first, which extracts a key from the *policy itself* if it can serve as the internal key (single-key spend), and only falls back to `K1` if no such extraction is possible. So passing `--internal-key K1` with a policy like `pk(K2)` produces `tr(K2)` (the optimizer picks K2; K1 is unused), not `tr(K1, pk(K2))`. The behaviour is upstream-correct but easily surprises a CLI user who expects "force K1 as internal".
- **Where:** `crates/md-codec/src/policy_compiler.rs::policy_to_bytecode` rustdoc + `crates/md-codec/src/bin/md/main.rs::cmd_from_policy` `--internal-key` argument doc string.
- **What:** rename the CLI flag and parameter doc to clarify the fallback semantic — e.g., `--unspendable-key <KEY>` (mirroring upstream naming) — and update the rustdoc on `policy_to_bytecode` to spell out the precedence rule. Optionally add an example in the CLI help showing how to force a specific internal key (use a policy that doesn't contain a single extractable key, or pre-build the descriptor manually).
- **Status:** resolved md-codec-v0.7.2. CLI flag renamed `--internal-key` → `--unspendable-key`; library parameter on `policy_to_bytecode` renamed `internal_key` → `unspendable_key` (Rust positional-args semantics: not ABI-breaking; existing callers compile unchanged). Module rustdoc gains a "Tap-context internal key — `unspendable_key` semantics" section that describes the upstream precedence rule (`extract_key` first, fallback parameter second). Workaround for "force this internal key" use case spelled out: build the `Tr` descriptor manually via `miniscript::Descriptor::new_tr` and pass through `WalletPolicy::from_descriptor`.
- **Tier:** v0.7.x (closed)

### `v010-p1-origin-paths-count-too-large-zero-message` — `OriginPathsCountTooLarge` Display message awkward when `count = 0`

- **Surfaced:** v0.10.0 Phase 1 reviewer (commit `3b38242`). Spec §3 line 457 explicitly endorses one variant covering both bounds (`count == 0` and `count > 32`), and the implementer's docstring at `crates/md-codec/src/error.rs:514–530` documents this correctly. However, the `#[error("OriginPaths count {count} exceeds maximum {max}")]` template renders as "OriginPaths count 0 exceeds maximum 32" for the count-zero case — grammatical but semantically weak ("0 doesn't exceed 32 in arithmetic terms; it's just structurally invalid").
- **Where:** `crates/md-codec/src/error.rs:524` (the `#[error(...)]` template on `BytecodeErrorKind::OriginPathsCountTooLarge`).
- **What:** consider rewording the template to cover both bounds explicitly, e.g., `"OriginPaths count {count} is out of range (must be 1..={max})"`. Phase 2 introduces the actual `decode_origin_paths` callsite that surfaces this message; that's a natural revisit point.
- **Why deferred:** the variant name + docstring already match the spec convention; the wording nit doesn't block Phase 2 and the actual check site lands in Phase 2, so any rewording is best done together with the implementation that exercises both bounds.
- **Status:** resolved by Phase 2 inline-fix (per Phase 2 reviewer recommendation). Template reworded to `"OriginPaths count {count} is out of range (must be 1..={max})"`. Resolved in the Phase-2-followup commit alongside MAX_PATH_COMPONENTS cap + OriginPaths helpers.
- **Tier:** v0.10-nice-to-have (closed)

### `v010-p2-origin-paths-round-trip-spec-byte-pin` — Example B round-trip test pins prefix only

- **Surfaced:** v0.10.0 Phase 2 reviewer (commit `1936b19`). The
  `encode_origin_paths_round_trip_three_paths` test at
  `crates/md-codec/src/bytecode/path.rs:1196–1227` asserts the first 5
  bytes of the encoded output (`bytes[0..=4] = [0x36, 0x03, 0x05, 0x05, 0xFE]`)
  but does not byte-pin the remaining 6 bytes (`04 61 01 01 C9 01`) of
  the explicit-form third path. Spec §2 line 157 pins the full 11-byte
  sequence `36 03 05 05 FE 04 61 01 01 C9 01`. The
  `assert_eq!(recovered, paths)` round-trip provides indirect
  verification but doesn't catch byte-level encoder drift in the
  explicit-path tail.
- **Where:** `crates/md-codec/src/bytecode/path.rs:1196–1227`
  (`encode_origin_paths_round_trip_three_paths`).
- **What:** strengthen the assertion to pin the full 11-byte sequence
  per spec §2:
  ```rust
  assert_eq!(
      bytes,
      vec![0x36, 0x03, 0x05, 0x05, 0xFE, 0x04, 0x61, 0x01, 0x01, 0xC9, 0x01],
      "must match spec §2 Example B byte sequence"
  );
  ```
  Optionally keep the existing prefix-byte asserts as
  documentation-of-layout, or replace them with the full-sequence
  assert.
- **Why deferred:** the round-trip eq-comparison provides indirect
  verification of the explicit-path tail bytes (an encoder bug there
  would cause decode mismatch), so the test is correct, just not
  spec-pinned at maximum strength. Phase 4 conformance vectors will
  also pin Example B's full byte sequence as a fixture, providing a
  second line of defense.
- **Status:** resolved by md-codec-v0.10.0 phase 4 (commit 2e61d38)
- **Tier:** v0.10-nice-to-have (closed)
- **Resolution:** Phase 4 added the `o2_vector_origin_paths_block_matches_spec_example_b` test in `crates/md-codec/src/vectors.rs:2693-2707`, which asserts the o2 corpus vector's `expected_bytecode_hex` contains the full 11-byte SPEC §2 Example B sequence `36030505fe04610101c901`. This is the "second line of defense" coverage the original entry pre-acknowledged. The `path.rs:1199` round-trip test itself remains prefix-pinned (still relies on the `assert_eq!(recovered, paths)` round-trip for the explicit-path tail), but the spec byte sequence is now durably pinned at the corpus layer; an encoder regression on the explicit-path tail would surface as a vector hex mismatch + corpus SHA delta.

### `v010-p3-tier-2-kiv-walk-deferred` — Tier 2 KIV walk in `placeholder_paths_in_index_order` is stubbed in v0.10.0

- **Surfaced:** v0.10.0 Phase 3 implementation. The 4-tier per-`@N`-path
  precedence chain in `WalletPolicy::placeholder_paths_in_index_order`
  (spec §4) defines Tier 2 as "walk the key-information vector for
  concrete-key policies, extracting per-key origin paths in
  placeholder-index order." Phase 3 ships this tier as a stub returning
  `Ok(None)` (always falls through to Tier 3 shared-path fallback).
- **Where:**
  - `crates/md-codec/src/policy.rs` — `WalletPolicy::try_extract_paths_from_kiv` (TODO comment in body).
  - `WalletPolicy::placeholder_paths_in_index_order` consults this method as Tier 2.
- **What:** wire up Tier 2 to walk the policy's key-information vector
  and extract `(fingerprint, origin_path)` for each placeholder in
  placeholder-index order. The natural input is the materialized
  `Descriptor<DescriptorPublicKey>` already built in `to_bytecode`, but
  `descriptor.iter_pk()` traverses in AST order — for `sortedmulti(...)`
  this yields lex-sorted-by-pubkey-bytes order, NOT placeholder-index
  order. A correct implementation must either (a) walk the inner
  `WalletPolicy.template` (a `Descriptor<KeyExpression>`) using each
  `KeyExpression`'s `index` field to map AST position → placeholder
  index — currently blocked because the fork's `WalletPolicy.template`
  field is private (no public accessor), or (b) perform the walk at the
  policy-construction layer (e.g., on `from_descriptor` ingestion) and
  cache per-`@N` paths in a new `WalletPolicy` field — the
  decoded-vs-source-of-truth state machine then mirrors
  `decoded_origin_paths`. The implementation choice and surface impact
  warrant a separate design pass.
- **Why deferred:** v0.10.0's hot path for per-`@N` divergence is the
  Tier 0 `EncodeOptions::origin_paths` override (test-vector generation)
  and the Tier 1 `decoded_origin_paths` round-trip stability source
  (any policy decoded from a `Tag::OriginPaths`-bearing bytecode). Both
  are fully wired and tested in Phase 3. Tier 2 only matters for the
  freshly-parsed concrete-key descriptor case — a path that exists in
  v0.x ≤ 0.9 today and silently flattens to shared-path. Stubbing Tier
  2 in v0.10.0 leaves that path's behavior IDENTICAL to v0.9 (Tier 3
  shared-path fallback fires, encoder emits `Tag::SharedPath`),
  preserving wire-format byte-equality for v0.9-shaped concrete-key
  inputs. The known bug — losing per-`@N` divergence on
  freshly-parsed-from-string concrete-key policies — is a known
  v0.x limitation that v0.10.0 does not regress and does not fully
  fix; v0.11 (or a v0.10.1) closes it with the API design above.
- **Status:** resolved by md-codec-v0.10.1 (commit `c3a290d`). Wired up `try_extract_paths_from_kiv` to walk `inner.template().iter_pk()` + `inner.key_info()` in lockstep and extract per-`@N` origin paths in placeholder-index order, using the fork's new public `template()` and `key_info()` accessors (apoelstra/rust-miniscript#2, available via the workspace `[patch]` block). Tier 2 is gated to skip when `decoded_shared_path` is populated, preventing dummy-key origin leakage on `from_bytecode`-materialized policies. The headline behavior change — concrete-key descriptors with divergent origin paths now emit `Tag::OriginPaths` instead of being silently flattened to `Tag::SharedPath` via Tier 3 — is pinned by the new `tier_2_drives_encoder_dispatch_change_from_v0_10_0` test in `crates/md-codec/src/policy.rs`.
- **Tier:** v0.11 → v0.10.1 (closed; folded into v0.10.1)

### `cli-policy-id-fingerprint-flag` — CLI rendering of `PolicyId::fingerprint()` short form

- **Surfaced:** v0.10 Phase 5 implementation 2026-04-29 (commit pending). Spec Q13 added the `PolicyId::fingerprint() -> [u8; 4]` API; the implementation plan suggested optional CLI integration for printing the short form via something like `md encode --fingerprint`.
- **Where:** `crates/md-codec/src/bin/md/main.rs` — `cmd_encode` currently prints `Policy ID: {12 words}` unconditionally (line ~381); there is no toggle to switch to a short hex form. The natural rendering is `0x{:08x}`.
- **What:** Add a CLI flag (or new subcommand) that renders the freshly-computed `PolicyId` as `fingerprint()` (8 hex chars / 4 bytes) instead of, or in addition to, the 12-word form. Use cases per Q13: log lines, CLI scripts, minimal-cost engraving anchor for users who don't want the full 12-word phrase. The library API ships in v0.10.0; only the CLI toggle is deferred.
- **Why deferred:** The obvious flag name `--fingerprint` is already taken in `md encode` for embedding **master-key** fingerprints (`@INDEX=HEX` form, BIP §"Fingerprints block"). Adding an output-rendering `--fingerprint` flag to the same subcommand creates a flag-name conflict that cannot be resolved without either renaming the existing flag (wire-affecting CLI break) or picking a different name (e.g., `--policy-id-fingerprint`, `--short-id`) which then no longer matches the API method name. Either choice deserves a small design pass with the user — not in scope for Phase 5's "small additive change" criterion. Library API is sufficient for downstream tools to use immediately.
- **Status:** resolved by commit `82af0ea` (folded into v0.10.0). User chose the rename path: existing `--fingerprint` renamed to `--master-key-fingerprint` (CLI break, no deprecation alias per pre-v1.0 break freedom); new `--policy-id-fingerprint` flag added as a boolean that additively prints `Policy ID fingerprint: 0x{:08x}` after the existing 12-word `Policy ID:` line. MIGRATION.md and CHANGELOG.md document the rename + addition.
- **Tier:** v0.11 (closed; folded into v0.10.0)

### `bip-byte-layout-examples-stale-v0_6-renumber` — BIP byte-layout examples reference pre-v0.6 tag values

- **Surfaced:** v0.10.0 Phase 6 implementer (commit `b5f00f9`). While editing `bip/bip-mnemonic-descriptor.mediawiki` to insert the v0.10 OriginPaths sections, the implementer noticed that several byte-layout examples elsewhere in the BIP still reference `Tag::Placeholder (0x32)` and `Tag::SharedPath (0x33)` — both stale per the v0.6 tag-renumber. Correct values per the current tag table: `Placeholder = 0x33`, `SharedPath = 0x34`. The v0.9.1 CHANGELOG noted that the rustdoc sweep for the v0.5→v0.6 renumber missed some sites; the BIP byte-layout examples are an analogous unfixed sweep miss that pre-dates v0.10.
- **Where:** `bip/bip-mnemonic-descriptor.mediawiki` — multiple inline byte-layout example blocks. Pre-edit-shift line numbers were 478, 519, 532, 582, 593; post-Phase-6 edits these have shifted but the stale references remain. Locate via `rg -n '0x32|0x33' bip/bip-mnemonic-descriptor.mediawiki` and check each occurrence against the v0.6+ tag table at the top of the file.
- **What:** sweep all `0x32` references (should be `0x33` for Placeholder) and `0x33`-as-SharedPath references (should be `0x34`). The Phase-6 implementer fixed exactly one occurrence (the path-declaration tag-table location adjacent to the v0.10 insertion); the rest were left to avoid scope creep into a v0.5→v0.6 sweep that pre-dates v0.10.
- **Why deferred:** stale references pre-date v0.10 (v0.5→v0.6 renumber sweep miss); not in Phase 6 scope; would balloon the v0.10 docs commit. Best done as a v0.10.0.1 patch cleanup after the v0.10.0 release ships.
- **Status:** resolved by Phase-6-followup commit (folded into v0.10.0). Swept all 5 stale occurrences in the byte-layout example for `wsh(multi(2,@0/**,@1/**))`: the example bytecode line + 4 annotation rows now correctly reference `Tag::SharedPath = 0x34`, `Tag::Multi = 0x08`, `Tag::Placeholder = 0x33`. Also fixed the "Key references" section (`0x32 <index>` → `0x33 <index>`).
- **Tier:** v0.10.0.1-cleanup (closed; folded into v0.10.0)

### `to-bytecode-multipath-shared-at-n-set-key-info-mismatch` — `to_bytecode` fails for multipath-shared `@N` policies

- **Surfaced:** v0.10.1 implementation 2026-04-29. Surfaced while writing the Tier 2 multipath-shared `@N` test (`tier_2_multipath_shared_at_n_collapses_to_single_path`): the Tier 2 walk itself works correctly (collapses 2 AST positions for `@0` into a single output slot), but `WalletPolicy::to_bytecode` fails with `PolicyScopeViolation("Invalid key information for WalletPolicy template")` because its dummy-key materialization step calls `set_key_info(&dummy_keys(count))` where `count == key_count()` (distinct placeholders) — but the fork's `set_key_info` requires `keys.len() == template.iter_pk().count()` (AST positions). For multipath-shared `@N` (e.g., `sh(multi(1,@0/<0;1>/*,@0/<2;3>/*))`), AST positions > distinct placeholders, so the count mismatch errors out.
- **Where:** `crates/md-codec/src/policy.rs` — `to_bytecode`, the `dummies = dummy_keys(count)` / `inner_clone.set_key_info(&dummies)` block (~line 415-420). The `dummy_keys(count)` helper takes a placeholder count; need to broadcast the per-`@N` dummy across all AST positions referencing that placeholder.
- **What:** rework the dummy-key materialization step to walk `inner.template().iter_pk()` and produce a `Vec<DescriptorPublicKey>` of length `iter_pk().count()`, picking dummy keys by `ke.index.0` (so multiple AST positions for the same `@N` get the same dummy). The placeholder map (`dummy_key[i] → i`) stays keyed by placeholder index. This is a pure encoder fix; no wire-format change. The Tier 2 multipath-shared test's end-to-end encode assertion was elided in v0.10.1 to defer this fix — the test currently asserts only the Tier 2 walk's output, with a comment pointing here.
- **Why deferred:** v0.10.1's scope is narrowly Tier 2 KIV walk wire-up. Multipath-shared `@N` end-to-end encoding is a pre-existing gap (predates v0.10) — `to_bytecode` has likely always failed for these inputs because the BIP 388 syntax is unusual and the codec's test corpus didn't include them. Fixing it requires a small but distinct change in the dummy materialization path; better to land as its own commit with its own focused test (`to_bytecode_round_trip_multipath_shared_at_n`) than to bolt onto v0.10.1.
- **Status:** open
- **Tier:** v0.10.2 or v0.11

### `tier-2-gate-revisit-when-public-set-key-info-lands` — revisit Tier 2 gate if public key-info mutation API is added

- **Surfaced:** v0.10.1 review 2026-04-29 (review report: `design/agent-reports/v0-10-1-tier-2-kiv-walk-review.md`, Finding 3 / §5).
- **Where:** `crates/md-codec/src/policy.rs` — the gate at the start of Tier 2 in `placeholder_paths_in_index_order` (~line 513): `if self.decoded_shared_path.is_none() { ... }`. Also see the `from_bytecode` rustdoc at lines 633–634 mentioning a "restore flow via `set_key_info`" — currently aspirational since `WalletPolicy::inner()` returns `&` (immutable) and there is no public mutation API.
- **What:** the v0.10.1 Tier 2 gate uses `decoded_shared_path == Some` as a proxy for "key_info is dummy-populated by `from_bytecode` and must not be Tier-2-walked." This is correct today because the only way to populate `decoded_shared_path` is via `from_bytecode`, which always sets `key_info` to dummies. **If a future md-codec release adds a public mutation API** (e.g., a `WalletPolicy::set_key_info(&mut self, keys: &[DescriptorPublicKey])` that lets callers replace the dummies with real keys after a `from_bytecode` round-trip — the natural restore-flow shape), then a policy could legitimately have `decoded_shared_path = Some` AND real (non-dummy) `key_info` simultaneously. In that scenario, today's gate would silently skip Tier 2 and re-emit the original on-wire shared path instead of the new real-key paths — a silent correctness bug.
- **Why deferred:** no public mutation API exists today; the gate is airtight in practice for v0.10.1's API surface. This is a forward-looking tracking item: ensure the gate is reconsidered (e.g., replaced with an explicit `key_info_is_dummy: bool` flag set inside `from_bytecode` and cleared by any public mutator) **at the same time** any public set-key-info-on-decoded-policy API lands.
- **Status:** open
- **Tier:** vNext-API-expansion (block on whichever release introduces public key-info mutation).

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

### `v0-5-tap-terminal-name-and-tag-to-bip388-name-parallel-tables` — consolidate parallel hand-maintained operator-name tables

- **Surfaced:** Phase 2 code-quality reviewer (mid-execution, returned to controller; not persisted to `design/agent-reports/` at the time)
- **Status:** resolved `aa318ea` (Pass-2 batch) — refactored `tap_terminal_name` to delegate to `tag_to_bip388_name` via a new `terminal_to_tag` (`Terminal → Option<Tag>`) adapter. `tag_to_bip388_name` is now `pub(crate)` and is the single source of truth for tap-context operator names; `tap_terminal_name` falls back to a literal `"sortedmulti_a"` for `Terminal::SortedMultiA` (no Tag counterpart exists). New regression test `tap_terminal_name_delegates_to_tag_to_bip388_name` enumerates 30 (Terminal, Tag) pairs and locks the byte-identical guarantee. Reviewed by feature-dev:code-reviewer subagent — DONE, no concerns; report at `design/agent-reports/pass-2-item-1-review-tap-terminal-name-refactor.md`.
- **Tier:** v0.5-nice-to-have (closed)

### `v0-5-t7-chunking-boundary-misnomer` — T7 fixture doesn't actually cross chunking boundary

- **Surfaced:** Phase 6 reviewer (commit `7d6e278`)
- **Status:** resolved `aa318ea` (Pass-2 batch) — lane (a) rename selected. Renamed `tr_multi_leaf_chunking_boundary_md_v0_5` → `tr_multi_leaf_right_spine_md_v0_5` in `crates/md-codec/src/vectors.rs` and regenerated `crates/md-codec/tests/vectors/v0.2.json` via `gen_vectors --output`. Also corrected the description's leaf count (claimed 7, actually 6). v0.2.json SHA pin in `tests/vectors_schema.rs` updated `4206cce1...e2ea9c230` → `39476f04...81a8de3eed`. T7 remains a useful 6-leaf right-spine asymmetric regression anchor distinct from T3-T5; chunking-boundary coverage is provided elsewhere in the corpus.
- **Tier:** v0.5-nice-to-have (closed)

### `cargo-toml-crates-io-metadata-fields` — add `keywords`, `categories`, `documentation`, `homepage` to crate manifest

- **Surfaced:** Phase 3 (Cargo rename) code-quality reviewer
- **Status:** resolved `aa318ea` (Pass-2 batch) — added `homepage = "https://github.com/bg002h/descriptor-mnemonic"`, `documentation = "https://docs.rs/md-codec"`, `keywords = ["bitcoin", "bip388", "wallet", "descriptor", "bech32"]`, and `categories = ["cryptography::cryptocurrencies", "encoding", "command-line-utilities"]` to `crates/md-codec/Cargo.toml`. Verified parsing via `cargo metadata --no-deps`. Note: `cargo publish` is still blocked separately by the `external-pr-1-hash-terminals` git-pin entry; this commit closes only the metadata-fields gap.
- **Tier:** v1+ (closed; was originally v1+ publish-prep but applied during Pass-2 cleanup)

### `v0-5-spec-plan-encode-tap-subtree-entry-depth-bug` — spec + plan say `target_depth=1` at outer entry; should be `0`

- **Surfaced:** Phase 4 implementer (commit `bca2804`); Phase 4 reviewer confirmed independently
- **Status:** resolved `75e22f2` (`chore(v0.5 m2): fix target_depth literal in spec + plan`, on the v0.5 feature branch; merged to main via `865f889`). Working code at `encode.rs:166` was already correct; the doc fix updated `design/SPEC_v0_5_multi_leaf_taptree.md` §4 and `design/IMPLEMENTATION_PLAN_v0_5_multi_leaf_taptree.md` Phase 4 Task 4.3 to match.
- **Tier:** v0.5-must-close-before-ship (closed)

### `v0-7-phase-1-integration-test-rebaseline` — rebaseline 17 integration-test failures using v0.5 byte literals

- **Surfaced:** v0.7.0 Phase 1 Track A rebaseline pass (the 27 enumerated unit tests). After fixing those, `cargo test -p md-codec --no-fail-fast` still has ~17 failures across `tests/cli.rs`, `tests/conformance.rs`, and `tests/vectors_*.rs` (e.g., `md_encode_path_bip48_nested_resolves_to_indicator_0x06`, `rejects_invalid_bytecode_unexpected_tag`, `taproot_key_path_only_round_trips`, `fingerprints_block_byte_layout_matches_bip_example`, `tap_leaf_subset_violation_carries_leaf_index`, `schema_2_contains_v0_4_corpus_additions`, etc.). Failures partition into the same v0.5→v0.6 byte-shift class as the unit tests: `Tag::SharedPath` 0x33→0x34, `Tag::Placeholder` 0x32→0x33, plus a few hand-crafted byte vectors and asserted error-kind payloads (`UnexpectedTag { expected: 0x33, .. }`).
- **Where:** `crates/md-codec/tests/cli.rs` (2 tests), `crates/md-codec/tests/conformance.rs` (~10 tests), `crates/md-codec/tests/vectors_*.rs` (~3 tests), `crates/md-codec/tests/build_test_vectors.rs`
- **What:** Apply the same symbolic-`Tag::Foo.as_byte()` rebaseline pattern used in the unit-test rebaseline (decode/encode/path commit) to the integration test files. Some tests may also need vectors-corpus regeneration (`schema_2_contains_v0_*_corpus_additions` and `build_test_vectors_has_expected_corpus_count`). Goal: `cargo test -p md-codec --no-fail-fast` reports 0 failures.
- **Why deferred:** The Phase 1 sub-task instruction explicitly enumerated exactly 27 unit tests in `bytecode::{decode,encode,path}::tests`; the integration-test failures fall outside that scope and were not flagged in the plan's failing-test inventory (plan §1.1.2 estimated ~38 across all modules but the actual count is higher). Folding them into a separate commit keeps the unit-test commit narrowly scoped per acceptance criterion #1 (`cargo test -p md-codec --lib` returns 0 failures).
- **Status:** resolved (this commit). All integration tests rebaselined to v0.6 byte codes with symbolic `Tag::Foo.as_byte()` refs where helpful. Three subset-violation tests (`rejects_subset_violation`, `taproot_rejects_out_of_subset_sha256`, `taproot_rejects_wrapper_alt_outside_subset`, plus `tap_leaf_subset_violation_*` in v0_5_type_wiring.rs) were rewritten to call the v0.6 opt-in `validate_tap_leaf_subset` API directly — `to_bytecode` is scope-agnostic post-v0.6-strip. One test (`tap_leaf_subset_violation_carries_leaf_index` in v0_5_taptree_roundtrip.rs) was removed: the leaf-index attribution it pinned is a Layer-3 concern md-signer-compat (Phase 4) will own. Two corpus-count tests in `vectors_schema.rs` updated for v0.6's regenerated 43-vector corpus and the deletions of `n_tap_leaf_subset`, `n_top_bare`, `n_sh_bare`. `cargo test --workspace` reports 0 failures.
- **Tier:** v0.7-blocker (closed)

### `decoded-string-data-memory-microopt` — drop `DecodedString.data`, replace with accessor backed by `data_with_checksum`

- **Surfaced:** Phase B bucket A reviewer (Opus 4.7) on commit `5f13812`
- **Status:** resolved `d79125d` (Pass-3 batch) — `pub data: Vec<u8>` field removed from `DecodedString`; replaced with `pub fn data(&self) -> &[u8]` returning a slice into the existing `data_with_checksum` field (`&data_with_checksum[..len - checksum_len]`, where `checksum_len = 13` for `BchCode::Regular` and `15` for `BchCode::Long`). Internal `decoded_strings` buffer in `decode.rs` restructured from `Vec<(Vec<u8>, BchCode)>` to `Vec<DecodedString>` to preserve the single allocation produced by the BCH layer. CHANGELOG `[Unreleased]` (planned 0.6.0) and MIGRATION `v0.5.x → v0.6.0` sections added. Reviewed by feature-dev:code-reviewer subagent — DONE_WITH_CONCERNS; the reviewer's Important finding (a `data_with_checksum`-substitution trap in the migration docs) was addressed inline before commit. Report at `design/agent-reports/pass-3-item-2-review-decoded-string-data-accessor.md`.
- **Tier:** v0.3 → 0.6.0 (closed; was originally filed as v0.3 breaking-window candidate but applied during Pass-3 cleanup for inclusion in the 0.6.0 breaking release)

### v0.10.0

### `md-per-at-N-path-tag-allocation` — allocate per-`@N` origin path tag in md1 bytecode

- **Surfaced:** 2026-04-29 mk1 v0.1 closure-design pass (Q-4). Originally tracked in DECISIONS.md / mk1 closure as the open question that punted the wire-format decision to this repo. Companion: `md-per-N-path-tag-allocation` in `bg002h/mnemonic-key` `design/FOLLOWUPS.md`.
- **Where:**
  - `crates/md-codec/src/bytecode/path/` — Tag-table allocation
  - `bip/bip-mnemonic-descriptor.mediawiki` §"Tag table" + §"Path declaration" — wire-format extension
  - `design/SPEC_v0_X_*.md` — for the version that lands the change
- **What:** BIP 388 wallet policies routinely use different origin paths per cosigner (e.g. `[fp1/48'/0'/0'/2']xpub_A` and `[fp2/48'/0'/0'/100']xpub_B` in the same multisig). md1 v0.x today carries one shared `Tag::SharedPath`; per-`@N` paths require a new tag in the unallocated 0x36+ range, or a backfill of the 0x24-0x32 range. The exact tag-byte allocation is the md1 wire-format question; mk1 has already declared the cross-format authority-precedence semantics on its side (mk1's `origin_path` is authoritative; md1's per-`@N` path is descriptive — see mk1 BIP §"Authority precedence"), so the only decision pending here is the byte allocation.
- **Why deferred:** Wire-format change in md1 (semver-breaking). Not yet scheduled for a specific md-codec version; happens whenever per-`@N` paths become a planned md release feature.
- **Coordination:** When this lands, the mk1 BIP §"Authority precedence" subsection is unchanged (semantics already capture the contract). The mk1 side needs no wire-format change; md1's BIP gains the tag-table entry and a §"Per-`@N` path declaration" subsection.
- **Status:** resolved md-codec-v0.10.0. Allocated `Tag::OriginPaths = 0x36` (new, unallocated 0x36+ range; not a 0x24-0x32 backfill) carrying a dense per-`@N` path block. Reclaimed header bit 3 (was reserved-must-be-zero in v0.x ≤ 0.9) as the OriginPaths-present flag. `MAX_PATH_COMPONENTS = 10` enforced uniformly on both `Tag::SharedPath` and `Tag::OriginPaths`. Encoder auto-detects divergent-path policies and emits `OriginPaths` when needed, falling back to `SharedPath` otherwise; new `decoded_origin_paths` field on `WalletPolicy` preserves round-trip byte-stability. New `PolicyId::fingerprint() → [u8; 4]` short-identifier API. BIP gains §"Per-`@N` path declaration" + §"PolicyId types" teaching subsection + §"Authority precedence with MK" cross-reference. Wire-format break: v0.x ≤ 0.9 decoders reject v0.10 OriginPaths-using encodings via `Error::ReservedBitsSet` (intended forward-compat behavior); shared-path encodings remain byte-identical to v0.9 modulo family-token roll. Companion `md-per-N-path-tag-allocation` in mk1 closes in lockstep (mk1 BIP §"Authority precedence" semantics unchanged; mk1 side needs no wire-format change).
- **Tier:** v0.10 (closed)

---

## Convention notes for future agents

If you are an implementer or reviewer subagent dispatched on a task and you identify **minor items** (Important or Minor severity per the standard review rubric) that you are NOT fixing in your own commit, append an entry to this file in the same commit. Use a `<short-id>` like `<phase>-<keyword>` (e.g., `6c-corpus-fixture-helper`, `8a-vectors-schema-comment`).

If you are running in a **parallel batch** with sibling agents, do NOT write to this file directly — return your follow-up items in your final report and the controller will append them. Two parallel agents writing here cause merge conflicts.

If you are **closing** an item, edit its entry from `Status: open` → `Status: resolved <COMMIT>` and move the entry to the "Resolved items" section. Don't delete entries.
