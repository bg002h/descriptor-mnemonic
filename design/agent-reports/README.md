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

Reports are Markdown. No strict schema, but typical sections:

```markdown
# Phase X bucket Y / Task X.Y

**Status:** DONE | DONE_WITH_CONCERNS | BLOCKED | NEEDS_CONTEXT
**Commit:** <SHA>
**File(s):** path1, path2

## Summary
<1-3 sentences>

## Implementation notes
<algorithmic decisions, alternatives considered, fixture choices>

## Test results
<count, gates, empirical observations>

## Follow-up items (for FOLLOWUPS.md)
- <short-id-suggestion>: <one-line description, with file/line if relevant>

## Concerns / deviations
<anything the implementer wants the controller to know>
```

This format mirrors what implementer prompts already ask for in their "Report Format" section; the addition is just persisting it to disk.
