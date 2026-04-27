# Phase B bucket B review — Opus 4.7

**Status:** APPROVE_WITH_FOLLOWUPS
**Subject:** commit `0993dc0` (`7-encode-path-override`)
**Reviewer model:** Opus 4.7 via general-purpose subagent
**Stage:** combined spec compliance + code quality (single pass)
**Role:** reviewer

## Findings

### Spec deviations

(none) — every Bucket B requirement present. 4-tier precedence implemented at `policy.rs:378-384` in correct order. All 22+ `to_bytecode` call sites updated. CLI warning replaced with actual application at `bin/wdm.rs:223`. Wire format unchanged for default-path case.

### Quality blockers

(none)

### Quality important (1)

- **Q-1**: `MIGRATION.md` follow-up missing for the Phase B breaking changes — (a) `to_bytecode(&self)` → `to_bytecode(&self, opts: &EncodeOptions)`, and (b) `impl Copy for EncodeOptions` removed (DerivationPath is not `Copy`). The commit body documents both, but the persistent `FOLLOWUPS.md` tracker only has the Phase A `wallet-policy-eq-migration-note` entry. Without a Phase B tracker, Phase G's MIGRATION.md will ship incomplete. **(Filed as `phase-b-encode-signature-and-copy-migration-note`.)**

### Quality nits (3)

- **N-1**: `policy.rs:1213-1222` override-wins test asserts only `bytes[2] == 0x05`. Add a `bytes != baseline` assertion (where baseline is the no-override re-encode) to catch a future bug where bytes[2] coincidentally happens to be 0x05 for an unrelated reason. **(Applied inline by controller in fixup commit.)**
- **N-2**: `cmd_bytecode` at `bin/wdm.rs:401` hardcodes `&EncodeOptions::default()` and has no `--path` flag. Reasonable for a debug-aid subcommand, but a comment noting the intentional asymmetry with `cmd_encode` would prevent future "fixes" by contributors. Not filed (too small).
- **N-3**: `to_bytecode_default_options_still_consult_decoded_shared_path` regression-guard is genuine, not theatre. No action; positive note.

## Disposition

| Finding | Action |
|---|---|
| Q-1 (MIGRATION.md tracker) | New FOLLOWUPS: `phase-b-encode-signature-and-copy-migration-note` (v0.2-nice-to-have, Phase G) |
| N-1 (test strengthening) | Applied inline in controller fixup commit |
| N-2 (cmd_bytecode comment) | Acknowledged; no action |
| N-3 (regression-guard) | Noted; positive |

## Verdict

APPROVE_WITH_FOLLOWUPS — bucket B clear to integrate.
