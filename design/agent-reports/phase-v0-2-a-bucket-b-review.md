# Phase A bucket B review — Opus 4.7

**Status:** APPROVE_WITH_FOLLOWUPS
**Subject:** commit `86ca5df` (`6a-bytecode-roundtrip-path-mismatch`)
**Reviewer model:** Opus 4.7 via general-purpose subagent
**Stage:** combined spec compliance + code quality (single pass)
**Role:** reviewer

## Findings

### Spec deviations

(none) — every Phase A spec requirement met. Wire format unchanged; vectors verify. New test uses `m/48'/0'/0'/2'`, distinguishing from both BIP 84 (the v0.1 fallback) and `m/44'/0'/0'` (the dummy-key origin) — catches both bugs the fix targets.

### Quality blockers

(none)

### Quality important (2)

- **Q-1**: `design/FOLLOWUPS.md:106` still shows `6a-bytecode-roundtrip-path-mismatch` as `Status: open` even though commit `86ca5df` claims `closes 6a-...`. Per the parallel-batch convention this is the controller's job. **(Closed by controller in this aggregation commit.)**
- **Q-2**: `WalletPolicy` derives `PartialEq, Eq` (`policy.rs:171`). With the new `decoded_shared_path` field, two logically-equivalent policies — one from `parse()` (`None`) and one from `from_bytecode()` (`Some(...)`) — now compare unequal. No in-tree test exercises this exact pair (existing equality test at `policy.rs:571` is parse↔parse), but downstream consumers comparing `WalletPolicy` across construction paths see a behavioral break. Recommended fix: rustdoc note on the field + MIGRATION.md follow-up. Hand-writing `PartialEq` to ignore the field is rejected because it would risk Hash/Eq inconsistency if `Hash` is ever derived later. **(Rustdoc note applied inline in controller fixup commit; MIGRATION.md addition filed as `wallet-policy-eq-migration-note` for Phase G.)**

### Quality nits (4)

- **N-1**: `policy.rs:298-312` precedence-chain rustdoc lists three tiers; Phase B will need to insert `EncodeOptions::shared_path` as tier 0. Implementer's report already flags this. (No new FOLLOWUPS entry — it's part of Phase B's natural scope.)
- **N-2**: Field rustdoc uses Unicode `→` while the file mixes `→` and `->`. Cosmetic.
- **N-3**: Test name 60+ chars; verbose but documents the dual assertion. Acceptable.
- **N-4**: `policy.rs:357` `clone().unwrap_or_else(...)` clones eagerly. Could be `as_ref().cloned()`-restructured but `DerivationPath` clone is cheap. Leave as-is.

## Disposition

| Finding | Action |
|---|---|
| Q-1 (close 6a in FOLLOWUPS) | Done by controller in this aggregation commit |
| Q-2 (Eq semantics rustdoc) | Applied inline in controller fixup commit |
| Q-2 (MIGRATION.md follow-up) | New FOLLOWUPS: `wallet-policy-eq-migration-note` |
| N-1 (Phase B rustdoc update) | Implicit in Phase B scope; no FOLLOWUPS entry needed |
| N-2/N-3/N-4 | Acknowledged; no action |

## Verdict

APPROVE_WITH_FOLLOWUPS — no rework required; bucket B clear to integrate.
