# md-codec v0.15.2 — FOLLOWUPS cleanup Implementation Plan

> **For agentic workers:** This is a tiny patch release working through the
> 7 LOW findings deferred from v0.15.1's spec/plan/per-phase reviews. The
> FOLLOWUPS entries themselves serve as the spec — each entry has a
> `Where:` and `What:` describing the specific change. No separate SPEC
> document is needed at this scale.

**Goal:** Close the v0.15.2-tier entries in `design/FOLLOWUPS.md` from the
v0.15.1 review cycle. Six small fixes + one wont-fix.

**Architecture:** Pure cleanup. No behavior changes; no library API
changes; no wire format changes; no new tests required (the LOWs are
about code shape, redundant attributes, and a doc nit).

**Tech Stack:** unchanged.

---

## Anchored to

- v0.15.2-tier entries in `design/FOLLOWUPS.md` at HEAD on `main` (post
  v0.15.1 merge `dbb8b47`).
- Per-phase review reports under `design/agent-reports/v0.15.1-*-review.md`
  (the source of each entry).
- Workflow per the standing rule
  (`feedback_iterative_review_every_phase.md`): per-phase review;
  reports persist to `design/agent-reports/`; critical/important fixed
  inline; LOWs from this release tracked in FOLLOWUPS under tier
  `v0.15.3` (or held there for review at the next minor cycle).

## v0.15.2 items: classification

| ID | Action | Notes |
|---|---|---|
| `v0.15.1-spec-l2-address-json-arg-row` | **fix-doc** | Add the `--json` row to the SPEC's address arg-semantics table |
| `v0.15.1-phase-1-low-1` | **fix-code** | Drop redundant `#[allow(dead_code)]` inside `#[cfg(test)]` |
| `v0.15.1-phase-2-low-1` | **wont-fix** | `--force-long-code` is intentional forward-compat; long-code mode dropped in v0.12. Document the intent inline and resolve as wont-fix until/unless long-code returns. |
| `v0.15.1-phase-2-low-2` | **fix-code** | Replace `_ =>` wildcard match in `parse_key` with explicit `Testnet \| Signet \| Regtest =>` arm; future bitcoin crate variants would now be a compile error rather than silent testnet routing |
| `v0.15.1-phase-3-low-1` | **fix-code** | Defensive guard at top of `build_descriptor` for the (clap-blocked) zero-arg case |
| `v0.15.1-phase-4-low-1` | **fix-code** | Add `account_xpub_testnet(path)` helper as a one-line wrapper around `account_xpub(path, Network::Testnet)`; mirrors the SPEC text |
| `v0.15.1-phase-5-low-1` | **fix-code** | Add `let _ = args.network_str;` after the existing `let _ = args.json;` for consistency under no-`json`-feature builds |

## File structure

```
crates/md-codec/src/bin/md/parse/keys.rs           # Phase-1-low-1 + Phase-2-low-2
crates/md-codec/src/bin/md/cmd/address.rs          # Phase-3-low-1 + Phase-5-low-1
crates/md-codec/src/bin/md/cmd/encode.rs           # Phase-2-low-1 (comment-only; wont-fix annotation)
crates/md-codec/tests/cmd_address.rs               # Phase-4-low-1
design/SPEC_v0_15_1_address_and_network.md         # Spec-l2 (touch the v0.15.1 spec retroactively)
design/FOLLOWUPS.md                                # Mark each entry resolved or wont-fix
crates/md-codec/Cargo.toml                         # Version bump 0.15.1 → 0.15.2
CHANGELOG.md                                       # [0.15.2] entry
MIGRATION.md                                       # v0.15.1 → v0.15.2 (no migration steps; pure cleanup)
```

## Conventions

- Each fix-code commit message format: `fix(v0.15.2): <followup-id> — <one-line>`.
- Wont-fix gets `chore(v0.15.2): close <followup-id> as wont-fix`.
- After all 7 commits + version bump + docs, one ship-tag commit + per-phase review.
- Per-phase test-and-clippy gate: `cargo test --workspace --features cli,json,cli-compiler` must remain at 365; clippy `-D warnings` must stay clean.

---

## Phase 0 — Pre-flight

- [ ] **Step 1: Confirm baseline**

```bash
cd /scratch/code/shibboleth/descriptor-mnemonic/.worktrees/v0.15.2
cargo test --workspace --features cli,json,cli-compiler 2>&1 | grep -E '^test result' | awk '{ok+=$4} END {print ok}'
# Expect: 365
cargo clippy --workspace --features cli,json,cli-compiler --all-targets -- -D warnings 2>&1 | tail -3
# Expect: clean
```

- [ ] **Step 2: Empty ship-tag commit**

```bash
git commit --allow-empty -m "chore(v0.15.2/phase-0): ship — pre-flight baseline confirmed (365 tests)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Phase 1 — Apply all 7 follow-ups

Single phase because each fix is 1-3 lines and they don't interact.

### Task 1.1: spec-l2 — add `--json` row to address arg table

**File:** `design/SPEC_v0_15_1_address_and_network.md`

- [ ] Find the `### md address arg semantics` table. The last row is `--json` mentioned in synopsis but absent from the table. Add row before the closing of the table:

```markdown
| `--json` | false | Emit JSON output (schema `md-cli/1`). |
```

- [ ] Commit:

```bash
git add design/SPEC_v0_15_1_address_and_network.md
git commit -m "fix(v0.15.2): v0.15.1-spec-l2-address-json-arg-row — add --json row to SPEC table

Cosmetic SPEC consistency fix; the v0.15.1 SPEC's address-args table
already lists every other arg with a row. The --json arg was only
mentioned in the CLI synopsis. No code impact.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

### Task 1.2: phase-1-low-1 — drop redundant `#[allow(dead_code)]`

**File:** `crates/md-codec/src/bin/md/parse/keys.rs`

- [ ] Locate the `ABANDON_TPUB_DEPTH4_BIP48` const inside the `#[cfg(test)]` block. Remove the `#[allow(dead_code)]` line above it. The compiler doesn't see the const outside test builds, so the allow is redundant.

- [ ] Run tests to confirm no regression:

```bash
cargo test --features cli --bin md parse::keys 2>&1 | tail -5
# Expect: 11 passed
```

- [ ] Commit:

```bash
git add crates/md-codec/src/bin/md/parse/keys.rs
git commit -m "fix(v0.15.2): v0.15.1-phase-1-low-1 — drop redundant #[allow(dead_code)] inside #[cfg(test)]

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

### Task 1.3: phase-2-low-2 — explicit Network match arms in `parse_key`

**File:** `crates/md-codec/src/bin/md/parse/keys.rs`

- [ ] Replace the wildcard arm in `parse_key` with an exhaustive match:

```rust
    let (expected_version, network_label) = match network {
        bitcoin::Network::Bitcoin => (MAINNET_XPUB_VERSION, "mainnet"),
        // BIP 32 testnet bytes cover testnet, testnet4, signet, and regtest.
        bitcoin::Network::Testnet
        | bitcoin::Network::Testnet4
        | bitcoin::Network::Signet
        | bitcoin::Network::Regtest => (TESTNET_XPUB_VERSION, "testnet"),
    };
```

bitcoin 0.32.8's `Network` enum has 5 variants (Bitcoin, Testnet,
**Testnet4**, Signet, Regtest) and is NOT `#[non_exhaustive]` (verified
by reading `bitcoin-0.32.8/src/network.rs` lines 70-88). Exhaustive
match makes any future bitcoin crate variant a compile error rather
than silently routing to the testnet path.

Note: `CliNetwork` only exposes 4 of the 5 variants on the CLI surface
(no `Testnet4`), so this arm is unreachable in practice from the CLI
today; it is exercised only if a future change adds `CliNetwork::Testnet4`.

- [ ] Build + test to confirm no regression:

```bash
cargo build --features cli,json --bin md 2>&1 | tail -3
# Expect: clean (no non-exhaustive-match error means current bitcoin 0.32 has exactly these 4 variants)
cargo test --features cli --bin md parse::keys 2>&1 | tail -5
# Expect: 11 passed
```

- [ ] Commit:

```bash
git add crates/md-codec/src/bin/md/parse/keys.rs
git commit -m "fix(v0.15.2): v0.15.1-phase-2-low-2 — explicit Network match in parse_key

Replaces the _ => wildcard with explicit Testnet | Signet | Regtest =>
arm. A future bitcoin crate Network variant (e.g., a hypothetical
'Liquid') would become a compile error rather than silently routing
to the testnet xpub-version path.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

### Task 1.4: phase-3-low-1 — defensive guard in build_descriptor

**File:** `crates/md-codec/src/bin/md/cmd/address.rs`

- [ ] Add the guard at the top of `build_descriptor`, before the `if let Some(template) = args.template` block:

```rust
fn build_descriptor(args: &AddressArgs<'_>) -> Result<Descriptor, CliError> {
    if args.phrases.is_empty() && args.template.is_none() {
        return Err(CliError::BadArg(
            "address requires either positional <STRING>... or --template <T> --key @i=<XPUB>; clap should have caught this — please report a bug".into(),
        ));
    }
    if let Some(template) = args.template {
        ...
    }
    ...
}
```

This is a defense-in-depth check; clap's `ArgGroup::required(true)` is the primary guard. If it ever fails (clap regression, custom invocation bypassing the parser, etc.), the user gets a clear `BadArg` exit 2 instead of `reassemble(&[])` exit 1.

- [ ] Run cmd_address tests; no regression expected (the existing `address_no_input_exits_2` should still hit clap first):

```bash
cargo test --features cli,json --test cmd_address 2>&1 | tail -5
# Expect: 13 passed
```

- [ ] Commit:

```bash
git add crates/md-codec/src/bin/md/cmd/address.rs
git commit -m "fix(v0.15.2): v0.15.1-phase-3-low-1 — defense-in-depth zero-arg guard in build_descriptor

clap's ArgGroup::required(true) is the primary guard. This adds a
runtime check returning CliError::BadArg if it ever fails, so users
get a clear exit-2 message instead of reassemble(&[]) exit-1.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

### Task 1.5: phase-4-low-1 — `account_xpub_testnet` wrapper

**File:** `crates/md-codec/tests/cmd_address.rs`

- [ ] Add the wrapper after `account_xpub`:

```rust
fn account_xpub_testnet(path: &str) -> Xpub { account_xpub(path, Network::Testnet) }
```

- [ ] Update the testnet test to use it:

```rust
    let xpub = account_xpub_testnet("m/84'/1'/0'");
```

(Replacing the existing `let xpub = account_xpub("m/84'/1'/0'", Network::Testnet);` line in `address_testnet_wpkh_receive_0_via_secondary_path`.)

- [ ] Run tests:

```bash
cargo test --features cli,json --test cmd_address 2>&1 | tail -5
# Expect: 13 passed
```

- [ ] Commit:

```bash
git add crates/md-codec/tests/cmd_address.rs
git commit -m "fix(v0.15.2): v0.15.1-phase-4-low-1 — add account_xpub_testnet wrapper

Mirrors the v0.15.1 SPEC text. Cosmetic; functionally identical to the
inlined call.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

### Task 1.6: phase-5-low-1 — `let _ = args.network_str;` suppressor

**File:** `crates/md-codec/src/bin/md/cmd/address.rs`

- [ ] In `cmd::address::run`, find the existing `let _ = args.json;` line (post-JSON-block fallthrough). Add the network_str suppressor next to it:

```rust
    let _ = args.json;
    let _ = args.network_str;
```

- [ ] Build with the json feature OFF to confirm no warning:

```bash
cargo build --no-default-features --features cli --bin md 2>&1 | tail -5
# Expect: clean (no unused-field warning)
```

- [ ] Build with default features back on (this is the standard build):

```bash
cargo build --features cli,json --bin md 2>&1 | tail -3
# Expect: clean
```

- [ ] Commit:

```bash
git add crates/md-codec/src/bin/md/cmd/address.rs
git commit -m "fix(v0.15.2): v0.15.1-phase-5-low-1 — suppressor on args.network_str

Mirrors the existing args.json suppressor for the no-json-feature build
path. Cosmetic.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

### Task 1.7: phase-2-low-1 — annotate `--force-long-code` as wont-fix

**File:** `crates/md-codec/src/bin/md/cmd/encode.rs`

- [ ] Replace the bare `let _ = args.force_long_code;` (around line 60) with:

```rust
    // --force-long-code: long-code mode was dropped in v0.12.0; the flag is
    // accepted for forward-compat (so older scripts don't break) but has no
    // effect. Status: wont-fix at v0.15.2 (FOLLOWUPS v0.15.1-phase-2-low-1).
    // Revisit only if a real long-code mode is reintroduced.
    let _ = args.force_long_code;
```

This makes the design intent visible in the code rather than only in FOLLOWUPS.

- [ ] Commit:

```bash
git add crates/md-codec/src/bin/md/cmd/encode.rs
git commit -m "chore(v0.15.2): close v0.15.1-phase-2-low-1 as wont-fix; annotate intent inline

--force-long-code is forward-compat scaffold only; long-code mode was
dropped in v0.12.0. Resolves the v0.15.2 FOLLOWUPS entry as wont-fix
with the rationale documented at the call site.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

### Task 1.8: Bump crate version

**File:** `crates/md-codec/Cargo.toml`

- [ ] Change `version = "0.15.1"` to `version = "0.15.2"`.

- [ ] Commit:

```bash
git add crates/md-codec/Cargo.toml Cargo.lock
git commit -m "chore(v0.15.2): bump crate version 0.15.1 → 0.15.2

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

### Task 1.9: CHANGELOG and MIGRATION

- [ ] Append `## [0.15.2] — 2026-05-03` to `CHANGELOG.md` after the existing 0.15.1 entry, with a `### Fixed` section listing the 6 closed FOLLOWUPS items and a `### Unchanged` clause.

- [ ] Append a `## v0.15.1 → v0.15.2` section to `MIGRATION.md` (no migration steps; pure cleanup).

- [ ] Commit each separately:

```bash
git add CHANGELOG.md
git commit -m "docs(v0.15.2): CHANGELOG entry

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
git add MIGRATION.md
git commit -m "docs(v0.15.2): MIGRATION entry — v0.15.1 → v0.15.2 (no migration steps)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

### Task 1.10: Update FOLLOWUPS — mark resolved/wont-fix

**File:** `design/FOLLOWUPS.md`

- [ ] For each of the 7 v0.15.2 entries, change `Status: open` to `Status: resolved <commit-sha>` (using `git log --oneline | grep <id>` to recover each fix commit's short SHA), or `Status: wont-fix — long-code dropped in v0.12.0; forward-compat stub` for `phase-2-low-1`.

- [ ] Commit:

```bash
git add design/FOLLOWUPS.md
git commit -m "chore(v0.15.2): close 7 FOLLOWUPS entries (6 resolved + 1 wont-fix)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

### Task 1.11: Phase 1 ship + final test/clippy gate

- [ ] Run the full matrix:

```bash
cargo test --workspace --features cli,json 2>&1 | grep -E '^test result' | awk '{ok+=$4} END {print "no-compiler total ok:", ok}'
cargo test --workspace --features cli,json,cli-compiler 2>&1 | grep -E '^test result' | awk '{ok+=$4} END {print "with-compiler total ok:", ok}'
# Expect: 357 / 365 (unchanged from v0.15.1)
cargo clippy --workspace --features cli,json,cli-compiler --all-targets -- -D warnings 2>&1 | tail -3
# Expect: clean
cargo package --no-verify -p md-codec 2>&1 | tail -3
# Expect: exits 0
```

- [ ] Empty ship-tag commit + release commit:

```bash
git commit --allow-empty -m "chore(v0.15.2/phase-1): ship — 6 FOLLOWUPS cleanups + 1 wont-fix; version bump

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
git commit --allow-empty -m "release: md-codec v0.15.2 — FOLLOWUPS cleanup

Six trivial code-quality cleanups closing v0.15.1's deferred LOW
findings; one entry (--force-long-code) closed as wont-fix with
inline annotation. No behavior changes; library API and wire format
unchanged. Test count unchanged from v0.15.1 (365).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

### Task 1.12: Per-phase review

- [ ] Dispatch `feature-dev:code-reviewer` against the full Phase 1 diff (the seven fixes + version bump + docs + FOLLOWUPS close-out). Save report to `design/agent-reports/v0.15.2-phase-1-review.md`. Address any HIGH/MEDIUM finding inline; LOWs lift into FOLLOWUPS under tier `v0.15.3` for the next minor or patch cycle.

---

## Merge + tag + push (out-of-phase; orchestrator handles after review)

```bash
# In main worktree at /scratch/code/shibboleth/descriptor-mnemonic
git fetch origin main
git merge --no-ff feat/v0.15.2 -m "Merge branch 'feat/v0.15.2' — md-codec v0.15.2 (FOLLOWUPS cleanup)"
cargo test --workspace --features cli,json,cli-compiler 2>&1 | grep -E '^test result' | awk '{ok+=$4} END {print ok}'
# Expect: 365
git tag -a v0.15.2 -m "md-codec v0.15.2 — FOLLOWUPS cleanup (6 resolved + 1 wont-fix)"
git push origin main
git push origin v0.15.2
git worktree remove .worktrees/v0.15.2
git branch -d feat/v0.15.2
```

---

## Verification (end-to-end, post-implementation)

- All 7 v0.15.2 FOLLOWUPS entries have `Status: resolved <SHA>` or `Status: wont-fix — ...`
- `design/agent-reports/v0.15.2-phase-1-review.md` exists and reports PASS
- `cargo test`, `cargo clippy -D warnings`, `cargo package` all clean
- `git tag -l v0.15.2` shows the annotated tag
- `git log origin/main --oneline -3` shows the merge commit at HEAD

## Out of scope

- Functional changes. v0.15.2 is pure cleanup.
- Wallet-id rename, the only HIGH-tier `wallet-id-is-really-template-id` FOLLOWUPS entry — unchanged scope from v0.15.1.
- Any new tests beyond what existing tests cover (the cleanups don't change behavior).

## Self-review

Spec coverage: every v0.15.2 FOLLOWUPS entry maps to a Task 1.N. Wont-fix
case has its own task. ✓

Placeholder scan: no `TBD`/`TODO` outside the `<commit-sha>` placeholders in
Task 1.10 (resolved at execution time). ✓

Type consistency: all changes are scoped to existing files; no new types
introduced. ✓
