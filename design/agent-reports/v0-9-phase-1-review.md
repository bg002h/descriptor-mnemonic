# v0.9.0 P1 review (opus)

**Date:** 2026-04-29
**Commit:** bad34a0 (`refactor(v0.9-p1): rename ChunkPolicyId → ChunkSetId`)
**Reviewer:** opus-4.7
**Branch:** `feature/v0.9-chunk-set-id-rename`

## Summary

Phase 1 is **needs-fixes-then-proceed**. The rename is overwhelmingly clean
across ~85 sites and survives the full 677-test workspace suite. The
declared rename table is hit accurately: type renames, error variants, the
`EncodeOptions::chunk_set_id_seed` field, the chunk-header field, test
helpers (`csid_a`/`csid_b`, `test_chunk_set_id`), prose, and the BIP
§"Chunk-set identifier" naming-note are all in good shape. Wire format,
Tier-3 `PolicyId`/`PolicyIdWords`/`WalletInstanceId`, the `policy_id` module
name, and the v0.7→v0.8 historical naming-note cross-link are correctly
preserved. Test corpus regeneration is consistent and the `V0_2_SHA256` pin
matches the on-disk file (`bb151bc8…b406e`). However, two real misses
escaped the sweep — one is a **public-API field rename omission** (F1, must
fix in P1 because the wire-compat rationale also makes API-stability
arguments sharper post-release), the other is a small comment bug (F2). One
pre-existing minor staleness from v0.8 (F3) was discovered in passing.

## Findings

### F1 — `Verifications.policy_id_consistent` field (and CLI/JSON surface) was not renamed (must fix)

The public `Verifications` struct in
`crates/md-codec/src/decode_report.rs:78` still carries
`pub policy_id_consistent: bool`, but the rustdoc one line above it
(`decode_report.rs:76`) already says "All chunks declared the same
`chunk_set_id`". By the same rationale that justified the whole P1 rename
(chunk-domain field semantics ≠ Tier-3 `PolicyId`), this field name is
itself collateral from the v0.8 mechanical sweep — git blame confirms it
was renamed `wallet_id_consistent → policy_id_consistent` in commit 35c8119
(v0.8.0). Leaving it now means the public type-level surface still encodes
the obsolete framing that the rest of P1 corrects.

Affected sites (call sites and reflections, all in
`/scratch/code/shibboleth/descriptor-mnemonic/`):

- `crates/md-codec/src/decode_report.rs:78` — field declaration
- `crates/md-codec/src/decode_report.rs:219, 238, 260` — field literals in tests
- `crates/md-codec/src/decode_report.rs:225` — assertion
- `crates/md-codec/src/decode.rs:163, 177` — populated in the decode pipeline
- `crates/md-codec/src/decode.rs:548, 571` — assertions in unit tests
- `crates/md-codec/src/bin/md/json.rs:158, 179, 308, 315` — JSON output struct field, mapping, and tests (CLI consumer-visible name)
- `crates/md-codec/src/bin/md/main.rs:407` — display-key string in human output
- `crates/md-codec/tests/cli.rs:170` — string-match against CLI JSON output

Recommended fix: rename to `chunk_set_id_consistent` everywhere above. This
is a `Verifications` struct field, which is `pub` — so it is technically a
public-API breaking rename, but P1 already breaks public API
(`Error::ChunkSetIdMismatch`, `EncodeOptions::chunk_set_id_seed`,
`ChunkSetIdSeed`), so bundling this rename into the same release-version
boundary is strictly better than letting it leak past v0.9.0 and require a
second future minor-version break.

The plan's rename table at `design/IMPLEMENTATION_PLAN_v0_9_chunk_set_id.md`
line 41 captured prose mentions of "wallet identifier" but did not
enumerate this field. Worth a brief note to the plan file post-fix so any
future rename inherits a complete enumeration.

### F2 — Stale module-name comment in `error.rs`

`crates/md-codec/src/error.rs:12-13`:

```
// `ChunkSetId` is defined in `chunk_set_id` and re-exported here so that
// `Error` variants can reference it without a cross-module path.
```

The module is named `policy_id` (intentionally — it owns both Tier-3
`PolicyId` and the chunk-domain `ChunkSetId`, per `lib.rs:157`'s
`pub mod policy_id;` declaration). The comment's reference to a module
called `chunk_set_id` is wrong; the implementer likely typed the comment
mid-rename. Trivial one-line fix:

```
// `ChunkSetId` is defined in `policy_id` (the module owns both Tier-3
// `PolicyId` and the chunk-domain `ChunkSetId`) and re-exported here so
// that `Error` variants can reference it without a cross-module path.
```

### F3 — Pre-existing v0.8 staleness in `policy_id.rs` test comments (informational; not a P1 regression)

Two stale test comments in
`crates/md-codec/src/policy_id.rs` were missed by v0.8's
`WalletId → PolicyId` mechanical sweep and are still present:

- `policy_id.rs:658` — `// Different inputs must produce different WalletIds.`
- `policy_id.rs:661` — `assert_ne!(id_a, id_b, "distinct inputs must yield distinct WalletIds");`

`git log -S 'distinct inputs must yield distinct WalletIds'` confirms these
landed in 35c8119 (v0.8.0) — they predate Phase 1. They refer to the
Tier-3 `PolicyId` via `compute_policy_id`, so the correct prose is now
`PolicyIds`. Strictly out of P1 scope, but cheap to sweep alongside F1/F2
since you'll be touching this neighborhood. If preferred, defer to a
follow-up entry; either decision is fine.

## Confirmations

- **Rename hit list complete** for the planned targets: `ChunkPolicyId`,
  `PolicyIdSeed`, `chunk_policy_id`, `policy_id_seed`, `PolicyIdMismatch`,
  `ReservedPolicyIdBitsSet`, `wid_a/wid_b`, `chunked_round_trip_max_wallet_id`,
  `test_wallet_id`, `expected_wallet_id` — `rg -nc` returns zero hits across
  `crates/md-codec/`, `design/POLICY_BACKUP.md`, `README.md`, and BIP
  except for one intentional historical reference at line 194 of the BIP
  (the new naming-note itself, which spells `ChunkPolicyId` to document the
  v0.8→v0.9 transition).

- **New names land in expected files** (`rg -nc` counts):
  `chunking.rs:76`, `policy_id.rs:60`, `vectors.rs:27`, `error.rs:8`,
  `encode.rs:28`, `conformance.rs:27`, plus appropriate hits across
  `tests/`, `bin/md/`, `lib.rs`, `BIP`, and `design/POLICY_BACKUP.md`.

- **Tier-3 surface preserved.** `PolicyId`, `PolicyIdWords`,
  `WalletInstanceId`, `compute_policy_id_for_policy`,
  `compute_wallet_instance_id`, `compute_policy_id`, `MdBackup::policy_id()`
  all present and unchanged. `lib.rs:157` `pub mod policy_id;` retained;
  `lib.rs:179-182` re-exports both `ChunkSetId` and the Tier-3 types from
  the same module, as designed.

- **encode.rs local-variable rename complete.** The three
  `expected_chunk_wid` sites are now `expected_chunk_set_id` at lines 284,
  347, 390, 470 (one extra at 284 is correctly `compute_policy_id` for
  Tier-3, others are chunk-header). `chunk_wallet_id → chunk_set_id` at
  lines 73/77 done; `tier3_wallet_id → tier3_policy_id` at lines 106-107
  done.

- **BIP §"Chunk-set identifier" naming-note (lines 192-194) is well-crafted.**
  Explicit on three points: (i) the field was "wallet identifier" in
  ≤ 0.8.x, (ii) v0.8's mechanical `ChunkPolicyId` rename was collateral —
  not a Policy ID and not a Wallet Instance ID, (iii) wire format
  unchanged across v0.8→v0.9. Cross-references to the existing v0.7→v0.8
  Policy-ID naming-note (now line 693) parse correctly: line 693 is the
  target it points at, and the rationale chain is complete.

- **Test corpus regeneration verified.**
  - `tests/vectors/v0.1.json` lines 296, 300, 352, 357 use
    `ReservedChunkSetIdBitsSet` / `ChunkSetIdMismatch` (and the
    descriptions reference "chunk-set-id").
  - `tests/vectors/v0.2.json` lines 1127, 1131, 1190, 1195 same.
  - `sha256sum tests/vectors/v0.2.json` →
    `bb151bc815cb693d030fc8f55619f834ba760932263230a760975131c05b406e`,
    matches `tests/vectors_schema.rs:251` `V0_2_SHA256` pin.
  - As stated in your task brief, the `family-generator` field still says
    "md-codec 0.8" — that is intentional for Phase 1 (regenerated again
    at Phase 4 with the version bump). No action.

- **Sibling crate `crates/md-signer-compat/` is unaffected.** `rg` for the
  old or new names returns zero hits — this crate doesn't use any of the
  renamed types directly.

- **20-bit byte-literal comments and section headings.** All "20-bit …"
  prose now consistently says "chunk-set identifier" /
  "chunk-set-id" / "ChunkSetId". The BIP heading at line 855 ("Why
  mandatory chunk-set identifier in chunked cards?") is correct. No stale
  "20-bit ChunkPolicyId" anywhere.

- **Test suite green.** `cargo test --workspace --all-features` →
  **677 tests passed**, 0 failed, 0 ignored. Matches your reported number.

- **No `TODO`/`FIXME`/`XXX` markers** referencing v0.9 or "rename" remain.

## Open questions for the implementer

1. **Is F1 in scope for P1, or do you want to defer it to a Phase 1.5 / a
   FOLLOWUPS entry to land before the Phase 4 release tag?** My
   recommendation: in scope for P1 because (a) the rest of P1 already
   breaks public API at the type level, so this is strictly cheaper to
   land in the same commit, (b) the JSON CLI output key is
   user-observable, (c) the rationale that justifies the type renames
   ("not a Policy ID — a chunk-set assembly identifier") applies one-to-one
   to this field. If you agree, the followups list is small (10 sites,
   listed in F1 above) and the test corpus does *not* assert on the JSON
   key by raw bytes (only `tests/cli.rs:170` does, and that's a string
   match easily updated).

2. **F3 sweep along with F1/F2, or split out as a tiny FOLLOWUPS entry?**
   Either works. If you sweep, the diff stays in `policy_id.rs` and the
   commit message can fold it into "fix v0.8-era stragglers surfaced
   during P1 review."

3. **Plan-table addendum?** Worth a short follow-up commit appending
   `Verifications.policy_id_consistent → chunk_set_id_consistent` to the
   IMPLEMENTATION_PLAN's rename table (line 41 area) so the plan file
   matches reality once F1 lands. Optional.
