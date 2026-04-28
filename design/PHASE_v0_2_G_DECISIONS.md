# Phase v0.2-G — Release prep decisions

The final v0.2 phase. Mostly process + docs + tag, not algorithmic. Controller-driven with one audit subagent (mirrors v0.1.0 Phase 10's closure pattern).

## G-1 — Workspace `[patch]` block: SHIP AS-IS, document explicitly

**Decision**: v0.2.0 ships with the workspace `[patch]` block intact, redirecting `apoelstra/rust-miniscript` to the local fork at `../rust-miniscript-fork`. Same approach v0.1.0 + v0.1.1 took. Documented in:

- `Cargo.toml` (root) — comment block above the `[patch]` section explaining the reason + pointing to the upstream PR
- `crates/wdm-codec/Cargo.toml` — comment above the `miniscript = { git = ..., rev = ... }` line
- `README.md` — "Building from source" / "Dependencies" section noting the patch
- `CHANGELOG.md` — Notes section reiterating the dep status
- v0.2.0 tag message — full rationale

**Rationale**: per user direction (2026-04-28). The v0.2 plan's "remove `[patch]` at Phase G" gate was aspirational; option (c) ("ship with `[patch]` documented") was named as the explicit fallback in the plan. v0.1.x precedent confirms this works for early-stage releases. When `apoelstra/rust-miniscript#1` merges, ship `wdm-codec-v0.2.1` that drops the `[patch]` block and bumps the SHA pin — tracked by the existing `external-pr-1-hash-terminals` FOLLOWUPS entry.

## G-2 — `CHANGELOG.md` (NEW)

**Decision**: create `CHANGELOG.md` at the repo root following Keep-a-Changelog format. Sections per release:
- **Breaking** — API + behavioral breaks (Phase A `WalletPolicy` Eq, Phase B `to_bytecode` signature + `EncodeOptions: !Copy`, Phase E `PolicyScopeViolation` removal, Phase F schema bump v1→v2)
- **Added** — new public API + new modules + new tests
- **Changed** — internal refactors that surface in behavior
- **Fixed** — clippy/format/CI fixes
- **Notes** — dep status (workspace `[patch]` block); MSRV; BIP draft state

Cover v0.1.1 → v0.2.0 (since v0.1.1 was the last tagged release). Style: match the level of detail in v0.1.0's tag-time inventory.

## G-3 — `MIGRATION.md` (NEW)

**Decision**: create `MIGRATION.md` at the repo root. Single section: "v0.1.x → v0.2.0". Three breaking changes with code-example before/after:

1. **`WalletPolicy` `PartialEq` semantics** — parse-built vs from_bytecode-built logically-equivalent policies now compare unequal. Recommended workaround: compare via `.to_canonical_string()` for construction-path-agnostic equality.
2. **`WalletPolicy::to_bytecode` signature change** — `(&self)` → `(&self, opts: &EncodeOptions)`. Migration: callers needing no override pass `&EncodeOptions::default()`. Plus `EncodeOptions` lost `Copy` (DerivationPath isn't Copy); kept `Clone + Default + PartialEq + Eq`. Callers assuming `Copy` need explicit `.clone()`.
3. **`PolicyScopeViolation` removed for fingerprints flag** — header bit 2 = 1 inputs no longer hit `PolicyScopeViolation`. Callers that intercepted the v0.1 error to "detect fingerprints support" should instead inspect `WdmBackup.fingerprints` / `DecodeResult.fingerprints` directly.

This file also closes the 3 tracker FOLLOWUPS entries (`wallet-policy-eq-migration-note`, `phase-b-encode-signature-and-copy-migration-note`, `phase-e-fingerprints-behavioral-break-migration-note`).

## G-4 — MSRV: unchanged at 1.85

**Decision**: workspace `rust-version = "1.85"`. Phase C's BM/Forney decoder uses pure arithmetic (no stdlib feature requiring a bump). Document in CHANGELOG Notes: *"MSRV: 1.85 (unchanged from v0.1.x)"* so consumers know no toolchain upgrade is required.

## G-5 — Public API audit

**Decision**: dispatch one Opus 4.7 subagent (worktree-isolated) to:
1. Install `cargo public-api` and `cargo semver-checks` (one-time, in the worktree).
2. Run `cargo public-api diff wdm-codec-v0.1.1..HEAD` for the public surface delta.
3. Run `cargo semver-checks check-release --baseline-rev wdm-codec-v0.1.1` for the SemVer-classification report.
4. Cross-reference findings against `MIGRATION.md` — every breaking change in `cargo semver-checks` output should be in `MIGRATION.md`; every API addition should be in `CHANGELOG.md` Added section.
5. Report findings; controller acts on any gaps.

## G-6 — Triage of 6 v0.2-nice-to-have items (excluding the 3 MIGRATION trackers)

**Apply inline in a single Phase G polish-sweep commit**:

- `phase-c-bch-decode-style-cleanups` — 4 stylistic cluster
- `phase-d-tap-decode-error-naming-parity` — encode/decode error message parity
- `phase-e-encoder-count-cast-hardening` — `fps.len() as u8` → `u8::try_from(...)`
- `p4-chunking-mode-stale-test-names` — sweep `force_chunked_*` to `ChunkingMode` terminology

**Defer to v0.2.x**:

- `p4-with-chunking-mode-builder` — additive builder when 3rd variant lands
- `phase-e-cli-fingerprint-flag` — CLI flag, post-release ergonomics

**Skipped per user direction**:

- `p10-bip-header-status-string` — BIP draft `Status:` line; deferred at v0.1.1, same call now

## G-7 — Cargo bump + tag-time gates

**Bump order**:
1. Phase G polish-sweep commit (G-6 inline fixes)
2. CHANGELOG.md commit (G-2)
3. MIGRATION.md commit (G-3)
4. Public-API-audit response commit if any gaps surface (G-5)
5. Update Cargo.toml `version = "0.2.0-dev"` → `version = "0.2.0"` + `cargo update -p wdm-codec`
6. **Regenerate v0.2.json**: the `generator: String` field embeds the wdm-codec version. The bump changes it from `"wdm-codec v0.2.0-dev"` to `"wdm-codec v0.2.0"`, **breaking the existing v0.2.json SHA lock.** Must regenerate v0.2.json after the bump and update the SHA constant in `tests/vectors_schema.rs`. Also update the SHA in the BIP draft.
7. Run all tag-time gates: test / clippy / fmt / doc / vectors --verify (both files)
8. Commit the version bump + regenerated v0.2.json + updated SHA constants as `release(v0.2.0): bump version + regenerate v0.2.json + close v0.2 cycle`
9. Push commit
10. Confirm CI fully green on the version-bump commit
11. Annotated tag `wdm-codec-v0.2.0` with comprehensive message
12. Push tag

## G-8 — Tag message contents

- One-paragraph release summary
- API surface inventory (what changed since v0.1.1)
- Breaking changes list (3 items, link to MIGRATION.md)
- Quality summary (test count, all gates clean)
- Dependency note (`[patch]` block status; `apoelstra/rust-miniscript#1` pending)
- BIP draft state (Taproot tree no longer forward-defined; Error-correction guarantees BM/Forney clause; Fingerprints block privacy paragraph; Test Vectors dual-file)
- Forward pointer to `design/FOLLOWUPS.md`

## G-9 — FOLLOWUPS.md final state at tag time

After Phase G aggregation, expected open count: ~9.

- 1 external (`external-pr-1-hash-terminals`)
- 1 v0.1-nice-to-have (`p10-bip-header-status-string`, deferred)
- 2 v0.2-nice-to-have (`p4-with-chunking-mode-builder`, `phase-e-cli-fingerprint-flag`)
- 3 v0.3 (deferred)
- 2 v1+ (deferred)

Resolved: ~50.

## G-10 — Push order

```bash
git push origin main
git push origin wdm-codec-v0.2.0
```

Same pattern as v0.1.1.

## Out of scope (Phase G)

- Coverage measurement (`cargo-llvm-cov`) — useful but not gating; defer post-tag if user wants
- crates.io publish — explicit user decision, defer
- BIP submission to bitcoin/bips — user-driven action, defer
- Any v0.2.1+ work

## Reference

- `design/IMPLEMENTATION_PLAN_v0.2.md` Phase G section
- `design/agent-reports/phase-10-task-controller-closure.md` — v0.1.0 Phase 10 controller closure pattern (Phase G mirrors this)
- `design/FOLLOWUPS.md` — current state
- `~/.claude/projects/-scratch-code-shibboleth/memory/feedback_subagent_workflow.md` — workflow rules
