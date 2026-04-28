# Phase 2 Implementer Report — Decoder: top-level dispatch + Wpkh/Sh inner

**Status**: DONE

## Files changed

```
crates/md-codec/src/bytecode/decode.rs | 368 insertions(+), 36 deletions(-)
```

## What was implemented

### Tasks 2.1–2.5: TDD round-trip tests + implementations

Three new decode helpers added between the existing `decode_wsh_inner` and `decode_tr_inner`:

- `decode_wpkh_inner`: reads one placeholder, calls `Descriptor::new_wpkh(key)`.
- `decode_sh_inner`: peeks next byte before recursive descent (recursion-bomb
  defense), dispatches `Tag::Wpkh` → `Descriptor::new_sh_wpkh(key)` and
  `Tag::Wsh` → `decode_wsh_body` + `Descriptor::new_sh_with_wsh(wsh)`.
  All other inner tags produce `PolicyScopeViolation`.

Top-level `decode_descriptor` updated: `Tag::Wpkh` and `Tag::Sh` promoted
to active arms; `Tag::Pkh`/`Tag::Bare` moved to a shared rejection arm with
v0.4-prefixed message; unknown bytes now emit `InvalidBytecode { UnknownTag }`
directly via `None` match arm (no intermediate `.ok_or()` needed).

Three new round-trip tests verified TDD red → green:
- `decode_wpkh_round_trip`
- `decode_sh_wpkh_round_trip`
- `decode_sh_wsh_sortedmulti_round_trip`

### Task 2.6: Restriction-matrix tests (9 tests)

All 9 decode-side rejection tests pass. The two "lower-level API" tests
(`decode_rejects_sh_inner_script_andv`, `decode_rejects_sh_key_slot_placeholder`)
were implemented via hand-rolled bytecode fed directly to `decode_template`
(which is the actual lower-level path — `WalletPolicy::from_bytecode` adds a
header layer that is unnecessary for this unit test).

Two pre-existing tests had string-match assertions updated to match the
v0.4-prefixed error messages:
- `decode_rejects_top_level_pkh`: `"Pkh"` → `"pkh"`
- `decode_rejects_non_top_level_fragment_at_top`: `"top level"` → `"top-level"`

### Module doc

Updated the crate-level doc comment in `decode.rs` to describe v0.4 scope
(Wpkh/Sh dispatch, restriction matrix, legacy rejection).

## Test counts

```
test result: ok. 412 passed; 0 failed  (lib unit tests)
... (integration + doc tests all pass)
TOTAL: 586 (baseline 574 + 12 new)
```

## Gate results

- **build**: `cargo build --workspace --all-targets` — PASS
- **test**: `cargo test -p md-codec` — 586/586 PASS
- **clippy**: `cargo clippy --workspace --all-targets -- -D warnings` — PASS
- **fmt**: `cargo fmt --check` — PASS

## Commit SHA

`9e81604` on `feature/v0.4-bip388-modern-surface`

## Concerns

None. The dispatch site was exactly as described in the pre-flight context.
`Descriptor::new_sh_with_wsh` is infallible (takes `Wsh<Pk>`) — confirmed
by the miniscript v13 API. The `Ok(x.map_err(...)?)` pattern triggered
clippy's `clippy::question_mark` lint; refactored to `let x = ...; Ok(x)`.
