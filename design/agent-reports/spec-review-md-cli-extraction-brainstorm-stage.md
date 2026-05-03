# Brainstorm-stage architect review — md-cli extraction

Date: 2026-05-03
Reviewer: feature-dev:code-architect (agent id `a67a222863eb20c74`)
Spec stage: brainstorm/early-design (pre-spec)
Subject: refactor moving the `md` binary out of `md-codec` into a new in-repo `md-cli` crate (B-state of a B → C plan)

## Verdict

Proceed with changes. B-state shape correct; no structural rethink.

## Critical issues (fixed inline before spec)

1. `cmd/vectors.rs` `#[path]` cross-tree reach — adopt `include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../md-codec/tests/vectors/manifest.rs"))` instead of either fixing the relative path or moving the manifest. Manifest stays at its current location.
2. Test classification of `template_roundtrip.rs` and `smoke.rs` was provisional in the brainstorm; spec must enumerate exactly which 21 files move (deferred to Phase 0 audit, with spec calling out the verification requirement).

## Important issues (folded into spec)

3. `#![allow(missing_docs)]` must carry over when `main.rs` moves — workspace lint applies to all members.
4. Phase ordering: original Phase 3 (test move) ahead of Phase 4 (strip md-codec) creates `cargo_bin("md")` ambiguity. Manifest swap (md-codec drops `[[bin]]`/features/CLI deps; md-cli gains them) must be atomic with the source move. Restructured to 5 phases: 0 audit, 1 scaffold + failing smoke, 2 atomic source-move + manifest swap, 3 move tests, 4 versions+CHANGELOG.
5. `miniscript = { workspace = true, optional = true }` in md-cli because `cli-compiler = ["miniscript/compiler"]`. Non-obvious dependency restructuring — spec specifies it.
6. `json` removal: spec adds explicit "library types carry no `serde` derives by design" policy paragraph.
7. v0.16.0 confirmed (lib API not stable; v1.0.0 would oversell).

## Deferred to FOLLOWUPS (low/nit)

- `crates/md-codec/Cargo.toml` description still says "with `md` CLI" — update to library-only.
- `categories = ["command-line-utilities"]` should move to md-cli.
- `vectors.rs` runtime default output dir is a CWD-relative assumption — pre-existing.
- Verify `insta` dev-dep usage (resolved: spec now says Phase 0 produces a definitive verdict, not deferred).

## Architectural alternative considered & rejected

`md-cli-lib` middle crate (so `main.rs` is a thin binary). Premature: the C-state unified binary won't `cargo add md-cli`; it'll path-dep md-codec and mk-codec directly. Flat `crates/md-cli/src/main.rs` is correct.
