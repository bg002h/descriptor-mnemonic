# Phases 9 + 10 — agent reports

**Branch**: `rename/v0.3-wdm-to-md`
**Date**: 2026-04-27

---

## Phase 9 — Auto-memory updates

**Files updated** (at `/home/bcg/.claude/projects/-scratch-code-shibboleth/memory/`, outside the git repo):

1. `project_shibboleth_wallet.md` — substantial rewrite. "Mnemonic Descriptor (MD)" as primary name; new "Format name evolution" section narrating SWB → WDM → MD; v0.2-mid-flight status replaced with "v0.3.0 shipped 2026-04-27"; all operational paths updated to `crates/md-codec/`; Shibboleth Wallet sibling-project section unchanged.
2. `project_followups_tracking.md` — light: WDM → MD in name + body; FOLLOWUPS path unchanged.
3. `feedback_subagent_workflow.md` — light: WDM → MD operational refs; v0.2 phase norms section left HISTORICAL.
4. `project_apoelstra_pr_check.md` — 3 operational `crates/wdm-codec/Cargo.toml` refs updated to `crates/md-codec/Cargo.toml` with HISTORICAL parentheticals.
5. `feedback_worktree_dispatch.md` — 2 operational refs WDM → MD; 3 gotchas themselves unchanged (still relevant).
6. `project_no_bash_shell_impl.md` — added dated update noting v0.3 turned out to be the rename, not the originally-anticipated substantive work; bash-impl decision still stands.
7. `MEMORY.md` — index entries updated for items affected by the rename + new entry added at bottom.
8. `feedback_agent_review.md` — UNCHANGED (purely historical).

**New file created**: `project_renamed_wdm_to_md_v0_3.md` — captures rename event, OLD/NEW table, wire-format-break note, family-stable promise reset, new SHAs, post-rename "How to apply" instructions.

**Verification**: 14 remaining `wdm-codec`/`wdm_codec` matches across all memory files; ALL in HISTORICAL contexts (parentheticals, rename-summary table, historical-narrative sentences). Zero naked operational references.

**Status**: DONE.

---

## Phase 10 — Past-release deprecation banners

**Tags edited** (notes body only — tags + assets untouched):
- `wdm-codec-v0.2.0`
- `wdm-codec-v0.2.1`
- `wdm-codec-v0.2.2`
- `wdm-codec-v0.2.3`

**Banner applied** (verbatim from decision log Pre-flight Gate 3):

> ⚠️ **DEPRECATED — superseded by `md-codec-v0.3.0`.** This release uses HRP `wdm` and crate name `wdm-codec`, both of which were renamed in v0.3.0 to `md` and `md-codec` respectively. The format is now called "Mnemonic Descriptor" (was "Wallet Descriptor Mnemonic"). **Wire format incompatibility:** strings produced by this release start with `wdm1...` and will not validate against v0.3.0 decoders, which expect `md1...` strings. Pin to this tag only for historical compatibility; new work should target [`md-codec-v0.3.0`](https://github.com/bg002h/descriptor-mnemonic/releases/tag/md-codec-v0.3.0) or later. Repository URL is unchanged.

**Verification**: All 4 releases open with the banner as the first line (verified via `gh release view <tag> --json body --jq .body | head -2`); original notes intact below; `gh release list` confirms all 4 tags still present with original titles + Pre-release status; no asset changes.

**Status**: DONE. (Note: the banner's link to `md-codec-v0.3.0` will 404 until that release is published — that's expected and self-resolves on release.)

---

## Phases 9 + 10 closure

✅ Both phases complete. All in-repo work for the rename done. Outstanding before tag:

1. **`bch-known-vector-repin-with-md-hrp` (v0.3-BLOCKER)** — controller decision required.
2. Final reviewer agent over cumulative diff (`git diff main...HEAD`).
3. Release sequence (merge to main, tag, push, draft GitHub Release).
