# Spec-stage architect review — md-cli extraction

Date: 2026-05-03
Reviewer: feature-dev:code-architect (agent id `a1fe4ad7d5ba3b6f3`)
Spec stage: written-spec review (pre-plan-writing)
Subject: `design/SPEC_md_codec_v0_16_library_only.md`

## Verdict

Proceed with changes. No critical blockers. Two important issues; several Q-answers folded in.

## Critical issues

None.

## Important issues (fixed inline before commit)

**I-1.** Spec's "Public-API surface on md-codec" listing implied flat re-exports for `tree::{Body, Node}` and `use_site_path::{Alternative, UseSitePath}` — but those modules are accessible only as `pub mod` paths, not via `pub use` re-exports. The bin uses module-path form so this is fine, but the spec's listing wording needed correction: split the list into "flat re-exports at the crate root" vs "reachable via `pub mod` path". Fixed.

**I-2.** Real bug-find. `cmd/vectors.rs` line ~41 has `#[cfg(feature = "json")]` gating JSON vector emission. After the move, md-cli has no `json` feature; serde/serde_json are unconditional. The guard would silently evaluate false and `md vectors` would stop emitting `.descriptor.json` files — a behavioral regression that violates the "pure code-move refactor" invariant. Phase 2 instructions updated to strip the `#[cfg]` guard, executing the block unconditionally. Pre-PR users with default features (the install default) got JSON output; post-PR users always do.

## Q-answers baked into the spec

- **Q1 (premature Phase 0 deliverables):** Architect spot-checked the two ambiguous test files. `smoke.rs` has zero `assert_cmd` (pure library test → stays in md-codec). `template_roundtrip.rs` uses `cargo_bin("md")` on line 10 (CLI test → moves). Provisional language replaced with ground-truth classification.
- **Q4 (cargo publish md-cli will fail with path-only dep):** `md-codec = { path = "../md-codec" }` works in-repo but rejected at `cargo publish` time. Added FOLLOWUPS entry: "C-state precondition: before transplanting md-cli to a third sibling repo, add `version = "0.16.0"` to the path dep."
- **Q7 (CHANGELOG location):** Repo already uses single root `CHANGELOG.md`; no per-crate CHANGELOG files. Spec parenthetical removed; per-crate sections (`## md-codec [0.16.0]`, `## md-cli [0.1.0]`) go into root CHANGELOG.
- **Q8 (acceptance criterion #4 operationalizability):** `md --version` post-move reports `md-cli 0.1.0`, not `md-codec 0.15.x` — this is a clap-derived behavior since `version` reads `CARGO_PKG_VERSION` of the producing crate. Acceptance criterion #4 carved out: subcommand list / `--help` structure / exit codes / golden snapshots match; `--version` differs by design.

## Architect's confirmations on spec details

- **Q2 (Phase 2 atomicity):** `git mv` is rename metadata, not a build operation. Cargo rebuilds from the manifest regardless of prior build state. The atomicity of Phase 2 as specified is correct; no hidden ordering issue inside the single commit.
- **Q3 (`#![allow(missing_docs)]` coverage):** `main.rs` line 1 inner attribute, when carried to md-cli's crate root, suppresses `missing_docs` for the entire crate. clap-derive macros generate inherent-impl items but don't emit `pub` items outside the suppressed scope. No sub-module needs its own `#![allow]`.
- **Q5 (resolver = "3" interactions):** No cross-crate optional-feature unification edge case applies. md-codec has zero `serde`; md-cli has `serde` unconditionally. Removal is total; no bleed-through.
- **Q6 (`cli-compiler = ["dep:miniscript", "miniscript/compiler"]`):** `dep:miniscript` is technically redundant when `miniscript/compiler` is also listed — `feature/feat` syntax implicitly activates the optional dep. The pair is harmless and more explicit; not changed.

## Low/nit (deferred to FOLLOWUPS, see spec)

- **N-1** (Phase 2 verification list could explicitly mention `compile.rs` builds under `default = []`): already covered by `cargo build --workspace` with default features. Not added.
- **N-2** (`smoke.rs` name collision risk): foreclosed by Q1's ground-truth classification — the existing smoke.rs stays in md-codec, so md-cli's new Phase-1 smoke.rs lives in a different crate's tests dir and there's no path conflict.
