# md-codec v0.15.1 — `md address` + `--network` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the `md address` subcommand wrapping the existing
`Descriptor::derive_address` library API, plus `--network mainnet|testnet|signet|regtest`
on `encode`/`verify`/`address` to support testnet xpubs and address rendering.

**Architecture:** Pure-additive CLI patch. The library API and wire format
do not change. Network only matters at the CLI boundary in two places:
(a) xpub-version validation in `parse_key` and (b) the final
`Address::p2*(_, network)` HRP/version selection inside `derive_address`.

**Tech Stack:** Rust 2024 (MSRV 1.85), clap 4.5 (with `derive` + `ValueEnum`),
bitcoin 0.32 (`Network` type, `Address`), miniscript 13.0.0 (already pinned),
insta (snapshot tests, already pinned). No new dependencies.

---

## Anchored to

- SPEC: `design/SPEC_v0_15_1_address_and_network.md` at HEAD (post r1+r2 review).
- Workflow: `~/.claude/plans/a-the-wallet-descriptor-quizzical-ladybug.md`
  (post-review, ephemeral).
- Standing rule: per-phase iterative agent review with reports persisted to
  `design/agent-reports/` and nits deferred to `design/FOLLOWUPS.md` tier
  `v0.15.2`.

## File structure

```
crates/md-codec/
├── Cargo.toml                                     # version bump 0.15.0 → 0.15.1
└── src/bin/md/
    ├── main.rs                                    # add CliNetwork ValueEnum, --network on Encode/Verify/Address, Address variant + dispatch
    ├── parse/
    │   └── keys.rs                                # add TESTNET_XPUB_VERSION; parse_key takes Network
    └── cmd/
        ├── mod.rs                                 # pub mod address;
        ├── encode.rs                              # EncodeArgs.network/network_str; threaded to parse_key + JSON
        ├── verify.rs                              # VerifyArgs.network; threaded to parse_key
        └── address.rs                             # NEW — md address subcommand impl
crates/md-codec/tests/
├── cmd_address.rs                                 # NEW — golden vectors at CLI boundary
├── cmd_encode.rs                                  # add encode_json_includes_network_field
└── snapshots/                                     # NEW snapshots for address --json
docs/json-schema-v1.md                             # add address --json section + network row on encode --json
crates/md-codec/README.md                          # add md address row to CLI table; quickstart
CHANGELOG.md                                       # [0.15.1] entry
MIGRATION.md                                       # v0.15.0 → v0.15.1 note (additive)
design/agent-reports/v0.15.1-phase-N-review.md     # per-phase review reports
design/FOLLOWUPS.md                                # add v0.15.2 tier definition; append low/nit findings at step 8
```

## Conventions used in this plan

- **TDD ordering:** Each task writes the failing test first, runs it to confirm failure, writes minimal impl, runs again to confirm pass, commits.
- **Run-test command (default):** `cargo test --workspace --features cli,json` unless a step says otherwise. Tests gated on the compiler feature use `cargo test --workspace --features cli,json,cli-compiler`.
- **Commit message prefix:** `feat(v0.15.1/phase-N): <subject>` for code; `test(v0.15.1/phase-N): ...` for test-only commits; `docs(v0.15.1): ...` for docs.
- **Per-phase ship tag:** Each phase ends with an empty commit `chore(v0.15.1/phase-N): ship`.
- **Don't `git add -A`:** stage paths explicitly (root has untracked local helpers per project memory).
- **Co-author trailer:** every commit ends with the standard trailer.
- **Per-phase review:** after the ship tag commit of each phase, dispatch `feature-dev:code-reviewer` against the phase's commits. Save its report to `design/agent-reports/v0.15.1-phase-N-review.md`. Address every CRITICAL and IMPORTANT (or HIGH/MEDIUM) finding inline in this session before starting the next phase. Append every LOW / NIT to the in-session followups list (see Phase 6 step 6 for the FOLLOWUPS.md merge).

---

## Phase 0 — Pre-flight (worktree exists; just baseline confirmation)

Phase goal: confirm worktree state matches expectations and tag a Phase 0 ship marker so per-phase commits have a clean predecessor.

### Task 0.1: Confirm baseline tests pass

- [ ] **Step 1: Confirm SPEC + reviews are committed**

```bash
cd /scratch/code/shibboleth/descriptor-mnemonic/.worktrees/v0.15.1
git log --oneline -5
# Expect last 3 commits: c5368ac (r2), c937b9f (r1 fixes), 3b6a391 (SPEC)
```

- [ ] **Step 2: Confirm baseline tests pass**

```bash
cargo test --workspace --features cli,json,cli-compiler 2>&1 | grep -E '^test result' | awk '{ok+=$4} END {print "Total ok:", ok}'
# Expect: Total ok: 340
```

- [ ] **Step 3: Confirm clippy clean**

```bash
cargo clippy --workspace --features cli,json,cli-compiler --all-targets -- -D warnings 2>&1 | tail -3
# Expect: Finished `dev` profile, no errors
```

### Task 0.2: Phase 0 ship tag

- [ ] **Step 1: Empty ship-tag commit**

```bash
git commit --allow-empty -m "$(cat <<'EOF'
chore(v0.15.1/phase-0): ship — pre-flight baseline confirmed (340 tests, clippy clean)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 1 — `parse_key` network plumbing

Phase goal: `parse_key` accepts a `bitcoin::Network` arg and routes the version-byte check between mainnet xpub and testnet/signet/regtest tpub. Existing call sites pass `Network::Bitcoin` so encode/verify behavior is unchanged externally.

> **Test-count note:** SPEC §Testing estimated +3 parse/keys tests; this phase ships +5 because we cover signet and regtest acceptance explicitly (BIP 32 uses the same testnet version bytes for all three, but routing tests document the contract).

### Task 1.1: Add testnet xpub fixture helper to a shared test module

The testnet xpub used in fixtures is derived from the abandon-mnemonic at `m/84'/1'/0'` (BIP 84 testnet account). Computing it once and pinning the literal string keeps tests fast and offline.

**Files:**
- Modify: `crates/md-codec/src/bin/md/parse/keys.rs` (test module)

- [ ] **Step 1: Derive the abandon-mnemonic testnet xpub once**

Run a one-off test that prints the value, then capture it:

```bash
cd /scratch/code/shibboleth/descriptor-mnemonic/.worktrees/v0.15.1
cat > /tmp/derive_tpub.rs <<'EOF'
use bitcoin::{Network, bip32::{DerivationPath, Xpriv, Xpub}, secp256k1::Secp256k1};
use std::str::FromStr;
fn main() {
    let mn = bip39::Mnemonic::parse("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about").unwrap();
    let seed = mn.to_seed("");
    let secp = Secp256k1::new();
    let master = Xpriv::new_master(Network::Testnet, &seed).unwrap();
    let path = DerivationPath::from_str("m/84'/1'/0'").unwrap();
    let xpriv = master.derive_priv(&secp, &path).unwrap();
    let xpub = Xpub::from_priv(&secp, &xpriv);
    println!("{}", xpub);
}
EOF
# Use a throwaway test in the codebase to print it (we have bip39+bitcoin available).
```

Actually, derive it inline by running this throwaway test (drop after capture):

Append to `crates/md-codec/src/bin/md/parse/keys.rs`:

```rust
#[cfg(test)]
#[test]
#[ignore]
fn derive_abandon_tpub_for_fixtures() {
    use bitcoin::{Network, bip32::{DerivationPath, Xpriv, Xpub}, secp256k1::Secp256k1};
    use std::str::FromStr;
    let mn = bip39::Mnemonic::parse("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about").unwrap();
    let seed = mn.to_seed("");
    let secp = Secp256k1::new();
    let master = Xpriv::new_master(Network::Testnet, &seed).unwrap();
    for p in ["m/84'/1'/0'", "m/48'/1'/0'/2'"] {
        let xpriv = master.derive_priv(&secp, &DerivationPath::from_str(p).unwrap()).unwrap();
        let xpub = Xpub::from_priv(&secp, &xpriv);
        eprintln!("{p}: {}", xpub);
    }
}
```

Run it:

```bash
cargo test --features cli --bin md derive_abandon_tpub_for_fixtures -- --ignored --nocapture 2>&1 | grep "m/8"
# Capture both stdout values. Then DELETE the throwaway test before committing.
```

- [ ] **Step 2: Capture the derived strings into stable consts at the top of the keys.rs test module**

Replace the throwaway test with two `pub(crate) const` values:

```rust
#[cfg(test)]
pub(crate) const ABANDON_TPUB_DEPTH3_BIP84: &str = "<paste m/84'/1'/0' tpub here>";
#[cfg(test)]
pub(crate) const ABANDON_TPUB_DEPTH4_BIP48: &str = "<paste m/48'/1'/0'/2' tpub here>";
```

(Place them just above the existing `#[cfg(test)] mod tests { ... }` block so they're scoped to test compilation but visible to other test modules in the bin via `crate::parse::keys::ABANDON_TPUB_*`.)

- [ ] **Step 3: Commit fixture consts**

```bash
git add crates/md-codec/src/bin/md/parse/keys.rs
git commit -m "$(cat <<'EOF'
test(v0.15.1/phase-1): pin abandon-mnemonic tpubs at m/84'/1'/0' and m/48'/1'/0'/2'

Captured via a one-off ignored test (since deleted) running the
abandon-mnemonic through bitcoin::bip32::Xpriv/Xpub with Network::Testnet.
Used by Phase 1+ network-routing tests and Phase 4 testnet golden vectors.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 1.2: Thread `Network` through `parse_key` (TDD)

**Files:**
- Modify: `crates/md-codec/src/bin/md/parse/keys.rs`

- [ ] **Step 1: Write the new failing tests for network routing**

Append to the `#[cfg(test)] mod tests` block in `parse/keys.rs`:

```rust
    use bitcoin::Network;

    #[test]
    fn accepts_tpub_under_testnet() {
        let p = parse_key(format!("@0={ABANDON_TPUB_DEPTH3_BIP84}").as_str(),
                          ScriptCtx::SingleSig, Network::Testnet).unwrap();
        assert_eq!(p.i, 0);
        assert_eq!(p.payload.len(), 65);
    }

    #[test]
    fn accepts_tpub_under_signet() {
        // Signet uses the same testnet version bytes per BIP 32.
        let p = parse_key(format!("@0={ABANDON_TPUB_DEPTH3_BIP84}").as_str(),
                          ScriptCtx::SingleSig, Network::Signet).unwrap();
        assert_eq!(p.i, 0);
    }

    #[test]
    fn accepts_tpub_under_regtest() {
        let p = parse_key(format!("@0={ABANDON_TPUB_DEPTH3_BIP84}").as_str(),
                          ScriptCtx::SingleSig, Network::Regtest).unwrap();
        assert_eq!(p.i, 0);
    }

    #[test]
    fn rejects_xpub_under_testnet() {
        let err = parse_key(format!("@0={XPUB_DEPTH4}").as_str(),
                            ScriptCtx::MultiSig, Network::Testnet).unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("expected testnet"), "got: {msg}");
    }

    #[test]
    fn rejects_tpub_under_mainnet() {
        let err = parse_key(format!("@0={ABANDON_TPUB_DEPTH3_BIP84}").as_str(),
                            ScriptCtx::SingleSig, Network::Bitcoin).unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("expected mainnet"), "got: {msg}");
    }
```

- [ ] **Step 2: Update existing tests to thread `Network::Bitcoin`**

In the same test module, every existing `parse_key(..., ScriptCtx::*)` call gains a third arg `Network::Bitcoin`. Find them all:

```bash
grep -n "parse_key(" crates/md-codec/src/bin/md/parse/keys.rs
```

There are 6 existing call sites in the test module; update each to add `, bitcoin::Network::Bitcoin` as the third argument. (Add `use bitcoin::Network;` at the top of the test module if not already present; the new tests rely on it.)

- [ ] **Step 3: Run tests; expect compile failure**

```bash
cargo test --features cli --bin md parse::keys 2>&1 | tail -10
# Expect: error[E0061] — parse_key takes 2 arguments, but 3 were supplied / etc.
```

- [ ] **Step 4: Add `TESTNET_XPUB_VERSION` constant + thread `Network` into `parse_key`**

In `crates/md-codec/src/bin/md/parse/keys.rs`, replace lines 4-5 and the `parse_key` signature/body:

```rust
const XPUB_LEN: usize = 78;
pub(crate) const MAINNET_XPUB_VERSION: [u8; 4] = [0x04, 0x88, 0xB2, 0x1E];
pub(crate) const TESTNET_XPUB_VERSION: [u8; 4] = [0x04, 0x35, 0x87, 0xCF];
```

Then change `parse_key`:

```rust
pub fn parse_key(arg: &str, ctx: ScriptCtx, network: bitcoin::Network) -> Result<ParsedKey, CliError> {
    let (i_str, xpub_str) = arg.split_once('=').ok_or_else(|| CliError::BadArg(
        format!("--key expects @i=XPUB, got: {arg}")
    ))?;
    let i = parse_index(i_str)?;
    let bytes = base58::decode_check(xpub_str)
        .map_err(|e| CliError::BadXpub { i, why: format!("base58check decode: {e}") })?;
    if bytes.len() != XPUB_LEN {
        return Err(CliError::BadXpub { i, why: format!("expected 78 bytes, got {}", bytes.len()) });
    }
    let (expected_version, network_label) = match network {
        bitcoin::Network::Bitcoin => (MAINNET_XPUB_VERSION, "mainnet"),
        // Testnet, Signet, Regtest all use the same testnet version bytes per BIP 32.
        _ => (TESTNET_XPUB_VERSION, "testnet"),
    };
    if bytes[0..4] != expected_version {
        return Err(CliError::BadXpub { i, why: format!(
            "expected {network_label} xpub version {:02X}{:02X}{:02X}{:02X}, got {:02X}{:02X}{:02X}{:02X}",
            expected_version[0], expected_version[1], expected_version[2], expected_version[3],
            bytes[0], bytes[1], bytes[2], bytes[3]
        )});
    }
    let depth = bytes[4];
    let expected_depth = match ctx { ScriptCtx::SingleSig => 3, ScriptCtx::MultiSig => 4 };
    if depth != expected_depth {
        return Err(CliError::BadXpub { i, why: format!(
            "expected depth {expected_depth} for this script context, got {depth}"
        )});
    }
    let mut payload = [0u8; 65];
    payload.copy_from_slice(&bytes[13..78]);
    Ok(ParsedKey { i, payload })
}
```

- [ ] **Step 5: Run tests; expect pass for parse::keys; encode/verify call sites still failing**

```bash
cargo test --features cli --bin md parse::keys 2>&1 | tail -5
# Expect: 11 passed (6 existing + 5 new), 0 failed
cargo build --features cli,json --bin md 2>&1 | tail -10
# Expect: errors at cmd/encode.rs and cmd/verify.rs — they call parse_key with 2 args
```

- [ ] **Step 6: Commit (build still broken at encode/verify; that's fine — Task 1.3 fixes them)**

```bash
git add crates/md-codec/src/bin/md/parse/keys.rs
git commit -m "$(cat <<'EOF'
feat(v0.15.1/phase-1): parse_key takes bitcoin::Network; routes xpub vs tpub

Adds TESTNET_XPUB_VERSION (BIP 32 0x043587CF; same constant covers
testnet, signet, and regtest). parse_key validates the right version
byte for the given Network. Existing tests re-pinned to
Network::Bitcoin; new tests cover the 5 routing cases (accept tpub
under testnet/signet/regtest, reject xpub under testnet, reject tpub
under mainnet).

Encode/verify call sites broken by this commit; fixed in next commit.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 1.3: Patch encode/verify call sites to pass `Network::Bitcoin` (no behavior change)

Phase 2 will replace the hardcoded `Network::Bitcoin` with the user's `--network` choice; for now, restoring the build is the goal.

**Files:**
- Modify: `crates/md-codec/src/bin/md/cmd/encode.rs`
- Modify: `crates/md-codec/src/bin/md/cmd/verify.rs`

- [ ] **Step 1: Update encode**

Edit `crates/md-codec/src/bin/md/cmd/encode.rs` line 22, replace:

```rust
    let parsed_keys = args.keys.iter().map(|k| parse_key(k, ctx)).collect::<Result<Vec<_>, _>>()?;
```

with:

```rust
    let parsed_keys = args.keys.iter().map(|k| parse_key(k, ctx, bitcoin::Network::Bitcoin)).collect::<Result<Vec<_>, _>>()?;
```

- [ ] **Step 2: Update verify**

Edit `crates/md-codec/src/bin/md/cmd/verify.rs` similarly. Find the `parse_key` call (one site) and add `, bitcoin::Network::Bitcoin` as the third arg.

- [ ] **Step 3: Build and run all tests**

```bash
cargo build --features cli,json --bin md 2>&1 | tail -3
# Expect: clean build
cargo test --workspace --features cli,json,cli-compiler 2>&1 | grep -E '^test result' | awk '{ok+=$4} END {print "Total ok:", ok}'
# Expect: Total ok: 345 (340 baseline + 5 new parse::keys network-routing tests)
```

- [ ] **Step 4: Commit**

```bash
git add crates/md-codec/src/bin/md/cmd/encode.rs crates/md-codec/src/bin/md/cmd/verify.rs
git commit -m "$(cat <<'EOF'
feat(v0.15.1/phase-1): pass Network::Bitcoin into parse_key from encode/verify

Restores the build; behavior unchanged. Phase 2 replaces the hardcoded
mainnet with the user's --network choice.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 1.4: Phase 1 ship tag

- [ ] **Step 1: Confirm test count**

```bash
cargo test --workspace --features cli,json,cli-compiler 2>&1 | grep -E '^test result' | awk '{ok+=$4} END {print "Total ok:", ok}'
# Expect: Total ok: 345
cargo clippy --workspace --features cli,json,cli-compiler --all-targets -- -D warnings 2>&1 | tail -3
# Expect: clean
```

- [ ] **Step 2: Empty ship-tag commit**

```bash
git commit --allow-empty -m "$(cat <<'EOF'
chore(v0.15.1/phase-1): ship — parse_key takes Network; tpub accepted under testnet/signet/regtest

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 1.5: Per-phase review

- [ ] **Step 1: Dispatch `feature-dev:code-reviewer` against Phase 1 commits**

Per the standing rule. Save the report to
`design/agent-reports/v0.15.1-phase-1-review.md`. Address every
HIGH/MEDIUM finding inline before starting Phase 2; queue every LOW
finding for the FOLLOWUPS append at Phase 6 step 6.

---

## Phase 2 — `--network` on encode/verify + `encode --json` network field

Phase goal: `md encode --network testnet --key @0=tpub...` and `md verify --network testnet ...` both work end-to-end. `encode --json` always emits a top-level `network` string field using CLI vocabulary.

### Task 2.1: Define `CliNetwork` ValueEnum + helper in main.rs

**Files:**
- Modify: `crates/md-codec/src/bin/md/main.rs`

- [ ] **Step 1: Add the ValueEnum + From + display helper**

Add this block in `main.rs` after the `mod` declarations and the `use clap::...` line, before `struct Cli`:

```rust
/// CLI-facing network selector. Maps to `bitcoin::Network`.
#[derive(Copy, Clone, Debug, clap::ValueEnum)]
enum CliNetwork {
    Mainnet,
    Testnet,
    Signet,
    Regtest,
}

impl From<CliNetwork> for bitcoin::Network {
    fn from(n: CliNetwork) -> Self {
        match n {
            CliNetwork::Mainnet => bitcoin::Network::Bitcoin,
            CliNetwork::Testnet => bitcoin::Network::Testnet,
            CliNetwork::Signet  => bitcoin::Network::Signet,
            CliNetwork::Regtest => bitcoin::Network::Regtest,
        }
    }
}

impl CliNetwork {
    /// Stable kebab-cased name for JSON output. Matches the clap
    /// `value_enum` rendering, NOT `bitcoin::Network::Display` (which
    /// emits "bitcoin" for mainnet — confusing for JSON consumers).
    fn as_str(self) -> &'static str {
        match self {
            CliNetwork::Mainnet => "mainnet",
            CliNetwork::Testnet => "testnet",
            CliNetwork::Signet  => "signet",
            CliNetwork::Regtest => "regtest",
        }
    }
}
```

- [ ] **Step 2: Build (just type-check this addition)**

```bash
cargo build --features cli,json --bin md 2>&1 | tail -3
# Expect: clean (CliNetwork is unused at this point — `dead_code` warning is fine; we'll use it in Task 2.2)
```

- [ ] **Step 3: Commit (intermediate; the unused-warning is transient)**

```bash
git add crates/md-codec/src/bin/md/main.rs
git commit -m "$(cat <<'EOF'
feat(v0.15.1/phase-2): CliNetwork ValueEnum + bitcoin::Network mapping + as_str helper

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 2.2: Wire `--network` into Encode and Verify variants + dispatch

**Files:**
- Modify: `crates/md-codec/src/bin/md/main.rs`
- Modify: `crates/md-codec/src/bin/md/cmd/encode.rs`
- Modify: `crates/md-codec/src/bin/md/cmd/verify.rs`

- [ ] **Step 1: Update `EncodeArgs` to carry network info**

Replace the `EncodeArgs` struct in `crates/md-codec/src/bin/md/cmd/encode.rs` lines 10-18:

```rust
pub struct EncodeArgs<'a> {
    pub template: &'a str,
    pub keys: &'a [String],
    pub fingerprints: &'a [String],
    pub network: bitcoin::Network,
    pub network_str: &'static str,
    pub force_chunked: bool,
    pub force_long_code: bool,
    pub policy_id_fingerprint: bool,
    pub json: bool,
}
```

Update the call to `parse_key` at line 22:

```rust
    let parsed_keys = args.keys.iter().map(|k| parse_key(k, ctx, args.network)).collect::<Result<Vec<_>, _>>()?;
```

In the JSON branch (lines 26-45), insert the `network` field as the second key (alphabetical order via BTreeMap will land it after `chunk_set_id` / `chunks` but before `phrase` / `policy_id_fingerprint` / `schema` — that's fine, the SPEC says "always present", not "first"):

```rust
    #[cfg(feature = "json")]
    if args.json {
        use crate::format::json::SCHEMA;
        let mut obj = serde_json::Map::new();
        obj.insert("schema".into(), SCHEMA.into());
        obj.insert("network".into(), args.network_str.into());
        if args.force_chunked {
            // ... unchanged
```

- [ ] **Step 2: Update `VerifyArgs` similarly (no JSON)**

Edit `crates/md-codec/src/bin/md/cmd/verify.rs`. Add `network: bitcoin::Network` to `VerifyArgs`. Update the `parse_key` call.

- [ ] **Step 3: Add `--network` to Encode and Verify variants in main.rs**

In `crates/md-codec/src/bin/md/main.rs` `enum Command::Encode {...}`, add this arg before `force_chunked`:

```rust
        /// Network for xpub validation (and JSON output labeling).
        #[arg(long, value_enum, default_value_t = CliNetwork::Mainnet)]
        network: CliNetwork,
```

Same arg in `Command::Verify {...}`. Place after `fingerprints`.

In `dispatch` for `Command::Encode`, destructure `network` and pass it through:

```rust
        Command::Encode {
            template, from_policy, context, path: _,
            keys, fingerprints, network, force_chunked, force_long_code,
            policy_id_fingerprint, json,
        } => {
            // ... template_str resolution unchanged ...
            cmd::encode::run(cmd::encode::EncodeArgs {
                template: &template_str, keys: &keys, fingerprints: &fingerprints,
                network: network.into(), network_str: network.as_str(),
                force_chunked, force_long_code, policy_id_fingerprint, json,
            })
        }
```

For `Command::Verify`:

```rust
        Command::Verify { strings, template, keys, fingerprints, network } => cmd::verify::run(cmd::verify::VerifyArgs {
            strings: &strings,
            template: &template,
            keys: &keys,
            fingerprints: &fingerprints,
            network: network.into(),
        }),
```

- [ ] **Step 4: Build and run existing tests (no behavior change for default mainnet)**

```bash
cargo build --features cli,json --bin md 2>&1 | tail -3
cargo test --workspace --features cli,json,cli-compiler 2>&1 | grep -E '^test result' | awk '{ok+=$4} END {print "Total ok:", ok}'
# Expect: Total ok: 345 (no new tests yet)
```

- [ ] **Step 5: Commit**

```bash
git add crates/md-codec/src/bin/md/cmd/encode.rs crates/md-codec/src/bin/md/cmd/verify.rs crates/md-codec/src/bin/md/main.rs
git commit -m "$(cat <<'EOF'
feat(v0.15.1/phase-2): --network on encode/verify; encode --json gains "network" field

EncodeArgs/VerifyArgs gain a Network parameter; EncodeArgs also carries
the static CliNetwork string for JSON serialization. The encode --json
output now always includes "network" (defaulting to "mainnet"), which
lets downstream tools piping encode --json into address --json know the
intended network without a separate flag.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 2.3: Integration tests for tpub-on-encode and `network` field on JSON

**Files:**
- Modify: `crates/md-codec/tests/cmd_encode.rs`

- [ ] **Step 1: Write the failing tests**

**Resolve the tpub literal first.** The same value appears in three independent
test crates (`parse/keys.rs`, `tests/cmd_encode.rs`, `tests/cmd_address_json.rs`)
because integration-test crates can't see `pub(crate)` consts inside the bin
crate. Run this to recover the literal value derived in Phase 1 Task 1.1:

```bash
cargo test --features cli --bin md derive_abandon_tpub_for_fixtures -- --ignored --nocapture 2>&1 | grep "m/84"
# Output line: m/84'/1'/0': tpubD...
```

Capture the `tpubD...` value. Use it verbatim in the snippets below (substitute
for `TPUB_FIXTURE`). If the throwaway test was already deleted (per Phase 1
Task 1.1 Step 2), re-derive it temporarily or copy the literal from
`parse/keys.rs`'s `ABANDON_TPUB_DEPTH3_BIP84` const.

Append to `crates/md-codec/tests/cmd_encode.rs` (substituting `TPUB_FIXTURE`):

```rust
const TPUB_FIXTURE: &str = "<the tpub string from m/84'/1'/0' — same value as ABANDON_TPUB_DEPTH3_BIP84 in parse/keys.rs>";

#[test]
fn encode_json_network_field_default_mainnet() {
    Command::cargo_bin("md").unwrap()
        .args(["encode", "wpkh(@0/<0;1>/*)", "--json"])
        .assert().success()
        .stdout(predicate::str::contains("\"network\": \"mainnet\""));
}

#[test]
fn encode_json_network_field_testnet() {
    Command::cargo_bin("md").unwrap()
        .args(["encode", "wpkh(@0/<0;1>/*)", "--network", "testnet",
               "--key", &format!("@0={TPUB_FIXTURE}"), "--json"])
        .assert().success()
        .stdout(predicate::str::contains("\"network\": \"testnet\""));
}

#[test]
fn encode_rejects_tpub_under_default_mainnet() {
    Command::cargo_bin("md").unwrap()
        .args(["encode", "wpkh(@0/<0;1>/*)", "--key", &format!("@0={TPUB_FIXTURE}")])
        .assert().code(1)
        .stderr(predicate::str::contains("expected mainnet"));
}
```

(The tpub literal is repeated in three places — `parse/keys.rs` const, this test, the testnet snapshot in Phase 5 — because integration-test crates can't reach `pub(crate)` items in the bin. Pinning the exact base58 string in each is the simplest way to keep tests offline; if it ever drifts, the tests fail loudly.)

- [ ] **Step 2: Run tests; expect pass (Task 2.2 already wired the behavior)**

```bash
cargo test --features cli,json --test cmd_encode 2>&1 | tail -10
# Expect: all tests pass (existing 4 + 3 new = 7)
```

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/tests/cmd_encode.rs
git commit -m "$(cat <<'EOF'
test(v0.15.1/phase-2): encode --json network field + tpub end-to-end

Three new integration tests:
- encode --json defaults network field to "mainnet"
- encode --network testnet --key @0=<tpub> --json emits "testnet"
- encode --key @0=<tpub> under default mainnet exits 1 with clear stderr

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 2.4: Phase 2 ship tag

- [ ] **Step 1: Confirm test count**

```bash
cargo test --workspace --features cli,json,cli-compiler 2>&1 | grep -E '^test result' | awk '{ok+=$4} END {print "Total ok:", ok}'
# Expect: Total ok: 348 (345 + 3 new cmd_encode tests)
cargo clippy --workspace --features cli,json,cli-compiler --all-targets -- -D warnings 2>&1 | tail -3
# Expect: clean
```

- [ ] **Step 2: Empty ship-tag commit**

```bash
git commit --allow-empty -m "$(cat <<'EOF'
chore(v0.15.1/phase-2): ship — --network on encode/verify; encode --json carries network

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 2.5: Per-phase review

Same protocol as Phase 1: dispatch `feature-dev:code-reviewer` against Phase 2 commits, save report to `design/agent-reports/v0.15.1-phase-2-review.md`, fix HIGH/MEDIUM inline, queue LOW for FOLLOWUPS append.

---

## Phase 3 — `cmd/address.rs` skeleton + `Address` variant + dual-input mode

Phase goal: `md address $PHRASE` and `md address --template "wpkh(@0/<0;1>/*)" --key @0=<xpub>` both produce a single mainnet receive-0 address. No `--chain`/`--change`/`--index`/`--count`/`--json` semantics yet (those land in Phases 4 and 5).

### Task 3.1: Create `cmd/address.rs` with the public surface

**Files:**
- Create: `crates/md-codec/src/bin/md/cmd/address.rs`
- Modify: `crates/md-codec/src/bin/md/cmd/mod.rs`

- [ ] **Step 1: Create `cmd/address.rs`**

```rust
use crate::error::CliError;
use crate::parse::keys::{parse_fingerprint, parse_key, ParsedFingerprint};
use crate::parse::template::{ctx_for_template, parse_template};
use md_codec::decode::decode_md1_string;
use md_codec::chunk::reassemble;
use md_codec::encode::Descriptor;

pub struct AddressArgs<'a> {
    pub phrases: &'a [String],
    pub template: Option<&'a str>,
    pub keys: &'a [String],
    pub fingerprints: &'a [String],
    pub network: bitcoin::Network,
    pub network_str: &'static str,
    pub chain: u32,
    pub index: u32,
    pub count: u32,
    pub json: bool,
}

pub fn run(args: AddressArgs<'_>) -> Result<(), CliError> {
    let descriptor = build_descriptor(&args)?;
    if !descriptor.is_wallet_policy() {
        return Err(CliError::BadArg(
            "address requires wallet-policy mode (Pubkeys TLV); supply --key @i=XPUB or use a wallet-policy-mode phrase".into(),
        ));
    }

    let _ = args.json;             // Phase 5 wires --json
    let _ = args.network_str;      // Phase 5 uses for JSON
    let _ = args.count;            // Phase 4 wires the loop

    // Phase 3 baseline: derive exactly one address at (chain, index).
    let addr = descriptor.derive_address(args.chain, args.index, args.network)?
        .assume_checked();
    println!("{addr}");
    Ok(())
}

fn build_descriptor(args: &AddressArgs<'_>) -> Result<Descriptor, CliError> {
    if let Some(template) = args.template {
        if args.keys.is_empty() {
            return Err(CliError::BadArg(
                "--key @i=<XPUB> required when --template is supplied".into()
            ));
        }
        let ctx = ctx_for_template(template);
        let parsed_keys = args.keys.iter().map(|k| parse_key(k, ctx, args.network)).collect::<Result<Vec<_>, _>>()?;
        let parsed_fps: Vec<ParsedFingerprint> = args.fingerprints.iter().map(|s| parse_fingerprint(s)).collect::<Result<Vec<_>, _>>()?;
        return Ok(parse_template(template, &parsed_keys, &parsed_fps)?);
    }
    // Phrase path
    if args.phrases.len() == 1 {
        Ok(decode_md1_string(&args.phrases[0])?)
    } else {
        let refs: Vec<&str> = args.phrases.iter().map(String::as_str).collect();
        Ok(reassemble(&refs)?)
    }
}
```

- [ ] **Step 2: Register the module**

Edit `crates/md-codec/src/bin/md/cmd/mod.rs` and add `pub mod address;` (alphabetically; the file ends with `verify`, so insert `address` first):

```rust
pub mod address;
pub mod bytecode;
#[cfg(feature = "cli-compiler")]
pub mod compile;
pub mod decode;
pub mod encode;
pub mod inspect;
pub mod vectors;
pub mod verify;
```

- [ ] **Step 3: Build (no Address variant yet — module is unreachable but compiles)**

```bash
cargo build --features cli,json --bin md 2>&1 | tail -3
# Expect: compile succeeds, with `dead_code` warnings on AddressArgs/run (transient)
```

- [ ] **Step 4: Commit**

```bash
git add crates/md-codec/src/bin/md/cmd/address.rs crates/md-codec/src/bin/md/cmd/mod.rs
git commit -m "$(cat <<'EOF'
feat(v0.15.1/phase-3): cmd/address.rs skeleton with dual-input descriptor builder

AddressArgs<'a> mirrors EncodeArgs<'a>'s borrow pattern. build_descriptor
forks on template-vs-phrases input mode (template path uses
parse_template; phrase path uses decode_md1_string or reassemble like
decode/inspect do). The wallet-policy gate fires after construction
with the runtime BadArg message; the empty-keys-with-template case is
caught earlier with a distinct message.

Phase 3 wires only the chain=0/index=0/count=1 baseline. Loop, --json,
and --change land in later phases.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 3.2: Add `Address` variant + dispatch in main.rs

**Files:**
- Modify: `crates/md-codec/src/bin/md/main.rs`

- [ ] **Step 1: Add the `Address` variant to `enum Command`**

Insert after the existing `Command::Compile {...}` arm (or at the end of the enum):

```rust
    /// Derive bitcoin addresses from a wallet-policy-mode descriptor.
    #[command(after_long_help = "EXAMPLES:\n  $ md address md1qq...\n  bc1q...")]
    Address {
        /// One or more md1 phrases. Mutually exclusive with --template.
        #[arg(num_args = 0..)]
        phrases: Vec<String>,
        /// BIP 388 template. Requires at least one --key. Mutually exclusive with phrases.
        #[arg(long, value_name = "TEMPLATE", conflicts_with = "phrases")]
        template: Option<String>,
        /// Concrete xpub for placeholder @i. Repeatable. Requires --template.
        #[arg(long = "key", value_name = "@i=XPUB", requires = "template")]
        keys: Vec<String>,
        /// Master-key fingerprint for placeholder @i. Repeatable. Requires --template.
        #[arg(long = "fingerprint", value_name = "@i=HEX", requires = "template")]
        fingerprints: Vec<String>,
        /// Network for xpub validation and address rendering.
        #[arg(long, value_enum, default_value_t = CliNetwork::Mainnet)]
        network: CliNetwork,
        /// Multipath alternative selector (0 = receive, 1 = change for canonical <0;1>/*).
        #[arg(long, default_value_t = 0)]
        chain: u32,
        /// Sugar for --chain 1.
        #[arg(long, conflicts_with = "chain")]
        change: bool,
        /// Starting index along the wildcard.
        #[arg(long, default_value_t = 0)]
        index: u32,
        /// Number of consecutive addresses to derive starting at --index.
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..=1000))]
        count: u32,
        /// Emit JSON output.
        #[arg(long)]
        json: bool,
    },
```

Then add the `ArgGroup` requirement: this needs to be expressed via clap derive's `#[command(group = ...)]` on the `Address` variant. Use:

```rust
    #[command(after_long_help = "EXAMPLES:\n  $ md address md1qq...\n  bc1q...",
              group = clap::ArgGroup::new("address_input").required(true).args(["phrases", "template"]))]
    Address {
        ... (as above)
    },
```

(Clap accepts `group = clap::ArgGroup::...` directly as a `#[command]` parameter; this enforces "exactly one of phrases or template" at parse time.)

- [ ] **Step 2: Wire the dispatch arm**

Add to the `dispatch` match in main.rs (after the `Command::Compile` arm):

```rust
        Command::Address {
            phrases, template, keys, fingerprints, network,
            chain, change, index, count, json,
        } => {
            let chain = if change { 1 } else { chain };
            cmd::address::run(cmd::address::AddressArgs {
                phrases: &phrases,
                template: template.as_deref(),
                keys: &keys,
                fingerprints: &fingerprints,
                network: network.into(),
                network_str: network.as_str(),
                chain,
                index,
                count,
                json,
            })
        }
```

- [ ] **Step 3: Build**

```bash
cargo build --features cli,json --bin md 2>&1 | tail -3
# Expect: clean
```

- [ ] **Step 4: Smoke-test manually (no integration test yet — Task 3.3 adds one)**

```bash
target/debug/md address --help 2>&1 | head -20
# Expect: usage line, all args listed, EXAMPLES block at bottom
target/debug/md address 2>&1
# Expect: clap usage error (missing required input group); exit 2
```

- [ ] **Step 5: Commit**

```bash
git add crates/md-codec/src/bin/md/main.rs
git commit -m "$(cat <<'EOF'
feat(v0.15.1/phase-3): Address variant in main; --change collapses to chain=1

ArgGroup enforces exactly one of <STRING>... or --template at clap level.
--key/--fingerprint require --template via clap. --change conflicts with
--chain. --count clap-clamped to 1..=1000.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 3.3: Smoke integration test for both input modes

**Files:**
- Create: `crates/md-codec/tests/cmd_address.rs`

- [ ] **Step 1: Derive the abandon-mnemonic mainnet xpub at m/84'/0'/0' for fixtures**

Use the existing `tests/address_derivation.rs:32-44` helper as the model. Define an inline helper at the top of the new test file:

```rust
#![allow(missing_docs)]

use assert_cmd::Command;
use bitcoin::Network;
use bitcoin::bip32::{DerivationPath, Xpriv, Xpub};
use bitcoin::secp256k1::Secp256k1;
use std::str::FromStr;

const ABANDON: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

fn account_xpub(path: &str, network: Network) -> Xpub {
    let mn = bip39::Mnemonic::parse(ABANDON).unwrap();
    let seed = mn.to_seed("");
    let secp = Secp256k1::new();
    let master = Xpriv::new_master(network, &seed).unwrap();
    let dp = DerivationPath::from_str(path).unwrap();
    let xpriv = master.derive_priv(&secp, &dp).unwrap();
    Xpub::from_priv(&secp, &xpriv)
}

fn encode_template_with_key(template: &str, key_arg: &str) -> String {
    let out = Command::cargo_bin("md").unwrap()
        .args(["encode", template, "--key", key_arg])
        .output().unwrap();
    assert!(out.status.success(), "encode failed: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout).unwrap().lines().next().unwrap().to_string()
}
```

- [ ] **Step 2: Write the smoke test for both input modes**

Append to `crates/md-codec/tests/cmd_address.rs`:

```rust
#[test]
fn address_template_mode_emits_bip84_receive_0() {
    // BIP 84 vector: abandon mnemonic at m/84'/0'/0'/0/0 → bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu
    let xpub = account_xpub("m/84'/0'/0'", Network::Bitcoin);
    let key_arg = format!("@0={xpub}");
    Command::cargo_bin("md").unwrap()
        .args(["address", "--template", "wpkh(@0/<0;1>/*)", "--key", &key_arg])
        .assert().success()
        .stdout(predicates::str::contains("bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu"));
}

#[test]
fn address_phrase_mode_round_trips_through_encode() {
    // First encode the wallet-policy-mode phrase, then derive its address-0 via the phrase.
    let xpub = account_xpub("m/84'/0'/0'", Network::Bitcoin);
    let key_arg = format!("@0={xpub}");
    let phrase = encode_template_with_key("wpkh(@0/<0;1>/*)", &key_arg);
    Command::cargo_bin("md").unwrap()
        .args(["address", &phrase])
        .assert().success()
        .stdout(predicates::str::contains("bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu"));
}

#[test]
fn address_template_without_key_exits_2_with_helpful_message() {
    Command::cargo_bin("md").unwrap()
        .args(["address", "--template", "wpkh(@0/<0;1>/*)"])
        .assert().code(2)
        .stderr(predicates::str::contains("--key @i=<XPUB> required"));
}

#[test]
fn address_phrase_template_only_exits_2_with_wallet_policy_message() {
    // Encode without --key (template-only mode); decode it; the resulting
    // descriptor lacks Pubkeys TLV → wallet-policy gate fires.
    let phrase = Command::cargo_bin("md").unwrap()
        .args(["encode", "wpkh(@0/<0;1>/*)"])
        .output().unwrap();
    let phrase = String::from_utf8(phrase.stdout).unwrap().lines().next().unwrap().to_string();
    Command::cargo_bin("md").unwrap()
        .args(["address", &phrase])
        .assert().code(2)
        .stderr(predicates::str::contains("requires wallet-policy mode"));
}

#[test]
fn address_no_input_exits_2() {
    Command::cargo_bin("md").unwrap()
        .args(["address"])
        .assert().code(2);
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test --features cli,json --test cmd_address 2>&1 | tail -15
# Expect: 5 passed
```

If any of the BIP 84 receive-0 assertions fail, capture the actual address and confirm against the BIP 84 mediawiki spec — the abandon-mnemonic vector at `m/84'/0'/0'/0/0` is universally pinned to `bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu`.

- [ ] **Step 4: Commit**

```bash
git add crates/md-codec/tests/cmd_address.rs
git commit -m "$(cat <<'EOF'
test(v0.15.1/phase-3): smoke tests — both input modes + 3 negative paths

Five tests:
- template+key mode emits BIP 84 receive-0
- phrase mode (after encode --key) round-trips to the same address
- --template without --key exits 2 with the empty-keys message
- phrase from template-only encode exits 2 with the wallet-policy message
- no input exits 2 (clap ArgGroup rejection)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 3.4: Phase 3 ship tag

- [ ] **Step 1: Confirm test count**

```bash
cargo test --workspace --features cli,json,cli-compiler 2>&1 | grep -E '^test result' | awk '{ok+=$4} END {print "Total ok:", ok}'
# Expect: Total ok: 353 (348 + 5 new cmd_address tests)
cargo clippy --workspace --features cli,json,cli-compiler --all-targets -- -D warnings 2>&1 | tail -3
# Expect: clean (drop any leftover dead_code allows once Address is reachable)
```

- [ ] **Step 2: Empty ship-tag commit**

```bash
git commit --allow-empty -m "$(cat <<'EOF'
chore(v0.15.1/phase-3): ship — md address subcommand baseline (chain=0/index=0/count=1)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 3.5: Per-phase review (same protocol)

---

## Phase 4 — `--chain`/`--change`/`--index`/`--count` semantics + golden vectors

Phase goal: full positional semantics of address subcommand work end-to-end. Mainnet AND testnet golden vectors pinned at the CLI boundary.

### Task 4.1: Loop over `count` indices in `cmd/address.rs`

**Files:**
- Modify: `crates/md-codec/src/bin/md/cmd/address.rs`

- [ ] **Step 1: Replace the single-address derive with a count-loop**

Edit `crates/md-codec/src/bin/md/cmd/address.rs`. Replace the section after the wallet-policy gate (the current `// Phase 3 baseline: derive exactly one address...` block) with:

```rust
    let _ = args.json;             // Phase 5 wires --json
    let _ = args.network_str;      // Phase 5 uses for JSON

    for k in 0..args.count {
        let i = args.index.checked_add(k).ok_or_else(|| CliError::BadArg(
            format!("--index + --count overflows u32: {} + {}", args.index, args.count)
        ))?;
        let addr = descriptor.derive_address(args.chain, i, args.network)?.assume_checked();
        println!("{addr}");
    }
    Ok(())
```

The overflow check matters because `--index` accepts the full u32 range; the clap clamp on `--count` to 1..=1000 keeps the worst case bounded.

- [ ] **Step 2: Build**

```bash
cargo build --features cli,json --bin md 2>&1 | tail -3
# Expect: clean
```

- [ ] **Step 3: Commit (the test for the loop lands in Task 4.2)**

```bash
git add crates/md-codec/src/bin/md/cmd/address.rs
git commit -m "$(cat <<'EOF'
feat(v0.15.1/phase-4): address loops over [index, index + count) with overflow guard

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 4.2: Golden-vector tests at the CLI layer

**Files:**
- Modify: `crates/md-codec/tests/cmd_address.rs`

- [ ] **Step 1: Add helper for testnet xpub derivation**

The mainnet helper exists from Phase 3; add a testnet variant. At the top of `crates/md-codec/tests/cmd_address.rs` after the existing `account_xpub` helper:

```rust
fn account_xpub_testnet(path: &str) -> Xpub { account_xpub(path, Network::Testnet) }

/// Independently derive the BIP 84 single-sig address using rust-bitcoin's
/// own bip32 + Address builders. Used to pin testnet (and any non-published
/// mainnet) golden vectors against a trusted secondary path.
fn expected_wpkh_address(account_xpub: &Xpub, chain: u32, index: u32, network: Network) -> String {
    use bitcoin::Address;
    use bitcoin::bip32::ChildNumber;
    use bitcoin::CompressedPublicKey;
    let secp = Secp256k1::new();
    let leaf = account_xpub
        .derive_pub(&secp, &[
            ChildNumber::Normal { index: chain },
            ChildNumber::Normal { index },
        ]).unwrap();
    let cpk = CompressedPublicKey(leaf.public_key);
    Address::p2wpkh(&cpk, network).to_string()
}
```

- [ ] **Step 2: Write the golden-vector tests**

Append to `crates/md-codec/tests/cmd_address.rs`:

```rust
#[test]
fn address_mainnet_wpkh_receive_0_and_1() {
    // BIP 84 published vectors:
    // m/84'/0'/0'/0/0 → bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu
    // m/84'/0'/0'/0/1 → bc1qnjg0jd8228aq7egyzacy8cys3knf9xvrerkf9g
    let xpub = account_xpub("m/84'/0'/0'", Network::Bitcoin);
    let key_arg = format!("@0={xpub}");
    let out = Command::cargo_bin("md").unwrap()
        .args(["address", "--template", "wpkh(@0/<0;1>/*)", "--key", &key_arg, "--count", "2"])
        .output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "expected 2 addresses, got {}: {stdout}", lines.len());
    assert_eq!(lines[0], "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu");
    assert_eq!(lines[1], "bc1qnjg0jd8228aq7egyzacy8cys3knf9xvrerkf9g");
}

#[test]
fn address_mainnet_wpkh_first_change() {
    // BIP 84 published change vector: m/84'/0'/0'/1/0 → bc1q8c6fshw2dlwun7ekn9qwf37cu2rn755upcp6el
    let xpub = account_xpub("m/84'/0'/0'", Network::Bitcoin);
    let key_arg = format!("@0={xpub}");
    Command::cargo_bin("md").unwrap()
        .args(["address", "--template", "wpkh(@0/<0;1>/*)", "--key", &key_arg, "--change"])
        .assert().success()
        .stdout(predicates::str::contains("bc1q8c6fshw2dlwun7ekn9qwf37cu2rn755upcp6el"));
}

#[test]
fn address_testnet_wpkh_receive_0_via_secondary_path() {
    // BIP 84 doesn't publish testnet vectors for the abandon-mnemonic;
    // we cross-check against rust-bitcoin's own derivation.
    let xpub = account_xpub_testnet("m/84'/1'/0'");
    let expected = expected_wpkh_address(&xpub, 0, 0, Network::Testnet);
    assert!(expected.starts_with("tb1q"), "expected tb1q... testnet address, got {expected}");
    let key_arg = format!("@0={xpub}");
    Command::cargo_bin("md").unwrap()
        .args(["address", "--template", "wpkh(@0/<0;1>/*)", "--key", &key_arg, "--network", "testnet"])
        .assert().success()
        .stdout(predicates::str::contains(&expected));
}

#[test]
fn address_count_max_succeeds() {
    let xpub = account_xpub("m/84'/0'/0'", Network::Bitcoin);
    let key_arg = format!("@0={xpub}");
    let out = Command::cargo_bin("md").unwrap()
        .args(["address", "--template", "wpkh(@0/<0;1>/*)", "--key", &key_arg, "--count", "1000"])
        .output().unwrap();
    assert!(out.status.success());
    let n = String::from_utf8(out.stdout).unwrap().lines().count();
    assert_eq!(n, 1000);
}

#[test]
fn address_count_over_max_clap_rejects() {
    Command::cargo_bin("md").unwrap()
        .args(["address", "--template", "wpkh(@0/<0;1>/*)", "--key", "@0=ignored", "--count", "1001"])
        .assert().code(2);
}

#[test]
fn address_chain_out_of_range_returns_1() {
    let xpub = account_xpub("m/84'/0'/0'", Network::Bitcoin);
    let key_arg = format!("@0={xpub}");
    Command::cargo_bin("md").unwrap()
        .args(["address", "--template", "wpkh(@0/<0;1>/*)", "--key", &key_arg, "--chain", "5"])
        .assert().code(1)
        .stderr(predicates::str::contains("out of range"));
}

#[test]
fn address_change_and_chain_together_rejected() {
    Command::cargo_bin("md").unwrap()
        .args(["address", "--template", "wpkh(@0/<0;1>/*)", "--key", "@0=ignored",
               "--change", "--chain", "1"])
        .assert().code(2);
}

#[test]
fn address_mainnet_wsh_multi_2of2_receive_0() {
    // Per SPEC §Testing: 2-of-2 wsh-multi at m/48'/0'/0'/2' from
    // abandon-mnemonic xpubs; cross-check against rust-bitcoin's
    // descriptor-derived address. Same xpub used twice for @0 and @1
    // (degenerate but the codec doesn't reject it; the resulting
    // descriptor is structurally a 2-of-2).
    use bitcoin::Address;
    use bitcoin::bip32::ChildNumber;
    use bitcoin::CompressedPublicKey;
    use miniscript::ScriptContext as _;
    let xpub = account_xpub("m/48'/0'/0'/2'", Network::Bitcoin);
    let key_arg = format!("@0={xpub}");
    let key_arg_b = format!("@1={xpub}");

    // Independently derive the expected wsh-multi address.
    let secp = Secp256k1::new();
    let leaf = xpub.derive_pub(&secp, &[
        ChildNumber::Normal { index: 0 },
        ChildNumber::Normal { index: 0 },
    ]).unwrap();
    let cpk = CompressedPublicKey(leaf.public_key);
    let pk = bitcoin::PublicKey::new(cpk.0);
    let script = bitcoin::blockdata::script::Builder::new()
        .push_int(2)
        .push_key(&pk).push_key(&pk)
        .push_int(2)
        .push_opcode(bitcoin::opcodes::all::OP_CHECKMULTISIG)
        .into_script();
    let expected = Address::p2wsh(&script, Network::Bitcoin).to_string();

    Command::cargo_bin("md").unwrap()
        .args(["address", "--template", "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))",
               "--key", &key_arg, "--key", &key_arg_b])
        .assert().success()
        .stdout(predicates::str::contains(&expected));
}
```

Note: the wsh-multi test uses the same xpub for both `@0` and `@1` to keep
the fixture self-contained. The codec doesn't enforce key uniqueness; the
resulting descriptor is structurally a 2-of-2 over two identical keys,
which derives a deterministic address that we cross-check.

- [ ] **Step 2 (run): Run tests**

```bash
cargo test --features cli,json --test cmd_address 2>&1 | tail -15
# Expect: 13 passed (5 from Phase 3 + 8 new — including wsh-multi)
```

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/tests/cmd_address.rs
git commit -m "$(cat <<'EOF'
test(v0.15.1/phase-4): golden vectors — mainnet receive 0/1, change/0; testnet receive 0; clap edges

Mainnet vectors anchor on BIP 84's published abandon-mnemonic test
vectors. Testnet vector cross-checks against rust-bitcoin's own
Address::p2wpkh derivation (BIP 84 doesn't publish testnet vectors
for the abandon mnemonic). Edge tests: --count 1000 succeeds;
--count 1001 clap-rejected; --chain 5 on <0;1>/* returns 1; --change
+ --chain together clap-rejected.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 4.3: Phase 4 ship tag

- [ ] **Step 1: Confirm test count**

```bash
cargo test --workspace --features cli,json,cli-compiler 2>&1 | grep -E '^test result' | awk '{ok+=$4} END {print "Total ok:", ok}'
# Expect: Total ok: 361 (353 + 8 new cmd_address tests including wsh-multi)
cargo clippy --workspace --features cli,json,cli-compiler --all-targets -- -D warnings 2>&1 | tail -3
# Expect: clean
```

- [ ] **Step 2: Empty ship-tag commit**

```bash
git commit --allow-empty -m "$(cat <<'EOF'
chore(v0.15.1/phase-4): ship — full --chain/--change/--index/--count semantics + goldens

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 4.4: Per-phase review (same protocol)

---

## Phase 5 — `--json` on `md address` + insta snapshots

Phase goal: `md address ... --json` emits the schema-tagged JSON shape; snapshots pinned via insta.

### Task 5.1: Wire `--json` branch in `cmd/address.rs`

**Files:**
- Modify: `crates/md-codec/src/bin/md/cmd/address.rs`

- [ ] **Step 1: Replace the loop body to support both text and JSON output**

Replace the loop section in `cmd/address.rs` (the one Phase 4 added) with:

```rust
    // Collect (chain, index, address) tuples first; then emit text or JSON.
    let mut rows: Vec<(u32, u32, String)> = Vec::with_capacity(args.count as usize);
    for k in 0..args.count {
        let i = args.index.checked_add(k).ok_or_else(|| CliError::BadArg(
            format!("--index + --count overflows u32: {} + {}", args.index, args.count)
        ))?;
        let addr = descriptor.derive_address(args.chain, i, args.network)?.assume_checked();
        rows.push((args.chain, i, addr.to_string()));
    }

    #[cfg(feature = "json")]
    if args.json {
        use crate::format::json::SCHEMA;
        let addresses: Vec<serde_json::Value> = rows.iter().map(|(c, i, a)| {
            serde_json::json!({ "chain": c, "index": i, "address": a })
        }).collect();
        let v = serde_json::json!({
            "schema": SCHEMA,
            "network": args.network_str,
            "addresses": addresses,
        });
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
        return Ok(());
    }
    let _ = args.json;             // when json feature off, silence unused warning

    for (_, _, addr) in &rows {
        println!("{addr}");
    }
    Ok(())
```

- [ ] **Step 2: Build**

```bash
cargo build --features cli,json --bin md 2>&1 | tail -3
# Expect: clean
```

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/src/bin/md/cmd/address.rs
git commit -m "$(cat <<'EOF'
feat(v0.15.1/phase-5): address --json — schema/network/addresses[{chain,index,address}]

Collects rows first, then forks on --json. Text mode unchanged: one
address per line. JSON mode emits pretty-printed schema-tagged shape
with the CLI vocabulary network string ("mainnet", not "bitcoin").

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 5.2: insta snapshot tests

**Files:**
- Create: `crates/md-codec/tests/cmd_address_json.rs`

- [ ] **Step 1: Write the snapshot tests**

```rust
#![allow(missing_docs)]
#![cfg(feature = "json")]

use assert_cmd::Command;
use bitcoin::Network;
use bitcoin::bip32::{DerivationPath, Xpriv, Xpub};
use bitcoin::secp256k1::Secp256k1;
use std::str::FromStr;

const ABANDON: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

fn account_xpub(path: &str, network: Network) -> Xpub {
    let mn = bip39::Mnemonic::parse(ABANDON).unwrap();
    let seed = mn.to_seed("");
    let secp = Secp256k1::new();
    let master = Xpriv::new_master(network, &seed).unwrap();
    let dp = DerivationPath::from_str(path).unwrap();
    let xpriv = master.derive_priv(&secp, &dp).unwrap();
    Xpub::from_priv(&secp, &xpriv)
}

#[test]
fn snapshot_wpkh_mainnet_receive_0_to_2() {
    let xpub = account_xpub("m/84'/0'/0'", Network::Bitcoin);
    let key = format!("@0={xpub}");
    let out = Command::cargo_bin("md").unwrap()
        .args(["address", "--template", "wpkh(@0/<0;1>/*)", "--key", &key, "--count", "3", "--json"])
        .output().unwrap();
    let body = String::from_utf8(out.stdout).unwrap();
    insta::assert_snapshot!("wpkh_mainnet_receive_0_to_2", body);
}

#[test]
fn snapshot_wpkh_mainnet_change_0() {
    let xpub = account_xpub("m/84'/0'/0'", Network::Bitcoin);
    let key = format!("@0={xpub}");
    let out = Command::cargo_bin("md").unwrap()
        .args(["address", "--template", "wpkh(@0/<0;1>/*)", "--key", &key, "--change", "--json"])
        .output().unwrap();
    let body = String::from_utf8(out.stdout).unwrap();
    insta::assert_snapshot!("wpkh_mainnet_change_0", body);
}

#[test]
fn snapshot_wpkh_testnet_receive_0() {
    let xpub = account_xpub("m/84'/1'/0'", Network::Testnet);
    let key = format!("@0={xpub}");
    let out = Command::cargo_bin("md").unwrap()
        .args(["address", "--template", "wpkh(@0/<0;1>/*)", "--key", &key, "--network", "testnet", "--json"])
        .output().unwrap();
    let body = String::from_utf8(out.stdout).unwrap();
    insta::assert_snapshot!("wpkh_testnet_receive_0", body);
}
```

- [ ] **Step 2: Generate snapshots**

```bash
INSTA_UPDATE=always cargo test --features cli,json --test cmd_address_json 2>&1 | tail -10
# Expect: 3 passed; 3 .snap files created in crates/md-codec/tests/snapshots/
```

- [ ] **Step 3: Re-run without UPDATE to confirm pinning works**

```bash
cargo test --features cli,json --test cmd_address_json 2>&1 | tail -5
# Expect: 3 passed (no env override needed)
```

- [ ] **Step 4: Commit**

```bash
git add crates/md-codec/tests/cmd_address_json.rs crates/md-codec/tests/snapshots/cmd_address_json__*.snap
git commit -m "$(cat <<'EOF'
test(v0.15.1/phase-5): insta snapshots for address --json (mainnet+testnet)

Three snapshot tests: mainnet wpkh receive 0..=2, mainnet wpkh
change/0, testnet wpkh receive 0. Snapshots pinned to disk in the
same commit. cargo test passes without INSTA_UPDATE override.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 5.3: Phase 5 ship tag

- [ ] **Step 1: Confirm test count**

```bash
cargo test --workspace --features cli,json,cli-compiler 2>&1 | grep -E '^test result' | awk '{ok+=$4} END {print "Total ok:", ok}'
# Expect: Total ok: 364 (361 + 3 snapshots)
cargo clippy --workspace --features cli,json,cli-compiler --all-targets -- -D warnings 2>&1 | tail -3
# Expect: clean
```

- [ ] **Step 2: Empty ship-tag commit**

```bash
git commit --allow-empty -m "$(cat <<'EOF'
chore(v0.15.1/phase-5): ship — md address --json + snapshots

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 5.4: Per-phase review (same protocol)

---

## Phase 6 — Docs, version bump, FOLLOWUPS append, ship

Phase goal: v0.15.1 ready to tag. Crate version bumped, all user-facing docs updated, every deferred LOW finding from prior phases appended to FOLLOWUPS.md under a v0.15.2 tier.

### Task 6.1: Bump crate version

**Files:**
- Modify: `crates/md-codec/Cargo.toml`

- [ ] **Step 1: Edit version**

In `crates/md-codec/Cargo.toml`, change `version = "0.15.0"` to `version = "0.15.1"`.

- [ ] **Step 2: Confirm build + tests**

```bash
cargo test --workspace --features cli,json,cli-compiler 2>&1 | grep -E '^test result' | awk '{ok+=$4} END {print "Total ok:", ok}'
# Expect: Total ok: 364
```

- [ ] **Step 3: Commit**

```bash
git add crates/md-codec/Cargo.toml Cargo.lock
git commit -m "$(cat <<'EOF'
chore(v0.15.1): bump crate version 0.15.0 → 0.15.1

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 6.2: README CLI table update

**Files:**
- Modify: `crates/md-codec/README.md`

- [ ] **Step 1: Add the address row to the CLI subcommand table**

Find the `## CLI` section (around line 32-48 per v0.15.0). The existing table has rows for `encode`/`decode`/`verify`/`inspect`/`bytecode`/`vectors`/`compile`. Insert a new row for `address` between `bytecode` and `vectors`:

```markdown
| `md address <STRING>... --chain N --index N [--count K]` (or `--template <T> --key @i=<XPUB>`) | Derive bitcoin addresses from a wallet-policy-mode descriptor. `--network mainnet|testnet|signet|regtest`, `--change` sugar, `--json` schema. |
```

Then below the table, append a "Network" paragraph:

```markdown
### Network selection

`md encode`, `md verify`, and `md address` accept `--network mainnet|testnet|signet|regtest` (default `mainnet`). The wire format does not carry network — it's a CLI-side convenience for xpub/tpub validation and address rendering. `md decode`/`inspect`/`bytecode` are network-agnostic; pass `--network` to `md address` when rendering addresses from a phrase that was originally built with non-mainnet keys.
```

- [ ] **Step 2: Commit**

```bash
git add crates/md-codec/README.md
git commit -m "$(cat <<'EOF'
docs(v0.15.1): README — md address row + Network selection paragraph

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 6.3: CHANGELOG entry

**Files:**
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Add the [0.15.1] entry**

At the top of CHANGELOG.md (after the `## [0.15.0]` block), insert:

```markdown
## [0.15.1] — 2026-05-03

### Added

- `md address` subcommand: derive bitcoin addresses from a
  wallet-policy-mode descriptor. Accepts either md1 phrases or a
  template + `--key @i=XPUB` (same shape as `md encode`). Args:
  `--chain N` / `--change`, `--index N`, `--count K` (clap-clamped to
  `1..=1000`), `--network mainnet|testnet|signet|regtest`, `--json`.
- `--network` flag on `md encode` and `md verify` accepting
  `mainnet|testnet|signet|regtest` (default `mainnet`). Routes
  xpub-version validation in `parse_key` between the BIP 32 mainnet
  (`0488B21E`) and testnet (`043587CF`) version bytes; the testnet
  bytes also cover signet and regtest per BIP 32.
- `encode --json` always emits a top-level `network` field carrying
  the CLI vocabulary string (`"mainnet"` / `"testnet"` / `"signet"` /
  `"regtest"`). Lets a script piping `encode --json` into `address --json`
  preserve the user's chosen network without a separate flag.
- Insta snapshots for `address --json` mainnet receive 0..=2, mainnet
  change/0, and testnet receive 0.

### Unchanged

- Library API. Wire format. Public `md_codec::*` exports.
```

- [ ] **Step 2: Commit**

```bash
git add CHANGELOG.md
git commit -m "$(cat <<'EOF'
docs(v0.15.1): CHANGELOG entry for 0.15.1

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 6.4: MIGRATION entry

**Files:**
- Modify: `MIGRATION.md`

- [ ] **Step 1: Add the v0.15.0 → v0.15.1 section**

At the top of MIGRATION.md, after the `# Migration guide` header and before the `## v0.14.x → v0.15.0` block, insert:

```markdown
## v0.15.0 → v0.15.1

Pure additive. **No source changes required for downstream library consumers.** Existing CLI invocations keep mainnet semantics by default.

### What's new

- `md address` subcommand and `--network` on `md encode` / `md verify` / `md address`.
- `encode --json` always carries a `network` field (defaulting to `"mainnet"`).

### What didn't change

- Wire format. Library API surface. Public `md_codec::*` exports. Existing CLI subcommands' output (text or JSON), except for the new `network` field on `encode --json`.

### Heads-up for JSON consumers

`encode --json` previously had no `network` field; it now always does. Strict-schema JSON consumers that reject unknown fields must add `network` to their accepted set. Lenient consumers (the common case) need no change.
```

- [ ] **Step 2: Commit**

```bash
git add MIGRATION.md
git commit -m "$(cat <<'EOF'
docs(v0.15.1): MIGRATION entry — v0.15.0 → v0.15.1 (additive)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 6.5: docs/json-schema-v1.md update

**Files:**
- Modify: `docs/json-schema-v1.md`

- [ ] **Step 1: Add `network` row to the `encode --json` table and the new `address --json` section**

In `docs/json-schema-v1.md`, find the `### encode --json` table (around line 12-18 of the file post-v0.15.0). Insert a new row before `phrase`:

```markdown
| `network` | string | yes (always) — `"mainnet"`/`"testnet"`/`"signet"`/`"regtest"` |
```

Then add a new section after `### compile --json`:

```markdown
### `address --json`
| Field | Type |
|---|---|
| `schema` | string |
| `network` | string — `"mainnet"`/`"testnet"`/`"signet"`/`"regtest"` |
| `addresses` | array of `{ "chain": u32, "index": u32, "address": string }` |

Example:

\`\`\`json
{
  "schema": "md-cli/1",
  "network": "mainnet",
  "addresses": [
    { "chain": 0, "index": 0, "address": "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu" }
  ]
}
\`\`\`
```

(Replace `\`\`\`` with actual triple-backticks; the markdown escape is just for clarity in this plan document.)

- [ ] **Step 2: Commit**

```bash
git add docs/json-schema-v1.md
git commit -m "$(cat <<'EOF'
docs(v0.15.1): json-schema-v1 — encode network row + address --json section

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 6.6: FOLLOWUPS — add v0.15.2 tier + append deferred findings

**Files:**
- Modify: `design/FOLLOWUPS.md`

- [ ] **Step 1: Add `v0.15.2` to the Tiers section**

Find the `## Tiers (definitions)` section in `design/FOLLOWUPS.md` (around line 35). After the existing tier definitions, add:

```markdown
- **`v0.15.2`**: low-severity findings deferred from v0.15.1 spec/plan/per-phase reviews. Targeted for the next patch release. Each entry cites the source review report under `design/agent-reports/v0.15.1-*.md`.
```

- [ ] **Step 2: Append entries for every deferred finding from Phase 1-5 reviews**

Find the `## Open items` section. Append (one entry per LOW/NIT finding surfaced in any of `design/agent-reports/v0.15.1-*-review.md`). Each entry follows the template at the top of FOLLOWUPS.md.

For the SPEC review (already on disk), the three deferrals are:

```markdown
### `v0.15.1-spec-l1-encode-json-network-row` — encode --json `network` row text not pre-pinned in SPEC

- **Surfaced:** SPEC review r1 (`design/agent-reports/v0.15.1-spec-review-r1.md` finding L1)
- **Where:** `docs/json-schema-v1.md` `encode --json` table
- **What:** SPEC said to add a `network` row to the encode-JSON schema table but didn't pin the row text. The IMPLEMENTATION_PLAN handled it inline; entry kept here for traceability and to document the SPEC-to-doc handoff.
- **Why deferred:** cosmetic; the impl-plan author wrote the row.
- **Status:** `resolved <Phase 6 commit SHA>`
- **Tier:** `v0.15.2`

### `v0.15.1-spec-l2-address-json-arg-row` — `--json` arg missing from address arg-semantics table

- **Surfaced:** SPEC review r1 (L2)
- **Where:** `design/SPEC_v0_15_1_address_and_network.md` Subcommand surface table
- **What:** All other args have rows; `--json` was only mentioned in the CLI synopsis. Cosmetic.
- **Why deferred:** doesn't affect implementation correctness; SPEC could be patched in a follow-up.
- **Status:** `open`
- **Tier:** `v0.15.2`

### `v0.15.1-spec-l3-test-baseline-citation` — baseline 340 stated without citation

- **Surfaced:** SPEC review r1 (L3)
- **Where:** SPEC §Testing
- **What:** Baseline 340 cited as fact without showing the cargo test output. SPEC fix added the inline citation post-r1; documenting closure here.
- **Why deferred:** trivial.
- **Status:** `resolved c937b9f`
- **Tier:** `v0.15.2`
```

Then for each PER-PHASE review report (Phases 1-5), append every LOW/NIT finding using the same format. The implementer should read each `design/agent-reports/v0.15.1-phase-N-review.md` and lift its LOW findings here. If a phase review found no LOW findings, no entry needed.

- [ ] **Step 3: Commit**

```bash
git add design/FOLLOWUPS.md
git commit -m "$(cat <<'EOF'
chore(v0.15.1): FOLLOWUPS — add v0.15.2 tier; append deferred LOW findings

Adds the v0.15.2 tier definition matching the existing v0.X tier
vocabulary. Lifts every LOW / NIT finding from the on-disk
spec/plan/per-phase review reports under design/agent-reports/v0.15.1-*.md.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

### Task 6.7: Final completion review

- [ ] **Step 1: Dispatch the completion reviewer**

Per the standing rule, dispatch `feature-dev:code-reviewer` against the full diff `main..HEAD`. Save report to `design/agent-reports/v0.15.1-final-review.md`. Address every CRITICAL/IMPORTANT finding inline in this session before the merge.

### Task 6.8: Phase 6 ship + release tag

- [ ] **Step 1: Final test/clippy/package check**

```bash
cargo test --workspace --features cli,json 2>&1 | grep -E '^test result' | awk '{ok+=$4} END {print "no-compiler total ok:", ok}'
cargo test --workspace --features cli,json,cli-compiler 2>&1 | grep -E '^test result' | awk '{ok+=$4} END {print "with-compiler total ok:", ok}'
# Expect: both ≥ 363 (no-compiler will be slightly lower because it skips compile.rs/cmd_compile tests)
cargo clippy --workspace --features cli,json,cli-compiler --all-targets -- -D warnings 2>&1 | tail -3
cargo package --no-verify -p md-codec 2>&1 | tail -3
# Expect: all clean; cargo package exits 0
```

- [ ] **Step 2: Empty ship-tag commit**

```bash
git commit --allow-empty -m "$(cat <<'EOF'
chore(v0.15.1/phase-6): ship — docs, version bump, FOLLOWUPS append

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 3: Final release commit**

```bash
git commit --allow-empty -m "$(cat <<'EOF'
release: md-codec v0.15.1 — md address + --network

Adds the md address subcommand wrapping the existing
Descriptor::derive_address library API, plus --network mainnet|testnet|
signet|regtest on encode/verify/address. Library API additive — no
source changes required for downstream library consumers. Wire format
unchanged from v0.13/v0.14/v0.15.

See CHANGELOG.md and MIGRATION.md for the full v0.15.0 → v0.15.1 details.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Merge + tag + push (out-of-phase; orchestrator handles after final review)

Mirroring the v0.15.0 procedure exactly. Performed in the main worktree, not `.worktrees/v0.15.1`.

```bash
# (in main worktree at /scratch/code/shibboleth/descriptor-mnemonic)
git fetch origin main
git merge --no-ff feat/v0.15.1 -m "Merge branch 'feat/v0.15.1' — md-codec v0.15.1 (md address + --network)"
cargo test --workspace --features cli,json,cli-compiler 2>&1 | grep -E '^test result' | awk '{ok+=$4} END {print "Total ok:", ok}'
# Expect: ≥ 363
git tag -a v0.15.1 -m "md-codec v0.15.1 — md address subcommand + --network on encode/verify/address"
git push origin main
git push origin v0.15.1
git worktree remove .worktrees/v0.15.1
git branch -d feat/v0.15.1
git worktree list   # confirm only main + agent isolation worktrees remain
```

---

## Verification (end-to-end, post-implementation)

- `cargo test --workspace --features cli,json` — ~360 tests
- `cargo test --workspace --features cli,json,cli-compiler` — ~363 tests
- `cargo clippy --workspace --features cli,json,cli-compiler --all-targets -- -D warnings` — clean
- `cargo package --no-verify -p md-codec` — exits 0
- `target/release/md address $PHRASE --network testnet --change --index 0 --count 3` — emits three `tb1q...` addresses
- `target/release/md address --template "wpkh(@0/<0;1>/*)" --key @0=<xpub> --json` — emits valid JSON with `schema: "md-cli/1"` and address objects
- `target/release/md encode wpkh(@0/<0;1>/*) --json | jq .network` — prints `"mainnet"`
- `git tag -l v0.15.1` shows the annotated tag
- `git log origin/main --oneline -3` shows the merge commit at HEAD
- All `design/agent-reports/v0.15.1-*` reports exist on disk
- `design/FOLLOWUPS.md` has every deferred LOW finding under tier `v0.15.2`

## Out of scope for this plan

- xpriv / mnemonic input (deferred; needs secret-handling discipline).
- `--account` flag (user explicitly dropped during brainstorming).
- Wire format changes (v0.15.x is wire-stable).
- `--network` on decode/inspect/bytecode (no semantic effect).
- Address validation (`md validate-address`) and bech32m flag — deferred.
- Multi-network parsing (auto-detect xpub vs tpub). Explicit selection is intentional.

---

## Self-review

Spec coverage check (skim each SPEC section, confirm a task implements it):
- ✅ Goal — Phase 1+2+3+4+5 cover both deliverables.
- ✅ Subcommand surface delta — Phase 2 (encode/verify), Phase 3 (Address variant), Phase 4 (loop semantics), Phase 5 (--json).
- ✅ md address arg semantics — all 11 args in Phase 3 Task 3.2 + Phase 4 Task 4.1.
- ✅ Default text output — Phase 3 Task 3.1, Phase 4 Task 4.1, Phase 5 Task 5.1.
- ✅ JSON output shape — Phase 5 Task 5.1.
- ✅ encode --json gains network field — Phase 2 Task 2.2 (impl), Task 2.3 (test).
- ✅ Exit codes (two distinct wallet-policy triggers + clap rejections) — Phase 3 Task 3.3 negative tests.
- ✅ Implementation surface (file-level changes) — full coverage in Phase 1-6.
- ✅ Network handling exhaustive table — implicitly covered (synthetic xpub stays mainnet; xpub_from_tlv_bytes untouched; only parse_key + Address::p2*(_, network) thread network).
- ✅ Testing — Phase 1 (parse::keys), Phase 2 (cmd_encode), Phase 3-4 (cmd_address), Phase 5 (cmd_address_json).
- ✅ Style & process — Conventions section.
- ✅ Out-of-scope — Out of scope section.

Placeholder scan: no `TBD`/`TODO`/`fill in details` outside the explicit `<paste ...>` placeholders for the testnet xpub literal (intentional — actual value is captured at execution time per Task 1.1).

Type consistency: `AddressArgs<'a>` field names match throughout Phase 3-5. `EncodeArgs.network` and `EncodeArgs.network_str` introduced together in Phase 2 Task 2.2 and used consistently. `CliNetwork::as_str()` defined in Phase 2 Task 2.1 used in Phase 2 Task 2.2 dispatch and Phase 3 Task 3.2 dispatch.

Test count math: 340 baseline + 5 (Phase 1) + 3 (Phase 2) + 5 (Phase 3) + 8 (Phase 4 incl. wsh-multi) + 3 (Phase 5) = 364. Matches Phase 6 verification.
