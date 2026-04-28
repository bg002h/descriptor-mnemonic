# Phase B bucket C review — Opus 4.7

**Status:** APPROVE_WITH_FOLLOWUPS
**Subject:** commit `231574d` (`7-serialize-derives`)
**Reviewer model:** Opus 4.7 via general-purpose subagent
**Stage:** combined spec compliance + code quality (single pass)
**Role:** reviewer

## Findings

### Spec deviations

(none) — every Bucket C dispatch requirement met.

- Strategy A confirmed: bin-private wrapper module; no serde derives sneaked onto `WalletPolicy`, `WdmBackup`, `EncodedChunk`, `DecodeResult`, `DecodeReport`, `Correction`, `Verifications`.
- File layout: `bin/wdm.rs` → `bin/wdm/main.rs`; new `bin/wdm/wdm_json.rs`; `Cargo.toml` `[[bin]] wdm.path` updated.
- All 7 wrapper types present with `From<&LibraryType>` impls and full `Serialize + Deserialize` round-trip.
- Field-set diff vs pre-commit `json!{}` literals: zero fields dropped, zero added.
- BTreeMap-backing claim verified: no `preserve_order` feature on `serde_json 1.0.149` in `Cargo.lock`. Wrapper struct fields are alphabetical so `to_string_pretty` reproduces v0.1.1 output byte-for-byte.
- Existing `tests/cli.rs` integration tests still pass; +2 new shape-stability tests.
- 8 unit tests in the bin module's `tests` (all pass); includes round-trip-via-serde tests.
- Wire format unchanged.

### Quality blockers

(none)

### Quality important

(none)

### Quality nits (4)

- **N-1**: `wdm_json.rs:75` `impl From<BchCode> for BchCodeJson` takes by value while every other `From` impl in the file borrows. `BchCode` is `Copy` so harmless, but inconsistent. **(Applied inline by controller in fixup commit — `From<&BchCode>` for consistency; call-site updated to `(&c.code).into()`.)**
- **N-2**: Module name `wdm_json` produces `wdm::wdm_json` path — stutter. Rust convention prefers `mod json;` at `bin/wdm/json.rs`. **(Applied inline by controller in fixup commit — `git mv` + updated `mod` declaration + 2 doc-comment refs.)**
- **N-3**: `confidence_debug` / `outcome_debug` use `format!("{c:?}")` to lock in the v0.1.1 contract. Couples the JSON contract to the library's `Debug` impl. A `serde(rename_all = "PascalCase")` enum mirror would be more durable. **(Filed as `cli-json-debug-formatted-enum-strings` for v1.0 stabilization.)**
- **N-4**: Phase E (fingerprints CLI exposure) will need a wrapper field; alphabetical order means `fingerprints` slots cleanly between `chunks` and `wallet_id_words`. Positive note; no action.

## Disposition

| Finding | Action |
|---|---|
| N-1 (signature consistency) | Applied inline in controller fixup commit |
| N-2 (rename `wdm_json.rs` → `json.rs`) | Applied inline in controller fixup commit |
| N-3 (Debug-formatted enums) | New FOLLOWUPS: `cli-json-debug-formatted-enum-strings` (v1.0) |
| N-4 (Phase E wrapper field) | Acknowledged; positive note |

## Verdict

APPROVE_WITH_FOLLOWUPS — bucket C clear; Phase B closes.
