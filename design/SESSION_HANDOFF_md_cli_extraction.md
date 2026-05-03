# Session handoff — md-cli extraction

This file bootstraps a fresh Claude Code session into the in-progress
md-cli extraction work. The previous session completed the
brainstorm/spec/plan cycle (with three architect-review rounds, all
critical/important fixes applied inline). All durable artifacts are on
disk; this file is the "boot doc."

## State at handoff

- **Branch:** `main` (NOT yet on `feat/md-cli-extraction`; Task 0 of the
  plan creates the branch).
- **Last commit on `main`:** `021a4a2 docs(md-cli-extraction):
  implementation plan + plan-stage architect review`.
- **Commits ahead of `origin/main`:** 3 (two spec commits + the plan
  commit). Unpushed; that's fine.
- **Working tree:** clean.

## Source-of-truth files

| Path | Role |
|---|---|
| `design/SPEC_md_codec_v0_16_library_only.md` | Spec — contracts, end state, non-goals, acceptance criteria |
| `design/IMPLEMENTATION_PLAN_md_cli_extraction.md` | 13-task plan with full code/commands/expected outputs per step |
| `design/agent-reports/spec-review-md-cli-extraction-brainstorm-stage.md` | First architect review (early design) |
| `design/agent-reports/spec-review-md-cli-extraction-spec-stage.md` | Second architect review (written spec) |
| `design/agent-reports/plan-review-md-cli-extraction.md` | Third architect review (written plan) — caught two real bugs (C1/C2), both fixed inline |

## Task seeds for fresh session

A fresh session should start by recreating the 13 implementation tasks.
Recommended `TaskCreate` calls (in order — task IDs auto-assign):

```
1.  Pre-flight: create feature branch feat/md-cli-extraction
2.  Phase 0: API audit + test classification → audit doc committed
3.  Phase 1: scaffold md-cli crate + failing smoke test (TDD baseline)
4.  Phase 1: architect review + report
5.  Phase 2: atomic source-move + manifest swap (single commit)
6.  Phase 2: architect review + report
7.  Phase 3: move CLI tests + snapshot fixtures
8.  Phase 3: architect review + report
9.  Phase 4: version bump + CHANGELOG entries
10. Phase 4: FOLLOWUPS entries (4 deferred items)
11. Phase 4: architect review + report
12. Final: whole-PR architect review + report
13. Final: operationalize acceptance criterion #4 (binary-behavior parity)
14. Final: push branch + open PR
```

(That's 14 actually — Task 13 in the plan was both "binary parity" and
"push+PR"; I split them for tracking. Cross-reference the plan's task
numbering, which is canonical.)

Each task corresponds to a numbered Task in the plan (Task 0 = task 1
above; plan Task 4 = task 5 above; etc.). Use the plan's checkbox steps
to drive execution.

## Execution mode (user pending)

The previous session offered two options at handoff time:

1. **Subagent-driven** (recommended) — fresh subagent per task, review
   between tasks, two-stage review per task. Skill:
   `superpowers:subagent-driven-development`.
2. **Inline execution** — execute tasks in the parent session, batched
   with checkpoints. Skill: `superpowers:executing-plans`.

If the user has already chosen, proceed with that skill. If not, ask.

## Repo conventions to respect (from CLAUDE.md memory)

- **Per-phase iterative-agent review is mandatory.** Brainstorm + spec +
  plan rounds are done. Per-phase + final remain. Reports persist to
  `design/agent-reports/`. Critical/important fixed inline; low/nit
  appended to `design/FOLLOWUPS.md` under tier `v0.16.x`.
- **No `git add -A`** — root has untracked local helpers (e.g.
  `resume_may1`); stage paths explicitly.
- **Feature branch:** `feat/md-cli-extraction`. PR opens against `main`.
- **Commit-message format:** `feat(md-cli): phase N — <scope>` for
  implementation; `docs(md-cli-extraction): <scope>` for review reports;
  `release: md-codec v0.16.0 + md-cli v0.1.0` for the version bump.

## Architect-review key facts (so a fresh session doesn't re-litigate)

- **Reading A** (preserve `json` feature flag on md-cli) is locked in.
  md-cli has `default = ["json"]`, `json = ["dep:serde",
  "dep:serde_json"]`, `cli-compiler = ["dep:miniscript",
  "miniscript/compiler"]`. Don't collapse the `json` feature.
- **Phase 3 has no source edits.** All source touches happen in Phase 2,
  including the `include!` pre-fixes for `template_roundtrip.rs` and
  `json_snapshots.rs` (architect's C1 bug-find).
- **Phase 2 needs `mkdir -p crates/md-cli/src`** before the flat-file
  `git mv`s (architect's C2 bug-find).
- **Test classification is ground-truth** (architect spot-checked):
  `smoke.rs` stays in md-codec; `template_roundtrip.rs` moves to md-cli.
  The "provisional" language was removed from the spec.
- **CHANGELOG preamble** must be updated in Phase 4 from "All notable
  changes to `md-codec`" to a per-crate-prefixed form.

## How to resume

1. Read `design/IMPLEMENTATION_PLAN_md_cli_extraction.md` end-to-end
   first.
2. Skim each architect-review report.
3. Recreate the 13-14 tasks in TaskList (above).
4. Pick execution mode (subagent-driven or inline).
5. Start at Task 0 (create branch).
