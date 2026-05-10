# v0.18 Phase 1 — Item J `--path` flag fix (per-phase report)

**Date:** 2026-05-09
**Branch:** `feat/v0.18-full-tap-and-nums-engraving`

## Scope

Fix `md encode --path <PATH>` being silently ignored. The bug originated when the flag was wired through `Command::Encode` for forward-compat but `cmd::encode::run` had no parameter to receive it; `main.rs:218` destructured `path: _,`. With the v0.18 round-trip integration test (Item F) needing explicit `path_decl` to satisfy the canonicity gate, this becomes the smallest unblocking phase.

## Artifacts

### Wiring

- `crates/md-cli/src/cmd/encode.rs`:
  - `EncodeArgs` gains `pub path: Option<&'a str>` with a 4-line doc-comment naming the override semantics (Divergent → Shared coercion + accepted forms).
  - `run`: when `args.path` is supplied, parse via `parse_path`, convert via `to_origin_path`, replace `descriptor.path_decl.paths` with `PathDeclPaths::Shared(parsed)`. `n` preserved.
- `crates/md-cli/src/main.rs`:
  - Line 218: `path: _,` → `path,` (destructure unblocked).
  - EncodeArgs constructor adds `path: path.as_deref()`.
  - clap help-string for `--path` updated from `Override the inferred shared derivation path.` to `Override the inferred origin path with a single shared path (flattens Divergent mode to Shared). Accepts named (bip44|48|49|84|86), hex (0xNN), or literal (m/...) forms.` (reviewer L1 mitigation).
- `crates/md-cli/src/parse/path.rs`:
  - Removed file-level `#![allow(dead_code)]` (now consumed).
  - Replaced stale "follow-up `cli-path-arg-routing` once the codec API surfaces it" header with a brief description of the routing.

### Tests added

Three integration tests in `crates/md-cli/tests/cmd_encode.rs`:

1. `encode_with_explicit_path_populates_path_decl` — positive: `--path 48'/0'/0'/2'` produces a phrase distinct from no-path baseline.
2. `encode_with_named_path_bip48` — named-path resolution: `--path bip48` produces the same phrase as `--path "48'/0'/0'/2'"`.
3. `encode_path_overrides_canonical_default` — different explicit paths produce different phrases.

All three are `#[cfg(feature = "cli-compiler")]` (use `--from-policy` for templating). The reviewer-L2 finding (no test for the raw-template code path) deferred to FOLLOWUPS as `v0.18-phase-1-low-2-cli-path-non-from-policy-test-gate`.

## Verification

- `cargo build -p md-cli --features cli-compiler` → clean (only 2 unrelated pre-existing dead-code warnings).
- `cargo test --workspace --all-features` → 398 pass (was 395 pre-Phase-1; +3 new = exact target).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` → clean.
- Live probe: `md encode ... --path bip48` and `md encode ... --path "48'/0'/0'/2'"` produce identical output (`md1qzfdsssj37tgpz2zj4qutvcfmkevym2s`); both differ from the no-path baseline (`md1qzq0j6qgjs54gk3aayrxh8yz7w`); and `--path "86'/0'/0'"` produces a third distinct phrase (`md1qz80tggrukszy59927eyyd3uhz3jwu`).
- `md encode --help` confirms the new help string renders correctly.

## Per-phase code-reviewer round

`feature-dev:code-reviewer` dispatched. Findings:

- **C: 0 / I: 0**
- **L1** — `--path` clap help string didn't mention the Divergent → Shared coercion. **Fixed inline.**
- **L2** — Three new tests are all `#[cfg(feature = "cli-compiler")]`; no test exercises `--path` against a raw template. **Deferred to FOLLOWUPS** (`v0.18-phase-1-low-2-cli-path-non-from-policy-test-gate`); reviewer noted parse/path.rs's existing unit `rejects_garbage` test plus the live probe cover the same surface area.

Net: 0C/0I after L1 fix.

## Exit gate

- ✅ `--path` flag now affects encoded phrase output.
- ✅ Three integration tests pin positive / named-path / different-paths.
- ✅ Workspace tests + clippy clean.
- ✅ Per-phase reviewer 0C/0I.
- ✅ L2 deferred to FOLLOWUPS.

Phase 1 closed; proceeding to Phase 2 (Item G — `--unspendable-key` xpub-form rejection).
