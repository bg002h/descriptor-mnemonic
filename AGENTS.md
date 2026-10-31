# descriptor-mnemonic

Paths below are relative to this Git root, including when working inside a crate.

## Start here

Read [CLAUDE.md](CLAUDE.md) explicitly; AGENTS.md does not load it automatically.
Read applicable ancestor guidance too. Existing process rules remain applicable;
verify historical architecture/status statements against source.
Read the shared [guide](https://github.com/bg002h/mnemonic-toolkit/blob/a619f6eefdc8a2c58c9ff8f290376ed371a1c677/docs/architecture/README.md) and [workflow](https://github.com/bg002h/mnemonic-toolkit/blob/a619f6eefdc8a2c58c9ff8f290376ed371a1c677/docs/architecture/WORKFLOW.md).
For sibling checkouts/worktrees, prefer local `../mnemonic-toolkit/docs/architecture/`;
remote fallbacks pin a delivered guide revision. See its workflow for refreshing the pin.
Check Git status, the current branch, Cargo.toml and Cargo.lock before editing.
Preserve unrelated work and use explicit task-owned paths when staging.

## Where behavior lives

- [md-codec](crates/md-codec/src/lib.rs) owns canonical payloads, descriptor composition/rendering and identities.
- [md-cli](crates/md-cli/src/main.rs) owns parsing, decomposition and seating.
- Preserve exact payload bit lengths, sparse origins/key metadata, and the distinction
  between [encoding/template/wallet identities](crates/md-codec/src/identity.rs)
  and [SkeletonKey](crates/md-codec/src/skeleton.rs).
- Preserve the separation of new-artifact admission from decoding/recovering existing
  material. [Partial decode](crates/md-codec/src/decode.rs) is a specific missing-origin option.
- Seating must reject assignments yielding different wallets; read
  [matching](crates/md-cli/src/seat/matching.rs) and [disposition](crates/md-cli/src/seat/disposition.rs).

## Contracts and checks

Read [wire format](design/SPEC_v0_30_wire_format.md), [policy](design/SPEC_v0_13_wallet_policy.md)
and the affected newer design amendments; the shared source map links the me-hosted
wire-8 design and distinguishes its md implementation owner.
Use vector, wire-version, partial-decode, mint/recovery and seating tests.
[xtask](crates/xtask/src/main.rs) generates the verdict table from recorded
[coordinator evidence](crates/md-codec/tests/fixtures/coordinator/evidence.jsonl);
preserve the distinction between that input and the generated table.
Check the miniscript patch, lock, vendoring, and downstream mk/toolkit/me/shared Refugium
pins together. CLI changes require the shared manual and utility GUI mirrors.
