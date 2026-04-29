# md-codec v0.9.0 — chunk-set-id rename + path-dictionary closures

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Per-phase opus reviewer reports persist to `design/agent-reports/v0-9-phase-N-review.md`.

**Goal:** Rename the chunk-header 20-bit field from `ChunkPolicyId` to `ChunkSetId` (and the seed override from `PolicyIdSeed` to `ChunkSetIdSeed`); close the testnet `0x16` BIP 48 nested-segwit path-dictionary gap; formalize the md1↔mk1 path-dictionary inheritance contract.

**Architecture:** Pure rename (no wire-format change for P1) + one-row additive wire extension (P2) + prose-only stewardship note (P3). Three FOLLOWUPS entries close, one stays open (`md-per-at-N-path-tag-allocation`, deferred to v1+).

**Tech Stack:** Same as v0.8.0 — Rust 2024, miniscript fork pinned to `f7f1689b…`, codex32-derived BCH layer.

---

## Scope

### In-scope (mk1-surfaced FOLLOWUPS closed in this release)

1. `chunk-set-id-rename` — symbol + prose rename across code, BIP, README, MIGRATION, design docs.
2. `md-path-dictionary-0x16-gap` — add testnet `m/48'/1'/0'/1'` at indicator `0x16`.
3. `path-dictionary-mirror-stewardship` — bidirectional release-checklist invariant.

### Out-of-scope (deferred)

- `md-per-at-N-path-tag-allocation` — wire-format-breaking; awaits multisig-feature scheduling.
- Any change to `PolicyId` (Tier-3 16-byte content-derived identifier).
- Any change to `WalletInstanceId` (derived xpub-aware identifier).
- Any change to `compute_policy_id_for_policy` / `compute_wallet_instance_id` semantics.

### Naming map (P1)

| Old (v0.8.0) | New (v0.9.0) | Notes |
|---|---|---|
| `ChunkPolicyId` | `ChunkSetId` | type, struct, all references |
| `PolicyIdSeed` | `ChunkSetIdSeed` | type — semantically seeds the chunk-set-id, not the policy-id |
| `EncodeOptions::policy_id_seed` | `EncodeOptions::chunk_set_id_seed` | option field name |
| `Error::PolicyIdMismatch { expected, got }` | `Error::ChunkSetIdMismatch { expected, got }` | **chunk-level** error (residue from v0.8 mechanical rename — see F1) |
| `Error::ReservedPolicyIdBitsSet` | `Error::ReservedChunkSetIdBitsSet` | chunk-header parser error (residue from v0.8) |
| `ChunkHeader::Chunked.policy_id` (struct field) | `ChunkHeader::Chunked.chunk_set_id` | **field rename** — sed-invisible (F2) |
| `Chunk.policy_id` (struct field) | `Chunk.chunk_set_id` | field rename |
| `chunk_policy_id` (vars/params) | `chunk_set_id` | local names |
| `test_wallet_id`, `expected_wallet_id`, `chunked_round_trip_max_wallet_id`, `wid_a`/`wid_b` | `test_chunk_set_id`, `expected_chunk_set_id`, `chunked_round_trip_max_chunk_set_id`, `csid_a`/`csid_b` | test-helper names (F2) |
| "Wallet identifier" / "wallet-id" / "wallet identifier bits" (BIP §, prose, comments) | "Chunk-set identifier" / "chunk-set-id" / "chunk-set-id bits" | ~76 prose hits across crate, design/, README, MIGRATION |

**Out-of-rename (must NOT change):** `PolicyId`, `PolicyIdWords`, `WalletInstanceId`, `compute_policy_id_for_policy`, `compute_wallet_instance_id`, `compute_policy_id`. These are Tier-3 / template-instance domain symbols and have no chunk-id semantics.

**Note:** `Error::WalletIdMismatch` does not exist in the codebase; the v0.8 rename mechanically renamed it to `Error::PolicyIdMismatch`. Per F1, that name is itself misclassified — the variant is chunk-level, not Tier-3 — and renames again to `Error::ChunkSetIdMismatch`.

---

## Phase 0: Open coordinated mk1 draft PR (F5)

**Files:**
- Modify in sibling repo `/scratch/code/shibboleth/mnemonic-key`: spec / BIP draft prose using `chunk_set_id` terminology.

**Steps:**

- [ ] **Step 1: Open mk1 draft PR**

Switch to `/scratch/code/shibboleth/mnemonic-key`, cut a `feature/chunk-set-id-terminology-update` branch, update mk1's spec and BIP draft to use the proposed `chunk_set_id` terminology with a forward-pointer to "pending md1 v0.9.0 release."

```bash
gh pr create --draft --title "spec: switch to chunk_set_id terminology (pending md1 v0.9.0)" --body "Tracks md1 v0.9.0 rename; will flip to ready-for-review once md-codec-v0.9.0 ships."
```

- [ ] **Step 2: Cross-link in this repo's PR body when opened in P4**

The md1 release-PR body must link the mk1 draft. The mk1 draft body must link the md1 release-PR. (Both PRs cite each other so reviewers see the full cross-repo shape from day one.)

This Phase 0 establishes the coordinated-draft state. Phase 4 step 11 (after md1 ships) will flip the mk1 draft to ready-for-review with the resolved md-codec commit pinned.

---

## Phase 1: chunk-set-id rename (TDD-style mass rename)

**Files:**
- Modify: `crates/md-codec/src/policy_id.rs` (~58 references, primary)
- Modify: `crates/md-codec/src/chunking.rs` (~20)
- Modify: `crates/md-codec/src/options.rs` (~17)
- Modify: `crates/md-codec/src/vectors.rs` (~13)
- Modify: `crates/md-codec/tests/conformance.rs` (~9)
- Modify: `crates/md-codec/src/encode.rs` (~8)
- Modify: `crates/md-codec/src/lib.rs` (~6, also rustdoc top-level "Identifiers" section)
- Modify: `crates/md-codec/src/error.rs` (~4)
- Modify: `crates/md-codec/src/decode.rs` (~4)
- Modify: `crates/md-codec/src/bin/md/main.rs` (~4)
- Modify: `crates/md-codec/tests/chunking.rs` (~2)
- Modify: `crates/md-codec/tests/common/mod.rs` (~1)
- Modify: `crates/md-codec/src/policy.rs` (~1)
- Modify: `crates/md-codec/tests/vectors/v0.2.json` (chunk_policy_id JSON field name — migration-bearing!)
- Modify: `bip/bip-mnemonic-descriptor.mediawiki` (§"Wallet identifier" header + prose)
- Modify: `README.md` (any chunk-header prose)
- Modify: `design/POLICY_BACKUP.md` (~9 references)
- Modify: `MIGRATION.md` (existing v0.7→v0.8 section + new v0.8→v0.9 section)
- Modify: `CHANGELOG.md` (new [0.9.0] section)

**Steps:**

- [ ] **Step 1: Confirm baseline grep**

```bash
rg -n 'ChunkPolicyId|PolicyIdSeed|policy_id_seed|chunk_policy_id|PolicyIdMismatch|ReservedPolicyIdBitsSet' crates/md-codec/src/ crates/md-codec/tests/
rg -nc 'wallet[-_ ][Ii]dentif|wallet[-_ ]ID|wallet[-_ ]id\b|wid_[ab]\b' crates/md-codec/ design/POLICY_BACKUP.md MIGRATION.md README.md
```

Expected: confirms F1 (`PolicyIdMismatch`/`ReservedPolicyIdBitsSet` are chunk-level — see review pass 1 finding F1) and F2 (`wallet_id` / `wid_a`/`wid_b` test helpers exist; ~76 prose hits). No `WalletIdMismatch` (zero hits) and no compound `ChunkPolicyId\w+` / `policy_id_seed\w+`.

- [ ] **Step 2: Run sed sweep over src + tests + bin + docs**

```bash
# Type names (CamelCase) — full word, no compound prefix in codebase
find crates/md-codec/src crates/md-codec/tests -type f \( -name '*.rs' -o -name '*.json' \) -exec sed -i \
  -e 's/\bChunkPolicyId\b/ChunkSetId/g' \
  -e 's/\bPolicyIdSeed\b/ChunkSetIdSeed/g' \
  -e 's/\bPolicyIdMismatch\b/ChunkSetIdMismatch/g' \
  -e 's/\bReservedPolicyIdBitsSet\b/ReservedChunkSetIdBitsSet/g' \
  {} +

# Snake-case names — \b\w+\b form is sufficient (F3: drop redundant compound-suffix patterns)
find crates/md-codec/src crates/md-codec/tests -type f \( -name '*.rs' -o -name '*.json' \) -exec sed -i \
  -e 's/\bchunk_policy_id\b/chunk_set_id/g' \
  -e 's/\bpolicy_id_seed\b/chunk_set_id_seed/g' \
  -e 's/\bexpected_wallet_id\b/expected_chunk_set_id/g' \
  -e 's/\btest_wallet_id\b/test_chunk_set_id/g' \
  -e 's/\bchunked_round_trip_max_wallet_id\b/chunked_round_trip_max_chunk_set_id/g' \
  -e 's/\bwid_a\b/csid_a/g' \
  -e 's/\bwid_b\b/csid_b/g' \
  {} +
```

- [ ] **Step 2.5: Hand-edit `policy_id:` struct field on `ChunkHeader::Chunked` and `Chunk` (sed-invisible)**

The bare name `policy_id` is also used pervasively as a sed-anchor risk: a global `s/\bpolicy_id\b/chunk_set_id/g` would FALSELY hit `compute_policy_id`, `policy_id_words`, `policy_id_for_policy`, `policy_id_seed` (we've already renamed the seed in Step 2 so the seed risk is moot, but the others remain). So renaming the struct field requires hand-edits, not sed.

```bash
rg -n 'policy_id:|policy_id,\|\.policy_id\b' crates/md-codec/src/chunking.rs crates/md-codec/src/vectors.rs crates/md-codec/tests/ | grep -v 'policy_id_words\|compute_policy_id\|policy_id_for_policy'
```

Hand-rename:
- `ChunkHeader::Chunked { ..., policy_id: ChunkSetId, ... }` field → `chunk_set_id`
- `Chunk { ..., policy_id: ..., ... }` field → `chunk_set_id`
- All destructuring sites: `ChunkHeader::Chunked { policy_id, count, .. }` → `ChunkHeader::Chunked { chunk_set_id, count, .. }` (or use `policy_id: chunk_set_id` rebinding shorthand if it preserves call sites better).
- All `.policy_id` field accesses on these structs.

This will cascade ~25 destructuring sites. Run `cargo build` after each batch to keep error volume manageable.

- [ ] **Step 3: Update prose mentions of "wallet identifier" / "wallet-id" → "chunk-set identifier" / "chunk-set-id" in rustdoc and code comments**

Manual sweep — rustdoc and inline comments only. ~21 occurrences in `chunking.rs` alone (variable names already swept in Step 2; this step is for prose).

```bash
rg -n '[Ww]allet identifier|[Ww]allet[- ][Ii][Dd]\b|wallet-id bits|"wallet[- _]id"' crates/md-codec/src/
```

Also rename error message strings in `error.rs`:
- "wallet identifier mismatch across chunks: …" → "chunk-set identifier mismatch across chunks: …"
- "reserved wallet-id bits set: …" → "reserved chunk-set-id bits set: …"

Do **not** touch BIP / README / POLICY_BACKUP yet (Step 7-8).

- [ ] **Step 4: Cargo build (clean compile target)**

```bash
cargo build --workspace --all-features
```

Expected: no errors. If errors: usually a compound-name miss; grep `error[E0` output, locate stragglers, hand-edit, re-run.

- [ ] **Step 5: Cargo test**

```bash
cargo test --workspace --all-features
```

Expected: all tests pass. Conformance tests check `every_error_variant_has_a_rejects_test` — that protects against silent variant drops.

- [ ] **Step 6: Regenerate v0.1.json AND v0.2.json corpora (F1 + F7)**

The error-variant rename (`PolicyIdMismatch → ChunkSetIdMismatch`, `ReservedPolicyIdBitsSet → ReservedChunkSetIdBitsSet`) changes the `expected_error_variant` strings in BOTH schema-1 and schema-2 corpus JSON files. v0.1.json is no longer "byte-identical regen" after this rename.

```bash
cargo run --features test-helpers --bin gen_vectors -- --output crates/md-codec/tests/vectors/v0.1.json --schema 1
cargo run --features test-helpers --bin gen_vectors -- --output crates/md-codec/tests/vectors/v0.2.json
```

SHA-pin updates land in Phase 4 step 4 after the P2 0x16 corpus addition (the SHA pin should reflect the final state).

- [ ] **Step 7: Update BIP draft prose**

`bip/bip-mnemonic-descriptor.mediawiki`:
- §"Wallet identifier" header (line ~188) → §"Chunk-set identifier"
- Within-section prose ("the wallet identifier is…") → "the chunk-set identifier is…"
- Cross-references elsewhere in the BIP body
- Add naming-note explaining the v0.8→v0.9 rename (parallel structure to the §"Wallet Instance ID" naming-note added in v0.8). Frame as "correcting collateral damage from v0.8" — see CHANGELOG `### Why a rename, again?` block (P4 step 2).

```bash
rg -n '[Ww]allet identifier|wallet_id\b|chunk_policy_id\b|[Pp]olicy [Ii]d [Mm]ismatch' bip/bip-mnemonic-descriptor.mediawiki
```

- [ ] **Step 8: Update README, MIGRATION, design/POLICY_BACKUP**

```bash
rg -l '[Ww]allet identifier|wallet_id\b|chunk_policy_id\b|ChunkPolicyId|PolicyIdSeed|policy_id_seed' README.md MIGRATION.md design/POLICY_BACKUP.md
```

- README.md: update any chunk-header prose.
- MIGRATION.md: add new `## v0.8.x → v0.9.0` section with mechanical sed snippet for consumer code.
- design/POLICY_BACKUP.md: rename in design rationale prose.

- [ ] **Step 9: Final compile + test + clippy + doc**

```bash
cargo build --workspace --all-features && \
cargo test --workspace --all-features && \
cargo clippy --workspace --all-features --all-targets -- -D warnings && \
cargo doc --workspace --all-features --no-deps
```

- [ ] **Step 10: Commit P1**

```bash
git add -A
git commit -m "$(cat <<'EOF'
refactor(v0.9-p1): rename ChunkPolicyId → ChunkSetId

Per mk1 v0.1 closure-design naming review (FOLLOWUPS entry
chunk-set-id-rename). The chunked-string-header 20-bit field
identifies a chunk-set assembly, not a policy or a wallet —
the prior name created naming friction with Tier-3 PolicyId
and WalletInstanceId.

Pure docs+symbols rename; wire format unchanged. Hard
precondition for mk1's BIP submission.

Renames:
- ChunkPolicyId → ChunkSetId
- PolicyIdSeed → ChunkSetIdSeed
- EncodeOptions::policy_id_seed → chunk_set_id_seed
- "Wallet identifier" (BIP §, prose) → "Chunk-set identifier"

PolicyId, PolicyIdWords, WalletInstanceId,
compute_policy_id_for_policy, compute_wallet_instance_id
intentionally untouched.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 11: Opus reviewer pass on P1**

Dispatch `Agent` with `model: opus`, `description: "v0.9 P1 chunk-set-id rename review"`. Persist final report to `design/agent-reports/v0-9-phase-1-review.md`. Address any issues raised before P2.

---

## Phase 2: testnet 0x16 path-dictionary entry + corpus vector

**Files:**
- Modify: `crates/md-codec/src/bytecode/path.rs` (3 sites: dictionary tuple, rustdoc indicator-list, negative-test arrays at ~363 and ~512; positive-test array at ~484)
- Modify: `bip/bip-mnemonic-descriptor.mediawiki` (insert `0x16` row between `0x15` and `0x17`)
- Modify: `crates/md-codec/tests/vectors.rs` (or wherever corpus vectors land — confirm in step 1)
- Modify: `crates/md-codec/tests/vectors/v0.2.json` (regenerated)

**Steps:**

- [ ] **Step 1: Locate corpus / round-trip vector definitions**

```bash
rg -l 'BIP 48.*testnet|0x15.*0x17|m/48.*1.*0' crates/md-codec/tests/ crates/md-codec/src/vectors.rs
```

- [ ] **Step 2: Write failing dictionary-positive test**

In `crates/md-codec/src/bytecode/path.rs` test module, add:

```rust
#[test]
fn indicator_0x16_decodes_to_bip48_testnet_p2sh_p2wsh() {
    let pd = path_dictionary();
    let path = pd.get(&0x16u8).expect("0x16 must map to a path after v0.9");
    assert_eq!(path.to_string(), "48'/1'/0'/1'");
}
```

- [ ] **Step 3: Run test to verify failure**

```bash
cargo test --package md-codec indicator_0x16_decodes_to_bip48_testnet_p2sh_p2wsh -- --nocapture
```

Expected: FAIL — current map has 0x16 absent.

- [ ] **Step 4: Add 0x16 row to dictionary**

Edit `crates/md-codec/src/bytecode/path.rs` line 23-30:
- Remove the `// Testnet (0x16 is reserved — intentional gap)` comment line.
- Insert `(0x16, DerivationPath::from_str("m/48'/1'/0'/1'").unwrap()),` between `0x15` and `0x17`.

- [ ] **Step 5: Update rustdoc indicator-list**

Line ~89: "known indicators (`0x01`–`0x07`, `0x11`–`0x15`, `0x17`)" → "(`0x01`–`0x07`, `0x11`–`0x17`)".

- [ ] **Step 6: Update both negative-test arrays AND positive FIXTURE (F4)**

Two distinct test arrays in `crates/md-codec/src/bytecode/path.rs` enumerate `0x16` as rejected:

- **Line 363** (`decode_rejects_reserved_indicator`): `for &b in &[0x00u8, 0x08, 0x10, 0x16, 0x18, 0xFD, 0xFF]` — pins `decode_path` rejecting `0x16` with `UnknownTag(0x16)`. Drop `0x16` from this array.
- **Line 512** (`unknown_indicator_returns_none`): `for &ind in &[0x00u8, 0x08, 0x10, 0x16, 0x18, 0xFD]` — pins `indicator_to_path(0x16) → None`. Drop `0x16` from this array.

Update the positive `FIXTURE` table at ~484: insert `(0x16, "m/48'/1'/0'/1'")` between `0x15` and `0x17` (sorted-order, matching BIP table layout).

- [ ] **Step 7: Run tests**

```bash
cargo test --package md-codec
```

Expected: PASS, including the new dictionary-positive test.

- [ ] **Step 8: Add corpus round-trip vector**

In the test-vector corpus, add a `wsh(...)` policy with `[fp/48'/1'/0'/1']xpub_TESTNET` origin path. The corpus generator should pick up indicator `0x16`. Run:

```bash
cargo run --features test-helpers --bin gen_vectors -- --output crates/md-codec/tests/vectors/v0.2.json
cargo test --package md-codec
```

Expected: vectors_schema.rs SHA-pin assertion will fail. Update SHA pin in step 11 of Phase 4 (release).

- [ ] **Step 9: BIP table update**

`bip/bip-mnemonic-descriptor.mediawiki` line ~349, between `0x15` and `0x17`:

```mediawiki
| <code>0x16</code> || <code>m/48'/1'/0'/1'</code> || BIP 48 testnet multisig P2SH-P2WSH
|-
```

Skip the F8 sub-note about `Tag::OrB = 0x16` — the BIP's table headers already disambiguate the namespaces, and adding the cross-reference is paranoia-grade noise.

- [ ] **Step 10: Compile, test, clippy, doc**

```bash
cargo build --workspace --all-features && \
cargo test --workspace --all-features && \
cargo clippy --workspace --all-features --all-targets -- -D warnings && \
cargo doc --workspace --all-features --no-deps
```

(Vectors-schema test will fail until SHA pin update in Phase 4. Verify only that failure mode.)

- [ ] **Step 11: Commit P2**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat(v0.9-p2): add testnet 0x16 BIP 48 nested-segwit path indicator

Mainnet had 0x06 = m/48'/0'/0'/1' (BIP 48 nested-segwit P2SH-P2WSH);
the testnet companion at 0x16 = m/48'/1'/0'/1' was missed in v0.x.
Closes md-path-dictionary-0x16-gap (mk1-surfaced).

Wire-additive: existing decoders treated 0x16 as reserved/rejected.
Forward-only — old encodings remain valid; new encodings using 0x16
require v0.9+ decoders.

mk1 inherits the entry by its byte-for-byte mirror clause.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 12: Opus reviewer pass on P2**

Persist to `design/agent-reports/v0-9-phase-2-review.md`. Verify wire-additivity argument and BIP-table parity with code.

---

## Phase 3: path-dictionary mirror stewardship prose

**Files:**
- Create: `design/RELEASE_PROCESS.md`
- Modify: `bip/bip-mnemonic-descriptor.mediawiki` (§"Path dictionary" trailing paragraph)

**Steps:**

- [ ] **Step 1: Add stewardship paragraph to BIP §"Path dictionary"**

After the dictionary table (around line ~354), add a paragraph:

> '''Cross-format inheritance.''' The path dictionary is contractually shared with the sibling Mnemonic Key (MK) format ([https://github.com/bg002h/mnemonic-key]) byte-for-byte. Any allocation, deletion, or renumbering in this table requires a coordinated update of MK's specification and reference implementation in the same release window; see <code>RELEASE_PROCESS.md</code> in the reference implementation for the lockstep checklist.

- [ ] **Step 2: Create design/RELEASE_PROCESS.md**

```markdown
# md-codec release process

## Release-checklist invariants

### Path-dictionary lockstep with mk1

Any change to the path dictionary in `crates/md-codec/src/bytecode/path.rs` (or
the BIP §"Path dictionary" table) requires an mk1 spec amendment in the same
release window. mk1 inherits md1's path dictionary byte-for-byte; silent drift
is a wire-format hazard.

**Before tagging a release that touches the path dictionary:**

- [ ] Open a coordinated PR in `bg002h/mnemonic-key` updating mk1's spec to
      mirror md1's table.
- [ ] Cross-link the two PRs in their bodies.
- [ ] Land both before tagging the md1 release; mk1 ships its corresponding
      version (or follow-on patch) in the same window.

### CLAUDE.md crosspointer maintenance

When a mk1-surfaced FOLLOWUPS entry resolves in this repo, update both:
- `design/FOLLOWUPS.md` — mark `Status: resolved <COMMIT>`.
- mk1's `design/FOLLOWUPS.md` companion — note the resolving md1 commit.
- `CLAUDE.md` — drop the entry from the "Currently open mk1-surfaced items"
  list.

### Wire-format SHA pin

Every release that ships canonical-vector changes updates `vectors_schema.rs`
SHA pin to the new corpus SHA, computed via `sha256sum tests/vectors/v0.2.json`
(or whichever vectors file is canonical at the time).
```

- [ ] **Step 3: Compile + doc smoke test**

```bash
cargo doc --workspace --all-features --no-deps
```

(Pure prose; should be a no-op for the build.)

- [ ] **Step 4: Commit P3**

```bash
git add -A
git commit -m "$(cat <<'EOF'
docs(v0.9-p3): formalize md1↔mk1 path-dictionary stewardship

Adds design/RELEASE_PROCESS.md with the lockstep-checklist
invariant and a CLAUDE.md-maintenance note for mk1-surfaced
FOLLOWUPS resolution. BIP §"Path dictionary" gains a
"Cross-format inheritance" paragraph pointing readers to
the release-process doc.

Closes path-dictionary-mirror-stewardship (mk1-surfaced).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 5: Opus reviewer pass on P3**

Persist to `design/agent-reports/v0-9-phase-3-review.md`. Light review — prose only.

---

## Phase 4: Release v0.9.0

**Files:**
- Modify: `crates/md-codec/Cargo.toml` (`0.8.0` → `0.9.0`)
- Modify: `crates/md-signer-compat/Cargo.toml` (bump if it depends on a renamed type — confirm)
- Modify: `crates/md-codec/tests/vectors_schema.rs` (SHA pin)
- Modify: `CHANGELOG.md` (new `[0.9.0]` section)
- Modify: `MIGRATION.md` (already updated in P1 step 8 — verify final form)
- Modify: `design/FOLLOWUPS.md` (mark 3 entries resolved)
- Modify: `CLAUDE.md` (drop 3 resolved entries from "Currently open" list, leave `md-per-at-N` and the new resolved cross-refs)
- Modify: sibling `mnemonic-key/design/FOLLOWUPS.md` (note resolving md-codec commit on companion entries) — separate sibling-repo commit

**Steps:**

- [ ] **Step 1: Bump versions**

First, audit md-signer-compat dependency surface (F9):

```bash
rg 'ChunkPolicyId|PolicyIdSeed|policy_id_seed|chunk_policy_id|PolicyIdMismatch|ReservedPolicyIdBitsSet' crates/md-signer-compat/
```

`crates/md-codec/Cargo.toml`:
```toml
version = "0.9.0"
```

Decide md-signer-compat bump magnitude:
- If audit returns hits in *public API surface*: minor bump 0.1.1 → 0.2.0.
- If hits are only in test/internal code: patch bump 0.1.1 → 0.1.2 (rebuild only).
- If zero hits: leave at 0.1.1, rebuild verifies.

- [ ] **Step 2: Update CHANGELOG.md**

Add `[0.9.0] — 2026-04-29` section. Lead with the "Why a rename, again?" callout (F6) before listing changes:

```markdown
## [0.9.0] — 2026-04-29

### Why a rename, *again*?

v0.8.0 renamed `WalletId → PolicyId` to align with BIP 388's policy-template
framing (Tier-3 = "the policy"). That rename mechanically renamed the
chunk-header 20-bit field `ChunkWalletId → ChunkPolicyId` and two error
variants (`WalletIdMismatch → PolicyIdMismatch`, `ReservedWalletIdBitsSet →
ReservedPolicyIdBitsSet`) along with it. On review for the mk1 BIP submission,
this turned out to be miscategorized: those names belong to the chunk-header
sub-domain and identify a chunk-set assembly — not a Policy ID, not a Wallet
Instance ID. v0.9.0 corrects the chunk-header sub-domain to
`ChunkSetId`/`ChunkSetIdMismatch`/`ReservedChunkSetIdBitsSet`. v0.8's
`PolicyId` and `WalletInstanceId` are stable and unchanged. We expect this to
be the last identifier rename in this family.

### Changed

- Renamed `ChunkPolicyId` → `ChunkSetId`, `PolicyIdSeed` → `ChunkSetIdSeed`, option `policy_id_seed` → `chunk_set_id_seed`, errors `PolicyIdMismatch` → `ChunkSetIdMismatch`, `ReservedPolicyIdBitsSet` → `ReservedChunkSetIdBitsSet`. Test-helper names (`test_wallet_id`, `expected_wallet_id`, `wid_a`/`wid_b`, etc.) renamed to `chunk_set_id` / `csid_*` mirror.
- BIP §"Wallet identifier" → §"Chunk-set identifier" with a naming-note explaining the v0.8→v0.9 correction.

### Added

- Path-dictionary indicator `0x16 = m/48'/1'/0'/1'` (BIP 48 testnet P2SH-P2WSH). Wire-additive: existing decoders rejected `0x16` as an unknown indicator.
- `design/RELEASE_PROCESS.md` documenting the md1↔mk1 path-dictionary lockstep release invariant.

### Wire format

- Unchanged for the rename portion (chunk-set-id is the same 20-bit field, just spelled differently).
- Additive for `0x16` (forward-only — old encodings remain valid; encodings using `0x16` need v0.9+ decoders).
- Test-vector corpus JSON files (`v0.1.json` and `v0.2.json`) regenerated with new error-variant strings; SHA pins updated accordingly.

### FOLLOWUPS closed (mk1-surfaced)

- `chunk-set-id-rename` (resolved <COMMIT>)
- `md-path-dictionary-0x16-gap` (resolved <COMMIT>)
- `path-dictionary-mirror-stewardship` (resolved <COMMIT>)
```

- [ ] **Step 3: Verify MIGRATION.md**

Confirm v0.8.x → v0.9.0 section from P1 step 8 is complete. Lead the section with a parallel "Why a rename, again?" paragraph (F6) so consumers see the framing before the sed snippet.

Sed snippet for consumer code:

```bash
# Type names
find . -type f -name '*.rs' -exec sed -i \
  -e 's/\bChunkPolicyId\b/ChunkSetId/g' \
  -e 's/\bPolicyIdSeed\b/ChunkSetIdSeed/g' \
  -e 's/\bPolicyIdMismatch\b/ChunkSetIdMismatch/g' \
  -e 's/\bReservedPolicyIdBitsSet\b/ReservedChunkSetIdBitsSet/g' \
  -e 's/\bchunk_policy_id\b/chunk_set_id/g' \
  -e 's/\bpolicy_id_seed\b/chunk_set_id_seed/g' \
  {} +
```

Note: `ChunkHeader::Chunked.policy_id` and `Chunk.policy_id` field accesses require hand-rename — sed cannot disambiguate from `compute_policy_id` etc. Consumers grepping `\.policy_id\b` should review hits manually.

- [ ] **Step 4: Regen BOTH corpora and pin new SHAs (F7)**

The error-variant rename in P1 changed `expected_error_variant` strings in v0.1.json; the 0x16 corpus addition in P2 changed v0.2.json. Both need fresh SHA pins.

```bash
cargo run --features test-helpers --bin gen_vectors -- --output crates/md-codec/tests/vectors/v0.1.json --schema 1
cargo run --features test-helpers --bin gen_vectors -- --output crates/md-codec/tests/vectors/v0.2.json
sha256sum crates/md-codec/tests/vectors/v0.1.json crates/md-codec/tests/vectors/v0.2.json
```

Update BOTH SHA constants in `crates/md-codec/tests/vectors_schema.rs`. Run:

```bash
cargo test --workspace --all-features
```

- [ ] **Step 5: Mark FOLLOWUPS resolved**

In `design/FOLLOWUPS.md`, move 3 entries from "Open" to "Resolved":
- `chunk-set-id-rename` (status: `resolved <COMMIT>`)
- `md-path-dictionary-0x16-gap` (status: `resolved <COMMIT>`)
- `path-dictionary-mirror-stewardship` (status: `resolved <COMMIT>`)

Leave `md-per-at-N-path-tag-allocation` open.

- [ ] **Step 6: Update CLAUDE.md crosspointer**

Drop 3 resolved entries from "Currently open mk1-surfaced items affecting md1" list; keep `md-per-at-N-path-tag-allocation`.

- [ ] **Step 7: Final test + clippy + doc + commit**

```bash
cargo build --workspace --all-features && \
cargo test --workspace --all-features && \
cargo clippy --workspace --all-features --all-targets -- -D warnings && \
cargo doc --workspace --all-features --no-deps

git add -A
git commit -m "$(cat <<'EOF'
release(v0.9.0): chunk-set-id rename + 0x16 path dictionary + stewardship

- Rename ChunkPolicyId → ChunkSetId (mk1 BIP-submission precondition)
- Add testnet 0x16 = m/48'/1'/0'/1' BIP 48 P2SH-P2WSH path-dictionary entry
- Formalize md1↔mk1 path-dictionary lockstep release process

Closes 3 mk1-surfaced FOLLOWUPS:
- chunk-set-id-rename
- md-path-dictionary-0x16-gap
- path-dictionary-mirror-stewardship

Wire-format: unchanged for rename; additive (0x16) for new dictionary entry.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 8: Push branch, open PR, wait for CI green, merge**

```bash
git push -u origin feature/v0.9-chunk-set-id-rename
gh pr create --title "release(v0.9.0): chunk-set-id rename + 0x16 path dictionary + stewardship" --body "..."
```

After CI green and merge, return to main:

```bash
git checkout main && git pull --ff-only
```

- [ ] **Step 9: Tag and release**

```bash
git tag -a md-codec-v0.9.0 <MERGE-COMMIT> -m "md-codec v0.9.0"
git push origin md-codec-v0.9.0
gh release create md-codec-v0.9.0 --title "md-codec v0.9.0" --notes-file <changelog-extract>
```

- [ ] **Step 10: Update sibling mnemonic-key FOLLOWUPS companion entries**

In `/scratch/code/shibboleth/mnemonic-key/design/FOLLOWUPS.md`:
- Mark `chunk-set-id-rename`, `md-path-dictionary-0x16-gap`, `path-dictionary-mirror-stewardship` as `Status: resolved by md-codec-v0.9.0 (commit ...)`.

This is a separate sibling-repo commit on its own branch (separate from the Phase 0 mk1 draft PR which handles the spec/BIP terminology update).

- [ ] **Step 11: Flip Phase 0 mk1 draft PR to ready-for-review**

In the mk1 draft PR opened in Phase 0:
- Pin the resolved md-codec commit/tag in the PR body.
- Mark ready-for-review.
- Land it in mk1's normal merge cadence.

mk1's BIP-submission gate is now cleared.

---

## Self-review checklist (post plan-review pass-1)

- [x] Spec coverage: all 3 in-scope FOLLOWUPS entries have a phase.
- [x] No placeholders: every step has actual code or an explicit grep/sed command.
- [x] Type consistency: `ChunkSetId`/`ChunkSetIdSeed`/`chunk_set_id_seed`/`ChunkSetIdMismatch`/`ReservedChunkSetIdBitsSet` consistent across phases.
- [x] Out-of-rename guardrails: `PolicyId`, `WalletInstanceId`, `compute_policy_id_for_policy`, `compute_policy_id`, `policy_id_words` not touched.
- [x] F1 addressed: error variants `PolicyIdMismatch`/`ReservedPolicyIdBitsSet` added to rename map.
- [x] F2 addressed: `policy_id:` field hand-rename step (Step 2.5) added; `wallet_id`/`wid_a`/`wid_b` test-helper renames added; ~76 prose sweep noted.
- [x] F3 addressed: redundant compound-suffix sed patterns dropped (\b form only).
- [x] F4 addressed: P2 step 6 explicitly lists both line 363 and line 512.
- [x] F5 addressed: Phase 0 added — open mk1 draft PR before md1 release-PR.
- [x] F6 addressed: "Why a rename, again?" callout in CHANGELOG (P4 step 2) and MIGRATION (P4 step 3).
- [x] F7 addressed: P4 step 4 regenerates BOTH v0.1.json and v0.2.json.
- [x] F8: skipped (paranoia-grade noise; BIP table headers disambiguate).
- [x] F9 addressed: P4 step 1 audits md-signer-compat first.

## Open questions for the implementer (carried from review pass 1)

- F-OQ1: Will the Error::*Mismatch rename require a v0.9.x patch if external consumers (mk1?) pattern-match on `Error::PolicyIdMismatch` by name? Grep mk1 before ship; if positive, file as a new mk1 FOLLOWUPS companion before the md1 PR opens.
- F-OQ2: Should `Error::ChunkSetIdMismatch`'s payload field names stay as `expected/got` or switch to `expected_chunk_set_id/got_chunk_set_id`? Decision: keep `expected/got` if the typed payload (`ChunkSetId`) is self-explanatory in rustdoc.
- F-OQ3: Phase 2's corpus addition (testnet xpub for `[fp/48'/1'/0'/1']xpub_TESTNET`) — confirm in P2 step 1 whether existing testnet xpub fixtures cover this, or generate fresh.
