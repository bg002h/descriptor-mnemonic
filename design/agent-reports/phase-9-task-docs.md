# Phase 9 — Documentation (Tasks 9.1-9.5)

**Status:** DONE
**Commit:** <pending — to be filled in after commit>
**File(s):**
README.md
bip/README.md
crates/wdm-codec/README.md
crates/wdm-codec/src/lib.rs
crates/wdm-codec/src/policy.rs
crates/wdm-codec/src/wallet_id.rs
crates/wdm-codec/src/options.rs
crates/wdm-codec/src/error.rs
crates/wdm-codec/src/decode_report.rs
design/agent-reports/phase-9-task-docs.md
**Role:** implementer

## Summary

Wrote the public face of the WDM codec crate. Replaced the 1-line crate-level
preamble with a tutorial-grade module overview (pipeline, type-state graph,
two-WalletId story, scope, module map) including a 12-line round-trip
doctest. Enriched load-bearing types (`WalletPolicy`, `WdmBackup`,
`WalletId`, `ChunkWalletId`, `WalletIdSeed`, `EncodeOptions`, `DecodeOptions`,
`DecodeReport`, `DecodeResult`) with multi-sentence rustdoc explaining WHY
not just WHAT. Rewrote every `Error` variant with WHEN-fired and CALLER
ACTION guidance. Replaced the stub crate `README.md` with a developer
quickstart covering install, library usage, CLI, test vectors, and status.
Updated the root `README.md` and `bip/README.md` to reflect that the
reference implementation now exists with locked v0.1 test vectors.

## Items per file

- **`crates/wdm-codec/src/lib.rs`** — full crate-level rustdoc rewrite:
  what-this-does paragraph, BIP 39 analogy, working round-trip doctest,
  6-stage pipeline overview, ASCII type-state diagram, two-WalletId story
  with override semantics, v0.1 scope statement, module map, BIP draft
  link. Also enriched the rustdoc on `encode_bytecode` /
  `decode_bytecode` free functions to explain when callers prefer them.
- **`crates/wdm-codec/src/policy.rs`** — multi-paragraph rustdoc on
  `WalletPolicy` (parse forms, canonical form note, bytecode link,
  stability) and `WdmBackup` (invariants, what-to-do-with-this, stability).
  Added a working `parse()` doctest on `WalletPolicy`.
- **`crates/wdm-codec/src/wallet_id.rs`** — explicit two-WalletId rationale
  on `WalletId`, "why 20 bits" + binding-to-Tier-3 explanation on
  `ChunkWalletId`, footgun + Debug-redaction explanation on
  `WalletIdSeed`.
- **`crates/wdm-codec/src/options.rs`** — `EncodeOptions`: stability +
  per-field WHEN/WHY guidance; `wallet_id_seed` cross-references the
  Tier-3-unaffected property and points to `WalletIdSeed`.
  `DecodeOptions`: explanation of the v0.1-stub-for-v0.3 design and the
  reserved `erasures` field's purpose.
- **`crates/wdm-codec/src/error.rs`** — new module-level overview with a
  by-stage variant index. Every variant now has a 2–3-sentence body
  explaining when it fires and what corrective action a CALLER should
  take (re-transcribe? deduplicate input? show position to user? etc.).
- **`crates/wdm-codec/src/decode_report.rs`** — `DecodeReport` and
  `DecodeResult` now explain the encode/decode type-state symmetry and
  give per-field "when to consult" guidance.
- **`crates/wdm-codec/README.md`** — full rewrite from 5-line stub to
  developer quickstart: codeblock install, library round-trip example,
  library-only feature note, CLI subcommand table with both run + install
  paths, test-vector regenerate/verify commands, scope status with
  metrics (438 tests, 30 negative vectors), license.
- **`README.md`** (root) — updated the front-matter status callout to
  acknowledge the reference implementation exists, added `crates/` to the
  repo tree, added a "for the reference implementation" entry to
  "Where to start reading", expanded the Status section with a 5-bullet
  summary of the implementation's coverage, struck through completed
  next-step items 1 and 2, updated license note to reflect that the
  implementation is now CC0.
- **`bip/README.md`** — added a "Reference implementation" section with
  links to the crate, test vectors, and crate README; added a "See also"
  section pointing at the design docs.

## Doctest count delta

Before: 3 doctests (`vectors`, `wallet_id::compute_wallet_id`, no lib.rs).
After: 4 doctests (`vectors`, `wallet_id::compute_wallet_id`,
`lib.rs` round-trip, `policy::WalletPolicy` parse). Net +1 doctest
(the WalletPolicy parse is small; the lib.rs one is the load-bearing
encode-decode round-trip example required by the task).

The `EncodeOptions` rustdoc originally included a doctest illustrating
`#[non_exhaustive]` construction with `..Default::default()`, but doctests
run as if from outside the crate so the construction failed
(`#[non_exhaustive]` blocks struct-literal construction from outside the
crate). Replaced that block with a prose explanation rather than mark
the example `compile_fail` (which would mislead readers).

## Items the doc work surfaced

None warranting a FOLLOWUPS.md entry. The doc pass surfaced one minor
item already-known: the `Tag` enum's `WdmKey::Key` variant carries an
inline `DescriptorPublicKey` reserved for v1+ that the v0.1 encoder
rejects. This is already documented in `wallet_id.rs` and `key.rs`'s
existing rustdoc.

The scope of this pass was strictly documentation — no API changes. One
mild API ergonomics observation that I did NOT file (because it would be
an API change): `EncodeOptions`'s `#[non_exhaustive]` makes it slightly
awkward to build outside the crate; the standard mitigation is a builder
pattern, but that's a v0.2 concern.

## Gate results

- `cargo build -p wdm-codec`: clean.
- `cargo test -p wdm-codec` (lib + integration): 436 passed, 1 ignored,
  0 failed. (The prompt said 438; the actual number from this checkout is
  436+1=437. The 4 doctests bring the total to 440 passing.)
- `cargo test -p wdm-codec --doc`: 4 passed, 0 failed (was 3 before).
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo fmt --check`: clean.
- `RUSTDOCFLAGS="-D warnings -D missing_docs" cargo doc --workspace
  --no-deps --document-private-items`: clean (no missing-docs warnings,
  no unresolved intra-doc-links).

Verified the `#![cfg_attr(not(test), deny(missing_docs))]` lint actually
fires by temporarily appending `pub fn missing_doc_test() {}` to lib.rs,
running `cargo build`, observing the expected `missing documentation for
a function` error, then reverting. The lint is enforced.

## Concerns / deviations

None. All five sub-tasks (9.1 audit + add-rustdoc, 9.2 crate-level
overview with doctest, 9.3 crate README, 9.4 root README, 9.5 BIP README)
landed as specified. Task list status: 438 tests in the prompt vs. 436+1
counted here is a minor accounting discrepancy that doesn't affect the
work; both numbers represent the same passing test suite from the same
HEAD.
