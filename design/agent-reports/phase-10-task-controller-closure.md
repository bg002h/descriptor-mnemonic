# Phase 10 — controller closure (Tasks 10.1, 10.3, 10.5, 10.6, 10.7, 10.8)

**Status:** DONE
**Commit:** `03aedc2` (audit closure + spec + README) + `fef8dcb` (.gitignore fix); tag `wdm-codec-v0.1.0`
**File(s):** `crates/wdm-codec/src/wallet_id.rs`, `design/IMPLEMENTATION_PLAN_v0.1.md`, `README.md`, `.gitignore`, `~/.claude/projects/-scratch-code-shibboleth/memory/project_shibboleth_wallet.md`
**Role:** controller (closure of Phase 10 gates)

## Summary

Closed Phase 10 release-prep gates. Subagent dispatch on 10.4 (api-audit) found 6 deviations; controller applied 1 code fix + 5 spec edits in `03aedc2`. Coverage measured (95.26% library line / 89.38% incl. CLI bins). Memory + root README status string updated. Tag `wdm-codec-v0.1.0` created locally.

## Per-task results

### 10.1 — full local CI

All five gates passed at HEAD:

- `cargo test -p wdm-codec`: 361 lib + 79 integration + 5 doc = 445 passing, 1 ignored
- `cargo clippy --workspace --all-targets -- -D warnings`: clean
- `cargo fmt --check`: clean
- `RUSTDOCFLAGS="-D warnings -D missing_docs" cargo doc --workspace --no-deps --document-private-items`: clean
- `cargo run --bin gen_vectors -- --verify crates/wdm-codec/tests/vectors/v0.1.json`: PASS (typed structural compare against the committed JSON)

### 10.2 — cross-platform CI (Linux / Windows / macOS)

**Deferred.** The Phase 5-F workflow at `.github/workflows/ci.yml` runs Linux on every push. Cross-platform sanity (Windows + macOS) requires a push for GitHub Actions to pick up. Tracked as `p10-cross-platform-ci-sanity` in FOLLOWUPS.md (controller-direct-add).

### 10.3 — line coverage ≥ 85%

Tool: `cargo-llvm-cov` (installed via `cargo install cargo-llvm-cov`; `LLVM_COV` and `LLVM_PROFDATA` env vars point at `/usr/bin/llvm-cov` and `/usr/bin/llvm-profdata` because the toolchain is Arch Linux's `rust` package, not rustup).

Result:
- Total: 89.38% region / 88.56% function / 89.03% line
- Library only (excl. CLI bins): 94.49% region / 94.63% function / 95.26% line
- CLI bins (`bin/wdm.rs`, `bin/gen_vectors.rs`): 0% — no `assert_cmd` integration tests; tracked as `7-cli-integration-tests` in FOLLOWUPS.md (v0.2)

The 5% library uncovered code is mostly: tracked v0.2 deferred features (taproot, fingerprints, inline keys), defensive `unreachable!`-style guards, and Display impls not exercised by `matches!` assertions. No genuinely concerning untested production paths.

### 10.4 — public API audit against IMPLEMENTATION_PLAN §3

Dispatched to Opus reviewer. Report saved at `design/agent-reports/phase-10-task-04-api-audit.md`. Found 6 deviations:

- D-1 (real code gap, was v0.1-blocker): `WalletId::as_bytes(&self) -> &[u8; 16]` missing — patched in `03aedc2`
- D-2 (was v0.1-blocker, spec drift): `compute_wallet_id` is split into bytes + policy variants — spec updated in `03aedc2`
- D-3 (was v0.1-blocker, spec drift): `shared_path` returns owned not borrowed — spec updated in `03aedc2`
- D-4 (editorial): `Error` has 11 additive variants beyond spec — spec updated with non-exhaustive contract clause
- D-5 (editorial): `BytecodeErrorKind` has 4 additive variants — same treatment
- D-6 (was v0.1-blocker, spec drift): `From<miniscript::Error>` impl removed per Phase 2 Issue 3 — spec updated to reflect

All 6 closed in `03aedc2`. No FOLLOWUPS.md entries needed (closed in same commit as discovery).

### 10.5 — every Error variant produced by ≥1 negative test

`cargo test -p wdm-codec --test error_coverage`: 5 passed. The strum-based exhaustiveness gate confirms every `ErrorVariantName` (mirror enum, 25 variants) has a corresponding `rejects_<snake>` test in `tests/conformance.rs`. One variant — `BytecodeErrorKind::MissingChildren` — has its conformance test `#[ignore]`d because the variant is defined but never emitted by v0.1 code paths (tracked as `6e-missing-children-unreachable`). No regression.

### 10.6 — tag

Annotated tag `wdm-codec-v0.1.0` created locally. Tag message includes the v0.1.0 surface inventory, dependency notes (apoelstra fork pin + workspace `[patch]` for hash-terminal support pending PR merge), quality summary (445 tests, 95% library coverage, all gates clean), and forward pointer to `design/FOLLOWUPS.md`. **Not pushed** per the project's git safety protocol (no proactive push without explicit user direction).

### 10.7 — root README status update

Two locations updated in `README.md`:
- Top-of-file blockquote (line 3): "Pre-Draft, AI only, not yet human reviewed" → "Pre-Draft, AI + reference implementation, awaiting human review"
- Status section (line 74): same string substitution + cross-references to coverage / FOLLOWUPS.md / test-vector lock

The BIP draft header (`bip/bip-wallet-descriptor-mnemonic.mediawiki:8`) still uses the older string; flagged in the memory file for next-touch update. Could be argued either way: the BIP draft is its own "Pre-Draft" artifact independently of the ref impl, so the older language remains technically accurate from the spec's standpoint. Not a blocker.

### 10.8 — memory file update

`~/.claude/projects/-scratch-code-shibboleth/memory/project_shibboleth_wallet.md` updated:
- Heading "(active; v0.1 implementation pending)" → "(active; v0.1 ref impl complete, awaiting tag + human review)"
- Status field updated with date stamp 2026-04-27 + concrete artifact list (444 tests, 95% line coverage, vectors SHA, FOLLOWUPS.md item count)
- "How to apply" section status convention bullet updated; flagged BIP draft header as the one remaining stale-string location

Memory file is per-user (not in repo).

## Hiccup worth noting

During the audit-closure commit, an initial `git add -A` accidentally staged 13,634 files under `target/` because the .gitignore was missing the canonical `/target/` and `**/target/` entries. Caught immediately, soft-reset, unstaged, re-committed cleanly. Followup commit `fef8dcb` added the missing .gitignore entries so future batch-stage operations are safe.

## Tag-time gate state

```
test:    361 lib + 79 integration + 5 doctest = 445 passing, 1 ignored
clippy:  clean
fmt:     clean
doc:     clean (RUSTDOCFLAGS="-D warnings -D missing_docs")
vectors: --verify PASS
coverage: 95.26% library line / 89.38% total
```

## Outstanding (controller adds to FOLLOWUPS.md after this report lands)

- `p10-cross-platform-ci-sanity` (v0.1-nice-to-have): Phase 5-F workflow runs Linux only locally; needs push to confirm GitHub Actions cross-platform behavior. Defer until first push.
- `p10-bip-header-status-string` (v0.1-nice-to-have): `bip/bip-wallet-descriptor-mnemonic.mediawiki:8` header still says "Pre-Draft, AI only, not yet human reviewed". Either align with the new ref-impl-aware string or leave as-is and document the spec-vs-impl status divergence.

Both are documentation-tier; neither blocks the tag.

## Next steps (controller hands back to user)

1. Push `main` and `wdm-codec-v0.1.0` tag to GitHub: `git push origin main && git push origin wdm-codec-v0.1.0`. Until pushed, the BIP draft's permalink at line 640 (`https://github.com/bg002h/descriptor-mnemonic/blob/e2e8368e51618ad82073c46faa7799fddb86e082/...`) does not resolve.
2. Monitor `apoelstra/rust-miniscript#1` for merge; once merged, bump the `Cargo.toml` SHA pin and remove the workspace `[patch]` redirect.
3. (Optional) submit BIP to bitcoin/bips for Pre-Draft review.
4. Address remaining v0.1-nice-to-have items in FOLLOWUPS.md as bandwidth permits.
