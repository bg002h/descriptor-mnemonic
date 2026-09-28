Verdict: 0 Critical, 0 Important, 1 Minor, 3 Nit

# R0 r2: `IMPLEMENTATION_PLAN_split_unadmitted.md`

Reviewer: independent R0 subagent, 2026-09-28. HEAD `d0c0131`, working tree clean. This round is a read-only check of the plan text against the source. No prototype was re-run, because r1's prototype already covers the unchanged code change.

## r1 findings: all resolved

| r1 | Where the plan resolves it | OK |
|---|---|---|
| I1 lock and one commit | Change §3: `Cargo.lock` regenerated; single commit `md-codec 0.48.4: ...` (convention of 53718d1) | yes |
| I2 row 1 source | Row 1 now uses the `include!` of `tests/common/vendored.rs`, with a floor of at least 60 | yes (see below) |
| I3 doc contract | Change §2 amends both docs, keeps the mint gate as the caller's duty, and says `mr create` mints through `split` | yes |
| M1 class coverage | New row `split_unadmitted_accepts_sortedmulti_a_unspendable` | yes. F-218 was skipped, which r1 marked optional |
| M2 row 4 | Row 5 now checks the chunk-set id on the refused shape | yes |
| M3 md-cli release | No md-cli version; pin moves (as in 0.45.1) | yes |
| M4 doc tests and `missing_docs` | `cargo test --locked --doc` is in the gate; the file carries a `//!` doc | yes |
| M5 FOLLOWUPS | Uses the template; `Companion:` lines both ways; `to_miniscript.rs:521-528` cited | yes (see N2) |
| N1 citation | Now `encode.rs:242-253` | yes |
| N2 structural errors | Change §2 states the error precedence | yes |
| N3 release surface | Change §3 | yes |
| N4 clippy `--workspace` | In the gate | yes |

## Checks on the new text

**Is `vendored.rs` include!-able, and does it give 60 or more?** Yes.
- The same `include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/common/vendored.rs"))` form is already used by `wire_version_8.rs`, `network_version_bytes.rs`, `liana_unspendable.rs` and `liana_to_miniscript.rs`.
- It provides `all_vendored_vector_names`, `load_vendored_phrase` and `decode_vendored`.
- `tests/vectors/*.phrase.txt` holds 68 files, and r1 measured all 68 as decoded and accepted by `split`. The floor of 60 holds, and stays non-vacuous with slack.

**Does a sortedmulti_a fixture exist?** Yes.
- `tests/mint_policy_does_not_reach_decode.rs:130` `kind_1_card_with_a_sortedmulti_a_leaf()` is self-contained. It uses only public `tree`/`tag`/`origin_path`/`use_site_path`/`tlv` types, and its own doc says to copy it rather than share it.
- r1 measured it: `split` gives `UnspendableWithSortedMultiA`; `split_unadmitted` gives 1 chunk that round-trips.
- `contradictory_card()` needs `bitcoin::secp256k1`, which md-codec's existing tests already use.

**Row 5 building blocks are public:**
- `derive_chunk_set_id` is re-exported at the root.
- `codex32::unwrap_string` is at `codex32.rs:139`.
- `ChunkHeader::read` is at `chunk.rs:69`, with the `pub chunk_set_id` field.

**Row-scoped filter.** No existing test name contains `split_unadmitted`, so `-E 'test(split_unadmitted_)'` matching 5 is exact. The root is a virtual workspace, so running without `-p` covers md-codec.

**Doc-comment rule.** The rule is sound and consistent with FOLLOWUPS `encode-time-policy-reaches-decode-via-the-encoding-id`.

## Minor

**M1. Importing `Descriptor` will fail to compile.** `vendored.rs:20` already does `use md_codec::Descriptor;`. The two copied fixtures come from a file that does `use md_codec::encode::Descriptor;`. Copying that import next to the `include!` gives E0252, which `vendored.rs`'s own header warns about. The plan should tell the implementer to drop the fixture file's `Descriptor` import and keep its other imports (`Tag`, `Body`/`InternalKey`/`Node`, the `origin_path` types, `UseSitePath`, `TlvSection`). The fix is cheap, but it is exactly the kind of error an implementer hits cold.

## Nit

**N1.** Row 5 uses `compute_md1_encoding_id(d)?`, so that test fn needs a `-> Result<(), md_codec::Error>` return type, or should use `.expect`.

**N2.** A FOLLOWUPS entry's status `resolved <COMMIT>` cannot name the commit it is part of. Write `resolved (md-codec 0.48.4)` in the commit, or fill in the SHA in a follow-up. The companion's `Companion:` line should also name the primary's id explicitly, since that entry does not exist yet.

**N3.** "Rows 1, 3 and 5 RED only by not compiling; rows 2 and 4 behavioural" is correct in spirit. Before the change, though, the whole file fails to compile. The behavioural RED for rows 2 and 4 means they fail against a stub `split_unadmitted = split`. Consider saying that the implementer should confirm this red with a stub.

## Verdict

GREEN to implement once M1 is folded in (a one-line instruction). The nits are optional.
