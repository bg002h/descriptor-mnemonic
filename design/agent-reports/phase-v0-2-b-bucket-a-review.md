# Phase B bucket A review — Opus 4.7

**Status:** APPROVE_WITH_FOLLOWUPS
**Subject:** commit `5f13812` (`5e-checksum-correction-fallback`)
**Reviewer model:** Opus 4.7 via general-purpose subagent
**Stage:** combined spec compliance + code quality (single pass)
**Role:** reviewer

## Findings

### Spec deviations

(none) — every dispatch criterion met. Wire format unchanged; vectors verify.

### Quality blockers

(none)

### Quality important

(none) — coordinate-system translation correct; `corrected_positions` from `bch_correct_*` over `data_with_checksum` indexes the same slice the new `corrected_char_at` reads.

### Quality nits (3)

- **N-1 (rustdoc)**: `encoding.rs:564-565` says BCH "never reports a `corrected_position` outside the data part" — ambiguous because `data_with_checksum` includes the checksum region while elsewhere "data part" excludes it. Disambiguate. **(Applied inline by controller in fixup commit.)**
- **N-2 (test rigor)**: `decode.rs:619-620` `q ↔ p` flip; in single-error case the assertion is correct, but the prompt could be tightened. Already mostly handled by belt-and-braces guard. No action.
- **N-3 (forward-look)**: the `corrected_char_at` accessor makes Phase F's dynamic negative vectors easier (can assert exact characters at checksum-region positions). No action; positive note.

### Memory note

Storing both `data` and `data_with_checksum` doubles symbol-array footprint (~26 bytes for Regular, ~30 for Long, plus Vec overhead). Negligible at v0.1 scale. An obvious micro-opt: drop `data` field, replace with a `data()` accessor backed by `data_with_checksum[..len-checksum_len]`. **(Filed as `decoded-string-data-memory-microopt` for v0.3 breaking-API window.)**

## Disposition

| Finding | Action |
|---|---|
| N-1 (rustdoc disambiguation) | Applied inline in controller fixup commit |
| N-2 (test rigor) | Acknowledged; no action |
| N-3 (Phase F note) | Noted; no action |
| Memory micro-opt | New FOLLOWUPS: `decoded-string-data-memory-microopt` (v0.3) |

## Verdict

APPROVE_WITH_FOLLOWUPS — bucket A clear to integrate.
