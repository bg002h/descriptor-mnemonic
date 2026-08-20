# CLAUDE.md — md1 (`descriptor-mnemonic`) repo notes for Claude Code sessions

This file is auto-loaded by Claude Code when starting a session in this repository.

## Cross-repo coordination with `bg002h/mnemonic-key` (mk1) and `bg002h/mnemonic-secret` (ms1)

mk1 is a sibling format (HRP `mk`, codex32-derived xpub backup) developed in `/scratch/code/shibboleth/mnemonic-key`. md1 and mk1 share the BCH plumbing (BIP 93 polynomials with HRP-mixing + per-format target residues — *forked*, not shared as a crate) and are designed to engrave alongside each other for foreign-xpub multisig recovery.

ms1 is a third sibling format (HRP `ms`, repo `bg002h/mnemonic-secret`) added 2026-05-03 for the secret-material slot (BIP-39 entropy / BIP-32 master seed / xpriv). Unlike md1↔mk1's forked BCH, ms1 adopts BIP-93 codex32 *directly* via Andrew Poelstra's `rust-codex32`. The three formats engrave together as a coherent backup bundle: md1 = template/policy, mk1 = xpubs, ms1 = secret. v0.1 of all three is single-string (threshold = 0); K-of-N share encoding is planned across the family in v0.2+, ms1 first because BIP-93 already specifies the math.

The previously-planned `mc-codex32` shared-crate extraction (originally gated on "both md+mk at v1.0 with cross-validated vectors") is **RETIRED as of 2026-05-03**: ms1 doesn't need it (uses `rust-codex32` directly), and md1↔mk1's HRP-mixed BCH isn't upstreamable to that crate. md1↔mk1 BCH stays forked indefinitely; the *pattern* will be documented in a future cross-repo `PATTERNS.md`. See `design/FOLLOWUPS.md` entry `mc-codex32-extraction-retired-2026-05-03` for the full record.

**Cross-repo follow-up convention:** when mk1 work surfaces an action item that affects md1 (rename, missing dictionary entry, wire-format extension, process invariant), the item is mirrored here:

- A primary entry lives in mk1's `design/FOLLOWUPS.md` at tier `cross-repo`.
- A companion entry is mirrored into this repo's `design/FOLLOWUPS.md` so md-codec sessions discover the action item natively from this repo's tracker.
- Both entries cite each other (`Companion:` line in each).
- When the md1-side action ships, both entries are updated in lockstep: the md1 entry is marked `resolved <COMMIT>`; the mk1 entry's `Status:` notes the resolving md1 commit.

**Currently open mk1-surfaced items affecting md1** (see `design/FOLLOWUPS.md` for full entries):

- (none — `md-per-at-N-path-tag-allocation` resolved in v0.10.0)

**Recently resolved:**

- **md-codec v0.10.0:** `md-per-at-N-path-tag-allocation` (Tag::OriginPaths = 0x36; header bit 3 reclaimed; per-`@N` divergent-path encoding shipped). mk1's companion `md-per-N-path-tag-allocation` closes in lockstep.
- **md-codec v0.9.0:** `chunk-set-id-rename`, `md-path-dictionary-0x16-gap`. mk1's BIP-submission gate was cleared in v0.9.0.

**Recently retired (invariants no longer in force):**

- **Path-dictionary mirror invariant (RETIRED).** md-codec v0.10.0 carried `Tag::OriginPaths = 0x36` (and v0.10.x its predecessor `Tag::SharedPath`) as a path-dictionary table compatible byte-for-byte with mk1's. md-codec v0.11 dropped path dictionaries from md1 entirely as part of the v0.11 wire-format cleanup — paths are now encoded explicitly via `OriginPath` (see `design/SPEC_v0_11_wire_format.md` §1.4: "Wire-layer dictionaries (path, use-site-path, shape). Considered and rejected for architectural cleanliness"). mk-codec v0.2.2 mirrored that retirement on the mk1 side: the prose mirror clause was dropped from mk-codec's path-dictionary doc-comments, mk-codec's SPEC, and the mk1 BIP draft, and mk1's path dictionary is now documented as **mk1-internal** (standalone). The mk1↔md1 lockstep mirror invariant — formalized in v0.9.0's stewardship contract and tracked as `path-dictionary-mirror-stewardship` in mk1's FOLLOWUPS — is therefore RETIRED. No md1-side change ships in this cross-repo coordination event; this is documentation-only on both sides.

## Manual coverage

The end-user manual for the m-format constellation lives in the sibling `bg002h/mnemonic-toolkit` repo at `docs/manual/`. The manual mirrors all four CLIs (`mnemonic`, `md-cli`, `ms-cli`, `mk-cli`) verbatim — `md-cli`'s mirror chapter is `docs/manual/src/40-cli-reference/42-md.md`. **Any flag/API change to `md-cli` in this repo must update that chapter in lockstep with the implementing PR.** The manual's `tests/lint.sh flag-coverage` step gates on missing flags. See `design/FOLLOWUPS.md` entry `manual-cli-surface-mirror` for the canonical record; primary entry lives in the toolkit repo.

## Other repo-specific notes

- **Default to ultracode (multi-agent orchestration) — refined policy** (2026-06-17, after an architect panel; verdict: keep default-ON, refine per-phase). Standing user directive, project-wide across the m-format constellation + seedhammer fork; does NOT require the per-turn `ultracode` keyword. **Default ON for every *substantial* task; token cost is not a constraint.** Trivial one-line/mechanical edits, version bumps, and plain Q&A run solo. **Per-phase pattern:** (1) **research/recon** — fan out parallel subagents; any agent handling **external protocol facts** (BIP-39, BCH/codec semantics, NDEF, RP2350 OTP, SDK behavior) MUST verify them against **authoritative source text**, not just the draft doc (guards against false-consensus on plausible-but-wrong facts — the "1 valid last word" class). (2) **design/spec/plan** — single author + the mandatory R0 loop. (3) **implementation** — a *single* subagent executes the GREEN plan in a worktree (NOT parallel re-implementations); TDD. (4) **post-implementation** — a **mandatory, non-deferrable** independent adversarial execution review over the whole diff (R0 = plan correctness; this catches implementation-introduced regressions TDD misses). (5) if Agent-API dispatch fails mid-session, **flag it explicitly** and defer the formal review to API recovery — never silently substitute inline self-review. Composes with — does not replace — the R0 gate; verbatim agent reports persist to `design/agent-reports/`.
- The reference implementation is in `crates/md-codec/`. Sibling crates: `crates/md-signer-compat/`.
- Implementation plans live in `design/IMPLEMENTATION_PLAN_v0_X_*.md`; per-phase opus reviews persist to `design/agent-reports/`.
- Per-phase TDD discipline: tests written before impl; the `superpowers:executing-plans` skill is the canonical sub-skill for plan execution in this repo.

## Parallel execution — this machine has 24 CPU cores

**Standing directive (2026-08-19): consider parallel execution for ALL tests,
cache generation and long calculations.** The defaults use almost none of the
box. Measured constellation-wide the same day: **824s → 204s (~4×)**.

- **Rust — `cargo nextest run --locked`**, not `cargo test`. `cargo test` runs
  each test *binary* serially; nextest spreads them over all cores. Per-repo
  measurements: mnemonic-toolkit 256s→49s, descriptor-mnemonic 40s→27s,
  mnemonic-engrave 33s→16s, mnemonic-secret 2s→0.3s. `cargo-nextest` 0.9.140 is
  installed.
- **Go — shard the package.** `-parallel` does NOTHING unless tests call
  `t.Parallel()`; the fork's `gui` package has 886 test funcs and zero of them.
  `mnemonic-engrave/scripts/gui-shard-test.sh <pkg> 24` took `./gui/` from 493s
  to 112s. It enumerates its partition from `go test -list` and **asserts the
  union is exhaustive before running**, so it cannot silently drop a test — any
  replacement must do the same.
- **Long independent work** — cache/corpus generation, fixture derivation, batch
  rendering — is a candidate too. Ask whether it is CPU-bound and independent
  before running it in a loop.

**Speed WITHOUT dropping debug_assertions.** Do NOT reach for `--release` to
speed tests up — it drops `debug_assertions` and overflow checks, so mutation
tests and invariant panics stop detecting things while still reporting green.
Raise the optimisation level instead and keep them:

```toml
[profile.test]
opt-level = 2

[profile.dev]
opt-level = 2
```

`debug-assertions` defaults to **true** on both profiles, so this is pure gain.
Measured on descriptor-mnemonic: execution **25.4s → 0.765s**, versus 0.775s for
`--release` — the same speed, with the checks intact. Verified empirically, not
inferred: at `opt-level = 2` both `cfg!(debug_assertions)` and an
`attempt to add with overflow` panic still fire. Cost is a slower first build of
dependencies, cached thereafter.

**Check what `/tmp` is before building there.** On this box it is a 32 GB tmpfs,
and a scratch worktree's `target/` filled it and killed a running test.

**Never run the same suite twice** to collect counts and failures separately.
Capture once to a file, then grep it — otherwise every measurement costs double.
