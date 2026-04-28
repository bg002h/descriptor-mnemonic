# Phase 7 Implementer Report — BIP Doc Updates

**Phase**: 7 (BIP doc updates)
**Agent**: implementer subagent
**Date**: 2026-04-28
**Commit**: `1d12102` on `feature/v0.5-multi-leaf-taptree`
**Status**: DONE

---

## Summary

All seven tasks of Phase 7 completed. `bip/bip-mnemonic-descriptor.mediawiki` updated for v0.5 multi-leaf TapTree admittance. No Rust code changed. `cargo build` clean. All tag pairs balanced.

---

## Task results

### Task 7.1 — §"Top-level descriptor scope": admit `tr(KEY, TREE)` — DONE

Replaced the `tr(KEY, SCRIPT)` RESERVED bullet with `tr(KEY, TREE)` admittance. Removed the RESERVED note and the "decoders MUST reject 0x08 with PolicyScopeViolation" normative text. New text: keypath-only, single-leaf, or multi-leaf; TREE may be a single tap-leaf or recursive TapTree (0x08) structure subject to BIP 341 depth-128 ceiling; Active (v0.5).

### Task 7.2 — §"Taproot tree" substantive rewrite — DONE

Replaced single-paragraph "exactly one tap-leaf / TapTree reserved for v1+" with a three-bullet enumeration of script-tree positions (absent / single tap-leaf / TapTree inner-node), followed by the recursive `[TapTree=0x08][LEFT_SUBTREE][RIGHT_SUBTREE]` bytecode description, BIP 388 curly-brace form as human-readable counterpart, and the peek-before-recurse + depth-128 MUST-reject normative. Also added a two-leaf bytecode example with ASCII art (`06 32 00 08 0c 1b 32 01 0c 1b 32 02`) and explanatory prose on recursive nesting.

### Task 7.3 — Tag table row 0x08 — DONE

Old: `TapTree (reserved for v1+) / RESERVED — admission deferred`
New: `TapTree / Multi-leaf TapTree inner-node framing (taproot script-tree subtree); recursive [TapTree=0x08][LEFT_SUBTREE][RIGHT_SUBTREE] (see §"Taproot tree") / Active (v0.5)`

Column count matches adjacent rows.

### Task 7.4 — FAQ updates — DONE

Three sub-tasks:
1. **Deferral Q&A preserved** — added "(Historical context — v0.4 deferral rationale.)" prefix to opening sentence; prose otherwise unchanged.
2. **New resolution Q&A added** — `===Why does v0.5 admit multi-leaf TapTree?===` inserted immediately after the deferral FAQ. Four numbered reasons (BIP 388 surface now stable; peek-before-recurse pattern reuse; BIP 341 depth-128 ceiling; rust-miniscript API thin-shim). Note that `validate_tap_leaf_subset` is unchanged.
3. **Single-leaf FAQ expanded** — replaced one-sentence "Single-leaf taproot is supported... Multi-leaf TapTree is deferred" with a full paragraph noting both forms are degenerate cases of the multi-leaf admission as of v0.5; bytecode UNCHANGED from v0.4.x; v0.5 is wire-additive.

### Task 7.5 — §"Test vectors" fixture references — DONE

Multiple updates:
- Introductory paragraph: added "and later (including v0.5.0)" to the schema-2 target range.
- Schema-2 section retitled: "Schema 2 (v0.2.0 / extended through v0.5.0)".
- Schema-2 positive vectors: 27 total documented. Fixture renames called out explicitly — `tr_keypath` → `tr_keypath_only_md_v0_5` (T1), `tr_pk` → `tr_single_leaf_pk_md_v0_5` (T2, RENAMED at v0.5 — bytecode unchanged). T3–T7 new multi-leaf fixtures enumerated with IDs and descriptions. v0.4 corpus and fingerprints vectors listed.
- Schema-2 negative vectors: `n_taptree_multi_leaf` removed; replaced with `n_taptree_single_inner_under_tr` (N1) and N1–N9 set documented. Count updated to >=47.
- Schema-2 SHA note: clarified as historical v0.4.x pin; v0.5.0 SHA will differ; removed "family-stable" phrasing that was specific to v0.4.x.
- Schema versioning: updated to mention v0.2.0 through v0.5.0 target range.

### Task 7.6 — TODO Phase 7 markers — NOT PRESENT

The plan referenced TODO markers at original lines 860-861 placed during v0.2 Phase D when 0x08 was reserved. These are not present in the current file — they were resolved in a prior phase. The family-stable SHA comment at that location already contained resolved prose without TODO. No action required.

### Task 7.7 — Commit — DONE

Commit `1d12102` — "docs(v0.5 phase 7): BIP draft updates for multi-leaf TapTree admittance"

---

## Verification

- `cargo build`: clean (`Finished dev profile`)
- `<code>` tag balance: 506/506 (balanced)
- `<pre>` tag balance: 12/12 (balanced)
- `<source>` tag balance: 3/3 (balanced)
- Status line: unchanged — "Pre-Draft, AI + reference implementation, awaiting human review"
- No stale RESERVED/v1+ text for 0x08 remains in file

---

## Line-number drift notes

The plan referenced original line numbers (85-89 for §scope; 534-540 for §Taproot tree; 391 for tag table; 860-861 for TODO markers). Current file had equivalent content at the same approximate positions (confirmed by grep + targeted reads). No significant drift requiring special handling.

---

## Files modified

- `/scratch/code/shibboleth/descriptor-mnemonic-v0.5/bip/bip-mnemonic-descriptor.mediawiki` — +51/-19 lines
