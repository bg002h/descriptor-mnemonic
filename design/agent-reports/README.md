# Agent reports archive

Verbatim final reports from subagent dispatches that produced commits, parallel-batch buckets, or other multi-step work. Persisted to disk so the audit trail of WHAT each agent reported (decisions, observations, follow-up items, deviations from spec) survives beyond the controller's conversation context.

## Why

The FOLLOWUPS.md tracker captures the *outcome* of each agent run (what items were deferred, which commit closed them). This directory captures the *raw report* — the implementer's full reasoning, including parts that didn't make it into FOLLOWUPS.md or commit messages but are useful for later audit:

- Algorithmic decisions and their alternatives considered
- Test fixture choices (and why)
- API surface compromises and their justifications
- Empirical results (e.g., bench numbers, rejection rates)
- Workarounds for upstream / language / tooling issues
- Cross-bucket integration concerns flagged at landing time

If a future review asks "why did Task X take that approach?", the answer is here in the raw report rather than reconstructed from `git log` + commit messages.

## File naming

```
design/agent-reports/
├── README.md                      # this file
├── phase-<P>-task-<N>.md          # single-agent task report (e.g., phase-5-task-A.md)
├── phase-<P>-bucket-<X>.md        # parallel-batch bucket report (e.g., phase-6-bucket-A.md)
├── phase-<P>-review-<commit>.md   # review subagent report (e.g., phase-5-review-308b2e1.md)
└── phase-<P>-fixup-<commit>.md    # fix-up implementer report
```

## Convention for future agent dispatches

When the controller dispatches an implementer or reviewer subagent, the prompt SHOULD include language like:

> Save your final report (the same text you return to me) to `design/agent-reports/<filename>.md` as part of your commit. Use the file-naming convention in `design/agent-reports/README.md`.

For **parallel-batch dispatches**, each agent saves to a distinct file (no conflicts since filenames embed the bucket id). The controller's post-batch aggregation step then reads these files and appends to `FOLLOWUPS.md` — no reliance on the controller's working memory.

For **single-agent dispatches**, the agent's commit includes the report file alongside the work commit; the controller can still aggregate but the raw report is durable independently.

## Format

Reports are Markdown. The header block is **required** (so an auditor can answer "what was changed/reviewed and why" from the file alone, without `git show`); body sections are conventional but flexible.

```markdown
# <Title — phase + task or bucket id>

**Status:** DONE | DONE_WITH_CONCERNS | BLOCKED | NEEDS_CONTEXT
**Commit:** <SHA(s) — list every commit produced>
**File(s):** <every file path read or modified, one per line if multiple>
**Role:** implementer | reviewer (spec) | reviewer (code-quality) | fixup

## Summary
<1-3 sentences>

[... body sections — see "Body conventions" below ...]
```

### Required header fields

- **`Status`** — one of the four values listed
- **`Commit`** — the SHA(s) of the commit(s) this report describes. For reviewer reports that don't produce a commit, list the commit being REVIEWED here (so the entry is greppable by SHA).
- **`File(s)`** — every file the report concerns. For implementers, list files modified or created. **For reviewers, list every file actually inspected**, even if the commit-under-review only touched a subset (e.g., a code-quality reviewer may also read a sibling file to confirm a pattern). This is what `git show <commit> --stat` cannot tell you.
- **`Role`** — one of `implementer`, `reviewer (spec)`, `reviewer (code-quality)`, `fixup`. Lets a future audit grep by role to find e.g. all spec reviews.

### Body conventions

```markdown
## Implementation notes        (implementers only)
<algorithmic decisions, alternatives considered, fixture choices>

## What was reviewed           (reviewers only — restate the spec / scope)
<one paragraph summarizing what the report's checks were against>

## Test results
<count, gates, empirical observations, any conditional skips>

## Findings                    (reviewers only)
<what passed, what didn't — typically a checklist or per-issue listing>

## Follow-up items (for FOLLOWUPS.md)
- <short-id-suggestion>: <one-line description, with file/line if relevant>

## Concerns / deviations
<anything the agent wants the controller to know>
```

The implementer prompts already ask for most of this in their "Report Format" section. The persistence step is just routing the same content to a file in addition to returning it to the controller.

## Required of every dispatched subagent

In the dispatch prompt, the controller MUST include language like:

> Save your final report verbatim to `design/agent-reports/<filename>.md` as part of your commit (or as a separate commit if you don't produce code). Include the required header block from `design/agent-reports/README.md` — Status, Commit, File(s) (every file you read or modified), Role.

For parallel-batch dispatches, also instruct: "do NOT write to `design/FOLLOWUPS.md` directly — the controller aggregates after the batch."
