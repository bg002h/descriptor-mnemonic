# Phase 4 architect review — md-cli extraction

Date: 2026-05-03
Reviewer: feature-dev:code-architect (agent id `a1de17c80a5eda859`)
Stage: post-Phase-4 review (Phase 4 commits `b7b1107` + `ba5cd8a`)
Subject: version bump + CHANGELOG + 11 FOLLOWUPS entries

## Verdict

Phase 4 is correct and complete. All version bumps, CHANGELOG entries, Cargo.lock entries, and FOLLOWUPS entries match their respective plan prescriptions verbatim. No critical or important issues found. Two nit observations noted below; neither requires a pre-Task-11 fix.

## Critical issues

None.

## Important issues

None.

## Low/nit issues

**N1. FOLLOWUPS entry format deviation: `Surfaced:` line wording on spec-stage entries is informal.** The four spec-mandated entries (`v0.16-md-codec-cargo-toml-description-stale`, `v0.16-md-codec-cargo-toml-categories-stale`, `v0.16-md-cli-vectors-default-out-dir-cwd-relative`, `v0.16-md-cli-md-codec-path-dep-needs-version-for-publish`) use `Surfaced: Spec self-review + brainstorm-stage architect review of the md-cli extraction PR.` rather than citing a specific report file with its path. All seven reviewer-surfaced entries correctly cite `design/agent-reports/<filename>.md`. The spec-stage entries predate the agent-report convention so the informal citation is acceptable; however, it does not resolve to a durable on-disk report if ever cross-referenced. No fix needed at this tier — acceptable as-is.

**N2. `v0.16-cargo-bin-md-invocation-caveat` Surfaced line references the correct report file.** The entry cites `design/agent-reports/phase-1-review-md-cli-extraction.md N1`. Confirmed file exists. False-alarm self-check during review; no action.

## Architect's confirmations

1. `crates/md-codec/Cargo.toml` line 3: `version = "0.16.0"`. Confirmed.
2. md-cli stays at `0.1.0` in `crates/md-cli/Cargo.toml` line 3. Confirmed.
3. SemVer correctness: dropping `[[bin]]`, all features, and CLI deps is a breaking change; 0.15.x → 0.16.0 is the correct minor-version bump under pre-1.0 convention. Confirmed.
4. Cargo.lock `[[package]] name = "md-codec" version = "0.16.0"` with dependencies `[bip39, bitcoin, thiserror]` — library-only dep set, no CLI deps. Confirmed. `[[package]] name = "md-cli" version = "0.1.0"` with full CLI dep set. Confirmed. No unrelated churn visible.
5. CHANGELOG preamble matches plan prescription word-for-word. Keep-a-Changelog and SemVer link references preserved. Confirmed.
6. Two new entries prepended in correct order: md-cli [0.1.0] first, md-codec [0.16.0] second, existing [0.15.2] third. Confirmed.
7. Both entries dated 2026-05-03. Confirmed.
8. md-cli entry correctly describes json and cli-compiler feature carryover and `md --version` behavioral note. Confirmed against plan template.
9. md-codec entry's Breaking changes section enumerates: binary removal, three feature removals, six optional-dep removals. Unchanged section enumerates: lib API, wire format, identity computations. Confirmed.
10. All 11 FOLLOWUPS entries present: 4 spec-mandated + 7 reviewer-surfaced. Short-ids, Status (`open`), and Tier values all consistent with plan and review source reports. Confirmed.
11. Tier values: `v0.16.1` for in-repo cleanup items (9 entries); `external` for publish-cycle items (`v0.16-md-cli-md-codec-path-dep-needs-version-for-publish`, `v0.16-md-cli-manifest-publish-fields-missing`). Confirmed appropriate.
12. Each reviewer-surfaced entry's `Surfaced:` line cites the correct source report file in `design/agent-reports/`. Confirmed for all 7.
13. Two commits for Phase 4 (`b7b1107` = Task 8, `ba5cd8a` = Task 9). No collapse or unexpected split. Confirmed.

## Architect's open questions

None. Task 11 (final review) has one surface worth a targeted scan: the stale `description` and `categories` fields in `crates/md-codec/Cargo.toml` are correctly deferred but are live in the branch; Task 11 should confirm they do not appear in any `cargo publish` dry-run or CI lint that would treat them as blocking.
