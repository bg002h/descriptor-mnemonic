# Phase 8 — generate + commit v0.1.json (Tasks 8.5, 8.6)

**Status:** DONE
**Commit:** `e2e8368` (e2e8368e51618ad82073c46faa7799fddb86e082)
**File(s):** `crates/wdm-codec/tests/vectors/v0.1.json` (NEW); read `crates/wdm-codec/src/bin/gen_vectors.rs` to drive the generation
**Role:** controller (no subagent — this is operational work the controller does after the implementer ships the binary)

## Summary

Ran `gen_vectors --output crates/wdm-codec/tests/vectors/v0.1.json` after Phase 8 code (commit `f241025`) shipped, verified the output via `--verify` (typed structural compare passed), inspected the JSON (482 lines, well-formed, 10 positive + 30 negative vectors), and committed.

## Generation results

```
$ cargo run -q -p wdm-codec --bin gen_vectors -- --output crates/wdm-codec/tests/vectors/v0.1.json
gen_vectors: wrote 10 vectors + 30 negative vectors to crates/wdm-codec/tests/vectors/v0.1.json

$ cargo run -q -p wdm-codec --bin gen_vectors -- --verify crates/wdm-codec/tests/vectors/v0.1.json
gen_vectors: PASS — committed file matches regenerated vectors (10 positive, 30 negative)
```

## Content hash

`1957b542ed0388b51f01a7b467c8e802942dc6d6507abffaefaf777c90f3cd2c` (SHA-256 of the committed file). Used by Task 8.8's BIP draft permalink section.

## Test impact

Phase 8 code (`f241025`) added the `vectors_schema.rs` integration test with a "committed-file guard" that skips when `tests/vectors/v0.1.json` is absent. After this commit, the guard activates — confirmed by running `cargo test -p wdm-codec --test vectors_schema` which shows 7 tests passing (up from 7 with one previously skipping, now active).

## Follow-up items

None from this controller-direct task; the implementer's `8-negative-fixture-placeholder-strings` follow-up (FOLLOWUPS.md, v0.1-nice-to-have) covers the 2 placeholder negative-vector inputs that don't fit the WDM-string shape (EmptyChunkList, PolicyTooLarge).
