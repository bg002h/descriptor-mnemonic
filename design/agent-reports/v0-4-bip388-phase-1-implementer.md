# Phase 1 Implementer Report — v0.4 BIP 388 Encoder Lift

**Status:** DONE

## Files changed

```
crates/md-codec/src/bytecode/encode.rs
```

## What was done

### Tasks 1.1–1.5: Encoder arms lifted (TDD)

- Added `ShInner` to the import line alongside `Wsh`.
- Replaced the old flat `Descriptor::Wpkh(_)` and `Descriptor::Sh(_)` rejection arms with structured dispatch:
  - `Descriptor::Wpkh(wpkh)` — emits `Tag::Wpkh` then delegates to `wpkh.as_inner().encode_template()`
  - `Descriptor::Sh(sh)` — emits `Tag::Sh` then matches `sh.as_inner()`:
    - `ShInner::Wpkh(wpkh)` — emits `Tag::Wpkh` + inner key
    - `ShInner::Wsh(wsh)` — emits `Tag::Wsh` + reuses existing `Wsh::encode_template`
    - `ShInner::Ms(_)` — returns `PolicyScopeViolation` mentioning "legacy P2SH"
  - `Descriptor::Pkh(_)` and `Descriptor::Bare(_)` — now share a single "top-level pkh()/bare()" message

Three positive TDD tests (written before implementation, verified FAIL then PASS):
- `encode_wpkh_single_key`
- `encode_sh_wpkh_single_key`
- `encode_sh_wsh_sortedmulti_2_of_3`

Two pre-existing tests renamed for accuracy (were testing inline-key rejection, not top-level rejection):
- `rejects_wpkh_top_level` → `rejects_wpkh_inline_key`
- `rejects_sh_top_level` → `rejects_sh_wpkh_inline_key`

### Task 1.6: Restriction-matrix negative tests (5 tests)

- `encode_rejects_sh_multi_legacy_p2sh`
- `encode_rejects_sh_sortedmulti_legacy_p2sh`
- `encode_rejects_top_level_pkh`
- `encode_rejects_top_level_bare`
- `encode_rejects_sh_via_inner_ms_arbitrary_miniscript`

All 5 constructed via `Descriptor::from_str()` parser route as specified (no non-existent constructors used).

## Test counts

```
test result: ok. 400 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 42 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 managed; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
```

Total: 574 passing (baseline 566 + 3 positive + 5 negative + 0 net from 2 renamed = +8).

## Gate results

- `cargo build --workspace --all-targets`: PASS
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS
- `cargo fmt --check`: PASS

## Concerns

None. The `Wsh::encode_template` method existed at `encode.rs:147-155` and was reused unchanged for `ShInner::Wsh`. `ShInner` import was clean. The two pre-existing tests that asserted "PolicyScopeViolation mentioning wpkh/sh" were updated to accurately reflect v0.4 semantics (they test inline-key rejection, which is what actually happens with wpkh/sh(wpkh) + empty_map now).
