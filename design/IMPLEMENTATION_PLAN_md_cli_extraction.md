# md-cli extraction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move the `md` binary out of `md-codec` into a new in-repo `md-cli` crate. Pure code-move refactor: no behavior change, no wire-format change, no new functionality. md-codec → library-only v0.16.0; md-cli → v0.1.0 (new, ships the `md` binary).

**Architecture:** Five phases on one feature branch / one PR. Phase 0 audits before any code moves; Phase 1 establishes a failing smoke-test TDD baseline; Phase 2 is the single atomic commit doing source-move + both crates' manifest swap; Phase 3 moves CLI tests; Phase 4 stamps versions + CHANGELOG. Per-phase iterative-agent review per repo convention; reports persist to `design/agent-reports/`. Critical/important fixed inline; low/nit appended to `design/FOLLOWUPS.md` under the next-patch tier.

**Tech Stack:** unchanged (Rust 2024, workspace `resolver = "3"`, `bitcoin 0.32`, `bip39 2.2`, `clap 4.5`, `assert_cmd 2.0`, `insta 1.40`, `miniscript 13.0` workspace-pinned).

---

## Anchored to

- Spec: `design/SPEC_md_codec_v0_16_library_only.md` (commits `87f2cf7` + `479e4b0` on `main`).
- Architect-review reports: `design/agent-reports/spec-review-md-cli-extraction-{brainstorm,spec}-stage.md`.
- Repo convention (CLAUDE.md memory `feedback_iterative_review_every_phase.md`): per-phase review; reports persist to `design/agent-reports/`; critical/important fixed inline; low/nit deferred to `design/FOLLOWUPS.md` under tier `v0.16.x` (or held there for review at the next minor cycle).
- CLAUDE.md repo note (`feedback_avoid_git_add_all.md`): stage paths explicitly; no `git add -A`.

## File structure

**Created** (all under `crates/md-cli/` unless otherwise noted):

```
crates/md-cli/Cargo.toml                          # Phase 1 (full manifest from spec)
crates/md-cli/src/main.rs                         # Phase 1 stub → Phase 2 real (git mv from md-codec)
crates/md-cli/src/error.rs                        # Phase 2 (git mv)
crates/md-cli/src/compile.rs                      # Phase 2 (git mv)
crates/md-cli/src/cmd/{address,bytecode,compile,decode,encode,inspect,mod,vectors,verify}.rs   # Phase 2 (git mv)
crates/md-cli/src/format/{json,mod,text}.rs       # Phase 2 (git mv)
crates/md-cli/src/parse/{keys,mod,path,template}.rs   # Phase 2 (git mv)
crates/md-cli/tests/smoke.rs                      # Phase 1 (failing smoke → passes after Phase 2; same filename as md-codec's lib-only smoke.rs but distinct test target)
crates/md-cli/tests/{cmd_address,cmd_address_json,cmd_bytecode,cmd_compile,cmd_decode,cmd_encode,cmd_inspect,cmd_verify,compile,exit_codes,help_examples,json_snapshots,scaffold,template_roundtrip,vector_corpus}.rs   # Phase 3 (git mv from md-codec)
crates/md-cli/tests/snapshots/                    # Phase 3 (git mv from md-codec)
design/agent-reports/phase-0-audit-md-cli-extraction.md          # Phase 0 audit deliverable
design/agent-reports/phase-{1,2,3,4}-review-md-cli-extraction.md # per-phase agent reviews
design/agent-reports/final-review-md-cli-extraction.md            # full-PR final review
```

**Modified:**

```
Cargo.toml                                        # Phase 1: workspace members += "crates/md-cli"
crates/md-codec/Cargo.toml                        # Phase 2 strip + Phase 4 version bump
crates/md-cli/src/cmd/vectors.rs                  # Phase 2 (#[path] → include!(concat!(env!(...))))
CHANGELOG.md                                      # Phase 4
design/FOLLOWUPS.md                               # Phase 4 (4 deferred entries)
```

## Test classification (from spec, ground-truth verified)

**Move to `md-cli/tests/`** (CLI integration tests; use `assert_cmd::cargo_bin("md")`):

```
cmd_address.rs  cmd_address_json.rs  cmd_bytecode.rs  cmd_compile.rs
cmd_decode.rs   cmd_encode.rs        cmd_inspect.rs   cmd_verify.rs
compile.rs      exit_codes.rs        help_examples.rs json_snapshots.rs
scaffold.rs     template_roundtrip.rs vector_corpus.rs
```

(15 files. `template_roundtrip.rs` and `vector_corpus.rs` both use `cargo_bin("md")` — `vector_corpus.rs` reclassified from lib to CLI by the Phase 0 audit; the corpus directory still stays in md-codec, the test reaches it via `CARGO_MANIFEST_DIR/../md-codec/tests/vectors`.)

**Stay in `md-codec/tests/`** (pure library tests):

```
address_derivation.rs  chunking.rs  forward_compat.rs  smoke.rs  wallet_policy.rs
```

(5 files. `smoke.rs` confirmed lib-only by architect — pure `md_codec::*` calls, no `assert_cmd`.)

**Move with bin:** `crates/md-codec/tests/snapshots/` → `crates/md-cli/tests/snapshots/`.

**Stays with md-codec:** `crates/md-codec/tests/vectors/` (the reference corpus).

---

## Pre-flight

### Task 0: Create feature branch

**Files:** none.

- [ ] **Step 1: Verify clean tree on `main` with the two spec commits ahead of `origin/main`**

```bash
git status
git log --oneline -3
```

Expected: working tree clean; HEAD is `479e4b0 docs(md-cli-extraction): preserve json feature flag on md-cli per user direction`.

- [ ] **Step 2: Create + switch to feature branch**

```bash
git switch -c feat/md-cli-extraction
```

Expected: switched to a new branch `feat/md-cli-extraction`.

- [ ] **Step 3: No commit yet** — branch creation is a no-op until Task 1 lands content. Skip.

---

## Phase 0 — API audit + test classification

Phase 0 has no code change. Output is a single audit document committed to `design/agent-reports/`. The audit serves as both the Phase 0 deliverable and the Phase 0 "review" (no separate reviewer dispatch needed for this phase — the audit IS the surface review).

### Task 1: API audit

**Files:**
- Create: `design/agent-reports/phase-0-audit-md-cli-extraction.md`

- [ ] **Step 1: List every `md_codec::` import in the bin**

```bash
grep -rEn "^use md_codec::" /scratch/code/shibboleth/descriptor-mnemonic/crates/md-codec/src/bin/md/ | sort -u
```

Expected: ~14-16 distinct import lines covering modules `decode`, `chunk`, `encode`, `header`, `identity`, `tag`, `tree`, `tlv`, `origin_path`, `use_site_path`. (No `bch::*`, no `bitstream::*`, no `varint::*`, no `validate::*` reach.)

- [ ] **Step 2: Verify each imported item is publicly accessible**

For each unique `md_codec::<module>::<item>` line from Step 1, confirm one of:
- `pub mod <module>` exists in `crates/md-codec/src/lib.rs` (module-path access works), OR
- `pub use <module>::{<item>, ...}` exists in `lib.rs` (flat re-export works).

Helper:

```bash
grep -E "^pub (mod|use) " /scratch/code/shibboleth/descriptor-mnemonic/crates/md-codec/src/lib.rs
```

Cross-reference the import list against this output by hand. The spec asserts zero items will need promotion; the audit confirms or refutes.

- [ ] **Step 3: Test classification — re-read each test file's first 30 lines**

For each of the 21 test files in `crates/md-codec/tests/*.rs`, confirm classification matches the spec by checking for `assert_cmd` / `Command::cargo_bin` usage:

```bash
for f in /scratch/code/shibboleth/descriptor-mnemonic/crates/md-codec/tests/*.rs; do
    echo "=== $(basename $f) ==="
    grep -E "assert_cmd|cargo_bin|use md_codec::" $f | head -5
done
```

Expected: the 14 CLI test files name `assert_cmd` or `cargo_bin`; the 6 lib test files import only from `md_codec::`.

- [ ] **Step 4: Decide `insta` retention on md-codec**

```bash
grep -rE "use insta|insta::" /scratch/code/shibboleth/descriptor-mnemonic/crates/md-codec/tests/{address_derivation,chunking,forward_compat,smoke,vector_corpus,wallet_policy}.rs
```

If empty → `insta` is CLI-only and gets dropped from md-codec dev-deps in Phase 2.
If non-empty → `insta` stays in md-codec dev-deps.

- [ ] **Step 5: Write the audit document**

Create `design/agent-reports/phase-0-audit-md-cli-extraction.md` with this structure:

```markdown
# Phase 0 audit — md-cli extraction

Date: <YYYY-MM-DD>
Branch: feat/md-cli-extraction
Spec: design/SPEC_md_codec_v0_16_library_only.md (commits 87f2cf7 + 479e4b0)

## API audit

### Imports detected in `crates/md-codec/src/bin/md/`

<list of unique `use md_codec::...` lines>

### Public-API resolution

| Imported item | Resolution | Action |
|---|---|---|
| `md_codec::decode::decode_md1_string` | `pub use decode::decode_md1_string` (lib.rs:37) | None |
| ... | ... | ... |

### Promotion candidates

<either "None — all imports resolve to public items." or a list of items needing `pub` promotion in this phase's commit>

## Test classification

| File | Classification | Reason |
|---|---|---|
| `cmd_address.rs` | move-to-md-cli | uses `assert_cmd::Command::cargo_bin("md")` |
| `address_derivation.rs` | stay-in-md-codec | only `md_codec::*` imports |
| ... | ... | ... |

## `insta` dev-dep verdict

<"drop from md-codec — no retained lib test imports `insta`" OR "keep on md-codec — used by tests/<file>.rs">
```

- [ ] **Step 6: Verify the audit before committing**

Re-read the document. Every claim must be defensible from the grep output of Steps 1-4.

- [ ] **Step 7: Commit the audit**

```bash
git add design/agent-reports/phase-0-audit-md-cli-extraction.md
git commit -m "$(cat <<'EOF'
docs(md-cli-extraction): phase 0 audit — API surface + test classification

Confirms zero md-codec public items need promotion and ground-truths the
14 CLI / 6 lib test split. `insta` retention verdict: <drop|keep>.

Phase 0 of the md-cli extraction (5-phase plan; spec at
design/SPEC_md_codec_v0_16_library_only.md).
EOF
)"
```

Expected: commit lands; `git log --oneline -1` shows `phase 0 audit`.

---

## Phase 1 — Scaffold md-cli + failing smoke test

Goal: create the new crate with manifest and entry-point stub, plus one failing CLI smoke test. Establishes the TDD invariant for Phase 2 to drive against.

### Task 2: Scaffold md-cli crate

**Files:**
- Modify: `Cargo.toml` (workspace root, line 3)
- Create: `crates/md-cli/Cargo.toml`
- Create: `crates/md-cli/src/main.rs`
- Create: `crates/md-cli/tests/smoke.rs`

- [ ] **Step 1: Add `crates/md-cli` to workspace members**

In `Cargo.toml`, line 3:

```toml
# Before:
members = ["crates/md-codec"]

# After:
members = ["crates/md-codec", "crates/md-cli"]
```

- [ ] **Step 2: Create `crates/md-cli/Cargo.toml`**

Full manifest content (matches spec §"`crates/md-cli/Cargo.toml` (new)"):

```toml
[package]
name = "md-cli"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "CLI for the Mnemonic Descriptor (MD) engravable BIP 388 wallet policy backup format"
keywords = ["bitcoin", "bip388", "wallet", "descriptor", "bech32"]
categories = ["cryptography::cryptocurrencies", "command-line-utilities"]

[lints]
workspace = true

[[bin]]
name = "md"
path = "src/main.rs"

[features]
default = ["json"]
json = ["dep:serde", "dep:serde_json"]
cli-compiler = ["dep:miniscript", "miniscript/compiler"]

[dependencies]
md-codec = { path = "../md-codec" }
clap = { version = "4.5", features = ["derive"] }
anyhow = "1.0"
regex = "1.10"
bitcoin = "0.32"
bip39 = "2.2.2"
serde = { version = "1.0", features = ["derive"], optional = true }
serde_json = { version = "1.0", optional = true }
miniscript = { workspace = true, optional = true }

[dev-dependencies]
assert_cmd = "2.0"
predicates = "3.1"
insta = { version = "1.40", features = ["json"] }
tempfile = "3.13"
```

- [ ] **Step 3: Create `crates/md-cli/src/main.rs` stub**

```rust
#![allow(missing_docs)]

fn main() {
    unimplemented!("md-cli scaffold; replaced atomically in Phase 2");
}
```

- [ ] **Step 4: Create `crates/md-cli/tests/smoke.rs`**

```rust
//! Phase-1 scaffold smoke test. Pinned to one canonical encode and reused as
//! the TDD invariant Phase 2's source-move must restore. Renamed in Phase 3
//! once the moved CLI test suite arrives.

use assert_cmd::Command;

#[test]
fn encode_wpkh_default_phrase() {
    let mut cmd = Command::cargo_bin("md").unwrap();
    cmd.args(["encode", "wpkh(@0/<0;1>/*)"]);
    cmd.assert()
        .success()
        .stdout("md1qqpqqxqxkceprx7rap4t\n");
}
```

(The expected output `md1qqpqqxqxkceprx7rap4t` matches the example in the existing `md encode --help` text.)

- [ ] **Step 5: Verify the smoke test fails as expected**

```bash
cargo test -p md-cli --test smoke
```

Expected: build succeeds (manifest is valid; stub `main.rs` compiles); test runs; **test FAILS** because the spawned `md` binary panics at `unimplemented!()` and exits non-zero, which fails the `.assert().success()` precondition. The failure mode is the TDD baseline — Phase 2 makes it pass by replacing the stub with the real `main.rs`.

- [ ] **Step 6: Verify md-codec still builds and its tests still pass**

```bash
cargo test -p md-codec
```

Expected: pass — Phase 1 has not touched md-codec.

- [ ] **Step 7: Stage Phase-1 files explicitly**

```bash
git add Cargo.toml crates/md-cli/Cargo.toml crates/md-cli/src/main.rs crates/md-cli/tests/smoke.rs
git status
```

Expected: 4 files staged; nothing else (per repo `feedback_avoid_git_add_all.md`, do not `git add -A`).

- [ ] **Step 8: Commit Phase 1**

```bash
git commit -m "$(cat <<'EOF'
feat(md-cli): phase 1 — scaffold crate + failing smoke test (TDD baseline)

Adds crates/md-cli to the workspace with a stub main.rs that panics. The
smoke test exercises `md encode wpkh(@0/<0;1>/*)` and asserts the canonical
phrase `md1qqpqqxqxkceprx7rap4t`. Test fails by design — Phase 2's atomic
source-move + manifest swap is what makes it pass.

Phase 1 of the md-cli extraction (5-phase plan; spec at
design/SPEC_md_codec_v0_16_library_only.md).
EOF
)"
```

Expected: commit lands; `cargo build --workspace` succeeds (no compile error).

### Task 3: Phase 1 architect review

**Files:** `design/agent-reports/phase-1-review-md-cli-extraction.md`

- [ ] **Step 1: Dispatch architect review of the Phase-1 commit**

Use `Agent` with `subagent_type: feature-dev:code-architect`. Brief the agent:

> Review the most recent commit on branch `feat/md-cli-extraction` (Phase 1 of the md-cli extraction). The commit scaffolds `crates/md-cli/` with a Cargo.toml, stub main.rs, and one failing smoke test. Spec is at `design/SPEC_md_codec_v0_16_library_only.md`; this is Phase 1 of a 5-phase plan in `design/IMPLEMENTATION_PLAN_md_cli_extraction.md`. Confirm the manifest matches the spec verbatim, the workspace members update is correct, the smoke test invokes the correct binary name and is expected to fail (TDD baseline), and `cargo build --workspace` is clean. Surface critical/important issues; defer low/nit. Length cap: 800 words.

- [ ] **Step 2: Persist the review report**

Save the agent's response to `design/agent-reports/phase-1-review-md-cli-extraction.md`. Format: header (date, agent id, scope), verdict, critical/important/low sections.

- [ ] **Step 3: Apply critical+important fixes inline**

If the architect surfaces any critical/important issue: fix it in a follow-up commit on the same branch (do not amend Phase 1's commit — per CLAUDE.md, prefer new commits). Re-run the smoke test to confirm it still fails as designed.

- [ ] **Step 4: Append low/nit to FOLLOWUPS draft**

Note any low/nit items in a temporary draft (final FOLLOWUPS write is in Phase 4). Format: tier `v0.16.x`, `Source: phase-1-review`, `Where:` and `What:` lines.

- [ ] **Step 5: Commit the review report (if not already committed alongside fixes)**

```bash
git add design/agent-reports/phase-1-review-md-cli-extraction.md
git commit -m "docs(md-cli-extraction): phase 1 architect review report"
```

---

## Phase 2 — Atomic source-move + manifest swap

Single commit. Touches both crates' manifests AND moves the source tree AND edits `cmd/vectors.rs`. Atomicity is required to avoid a broken intermediate build (md-codec's `[[bin]]` pointing at a `src/bin/md/main.rs` that no longer exists, or md-cli's manifest claiming a binary that has no source).

### Task 4: Source-move + manifest swap

**Files:**
- Delete (via `git rm` of stub then `git mv` of real): `crates/md-cli/src/main.rs` (Phase-1 stub)
- Modify: `crates/md-codec/Cargo.toml` (strip `[[bin]]`, features, CLI deps, CLI dev-deps)
- Modify: `crates/md-cli/src/cmd/vectors.rs` (replace `#[path]` with `include!(concat!(env!(...)))`)
- Move (via `git mv`): the entire `crates/md-codec/src/bin/md/` tree into `crates/md-cli/src/`

- [ ] **Step 1: Remove the Phase-1 stub main.rs (so the real one can take its place)**

```bash
git rm crates/md-cli/src/main.rs
```

Expected: the stub is staged for deletion. Tree compiles broken at this point (md-cli has manifest pointing at a missing main.rs); that's fine — we don't run `cargo build` until Step 9.

- [ ] **Step 2: `git mv` the bin source tree from md-codec to md-cli**

`git mv` of a single file requires the parent directory of the destination to exist; `git mv` of a directory works whether or not the destination exists (it renames). After Step 1's `git rm`, `crates/md-cli/src/` is empty in the index but the working-tree directory may have been removed; create it first:

```bash
mkdir -p crates/md-cli/src
```

Then run all 6 moves:

```bash
git mv crates/md-codec/src/bin/md/main.rs    crates/md-cli/src/main.rs
git mv crates/md-codec/src/bin/md/error.rs   crates/md-cli/src/error.rs
git mv crates/md-codec/src/bin/md/compile.rs crates/md-cli/src/compile.rs
git mv crates/md-codec/src/bin/md/cmd       crates/md-cli/src/cmd
git mv crates/md-codec/src/bin/md/format    crates/md-cli/src/format
git mv crates/md-codec/src/bin/md/parse     crates/md-cli/src/parse
```

After: `crates/md-codec/src/bin/` is empty (just the now-empty `md/` directory); remove it:

```bash
rmdir crates/md-codec/src/bin/md
rmdir crates/md-codec/src/bin
```

(`rmdir` only succeeds on empty directories — safety check that the move was complete.)

Expected: `git status` shows ~25 file renames from `crates/md-codec/src/bin/md/...` to `crates/md-cli/src/...`.

- [ ] **Step 3: Edit `crates/md-cli/src/cmd/vectors.rs` to replace the `#[path]` reach**

The current top of the file:

```rust
use crate::error::CliError;
use crate::parse::keys::ParsedFingerprint;
use crate::parse::template::parse_template;
use std::path::PathBuf;
use std::fs;

#[path = "../../../../tests/vectors/manifest.rs"]
mod manifest;
use manifest::MANIFEST;
```

Replace lines 7-9 with:

```rust
mod manifest {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../md-codec/tests/vectors/manifest.rs"));
}
use manifest::MANIFEST;
```

The existing `#[cfg(feature = "json")]` block at line 41 is **preserved** (md-cli inherits the `json` feature flag).

- [ ] **Step 4: Pre-fix corpus paths in the three test files Phase 3 will move**

Three files in `crates/md-codec/tests/` reach the `vectors/` corpus by a path that breaks the moment `git mv` lands the file in `md-cli/tests/` (the `vectors/` directory stays in `md-codec`). Phase 3 is a "no source edits" phase, so the path swaps belong in Phase 2's commit. The `CARGO_MANIFEST_DIR/../md-codec/tests/vectors/...` form resolves correctly *both* in md-codec (`crates/md-codec/../md-codec/tests/vectors/...` ⇒ `crates/md-codec/tests/vectors/...`) *and* post-Phase-3 in md-cli (`crates/md-cli/../md-codec/tests/vectors/...`). One form, both states correct.

**Files (a) and (b): `include!` form.**

In `crates/md-codec/tests/template_roundtrip.rs` line 4, change:

```rust
mod manifest {
    include!("vectors/manifest.rs");
}
```

to:

```rust
mod manifest {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../md-codec/tests/vectors/manifest.rs"));
}
```

Same change in `crates/md-codec/tests/json_snapshots.rs` line 7.

**File (c): `format!`/`env!` form.** `crates/md-codec/tests/vector_corpus.rs` line 13 currently reads:

```rust
let committed = format!("{}/tests/vectors", env!("CARGO_MANIFEST_DIR"));
```

Change to:

```rust
let committed = format!("{}/../md-codec/tests/vectors", env!("CARGO_MANIFEST_DIR"));
```

Same `CARGO_MANIFEST_DIR/..`-walk-back trick — resolves to `md-codec/tests/vectors` regardless of which crate the test target ends up in.

(Sanity-check no other test file reaches the corpus by a path: `grep -rnE "tests/vectors|vectors/manifest" /scratch/code/shibboleth/descriptor-mnemonic/crates/md-codec/tests/*.rs` should return only these three.)

- [ ] **Step 5: Strip `crates/md-codec/Cargo.toml`**

Replace the file contents with:

```toml
[package]
name = "md-codec"
version = "0.15.2"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "Reference implementation of the Mnemonic Descriptor (MD) format for engravable BIP 388 wallet policy backups, with `md` CLI"
readme = "README.md"
homepage = "https://github.com/bg002h/descriptor-mnemonic"
documentation = "https://docs.rs/md-codec"
keywords = ["bitcoin", "bip388", "wallet", "descriptor", "bech32"]
categories = ["cryptography::cryptocurrencies", "encoding", "command-line-utilities"]

[lints]
workspace = true

[lib]
name = "md_codec"

[dependencies]
bitcoin = "0.32"
thiserror = "2.0"
bip39 = "2.2.2"
```

Notes for the implementer:

- **Version stays at `0.15.2`** in this phase. Phase 4 bumps to `0.16.0`.
- **`description` and `categories` retain the misleading "with `md` CLI" / `"command-line-utilities"` strings**. Per FOLLOWUPS deferral, these get cleaned up later — not in this PR.
- **`[dev-dependencies]` block is omitted entirely** if Phase 0 Step 4 found `insta` is CLI-only. **If Phase 0 said keep `insta`**, append:
  ```toml

  [dev-dependencies]
  insta = { version = "1.40", features = ["json"] }
  ```

- [ ] **Step 6: `cargo build --workspace`**

```bash
cargo build --workspace
```

Expected: success. md-codec compiles as a library only; md-cli compiles with the moved source.

- [ ] **Step 7: `cargo build --workspace --all-features`**

```bash
cargo build --workspace --all-features
```

Expected: success. Exercises the `cli-compiler` feature (and `json`, which is already in default).

- [ ] **Step 8: `cargo check -p md-cli --all-targets` returns zero warnings**

```bash
cargo check -p md-cli --all-targets 2>&1 | tee /tmp/md-cli-check.log
grep -E "^warning:" /tmp/md-cli-check.log
```

Expected: no `^warning:` lines. (`#![allow(missing_docs)]` at `crates/md-cli/src/main.rs` line 1 — auto-carried by the `git mv` — suppresses workspace-lint output.)

- [ ] **Step 9: Run the smoke test (now expected to pass)**

```bash
cargo test -p md-cli --test smoke
```

Expected: PASS. The TDD invariant from Phase 1 is satisfied.

- [ ] **Step 10: Stage Phase-2 changes**

```bash
git add crates/md-codec/Cargo.toml \
        crates/md-cli/src/cmd/vectors.rs \
        crates/md-codec/tests/template_roundtrip.rs \
        crates/md-codec/tests/json_snapshots.rs \
        crates/md-codec/tests/vector_corpus.rs
git status
```

Note: the `git mv` operations from Step 2 and `git rm` from Step 1 are already staged. Step 10 only adds the file edits (5 files).

Expected: ~30 changes staged (1 deletion of stub main.rs, ~25 renames, 5 file edits — md-codec manifest, cmd/vectors.rs, template_roundtrip.rs, json_snapshots.rs, vector_corpus.rs).

- [ ] **Step 11: Commit Phase 2**

```bash
git commit -m "$(cat <<'EOF'
feat(md-cli): phase 2 — atomic source-move + manifest swap

Moves crates/md-codec/src/bin/md/* to crates/md-cli/src/* (flattening the
bin/md/ nesting). Replaces cmd/vectors.rs's cross-tree #[path] reach with a
portable include!(concat!(env!("CARGO_MANIFEST_DIR"), ...)). Pre-fixes the
same CARGO_MANIFEST_DIR/.. trick in three test files (template_roundtrip.rs,
json_snapshots.rs, vector_corpus.rs) so Phase 3's git mv doesn't break
compile — the post-move-correct path resolves identically pre-move
(CARGO_MANIFEST_DIR/.. dance lands on md-codec/tests/vectors/... from
either crate's test harness). Strips md-codec's [[bin]], [features] block,
and CLI optional deps. md-cli's manifest from Phase 1 carries the json +
cli-compiler features verbatim.

Workspace builds with default features and --all-features. The Phase-1
smoke test, which failed by design before this commit, now passes — TDD
invariant satisfied.

Phase 2 of the md-cli extraction (5-phase plan; spec at
design/SPEC_md_codec_v0_16_library_only.md).
EOF
)"
```

Expected: commit lands; `cargo test --workspace` passes (the existing CLI integration tests in `md-codec/tests/` continue to work because md-codec's `Cargo.toml` no longer claims a `[[bin]]`, but the tests find `md` via workspace-binary resolution from md-cli; this is verified empirically because Phase 3 hasn't run yet but `cargo_bin("md")` resolves uniquely once md-codec drops `[[bin]]`).

### Task 5: Phase 2 architect review

**Files:** `design/agent-reports/phase-2-review-md-cli-extraction.md`

- [ ] **Step 1: Dispatch architect review of the Phase-2 commit**

Brief the architect:

> Review the Phase-2 commit on branch `feat/md-cli-extraction` of the md-cli extraction. Spec: `design/SPEC_md_codec_v0_16_library_only.md`. Plan: `design/IMPLEMENTATION_PLAN_md_cli_extraction.md` (Phase 2 starts at Task 4). The plan's Phase-1 already creates the full md-cli/Cargo.toml; Phase 2 only edits md-codec/Cargo.toml plus four source files (cmd/vectors.rs, template_roundtrip.rs, json_snapshots.rs, vector_corpus.rs). Critical concerns: (1) the `#[path]` → `include!(concat!(env!(...)))` substitution in `crates/md-cli/src/cmd/vectors.rs` builds correctly under both `default` and `--no-default-features`; (2) `crates/md-codec/Cargo.toml` no longer has `[[bin]]`, `[features]`, or any CLI-only dep; (3) the three corpus-path pre-fixes in `template_roundtrip.rs` / `json_snapshots.rs` (`include!` form) and `vector_corpus.rs` (`format!`/`env!` form) resolve correctly while the files are still in `md-codec/tests/` (CARGO_MANIFEST_DIR/.. trick) and will continue to resolve correctly post-Phase-3; (4) the existing `#[cfg(feature = "json")]` gate in `cmd/vectors.rs` is preserved; (5) the workspace lint `missing_docs` does not fire on md-cli (the `#![allow]` at main.rs line 1 covers the whole crate). Length cap: 1000 words.

- [ ] **Step 2: Persist the report to `design/agent-reports/phase-2-review-md-cli-extraction.md`**

- [ ] **Step 3: Apply critical+important fixes inline (new commits, do not amend Phase 2)**

If anything material surfaces: fix in a follow-up commit. Re-run `cargo build --workspace --all-features` and `cargo test -p md-cli --test smoke` to confirm.

- [ ] **Step 4: Append low/nit to the FOLLOWUPS draft**

- [ ] **Step 5: Commit the review report**

```bash
git add design/agent-reports/phase-2-review-md-cli-extraction.md
git commit -m "docs(md-cli-extraction): phase 2 architect review report"
```

---

## Phase 3 — Move CLI tests + snapshot fixtures

### Task 6: Move CLI integration tests

**Files:**
- Move (via `git mv`): 15 test files from `crates/md-codec/tests/` to `crates/md-cli/tests/`.
- Move (via `git mv`): `crates/md-codec/tests/snapshots/` to `crates/md-cli/tests/snapshots/`.

The Phase-1 scaffold smoke test at `crates/md-cli/tests/smoke.rs` stays as-is. The existing `crates/md-codec/tests/smoke.rs` (pure library test) stays in md-codec. Same filename in two different crates' tests dirs — fine, distinct test targets.

- [ ] **Step 1: `git mv` the 15 CLI integration tests**

```bash
git mv crates/md-codec/tests/cmd_address.rs       crates/md-cli/tests/cmd_address.rs
git mv crates/md-codec/tests/cmd_address_json.rs  crates/md-cli/tests/cmd_address_json.rs
git mv crates/md-codec/tests/cmd_bytecode.rs      crates/md-cli/tests/cmd_bytecode.rs
git mv crates/md-codec/tests/cmd_compile.rs       crates/md-cli/tests/cmd_compile.rs
git mv crates/md-codec/tests/cmd_decode.rs        crates/md-cli/tests/cmd_decode.rs
git mv crates/md-codec/tests/cmd_encode.rs        crates/md-cli/tests/cmd_encode.rs
git mv crates/md-codec/tests/cmd_inspect.rs       crates/md-cli/tests/cmd_inspect.rs
git mv crates/md-codec/tests/cmd_verify.rs        crates/md-cli/tests/cmd_verify.rs
git mv crates/md-codec/tests/compile.rs           crates/md-cli/tests/compile.rs
git mv crates/md-codec/tests/exit_codes.rs        crates/md-cli/tests/exit_codes.rs
git mv crates/md-codec/tests/help_examples.rs     crates/md-cli/tests/help_examples.rs
git mv crates/md-codec/tests/json_snapshots.rs    crates/md-cli/tests/json_snapshots.rs
git mv crates/md-codec/tests/scaffold.rs          crates/md-cli/tests/scaffold.rs
git mv crates/md-codec/tests/template_roundtrip.rs crates/md-cli/tests/template_roundtrip.rs
git mv crates/md-codec/tests/vector_corpus.rs     crates/md-cli/tests/vector_corpus.rs
```

- [ ] **Step 2: `git mv` the snapshots directory**

```bash
git mv crates/md-codec/tests/snapshots crates/md-cli/tests/snapshots
```

- [ ] **Step 3: `cargo test --workspace` passes**

```bash
cargo test --workspace
```

Expected: all tests pass. The 15 moved files now build against md-cli's `[[bin]] md`; the 5 lib tests continue against md-codec; the `tests/vectors/` corpus stays in md-codec (vector_corpus.rs reaches it via the Phase-2 pre-fixed `CARGO_MANIFEST_DIR/..` path); `cargo_bin("md")` resolves uniquely.

- [ ] **Step 4: Sanity-check no test paths reach back into md-codec**

```bash
grep -rE "crates/md-codec" /scratch/code/shibboleth/descriptor-mnemonic/crates/md-cli/tests/ 2>&1 | grep -v "^Binary"
```

Expected: empty (no moved test hardcodes a path back into md-codec). If any tests do, audit them — most likely the `cmd/vectors.rs` runtime path is leaking through, but that lives in src, not tests.

- [ ] **Step 5: Stage Phase-3 changes**

```bash
git status
git add crates/md-codec/tests crates/md-cli/tests
```

Expected: 15 file renames + 1 directory rename (snapshots).

- [ ] **Step 6: Commit Phase 3**

```bash
git commit -m "$(cat <<'EOF'
feat(md-cli): phase 3 — move CLI integration tests + snapshots

Moves the 15 assert_cmd-based CLI integration tests and the tests/snapshots
directory from crates/md-codec/tests/ to crates/md-cli/tests/. The 5
remaining test files in crates/md-codec/tests/ (address_derivation,
chunking, forward_compat, smoke, wallet_policy) are pure library tests
and stay. tests/vectors/ (the format reference corpus) also stays with
md-codec; vector_corpus.rs (CLI test that regenerates and diffs the
corpus) moves with the other CLI tests and reaches the corpus via the
Phase-2 pre-fixed CARGO_MANIFEST_DIR/.. path.

The Phase-1 scaffold smoke at crates/md-cli/tests/smoke.rs is unchanged;
the same-named smoke.rs in md-codec/tests/ is a different test in a
different crate, so there's no path collision.

cargo test --workspace passes the same set of tests as pre-PR.

Phase 3 of the md-cli extraction (5-phase plan; spec at
design/SPEC_md_codec_v0_16_library_only.md).
EOF
)"
```

### Task 7: Phase 3 architect review

**Files:** `design/agent-reports/phase-3-review-md-cli-extraction.md`

- [ ] **Step 1: Dispatch architect review**

Brief the architect:

> Review the Phase-3 commit on branch `feat/md-cli-extraction`. Spec at `design/SPEC_md_codec_v0_16_library_only.md`; plan at `design/IMPLEMENTATION_PLAN_md_cli_extraction.md`. Verify: (1) all 15 listed CLI test files moved cleanly with no source edit (the vector_corpus.rs source edit landed in Phase 2's pre-fix step, not Phase 3); (2) snapshots directory moved; (3) the 5 lib tests stayed; (4) `tests/vectors/` corpus stayed in md-codec; (5) `cargo test --workspace` passes the same number of tests as on `main` pre-PR (architect will need to reconstruct the pre-PR count from the spec/git log). Surface any path or import that broke silently. Length cap: 800 words.

- [ ] **Step 2: Persist + commit the review report**

```bash
git add design/agent-reports/phase-3-review-md-cli-extraction.md
git commit -m "docs(md-cli-extraction): phase 3 architect review report"
```

---

## Phase 4 — Versions + CHANGELOG + FOLLOWUPS entries

### Task 8: Version bump + CHANGELOG entries

**Files:**
- Modify: `crates/md-codec/Cargo.toml` (version 0.15.2 → 0.16.0)
- Modify: `CHANGELOG.md` (add md-codec 0.16.0 + md-cli 0.1.0 entries)

- [ ] **Step 1: Bump md-codec version**

In `crates/md-codec/Cargo.toml`, change line 3:

```toml
# Before:
version = "0.15.2"

# After:
version = "0.16.0"
```

- [ ] **Step 2: Add CHANGELOG entries**

First, capture today's date for the entries:

```bash
TODAY=$(date +%Y-%m-%d)
echo "$TODAY"
```

Then in `CHANGELOG.md`:

**Preamble update.** The existing preamble reads "All notable changes to `md-codec` are documented in this file." Update to cover both crates. Replace that line with:

```markdown
All notable changes to `md-codec` and `md-cli` are documented in this file. Each release entry is prefixed with the crate name (`## md-codec [0.16.0]`, `## md-cli [0.1.0]`). Pre-split releases (md-codec ≤ 0.15.2) lack the prefix.
```

(Adjust the surrounding paragraph if the existing preamble has more text — preserve the Keep-a-Changelog and SemVer links from the existing file.)

Then prepend two new entries above the existing `## [0.15.2] — 2026-05-03` block. Substitute the value of `$TODAY` for `DATE` below:

```markdown
## md-cli [0.1.0] — DATE

Initial release. The `md` binary and its source tree (`cmd/`, `format/`,
`parse/`, `compile.rs`, `error.rs`, `main.rs`) were extracted from
`md-codec` 0.15.2; see md-codec [0.16.0] entry below for the breaking
change on the producing side. The `json` and `cli-compiler` feature flags
carry over from md-codec verbatim:

- `default = ["json"]` — `--json` output paths and JSON vector emission
  build by default.
- `cli-compiler` (opt-in) — gates the `compile` subcommand and
  `encode --from-policy` via `miniscript/compiler`.

No CLI behavior change vs. md-codec 0.15.2; `md --version` now reports
`md-cli 0.1.0` instead of `md-codec 0.15.x` (clap derives `--version` from
the producing crate's `CARGO_PKG_VERSION`).

## md-codec [0.16.0] — DATE

Library-only release. The `md` binary and the `cli`, `cli-compiler`, and
`json` features have been extracted to a new `md-cli` crate (see md-cli
[0.1.0] entry above). No wire-format change; no library API removal.

### Breaking changes

- `cargo install md-codec` no longer ships an `md` binary. Install
  `md-cli` instead: `cargo install md-cli`.
- The `cli`, `cli-compiler`, and `json` Cargo features are gone from
  md-codec; downstream consumers using `default-features = false,
  features = ["cli", ...]` must migrate to depending on `md-cli`.
- Optional dependencies `clap`, `anyhow`, `miniscript`, `regex`, `serde`,
  `serde_json` are no longer in md-codec's dependency graph.

### Unchanged

- Library public API (`pub use`s and `pub mod`s in `lib.rs`).
- v0.11 wire format and BCH primitives.
- All format identity computations (PolicyId, EncodingId, TemplateId).
```

- [ ] **Step 3: Verify `cargo build --workspace` still succeeds**

```bash
cargo build --workspace
```

Expected: pass. Version bumps don't affect compilation.

- [ ] **Step 4: Stage and commit**

```bash
git add crates/md-codec/Cargo.toml CHANGELOG.md
git commit -m "$(cat <<'EOF'
release: md-codec v0.16.0 (library-only) + md-cli v0.1.0

md-codec drops to library-only: no [[bin]], no features, no CLI deps.
md-cli ships the md binary at v0.1.0 with json + cli-compiler features
carried over from md-codec verbatim. CHANGELOG entries on both sides
document the breaking change and the carryover semantics.

Phase 4 of the md-cli extraction (5-phase plan; spec at
design/SPEC_md_codec_v0_16_library_only.md).
EOF
)"
```

### Task 9: FOLLOWUPS entries

**Files:**
- Modify: `design/FOLLOWUPS.md`

- [ ] **Step 1: Append the four deferred entries**

Read the current `design/FOLLOWUPS.md` to find the next-patch tier (likely `v0.16.x` or `v0.16.1` — match the existing pattern).

Append the four entries from the spec's "Deferred to FOLLOWUPS" section:

```markdown
### v0.16.x tier (from md-cli extraction PR review cycle)

#### `md-codec-cargo-toml-description-stale`
Source: spec self-review + brainstorm-stage architect review
Where: `crates/md-codec/Cargo.toml` line 8 (`description = "..."`)
What: Description still says "with `md` CLI" — md-codec is library-only as
of 0.16.0. Update to library-only phrasing (e.g. "Reference implementation
of the Mnemonic Descriptor (MD) format for engravable BIP 388 wallet
policy backups"). Drop the trailing "with `md` CLI" clause.
Companion: none (md1-only).

#### `md-codec-cargo-toml-categories-stale`
Source: spec self-review + brainstorm-stage architect review
Where: `crates/md-codec/Cargo.toml` line ~13 (`categories = [..., "command-line-utilities"]`)
What: `"command-line-utilities"` is now md-cli-only. Remove from md-codec's
categories list.
Companion: none.

#### `md-cli-vectors-default-out-dir-cwd-relative`
Source: spec-stage architect review
Where: `crates/md-cli/src/cmd/vectors.rs` line ~12 (`out_dir = PathBuf::from(out.unwrap_or_else(|| "crates/md-codec/tests/vectors".into()))`)
What: The default output directory is a CWD-relative path that only
resolves correctly when invoked from the workspace root. Pre-existing bug
not introduced by the extraction PR. Either make it a required arg, or
resolve from `CARGO_MANIFEST_DIR`, or document the CWD requirement
explicitly in the subcommand's `--help`.
Companion: none.

#### `md-cli-md-codec-path-dep-needs-version-for-publish`
Source: spec-stage architect review (Q4)
Where: `crates/md-cli/Cargo.toml` (`md-codec = { path = "../md-codec" }`)
What: Cargo rejects path-only deps at `cargo publish` time. Before the
C-state transplant (md-cli moves to a third sibling repo) or any direct
`cargo publish md-cli`, the dep must gain a `version` field:
`md-codec = { path = "../md-codec", version = "0.16.0" }`. Path-only is
fine for in-repo development now. Surface this at the C-state cycle's
brainstorm.
Companion: none.
```

- [ ] **Step 2: Stage and commit**

```bash
git add design/FOLLOWUPS.md
git commit -m "$(cat <<'EOF'
docs(md-cli-extraction): file 4 deferred FOLLOWUPS entries

Per spec § "Deferred to FOLLOWUPS": Cargo.toml description/categories
cleanup on md-codec; vectors.rs CWD-relative default; C-state path-dep
version-field precondition.

Phase 4 of the md-cli extraction (5-phase plan).
EOF
)"
```

### Task 10: Phase 4 architect review

**Files:** `design/agent-reports/phase-4-review-md-cli-extraction.md`

- [ ] **Step 1: Dispatch architect review**

Brief the architect:

> Review Phase 4's two commits on branch `feat/md-cli-extraction`: (1) version bump + CHANGELOG; (2) FOLLOWUPS entries. Verify: md-codec version is `0.16.0`; md-cli stays at `0.1.0`; CHANGELOG has both entries with consistent date stamps; both entries name the breaking change correctly; FOLLOWUPS has 4 new entries under the v0.16.x tier with correct citations. Spec at `design/SPEC_md_codec_v0_16_library_only.md`. Length cap: 600 words.

- [ ] **Step 2: Persist + commit the report**

```bash
git add design/agent-reports/phase-4-review-md-cli-extraction.md
git commit -m "docs(md-cli-extraction): phase 4 architect review report"
```

---

## Final review + acceptance + PR

### Task 11: Final whole-PR architect review

**Files:** `design/agent-reports/final-review-md-cli-extraction.md`

- [ ] **Step 1: Dispatch a final architect review of the entire branch**

Brief the architect:

> Final review of the complete `feat/md-cli-extraction` branch (~5-10 commits). Spec: `design/SPEC_md_codec_v0_16_library_only.md`. Plan: `design/IMPLEMENTATION_PLAN_md_cli_extraction.md`. Per-phase reports already in `design/agent-reports/phase-{0,1,2,3,4}-*-md-cli-extraction.md`.
>
> Verify all 9 acceptance criteria from spec § "Acceptance criteria":
> 1. `cargo build --workspace` succeeds.
> 2. `cargo build --workspace --all-features` succeeds.
> 3. `cargo test --workspace` passes the same set of tests as on `main` pre-PR (modulo any Phase-0 splits — none expected per spec).
> 4. `cargo install --path crates/md-cli` produces an `md` binary whose subcommand list, `--help` structure, exit codes, and golden snapshots match the pre-PR `md` binary; `--version` differs by design.
> 5. `cargo check -p md-cli --all-targets` and `cargo check -p md-codec --all-targets` both return zero warnings.
> 6. `crates/md-codec/Cargo.toml` has no `[[bin]]`, no `[features]` block, and no CLI-only deps.
> 7. CHANGELOG entries land for both crates.
> 8. Per-phase agent-review reports persist under `design/agent-reports/`.
> 9. FOLLOWUPS entries filed for the deferred items.
>
> Surface any cross-phase issues missed by per-phase reviews. Surface anything that should block PR merge. Length cap: 1500 words.

- [ ] **Step 2: Persist + commit the final report**

```bash
git add design/agent-reports/final-review-md-cli-extraction.md
git commit -m "docs(md-cli-extraction): final whole-PR architect review report"
```

- [ ] **Step 3: Apply any final critical/important fixes inline**

If the final review surfaces blockers: fix in follow-up commits. Re-run all 9 acceptance checks.

### Task 12: Operationalize acceptance criterion #4 (binary-behavior parity)

**Files:** none (ephemeral verification).

- [ ] **Step 1: Capture pre-PR `md --help` output for parity comparison**

Before this branch existed, the `md` binary came from `md-codec`. Reconstruct that snapshot:

```bash
git stash  # if any uncommitted changes
git switch main
# (rewind to the merge commit before any md-cli work, in case spec commits affect anything)
git switch --detach 2d6c332  # last v0.15.2 release commit, pre-extraction
cargo build -p md-codec --release
./target/release/md --help > /tmp/md-help-pre.txt
./target/release/md encode --help > /tmp/md-encode-help-pre.txt
git switch feat/md-cli-extraction
```

- [ ] **Step 2: Capture post-PR `md --help` output**

```bash
cargo build -p md-cli --release
./target/release/md --help > /tmp/md-help-post.txt
./target/release/md encode --help > /tmp/md-encode-help-post.txt
```

- [ ] **Step 3: Diff and confirm only-`--version`-differs**

```bash
diff /tmp/md-help-pre.txt /tmp/md-help-post.txt
diff /tmp/md-encode-help-pre.txt /tmp/md-encode-help-post.txt
```

Expected diff: at most a `version: 0.15.2` → `version: 0.1.0` line in `--help` output (clap embeds version in the `--help` header). Subcommand list, arg semantics, examples, exit codes — identical.

If the diff shows anything beyond the version string: stop and investigate. Probably a missed strip or a feature gate that flipped state.

- [ ] **Step 4: Confirm `md --version` differs by design**

```bash
git switch --detach 2d6c332
cargo build -p md-codec --release
./target/release/md --version    # → "md 0.15.2" (from md-codec)
git switch feat/md-cli-extraction
cargo build -p md-cli --release
./target/release/md --version    # → "md 0.1.0" (from md-cli)
```

Expected: versions differ as documented in spec § "Acceptance criteria" #4.

- [ ] **Step 5: No commit; this task produces operational evidence, not artifacts**

### Task 13: Push and open PR

**Files:** none.

- [ ] **Step 1: Verify clean tree, ready to push**

```bash
git status
git log --oneline main..HEAD
```

Expected: working tree clean; ~10-12 commits ahead of `main`.

- [ ] **Step 2: Push the branch**

```bash
git push -u origin feat/md-cli-extraction
```

- [ ] **Step 3: Open PR**

```bash
gh pr create --title "md-cli extraction: md-codec v0.16.0 (library-only) + new md-cli v0.1.0" --body "$(cat <<'EOF'
## Summary

- Pure code-move refactor: extracts the `md` binary from `md-codec` into a new in-repo `md-cli` crate.
- `md-codec` becomes library-only at v0.16.0 (breaking change: no `[[bin]]`, no `cli`/`cli-compiler`/`json` features).
- `md-cli` v0.1.0 ships the `md` binary; `json` + `cli-compiler` feature flags carry over verbatim.
- No wire-format change. No library API change. CLI behavior unchanged except `md --version` (reports `md-cli 0.1.0`).

Spec: `design/SPEC_md_codec_v0_16_library_only.md`
Plan: `design/IMPLEMENTATION_PLAN_md_cli_extraction.md`
Per-phase + final agent reviews: `design/agent-reports/phase-*-md-cli-extraction.md`

## Test plan

- [x] `cargo build --workspace` (default features)
- [x] `cargo build --workspace --all-features`
- [x] `cargo test --workspace` (same set of tests as `main` pre-PR)
- [x] `cargo check -p md-cli --all-targets` — zero warnings
- [x] `cargo check -p md-codec --all-targets` — zero warnings
- [x] `cargo install --path crates/md-cli` produces an `md` binary
- [x] `md --help` output structure identical to pre-PR (only version string differs)
- [x] All 9 spec § "Acceptance criteria" passes (see final review report)

## Out of scope (deferred)

- mk-codec / codex32 wiring (the C-state vision; separate brainstorm/spec/plan cycle).
- C-state transplant of md-cli to a third sibling repo.
- `crates/md-codec/Cargo.toml` `description`/`categories` cleanup (filed under FOLLOWUPS v0.16.x tier).
EOF
)"
```

Expected: PR opens; URL printed.

---

## Self-review

**Spec coverage check.** Each spec section maps to a task:

- Spec § "Goal" → Task 0-13 in aggregate.
- Spec § "Non-goals" → enumerated in plan header + reinforced by Task 4 (no behavior change).
- Spec § "End state" → directly produced by Tasks 2 + 4 + 6.
- Spec § "Manifest changes / md-codec" → Task 4 Step 4.
- Spec § "Manifest changes / md-cli" → Task 2 Step 2.
- Spec § "Manifest changes / workspace" → Task 2 Step 1.
- Spec § "Source-tree changes / Move" → Task 4 Step 2.
- Spec § "Source-tree changes / Edit cmd/vectors.rs" → Task 4 Step 3.
- Spec § "Source-tree changes / Carry #![allow(missing_docs)]" → Task 4 Step 2 (preserved automatically by `git mv`); Task 4 Step 8 verifies.
- Spec § "Test handling" → Task 1 Step 3 (classification verified) + Task 6 (move).
- Spec § "Public-API surface on md-codec" → Task 1 Step 2 (audit).
- Spec § "serde / json policy" → Task 4 Step 4 (md-codec strip) + Task 2 Step 2 (md-cli features) + Task 4 Step 3 (cmd/vectors.rs gate preserved).
- Spec § "Phase plan / Phase 0" → Task 1.
- Spec § "Phase plan / Phase 1" → Tasks 2 + 3.
- Spec § "Phase plan / Phase 2" → Tasks 4 + 5.
- Spec § "Phase plan / Phase 3" → Tasks 6 + 7.
- Spec § "Phase plan / Phase 4" → Tasks 8 + 9 + 10.
- Spec § "Risks & mitigation" → addressed by Task ordering (atomicity in Task 4) + verification steps in Tasks 2/4/6/8.
- Spec § "Rollback" → Task 0 (single feature branch); rollback is `git switch main` + branch delete.
- Spec § "Deferred to FOLLOWUPS" → Task 9.
- Spec § "Acceptance criteria" 1-9 → Task 11 (final review) + Task 12 (binary parity verification).

No gaps detected.

**Placeholder scan.** Re-read each task for the patterns from "No Placeholders" list:

- "TBD" / "TODO" / "implement later": not present.
- "Add appropriate error handling" / vague guidance: not present (every step shows the exact code or command).
- "Write tests for the above": not present (the smoke test code is fully written in Task 2 Step 4; CLI tests come pre-written from md-codec).
- "Similar to Task N": not present (each task's commands are self-contained even where similar).
- Steps without exact commands: not present.
- References to undefined functions/types: not present.

One soft spot: Task 4 Step 4 says "If Phase 0 said keep `insta`, append the line." That's conditional based on Task 1 Step 4's verdict. Acceptable — the verdict is produced before Task 4 runs and the conditional logic is stated explicitly with the exact line to append.

**Type/identifier consistency check:**

- The new crate name is `md-cli` everywhere (not `md_cli`, not `md-bin`, not `mdcli`).
- The binary name is `md` (matches today; matches `[[bin]] name = "md"` in md-cli/Cargo.toml).
- The smoke test function `encode_wpkh_default_phrase` is referenced only once (Task 2 Step 4); the file rename in Task 6 Step 1 doesn't touch the function name.
- Workspace member path `crates/md-cli` is consistent across Tasks 2, 4, 6, 8.
- Branch name `feat/md-cli-extraction` is consistent across Tasks 0, 11, 13.
- Spec commit references (`87f2cf7`, `479e4b0`) and pre-extraction tag commit (`2d6c332`) appear consistently.

No inconsistencies detected.

---

## Conventions

- Commit message format: `feat(md-cli): phase N — <scope>` for implementation commits; `docs(md-cli-extraction): <scope>` for design/audit/review-report commits; `release: md-codec v0.16.0 + md-cli v0.1.0` for the version-bump commit.
- Per-phase agent-review reports go to `design/agent-reports/phase-N-review-md-cli-extraction.md` (consistent with v0.15.x phase-review naming).
- Per-phase architect dispatches use `subagent_type: feature-dev:code-architect` with explicit length caps (matches CLAUDE.md memory `feedback_ultraplan_handoff.md`).
- Branch is `feat/md-cli-extraction`. PR title and body are stamped by Task 13.
- All staging is path-explicit (`git add <path1> <path2>`); no `git add -A` or `git add .` per CLAUDE.md memory `feedback_avoid_git_add_all.md`.
