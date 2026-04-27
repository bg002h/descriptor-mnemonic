# Phase A bucket A review — Opus 4.7

**Status:** APPROVE_WITH_FOLLOWUPS
**Subject:** commit `fbbe6ec` (`p4-chunking-mode-enum`)
**Reviewer model:** Opus 4.7 via general-purpose subagent
**Stage:** combined spec compliance + code quality (single pass)
**Role:** reviewer

## Findings

### Spec deviations

(none) — every dispatch requirement met. Wire format unchanged; vectors verify.

### Quality blockers

(none)

### Quality important

(none)

### Quality nits (5)

- **N-1**: `chunking.rs:345` uses `if matches!(mode, ChunkingMode::Auto)` — non-idiomatic for a 2-variant `Copy + Eq` enum and not future-proof against new variants (Phase D taproot may add `MaxChunkBytes(u8)` per BIP §"Chunking" line 438). Replace with exhaustive `match`. **(Applied inline by controller in fixup commit.)**
- **N-2**: 3 stale test names + inline comments in `chunking.rs` (1072, 1164, 1178) and `decode.rs:231` (`force_chunking_opts`) still use the pre-rename terminology. Test-only cosmetic. **(Filed as `p4-chunking-mode-stale-test-names`; defer to a single sweep.)**
- **N-3**: same as N-2 (the `force_chunking_opts` helper). Folded into the same FOLLOWUPS entry.
- **N-4**: `options.rs:34-36` field rustdoc could cross-reference `ChunkingMode` directly, not just `with_force_chunking`. Nit; folded into N-2's sweep.
- **N-5**: `policy.rs:461` doc-link fix line is now ~120 chars vs surrounding ~80. Reflow optional. Folded.

### Forward-looking note

`Auto` / `ForceChunked` shape leaves room for additive variants without re-breaking the API — good for Phase D (Taproot) and BIP §"Chunking" `MaxChunkBytes(u8)` extension. Keeping `with_force_chunking(bool)` as the only builder is correct for v0.2; once a third variant lands, add `with_chunking_mode(ChunkingMode)`. **(Filed as `p4-with-chunking-mode-builder`.)**

## Disposition

| Finding | Action |
|---|---|
| N-1 (matches→match) | Applied inline in controller fixup commit |
| N-2/N-3/N-4/N-5 (test-name + comment sweep) | New FOLLOWUPS: `p4-chunking-mode-stale-test-names` |
| Forward-look (`with_chunking_mode` builder) | New FOLLOWUPS: `p4-with-chunking-mode-builder` |

## Verdict

APPROVE_WITH_FOLLOWUPS — no rework required; bucket A clear to integrate.
