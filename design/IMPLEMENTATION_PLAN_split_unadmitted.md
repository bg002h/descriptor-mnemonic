# IMPLEMENTATION PLAN — md-codec 0.48.4: `chunk::split_unadmitted`

> **Status: R0 GREEN at r2** (r2: 0C/0I/1M/3N, folded; r1: 0C/3I/5M/4N, folded; `agent-reports/R0-split-unadmitted-plan-r1.md`). Author: the Refugium "Build process"
> thread, 2026-09-28. This is phase **P0** of the mr plan
> (`bg002h/mnemonic-refugium` `design/IMPLEMENTATION_PLAN_mr_v0_1.md`,
> PR #3).
>
> **Why:** the Refugium spec (`SPEC_refugium_mr1.md` §9 Q17, closed by
> the owner 2026-09-28) needs md-codec to print the (0,0) md1 card of an
> `mr1` card even when md's mint policy would refuse it, because an `mr1`
> card that already exists must always be readable and printable. A
> single unadmitted string already exists as
> `codex32::wrap_payload(encode_payload_unadmitted(d))`; the chunked form
> does not, because `chunk::split` calls the admitting `encode_payload`.
>
> **Baseline:** `18b4a76` (md-codec 0.48.3, md-cli 0.20.3).

## Machine-verified facts

- `chunk.rs:241 split` calls `encode_payload(d)` (l.246), the admitting
  encoder; its chunk-set id comes from `compute_md1_encoding_id` (l.249),
  which already uses the non-admitting `encode_payload_for_identity`
  (`identity.rs:40`).
- `encode.rs:198 encode_payload_unadmitted` is public and emits the same
  bytes as `encode_payload` whenever the latter succeeds (both call
  `encode_payload_inner`; admission only adds refusals, `encode.rs:242-253`).
- `lib.rs:53-56` re-exports `split` at the crate root.
- `tests/mint_policy_does_not_reach_decode.rs` has `contradictory_card()`,
  an F-217 shape that `encode_payload` refuses and decode reads.
- md-cli pins `md-codec = { path = "../md-codec", version = "=0.48.3" }`
  (`crates/md-cli/Cargo.toml:28`).

## Change

1. `chunk.rs`: move the body of `split` into a private
   `split_with(d, admitted: bool)` that calls `encode_payload` or
   `encode_payload_unadmitted`; `split(d)` = `split_with(d, true)`
   (unchanged behaviour); new public `split_unadmitted(d)` =
   `split_with(d, false)`. Re-export it from `lib.rs` beside `split`.
2. **Doc comments state the rule this adds.** `encode_payload_unadmitted`
   says "never for minting"; `split_unadmitted` emits card strings. Both
   docs now say: re-emitting a card that already exists (printing the
   md1 carried inside an existing `mr1` card) is not minting; the mint
   gate is the caller's job, and a tool that creates a new card must use
   `split` / `encode_md1_string`. `split_unadmitted`'s doc also says that
   structural errors (and the > 64-chunk refusal) still surface, and that
   when a descriptor fails both an admission rule and the chunk cap,
   `split` reports the admission error first and `split_unadmitted` the
   cap. The `mr` tool's `create` mints through the admitting `split`;
   only its re-emit paths use `split_unadmitted` (recorded in the mr plan).
3. Version 0.48.3 → **0.48.4** (additive: a patch under Cargo semver),
   md-cli's pin → `=0.48.4`, **`Cargo.lock` regenerated**, CHANGELOG
   `## md-codec [0.48.4]` entry, all in one commit titled
   `md-codec 0.48.4: chunk::split_unadmitted` (convention of 53718d1).
   md-cli is not released: its pin moves with no md-cli version (as in
   0.45.1). No MIGRATION entry. Nothing else embeds the version
   (`crate_pin.rs` pins mnemonic-io-lib; vectors carry no md-codec
   version; md-codec is a path dependency, so vendor-freshness is
   unaffected).
4. `design/FOLLOWUPS.md`, in the file's entry template: a companion
   entry `refugium-q17-unadmitted-split` (tier `cross-repo`, `resolved (md-codec 0.48.4)`)
   with a `Companion:` line naming the primary entry by its id
   `mr-md-codec-unadmitted-split` in `bg002h/mnemonic-refugium`, which
   the mr work adds to `mnemonic-refugium`'s tracker (created in its M1)
   with a `Companion:` line back; and a note entry
   `refugium-liana-key-per-member` recording that md-codec's Liana key
   stays `<0;1>` by its own spec (`to_miniscript.rs:521-528`) and the
   `mr` tool derives per-member keys itself (no md-codec change).
5. Manual: no md-cli flag changes, so no manual update.

## Tests (RED first, file `tests/split_unadmitted.rs`, prefix `split_unadmitted_`)

| Row | Asserts |
|---|---|
| `split_unadmitted_matches_split_on_corpus` | the corpus loaded as `tests/common` does (`include!` of `tests/common/vendored.rs`); asserts at least 60 vectors load, so the test cannot pass on an empty list; for each, `split_unadmitted` returns strings identical to `split`'s |
| `split_unadmitted_accepts_f217_shape` | `contradictory_card()` (copied into the test): `split` errs with `OriginKeyContradiction`, `split_unadmitted` succeeds |
| `split_unadmitted_roundtrip_f217` | its output reassembles with `reassemble` to a descriptor equal to the input |
| `split_unadmitted_accepts_sortedmulti_a_unspendable` | the SPEC §6 kind-1 `sortedmulti_a` card: `split` errs with `UnspendableWithSortedMultiA`, `split_unadmitted` succeeds and round-trips |
| `split_unadmitted_chunk_set_id_of_refused_shape` | on the F-217 card, every header's chunk-set id equals `derive_chunk_set_id(&compute_md1_encoding_id(d)?)` |

Fixtures: copy `contradictory_card()` and `kind_1_card_with_a_sortedmulti_a_leaf()`
(`tests/mint_policy_does_not_reach_decode.rs:130`) with their imports
**except** `use md_codec::encode::Descriptor;`, which clashes with the
`use md_codec::Descriptor;` inside the `include!`d `vendored.rs` (E0252).
Row 5 uses `.expect`, not `?`. The whole file fails to compile before the
change; confirm rows 2 and 4 are behavioural REDs by first landing a stub
`split_unadmitted` that just calls `split` and watching them fail. The
file carries a `//!` doc so `missing_docs` stays quiet.

Build gate: `cargo nextest run --locked --workspace` (or `cargo test
--locked --workspace` where nextest is not installed), `cargo test
--locked --doc`, `cargo fmt --check`, `cargo clippy --locked --workspace
--all-targets --all-features -- -D warnings`, and the row-scoped run
`cargo nextest run --locked -E 'test(split_unadmitted_)'` matching 5.

## Out of scope

Any change to `split`, `encode_md1_string`, admission rules, or the
Liana key; a release tag (the owner tags; the mr workspace pins a git rev
until then).
