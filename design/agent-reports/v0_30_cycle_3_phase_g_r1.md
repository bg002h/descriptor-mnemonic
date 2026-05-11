# Phase G — code review r1 (2026-05-10)

**Working tree:** dirty at HEAD `377745c`; not yet committed.

**Scope:** md-codec v0.30 Cycle 3 Phase G (FINAL phase of Cycle 3) — error-taxonomy sweep. Add `OperatorContextViolation { tag: Tag, context: ContextKind }` + `ContextKind` enum; finalize `NUMSSentinelConflict` doc (variant + raise-sites shipped in Phase F); purge v0.11 prose from 5 variants; update `ChunkHeaderChunkedFlagMissing` doc + format for SPEC §2.2.

**Files reviewed:** `crates/md-codec/src/error.rs`, `crates/md-codec/src/tag.rs` (derive check), `crates/md-codec/src/validate.rs` (ForbiddenTapTreeLeaf retention check), `design/FOLLOWUPS.md`, `design/SPEC_v0_30_wire_format.md` §11.

---

## Critical (block ship)

None.

## Important (must fix before ship)

None.

## Low (file as FOLLOWUP — fixed inline)

### L-1 — `OperatorContextViolation` doc-comment had 7 lines of narrative (FIXED INLINE)

- **Where:** `crates/md-codec/src/error.rs:197–205` (pre-fix).
- **What:** Doc-comment repeated the FOLLOWUP entry's explanation of why all three `ContextKind` arms lack live fire sites. Violates `feedback_terse_code.md` ("short doc-comments, no narrative prose").
- **Fix (applied):** Trimmed to 3 lines citing SPEC §11 and the FOLLOWUP entry by ID.

## Nit (optional polish — no action)

### N-1 — Test names diverge from sub-plan TDD spec (defensible given stub-only decision)

- **Where:** `error.rs:407, 419` (`_constructs`, `_display` suffixes vs sub-plan's `_multi_body`, `_conflict`).
- **What:** Names accurately describe the stub-only nature. No action.

### N-2 — `tag: crate::tag::Tag` is a positive SPEC-aligning deviation from sub-plan

- **Where:** `error.rs:209`.
- **What:** Sub-plan said `tag: u8`; implementation used `Tag`. SPEC §11 says `Tag`. Implementation is SPEC-correct. No action.

---

## Correctness checklist (all passed)

| # | Check | Result |
|---|---|---|
| 1 | `ContextKind` variants = exactly `TopLevel`/`TapLeaf`/`MultiBody` | PASS (error.rs:10–15; matches SPEC §11 line 338) |
| 1 | `ContextKind` derives include `Debug, PartialEq, Eq` | PASS (error.rs:8) |
| 2 | `OperatorContextViolation.tag` is `Tag` (SPEC-correct) | PASS (error.rs:209) |
| 2 | `Tag` has `Debug` derive | PASS (tag.rs:14) |
| 3 | `NUMSSentinelConflict` format string contains `§7` and `§11` | PASS (error.rs:379) |
| 4 | `grep v0.11 error.rs` = 0 | PASS (verified: 0 occurrences) |
| 4 | No v0.11 in doc-comments (separate from format strings) | PASS |
| 5 | `ChunkHeaderChunkedFlagMissing` doc cites §2.2 not §9.3 | PASS (error.rs:238) |
| 5 | "4-bit version field" (not stale "3-bit") | PASS (error.rs:239) |
| 5 | Format string updated | PASS (error.rs:240) |
| 6 | `operator_context_violation_constructs` exercises variant + Display | PASS (error.rs:407–414) |
| 6 | `nums_sentinel_conflict_display` pins §7 + §11 substrings | PASS (error.rs:419–424) |
| 7 | `v0.30-phase-a-r1-nit-1` marked resolved | PASS (FOLLOWUPS.md:510) |
| 7 | `v0.30-phase-b-r1-low-1` marked resolved | PASS (FOLLOWUPS.md:519) |
| 7 | `v0.30-phase-g-operator-context-violation-unwired` filed | PASS (FOLLOWUPS.md:549–559) |
| 8 | Stub-only `OperatorContextViolation` analysis correct | PASS — `validate_tap_script_tree` raises narrower `ForbiddenTapTreeLeaf` (validate.rs:141–162); MultiBody is structurally unreachable post-Phase-C; TopLevel is parser-side |
| 9 | `ForbiddenTapTreeLeaf` retention appropriate (no unification with `OperatorContextViolation`) | PASS — narrower variant is more actionable |
| 11a | 6 ignored tests remain in tree.rs (Phase H) | PASS |
| 11b | help_examples 2 fails (Phase H FOLLOWUP unchanged) | PASS |
| 11c | All other test suites green | PASS — 217 lib / 0 / 6; all integration green |
| 11d | `v0.11` = 0 in error.rs | PASS |
| 11e | Phase G FOLLOWUP targets resolved | PASS |

---

## Verdict

**Ship.** L-1 fixed inline (doc-comment trimmed from 7 to 3 lines per CLAUDE.md terse-code guideline). 0C/0I/1L (fixed)/2N. Cycle 3 exit criteria fully satisfied; Phase G is the cycle-closing commit.

Post-commit Cycle 3 exit state: HEAD = Phase G commit (TBD); 4 atomic commits on origin/main atop `7944f00`; 217 lib pass / 6 ignored (12 → 6 in tree.rs via Phase C + F lifts); integration tests green except help_examples (Phase H FOLLOWUP); workspace clippy clean. Phase H (corpus regen) inherits 6 tree.rs ignored tests + help_examples drift + `OperatorContextViolation` stub-only state.
