# PLAN — md-codec encoder `k ≤ n` gate (`encode-accepts-k-greater-than-n`)

**Date:** 2026-06-12 · **Crate:** `md-codec` · **SemVer:** PATCH `0.35.1 → 0.35.2`
**Source SHAs (citations grounded against these):** descriptor-mnemonic `origin/main` = `31e5895`; companion mnemonic-toolkit `origin/master` = `6899670` (recon was `dbdacfb`; one docs-only commit since, no code drift).
**FOLLOWUP:** `encode-accepts-k-greater-than-n` — md-codec PRIMARY (`design/FOLLOWUPS.md`), mnemonic-toolkit COMPANION. Surfaced 2026-06-11 (stress Cycle B P8). Cycle-prep recon: `mnemonic-toolkit/cycle-prep-recon-encode-accepts-k-greater-than-n.md`.

---

## 1. Problem (grounded)

`md-codec`'s encoder and decoder are **asymmetric** on the `k ≤ n` invariant of k-of-n threshold/multi bodies:

- **Decode REJECTS k > n** — `src/tree.rs:229-231` (`Tag::Multi | SortedMulti | MultiA | SortedMultiA` arm) and `:241-243` (`Tag::Thresh` arm) both do `if k as usize > count { return Err(Error::KGreaterThanN { k, n: count }); }`.
- **Encode ACCEPTS k > n** — `write_node` (`src/tree.rs:79`) gates only the `1..=32` ranges, never `k ≤ n`:
  - `Body::Variable { k, children }` arm (`:90-104`): checks `1..=32` for `k` (→ `ThresholdOutOfRange`, `:92-94`) and for `children.len()` (→ `ChildCountOutOfRange`, `:95-99`), then writes bits. **No `k ≤ n` check.**
  - `Body::MultiKeys { k, indices }` arm (`:106-121`): the identical `1..=32` pair (`:108-114`), then writes bits. **No `k ≤ n` check.**

**Consequence:** all three public encode doors transit the same ungated `write_node` — `encode_payload` (`src/encode.rs:88`), `encode_md1_string` (`src/encode.rs:115`), and `chunk::split` → `encode_payload` (`src/chunk.rs:240`). So e.g. `wsh(multi(3,@0,@1))` (k=3, n=2) **encodes to a real md1 card that no decoder will ever read back** (`KGreaterThanN` at decode). This is an *engrave-but-can't-restore* gap — same family as the Cycle-A `bundle-accepts-sortedmulti-in-combinator-restore-cannot`.

The pinning characterization cell already exists and is RED-armed: `tests/proptest_to_miniscript.rs:654` `p8_encode_accepts_k_greater_than_n_decode_rejects` asserts encode-Ok + decode-Err today, with a doc-comment (`:642-652`) that pre-declares the flip protocol ("If this cell starts failing because encode_payload begins REJECTING k > n, the gap was closed: resolve the FOLLOWUP and invert this cell").

## 2. The fix

Add a `k ≤ n` gate to **both** `write_node` arms, **after** the existing `1..=32` range checks (so an out-of-range `k` still reports `ThresholdOutOfRange`, not `KGreaterThanN`), reusing the existing variant `Error::KGreaterThanN { k: u8, n: usize }` (`src/error.rs:108`, `#[error("threshold k={k} exceeds child count n={n}; require k ≤ n")]`). This is the exact mirror of the decode-side rejects.

`Body::Variable` arm — insert after `:99`, before `:100` (`w.write_bits((*k - 1)…)`):
```rust
if *k as usize > children.len() {
    return Err(Error::KGreaterThanN { k: *k, n: children.len() });
}
```
`Body::MultiKeys` arm — the `1..=32` range pair spans `:108-115` (the `ChildCountOutOfRange` if-block closes at `:115`); insert **after `:115`, before `:116`** (R0 M1 — the original `:114`/`:116` cite was off-by-one; the gate goes after the closing brace, not inside the if-block):
```rust
if *k as usize > indices.len() {
    return Err(Error::KGreaterThanN { k: *k, n: indices.len() });
}
```

~8 LOC. No new error variant. No wire-format change (see §4).

### Scope decision: `write_node` ONLY (not `validate.rs`)
`src/validate.rs::validate_placeholder_usage` (`:62`/`:67`) also walks `Body::Variable`/`Body::MultiKeys` but checks a *different* invariant (placeholder usage), not k/n. **Recommendation: gate in `write_node` only** — it is the single wire-emit chokepoint covering all three encode doors and is the exact symmetric mirror of the decode-side reads. Adding a second copy in `validate.rs` would be defense-in-depth for a hypothetical caller that runs `validate()` but never encodes, at the cost of a second source of truth that can drift from the decode rejects. **R0 to confirm** write_node-only vs. also-validate.rs. (Default: write_node-only.)

## 3. TDD test plan (RED before impl)

All in `tests/proptest_to_miniscript.rs` using `tests/common/mod.rs` helpers (`wrap` `:31`, `multikeys` `:43`, `thresh_node` `:61`, `keyarg` `:37`, `descriptor_from_tree` `:234`).

1. **Invert the pre-wired cell** — rename `p8_encode_accepts_k_greater_than_n_decode_rejects` → `p8_encode_rejects_k_greater_than_n` and flip it: `encode_payload(&d)` must now return `Err(KGreaterThanN { k: 3, n: 2 })` (was `.expect(...)` Ok); `encode_md1_string(&d)` likewise `Err`. (MultiKeys arm; the loud Cycle-B characterization cell, resolving the FOLLOWUP.) Update the doc-comment to record the gate landed.
2. **NEW — Variable/Thresh arm reject:** `thresh_node(3, vec![keyarg(Tag::PkK,0), keyarg(Tag::PkK,1)])` wrapped via `wrap(Tag::Wsh, …)` + `descriptor_from_tree` → `encode_payload` must `Err(KGreaterThanN { k: 3, n: 2 })`. (The MultiKeys cell does NOT exercise the `Body::Variable` arm — both arms need a red-first cell.)
3. **NEW — boundary k = n encodes OK (both arms):** k=2-of-n=2 multi AND k=2-of-n=2 thresh both `encode_payload(...).is_ok()` — proves the gate is `>` not `>=` and does not over-reject the valid equal case. (Guards the off-by-one.)

RED proof: cells 1-2 fail against current `31e5895` (encode currently succeeds); all pass after the gate. Cell 3 passes both before and after (valid input unaffected) — it pins that the fix doesn't regress the boundary.

GREEN gate: full `cargo test -p md-codec` (the existing round-trip/wire proptests only generate k ≤ n, so none regress) + `cargo clippy --all-targets -- -D warnings` + `cargo fmt --all --check` (R0 M2 — confirmed: `.github/workflows/ci.yml:49-57` runs a dedicated fmt-check job, so this is unconditional).

**Anti-regression backstop (R0 M5):** the gate-after-range ordering is ALREADY test-pinned — `p8_encode_rejects_out_of_range_multi_k` / `p8_encode_rejects_out_of_range_thresh_k` (`proptest_to_miniscript.rs:531-569`) feed inputs that are out-of-range AND k>n, asserting `ThresholdOutOfRange`. If the new gate were placed before the range checks, both cells would flip to `KGreaterThanN` and go red. So the existing suite enforces the diagnostic precedence; no new test needed for it.

## 4. Wire-compat / SemVer

**PATCH (`0.35.1 → 0.35.2`).** The gate only NARROWS the encode domain; every payload it newly rejects was *already undecodable* (`KGreaterThanN` at decode), so **no valid wire bytes change and no existing decodable card is affected**. No public type/signature change (the error variant already exists). This is a bugfix: rejecting input that produced unreadable output. Mirrors the `0.35.1` PATCH framing (error→success there; success→error here, but in both cases no *valid*-card output changes).

## 5. Release ritual (md-codec 0.35.2)

1. `crates/md-codec/Cargo.toml` version `0.35.1 → 0.35.2`.
2. `crates/md-cli/Cargo.toml:28` exact pin `md-codec = { path = "../md-codec", version = "=0.35.1" }` → `=0.35.2` (md-cli is the in-repo consumer; exact-pinned, so it bumps in lockstep — **md-cli's own version stays unchanged**, per the ms-cli `=0.4.1/2/3` precedent: a dependency-pin bump in the git-tag-only bin crate is not its own release).
3. Root `CHANGELOG.md` — new `## md-codec [0.35.2] — 2026-06-12` PATCH entry (Fixed: the `encode-accepts-k-greater-than-n` gate; note the symmetry with decode + the no-valid-card-change wire-compat statement). **R0 M4:** add one line noting **md-cli's template encode path inherits the refusal** — `md-cli` lexes `multi(k,@…)` itself (not via rust-miniscript), so a k>n template input now errors with `KGreaterThanN` instead of emitting an unrestorable card (the fix working; no md-cli fixture relies on k>n — R0 Q4 verified clean).
4. `Cargo.lock` refresh (`cargo update -p md-codec` within the repo or it updates on build).
5. Flip the md-codec `design/FOLLOWUPS.md::encode-accepts-k-greater-than-n` Status → resolved (cite 0.35.2).
6. Commit (stage explicit paths) + tag `md-codec-v0.35.2`. **R0 M3:** 0.35.1 (`762a4f8`) was published crates.io-only and left UNTAGGED — but that breaks a ~30-tag convention (every release back to v0.3.0 is tagged) and reads as an oversight, not policy. So tag 0.35.2 per the dominant convention; **surface to the user at release time** whether to also back-tag `md-codec-v0.35.1` at `762a4f8` in the same push.
7. **STOP — `cargo publish -p md-codec` is a user-authorized, irreversible crates.io action.** Surface for explicit authorization; do NOT publish autonomously.
8. **Toolkit tail (after publish, separate):** `cargo update -p md-codec` in mnemonic-toolkit (the `md-codec = "0.35"` req at `crates/mnemonic-toolkit/Cargo.toml:36` is semver-compatible → lockfile-only, no Cargo.toml edit), run the full toolkit suite (expected GREEN — verified no toolkit golden/vector can embed a k>n card; both toolkit emit paths reject k>n before reaching md-codec), resolve the toolkit companion FOLLOWUP. Likely a NO-BUMP/lockfile chore toolkit-side, or folded into the next toolkit release train.

## 6. Lockstep / no-op checks
- **No clap surface change** anywhere (pure library validation) → **no manual-mirror, no GUI `schema_mirror`, no quickstart** lockstep.
- **md-cli `--help` unchanged** → no `docs/manual/src/40-cli-reference/42-md.md` update.
- **Toolkit is NOT exposed in the interim** (recon-verified at `dbdacfb`: `pre_check_threshold` `bundle_unified.rs:67-90` + `synthesize_unified` reject `synthesize.rs:790-794` + 2 more synthesize gates + rust-miniscript `Threshold::new` k≤n on the intake path). So there is no urgency forcing the toolkit pin-bump ahead of the publish; the library gate is the durable fix (toolkit's hand-maintained gates can drift, the library invariant cannot).

## 7. R0 questions
1. **write_node-only vs. also validate.rs?** (Default recommendation: write_node-only — single chokepoint, symmetric with decode. Confirm.)
2. **Gate placement** — after the `1..=32` range checks (so out-of-range `k` reports `ThresholdOutOfRange`, not `KGreaterThanN`). Confirm this ordering is the desired diagnostic precedence (matches decode, where 5-bit reads force `1..=32` before the k>n check).
3. **Test count** — is the 3-cell set (invert P8 + thresh-arm reject + k=n boundary-OK both arms) sufficient, or also add a property (random valid k≤n always encodes; random k>n always `KGreaterThanN`)? (Lean: the 3 deterministic cells + the existing round-trip proptests suffice; a dedicated k>n property is optional.)
4. **md-cli surface** — confirm md-cli has no command that would now surface a NEW error path to the user (it shouldn't — md-cli builds Nodes from parsed/validated descriptors, same as the toolkit; but verify no md-cli golden/test asserts a k>n card encodes).

## 8. Risks
- **Low.** ~8 LOC mirroring existing decode logic; reuses an existing error variant; narrows encode to the already-decodable domain. The one off-by-one risk (`>` vs `>=`) is pinned by the k=n boundary-OK cell. The md-cli exact-pin lockstep is the only release-ritual gotcha (forgetting it breaks the in-repo build).
