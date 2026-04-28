# Phase 5 Implementer Report — BIP Doc Edits

**Date**: 2026-04-27
**Branch**: `feature/v0.4-bip388-modern-surface`
**Commit**: `f8b8018`
**Status**: COMPLETE

## File modified

- `bip/bip-mnemonic-descriptor.mediawiki` — 164 insertions, 48 deletions

## Edits applied (8 total)

1. **Edit 1**: Replaced lines 67-73 scope paragraph with condensed prose + italic forward reference to §"Top-level descriptor scope" and §"Frequently Asked Questions".
2. **Edit 2**: Inserted new top-level section `==Top-level descriptor scope==` (lines 73–112) with allow-list (wpkh/wsh/sh-wpkh/sh-wsh/tr) and reject-list framed as "narrower than BIP 388".
3. **Edit 3**: Inserted subsection `===Sh wrapper restriction matrix===` under §Top-level scope with wikitable (peek-before-recurse, MUST mandate).
4. **Edit 4** (bonus — also added §"Default derivation paths" as subsection): Tag table gained "Disposition" column for all rows; 6 rows carry non-empty dispositions (Pkh REJECTED, Sh ACTIVE per matrix, Wpkh ACTIVE, Wsh ACTIVE, Bare REJECTED, TapTree RESERVED).
5. **Edit 5**: New `===Default derivation paths===` subsection with 5-row table (wsh/tr existing + wpkh/sh-wpkh/sh-wsh new).
6. **Edit 6**: New top-level `==Frequently Asked Questions==` section (7 Q&A) inserted between §Rationale and §Backwards Compatibility.
7. **Edit 7**: Two `<!-- TODO Phase 7: ... -->` markers inserted at v0.2.json SHA line and family-stable note line.
8. **Edit 8**: No-op — no "v0.4 decoders MUST" framing existed in the file.

## Verification

- `grep -n 'Top-level descriptor scope'` → line 73 (section heading)
- `grep -n 'Sh wrapper restriction matrix'` → line 114
- `grep -n 'Frequently Asked Questions'` → line 798
- `grep -nc 'TODO Phase 7'` → 2
- Table balance: 9 `{|` opens, 9 `|}` closes — balanced
