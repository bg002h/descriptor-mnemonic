# Phase 11 Implementer Report — Version bump + vector regen + FOLLOWUPS close

**Phase**: 11 — release prep (version bump, vector regen, FOLLOWUPS close); push/tag/release deferred to user
**Branch**: `feature/v0.5-multi-leaf-taptree`
**HEAD before commit**: `78e9e0b` (Phase 10)
**Status**: DONE

---

## Modified scope (per controller)

The user is handling final push/tag/release. This subagent's scope was Tasks
11.1–11.4 of the plan (version bump → vector regen → SHA updates → FOLLOWUPS
close → final gates → single release-style commit). Tasks 11.5/11.6 (push, tag,
GitHub release, worktree cleanup) are explicitly user-owned.

---

## Tasks completed

### Task 11.1 — Version bump

- `crates/md-codec/Cargo.toml:3`: `0.4.1` → `0.5.0`.
- `RUSTUP_TOOLCHAIN=stable cargo update --workspace` refreshed `Cargo.lock`.
  Only `md-codec v0.4.1 → v0.5.0` updated; no other dependency churn (3
  unchanged dependencies behind latest, all unrelated to the bump).

### Task 11.2 — Vector regeneration with new family token

The bump rolls the family-stable generator token
(`concat!("md-codec ", CARGO_PKG_VERSION_MAJOR, ".", CARGO_PKG_VERSION_MINOR)`)
from `"md-codec 0.4"` → `"md-codec 0.5"`. Both vector files regenerated:

- `cargo run --bin gen_vectors -- --output crates/md-codec/tests/vectors/v0.2.json --schema 2` → 27 positive + 51 negative
- `cargo run --bin gen_vectors -- --output crates/md-codec/tests/vectors/v0.1.json --schema 1` → 10 positive + 30 negative

Both runs reported `family generator = "md-codec 0.5"; full crate version = "0.5.0"`.

**New SHA-256 digests** (lowercase hex):

| File | Was (v0.4.1 family token) | Now (v0.5 family token) |
|---|---|---|
| `crates/md-codec/tests/vectors/v0.1.json` | `bb2bcc78835d519c7f7595994c6113ef62c379cee99e4d62288772834d4f1c26` | `6d5dd831d05ab0f02707af117cdd2df5f41cf08457c354c871eba8af719030aa` |
| `crates/md-codec/tests/vectors/v0.2.json` | `7d801228ab3529f2df786c50ff269142fae2d8e896a7766fb8eb9fcf080e328d` | `4206cce1f1977347e795d4cc4033dca7780dbb39f5654560af60fbae2ea9c230` |

(v0.2.json's pre-Phase-11 SHA `7d801228...` was the Phase 6 regeneration with
multi-leaf fixtures still under the v0.4 family token. v0.4.1 baseline was
`caddad36...`.)

**Updates landed**:

- `crates/md-codec/tests/vectors_schema.rs:252` — `V0_2_SHA256` constant updated. (No `V0_1_SHA256` constant exists; v0.1.json is checked by structural-equality test only.)
- `CHANGELOG.md` `[0.5.0]` entry — both SHA bullets updated. Replaced the Phase-10 placeholder text ("UNCHANGED — Phase 11 will re-bump") with the actual new digests. Added a "Family-stable promise resets at v0.5.0" line consistent with prior major releases (v0.3 / v0.4).
- `MIGRATION.md` v0.4.x → v0.5.0 §"Test vector SHAs" — both SHA lines + family-token note updated.

### Task 11.3 — Final gates

All gates green:

| Gate | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **634 passed, 0 failed, 0 ignored** (matches Phase 9/10 baseline) |
| `cargo run --bin gen_vectors -- --verify crates/md-codec/tests/vectors/v0.1.json` | PASS — committed file matches regenerated schema-1 vectors (10 + 30) |
| `cargo run --bin gen_vectors -- --verify crates/md-codec/tests/vectors/v0.2.json` | PASS — committed file matches regenerated schema-2 vectors (27 + 51) |
| `cargo fmt --check` | clean (no diff) |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean (no warnings) |

The new `V0_2_SHA256` lock test (`v0_2_sha256_lock_matches_committed_file`) passed under the updated digest, confirming the regenerated file matches the constant.

### Task 11.4 — FOLLOWUPS housekeeping

Moved `v0-5-multi-leaf-taptree` from "Open items" to "Resolved items" in
`design/FOLLOWUPS.md` (now first under Resolved, ahead of the previously-most-recent
`v0-5-stale-v0-4-message-strings-sweep`).

The closing entry's `Status: resolved <release-commit-sha>` placeholder MUST
be updated post-tag to the actual release commit SHA (the commit produced by
this report's commit step). Per the plan's Task 11.5 Step 3, this is a
follow-up edit + commit on `main` after merge.

### Task 11.5 — Single release commit

Committed all changes (8 files) as one release-style commit using the plan's
template (Phase 11 Task 11.3) with the test count corrected to 634 (not 638).

---

## Files modified

```
M  CHANGELOG.md                          # SHAs + family-stable promise line
M  Cargo.lock                            # md-codec 0.4.1 → 0.5.0
M  MIGRATION.md                          # SHAs + family-token note
M  crates/md-codec/Cargo.toml            # version 0.4.1 → 0.5.0
M  crates/md-codec/tests/vectors/v0.1.json  # regenerated with v0.5 token
M  crates/md-codec/tests/vectors/v0.2.json  # regenerated with v0.5 token
M  crates/md-codec/tests/vectors_schema.rs  # V0_2_SHA256 updated
M  design/FOLLOWUPS.md                   # v0-5-multi-leaf-taptree → resolved
```

---

## What this report does NOT cover (user-owned)

Per the controller's modified Phase 11 instructions:

- ❌ NOT pushed to `origin`
- ❌ NOT tagged (`md-codec-v0.5.0`)
- ❌ NOT released on GitHub (`gh release create`)
- ❌ NO PR opened
- ❌ Worktree NOT removed

The user will perform those final steps after reviewing this commit.

---

## Concerns / deferrals

None. All gates clean, all SHA updates landed coherently, vectors verify
under the new family token, and the FOLLOWUPS entry transition is in place
(modulo the post-tag SHA pinning, which is by-design per plan Task 11.5
Step 3).

The placeholder string `<release-commit-sha>` in the FOLLOWUPS resolved entry
is intentional — it will be replaced with the canonical commit SHA on `main`
after merge, per the plan's release-flow.

---

## Final status

**DONE** — Phase 11 release prep complete. Worktree clean, all gates green,
single release commit in place, awaiting user push/tag/release.
