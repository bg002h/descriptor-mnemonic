# followup-batch-1-bucket-E — CLI Integration Tests

| Field   | Value                                                                 |
|---------|-----------------------------------------------------------------------|
| Status  | DONE                                                                  |
| Commit  | 1ccc1d4df798dafd0a5d44408ed9856fa517f9c8                             |
| File(s) | `crates/wdm-codec/Cargo.toml`, `crates/wdm-codec/tests/cli.rs`       |
| Role    | implementer                                                           |

## Closed short-ids

- `7-cli-integration-tests`

## What was done

Added `assert_cmd = "2"` and `predicates = "3"` to `[dev-dependencies]` in
`crates/wdm-codec/Cargo.toml`.  (`predicates` must be declared explicitly —
`assert_cmd` uses it internally but does not re-export it as a public crate.)

Created `crates/wdm-codec/tests/cli.rs` with 12 integration tests covering all
six `wdm` subcommands:

| # | Test name | Subcommand | Type |
|---|-----------|------------|------|
| 1 | `wdm_encode_default` | `encode` | happy |
| 2 | `wdm_encode_json` | `encode --json` | happy |
| 3 | `wdm_encode_force_chunked` | `encode --force-chunked` | happy |
| 4 | `wdm_decode_round_trip` | `encode` → `decode` | happy |
| 5 | `wdm_verify_match` | `verify --policy <same>` | happy |
| 6 | `wdm_verify_mismatch` | `verify --policy <different>` | happy |
| 7 | `wdm_inspect_outputs_chunk_header` | `inspect` | happy |
| 8 | `wdm_bytecode_outputs_lowercase_hex` | `bytecode` | happy |
| 9 | `wdm_encode_unparseable_policy_exits_nonzero` | `encode` invalid | error |
| 10 | `wdm_decode_invalid_string_exits_nonzero` | `decode` invalid | error |
| 11 | `wdm_vectors_returns_json_top_level_object` | `vectors` | happy |
| 12 | `wdm_unknown_subcommand_exits_nonzero` | bad subcommand | error |

## Test results

```
cargo test -p wdm-codec --test cli

running 12 tests
test wdm_decode_invalid_string_exits_nonzero ... ok
test wdm_encode_default ... ok
test wdm_encode_unparseable_policy_exits_nonzero ... ok
test wdm_encode_json ... ok
test wdm_bytecode_outputs_lowercase_hex ... ok
test wdm_inspect_outputs_chunk_header ... ok
test wdm_unknown_subcommand_exits_nonzero ... ok
test wdm_encode_force_chunked ... ok
test wdm_decode_round_trip ... ok
test wdm_verify_match ... ok
test wdm_verify_mismatch ... ok
test wdm_vectors_returns_json_top_level_object ... ok

test result: ok. 12 passed; 0 failed; 0 ignored
```

Clippy: clean (`cargo clippy --workspace --all-targets -- -D warnings`).

`cargo fmt --check`: The only diff reported is a pre-existing `use` import
reordering in `crates/wdm-codec/src/bytecode/decode.rs` (modified by another
parallel branch, not by this task).  `tests/cli.rs` itself is fmt-clean.

## Notes

- No bugs in `bin/wdm.rs` were surfaced by the tests; all subcommands behaved
  as expected.
- The `cli` feature is already in `default`, so `cargo test -p wdm-codec --test
  cli` builds the binary without any extra flags.
- `predicates` version `3` (latest) was chosen; `assert_cmd 2` pulls an older
  `predicates 2` transitively but Cargo resolves both via feature unification.
