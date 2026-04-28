# Rename Workflow (reusable runbook)

This document is the canonical procedure for renaming the project's identifier surface — HRP, crate name, library name, binary name, BIP filename, exposed type/function names, repo name (optional), and externally-visible strings. It is parameterized so each rename plugs in `OLD_NAME` / `NEW_NAME` / `OLD_HRP` / `NEW_HRP` / etc.

A rename is a **wire-format-breaking change** because the HRP enters the polymod via HRP-expansion (BIP §"Checksum"). It MUST land as a SemVer-breaking release: `0.X+1.0` pre-1.0 or `X+1.0.0` at 1.0+.

---

## When to use

Trigger this workflow when any of these change:
- The bech32 HRP (string used in `hrp1...` prefix)
- The crate package name (`Cargo.toml` `[package] name`)
- The library name (`[lib] name`)
- The binary name (`[[bin]] name`) used by end-users at the shell
- The acronym expansion in normative text ("Wallet Descriptor Mnemonic" → "Mnemonic Descriptor")
- The BIP filename or title

Do NOT use this workflow for: bug fixes (use a patch release), feature work (use a phase-style plan), internal refactors (use code-quality reviewer). This workflow exists because identifier renames touch dozens of files and have wire-format consequences that other workflows don't.

---

## Pre-flight (HARD GATES — must complete before doc-writing)

### Gate 1: HRP collision vet

The new HRP MUST be vetted against:

1. **SLIP-0173 registry**: `https://raw.githubusercontent.com/satoshilabs/slips/master/slip-0173.md`. Fetch and confirm the proposed HRP is not present.
2. **Lightning HRPs**: `lnbc`, `lntb`, `lnbcrt`, `lnsb`, `lno`, `lni`, `lnr` (BOLT 11, BOLT 12).
3. **Liquid HRPs**: `ex`, `lq`, `el`, `tlq`, `ert` (sidechain, not in SLIP-173).
4. **Codex32 HRP**: `ms` (BIP 93). Already used.
5. **Nostr HRPs**: `npub`, `nsec`, `note`, `nevent`, `nprofile`, `naddr`, `nrelay` (NIP-19).
6. **Cosmos chain HRPs**: enumerated per chain; do a web search for `"<hrp>1" cryptocurrency` to catch any in-use prefix.
7. **General web search**: `"<hrp>1" bech32` and `"<hrp>" hrp` to surface anything not in formal registries.

**Exit criteria:** No collision found OR collision found and explicitly accepted by the user with rationale captured in the decision log.

### Gate 2: Decision matrix confirmed by user

Capture the following in the decision log file BEFORE any code touches:

| Item | Old | New | Notes |
|---|---|---|---|
| HRP | `<OLD_HRP>` | `<NEW_HRP>` | enters polymod via HRP-expansion |
| Crate package | `<old-codec>` | `<new-codec>` | `Cargo.toml [package] name` |
| Library name | `<old_codec>` | `<new_codec>` | `Cargo.toml [lib] name` (snake_case) |
| Binary name | `<old>` | `<new>` | end-user shell command |
| BIP filename | `bip-<old-title>.mediawiki` | `bip-<new-title>.mediawiki` | git mv |
| BIP title | `<Old Expansion>` | `<New Expansion>` | first heading + abstract |
| Tag prefix | `<old>-codec-vX.Y.Z` | `<new>-codec-vX.Y.Z` | new tags only; old tags stay |
| Repo name | `<old-repo>` | `<new-repo>` (optional) | GitHub URL change has cascading effects |

Additional confirmed-by-user decisions:
- **SemVer target**: `0.X+1.0` or `X+1.0.0`?
- **Past releases policy**: leave intact; deprecate via release-note edits; or unlist?
- **Repo name change Y/N**: GitHub URL is referenced in many places; weigh blast radius.
- **MIGRATION.md scope**: how should consumers translate? (Old vectors don't validate against new HRP; old engraved cards don't either if any exist in the wild.)

### Gate 3: Past-release deprecation policy

If past tags stay live (default), pre-draft the deprecation banner that gets prepended to each historical GitHub Release's notes. Example template:

> ⚠️ **DEPRECATED — superseded by `<new>-codec-vX.0.0`.** This release uses HRP `<old_hrp>` and crate name `<old-codec>`, both of which are renamed in vX.0.0 to `<new_hrp>` / `<new-codec>` respectively. **Wire format incompatibility:** strings produced by this release will not validate against vX.0.0 decoders and vice versa. Pin to this tag only for historical compatibility; new work should target the renamed crate.

---

## Phase 0: Discovery (Explore agent — very thorough)

**Dispatch:** `Agent(subagent_type=Explore, ...)` with thoroughness=very thorough.

**Prompt template** (fill in OLD_NAME tokens):

> Enumerate every occurrence of the following identifiers across the entire repository at `/scratch/code/shibboleth/descriptor-mnemonic`. Be exhaustive — this is a rename inventory and every miss costs a follow-up commit.
>
> Identifier tokens to find (case-sensitive matches):
> - `<OLD_HRP>` (lowercase, the bech32 HRP — also `<OLD_HRP>1` as a string prefix in test data)
> - `<OLD_NAME>` (lowercase, in identifiers, filenames, paths)
> - `<OLD_NAME_UPPER>` (uppercase, in constants, types named like `<OLD_NAME_UPPER>Backup`)
> - `<Old_Name>` (PascalCase, in type names)
> - `<old_name>` (snake_case, in module names, functions)
> - The expanded acronym phrase `<Old Acronym Expansion>` (and lowercase, hyphenated variants)
>
> Categorize findings into:
> 1. **Code identifiers** (Rust types, functions, modules, constants)
> 2. **Doc comments** (`///`, `//!`)
> 3. **String literals** (error messages, log lines, CLI help, format!())
> 4. **Filenames + directory names** (anything matching `*<old_name>*` in path)
> 5. **CI config** (`.github/workflows/*.yml`)
> 6. **Test vector contents** (committed JSON files, expected strings)
> 7. **BIP normative text** (`bip/*.mediawiki`)
> 8. **Tier-2 docs** (`README.md`, `CHANGELOG.md`, `MIGRATION.md`, `design/*.md`)
> 9. **Cargo manifest fields** (`Cargo.toml` package/lib/bin names, descriptions)
> 10. **External-facing strings** (CLI `--help` output, `gen_vectors` generator string, version strings)
> 11. **Auto-memory files** (only `/home/bcg/.claude/projects/-scratch-code-shibboleth/memory/`)
>
> For each finding, output: file path:line, the matched text, and a one-word safety tag:
> - `MECHANICAL` — pure token replace, no semantic risk
> - `CONTEXTUAL` — needs to read surrounding code to confirm replacement is correct
> - `WIRE` — touches polymod input or HRP-expansion, replacement requires regen
> - `HISTORICAL` — appears in CHANGELOG/audit/decision-log; do NOT rewrite (record of what was true at the time)
> - `EXTERNAL` — external string visible to end users; replacement language matters (e.g., the deprecation banner)
>
> Output to: `design/agent-reports/v0-X-Y-rename-discovery.md`. Also surface a one-paragraph "surprises" section at the top noting anything the rename WILL break that wasn't on the obvious list.

**Exit criteria:** discovery report written, controller has read it end-to-end.

---

## Phase 1: Plan (Plan agent)

**Dispatch:** `Agent(subagent_type=Plan, ...)`.

**Prompt template:**

> Read `design/RENAME_WORKFLOW.md`, `design/RENAME_<tag>.md` (decision log), and `design/agent-reports/v0-X-Y-rename-discovery.md` (full discovery inventory). Produce an execution plan grouped by phase per the workflow doc, with these properties:
>
> - Each phase has a clear entry condition (what must be true before starting) and exit condition (what must be true to mark it done).
> - Within each phase, group edits by safety tag (MECHANICAL first, then CONTEXTUAL, then WIRE, then EXTERNAL last).
> - For WIRE-tagged items, call out the test vector regen + SHA pin updates as their own atomic step.
> - For HISTORICAL items, emit a NO-OP note and explicitly skip them.
> - For each phase, name the gates that must pass: `cargo test -p <new-codec>`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, `gen_vectors --verify` for both vector files.
> - Estimate scope: file count touched, identifier count touched, expected SHA churn.
> - Note any phase that can be done in parallel by independent subagents (most can't, since renames cascade).
>
> Output to: `design/IMPLEMENTATION_PLAN_v0_X_rename.md`.

**Exit criteria:** plan file written, controller has read it, controller has identified any phase that needs decision-log update before execution.

---

## Phase 2: Spec/BIP rename (do first)

The BIP is the source of truth. Rename it before code so reviewers can cross-check the implementation against the updated normative text.

Steps:
1. `git mv bip/bip-<old-title>.mediawiki bip/bip-<new-title>.mediawiki`
2. Edit BIP first heading: `<Old Title>` → `<New Title>`
3. Edit abstract paragraph
4. Edit every prose mention of the old name
5. **HRP-expansion constants**: BIP §"Checksum" lists pre-computed HRP-expansion bytes. For HRP `wdm` (3 chars) the expansion is `[3, 3, 3, 0, 23, 4, 13]` (length 7 = 2*3 + 1). For HRP `md` (2 chars) the expansion is `[3, 3, 0, 13, 4]` (length 5 = 2*2 + 1). Recompute and update the literal in §"Checksum" item 1.
6. Search the BIP for any reference to `<OLD_HRP>1` as a string prefix in examples — replace with `<NEW_HRP>1`.

**No example mnemonics in the BIP need updating yet** — those are produced by the implementation and will be regenerated in Phase 6.

---

## Phase 3: Cargo + lib/bin renames

Steps in order:
1. `Cargo.toml` (workspace root): no changes typically.
2. `crates/<old-codec>/Cargo.toml`:
   - `[package] name = "<old-codec>"` → `"<new-codec>"`
   - `[lib] name = "<old_codec>"` → `"<new_codec>"`
   - `[[bin]] name = "<old>"` → `"<new>"` (the user-facing CLI command)
   - `[[bin]] path = "src/bin/<old>/main.rs"` → `"src/bin/<new>/main.rs"`
   - `description` field if it contains the old expanded acronym
3. `git mv crates/<old-codec> crates/<new-codec>`
4. `git mv crates/<new-codec>/src/bin/<old> crates/<new-codec>/src/bin/<new>`
5. `cargo update` to refresh `Cargo.lock` (will rename the package entry).

**Gate after this step:** `cargo build` should succeed (some `use <old_codec>::...` paths inside the renamed crate may still exist; fix in next phase).

---

## Phase 4: Identifier mass-rename

Use Edit tool with `replace_all: true` for safe mechanical tokens. Use grep + targeted Edit for ambiguous tokens.

Order (least risky to most):
1. `use <old_codec>::` → `use <new_codec>::` across all crate files
2. Module names containing the old token (rename file + update `mod` declarations)
3. Type names: `<Old>Backup` → `<New>Backup`, `<OLD>_GENERATOR` constants, etc. (use rust-analyzer if available; otherwise grep + Edit)
4. Function names containing the old token
5. Test names: `tests/<old>_<rest>.rs` → `tests/<new>_<rest>.rs` (git mv); `fn <old>_*` test functions

For each rename, run `cargo build` after every batch of ~5 files to surface compile errors before they pile up.

---

## Phase 5: String literal sweep

Target categories (all CONTEXTUAL — read surrounding code):
- Error messages: `Error::PolicyScopeViolation("v0.1 does not support wpkh()...")` style strings
- CLI help text in clap derive macros (`#[arg(help = "...")]`)
- Doc comments at module/type/function level
- Format string literals in `format!()`, `println!()`, `eprintln!()`
- Test assertion messages
- `gen_vectors` stderr generator log: `"family generator = \"<old> 0.X\""` → `"<new> 0.X"`

**Family-stable generator string**: `pub const GENERATOR_FAMILY: &str = concat!("<old-codec> ", env!("CARGO_PKG_VERSION_MAJOR"), ".", env!("CARGO_PKG_VERSION_MINOR"));` becomes `concat!("<new-codec> ", ...)`. This change resets the family-stable promise — future v0.X.Y patches share new SHA, but vX.0.0+ vectors will not match v(X-1).Y SHAs.

---

## Phase 6: Test vector regeneration (WIRE)

This is the wire-format-breaking step. After it, every old vector is invalid and every new vector has a different SHA.

Steps:
1. Run `gen_vectors --output crates/<new-codec>/tests/vectors/v0.1.json --schema 1` (regenerates with new HRP + new generator string).
2. Run `gen_vectors --output crates/<new-codec>/tests/vectors/v0.2.json --schema 2`.
3. `sha256sum` both files; capture new SHAs.
4. Update `V0_1_SHA256` and `V0_2_SHA256` constants in `tests/vectors_schema.rs` to the new values.
5. Update BIP §"Test vectors" SHA reference (if present) to new v0.2 SHA.
6. Run `cargo test -p <new-codec>` — expect green.

Document in decision log: old SHA → new SHA mappings for both files.

---

## Phase 7: CI / release infra

Files:
- `.github/workflows/*.yml`: any reference to `<old-codec>` crate name, `<old>` binary name, `<old>-codec-v` tag patterns.
- `.gitattributes`: `crates/<old-codec>/tests/vectors/*.json text eol=lf` → `crates/<new-codec>/tests/vectors/*.json text eol=lf`.
- Any release script or Justfile/Makefile.

Tag prefix policy: future tags use `<new-codec>-vX.Y.Z`. Old `<old-codec>-vX.Y.Z` tags stay intact (per past-release deprecation policy in pre-flight Gate 3).

---

## Phase 8: Documentation sweep

- `README.md`: full rewrite of name references; add a "Renamed from `<old-codec>`" admonition near the top.
- `CHANGELOG.md`: add new vX.0.0 entry. **Do NOT rewrite past entries** — they are historical record.
- `MIGRATION.md`: add new section `vX-1.Y → vX.0.0`. Cover: HRP change, crate rename, identifier renames, vector SHA churn, deprecation of old tags, repo URL stability (or change).
- `design/FOLLOWUPS.md`: close any open items the rename addresses; add SLIP-0173 PR follow-up; add anything discovered during execution that didn't make this rename.

---

## Phase 9: Memory updates (auto-memory)

Files at `/home/bcg/.claude/projects/-scratch-code-shibboleth/memory/`:
- Any `project_*.md` referencing the old crate/repo/HRP name.
- `MEMORY.md` index hooks.

Add a new memory: `project_renamed_<old>_to_<new>.md` capturing the rename event, the SemVer cut, and the family-stable SHA reset. Future sessions should know that vX.0.0 is the rename boundary.

---

## Phase 10: Past-release deprecation

For each old GitHub Release (`<old-codec>-vX.Y.Z`):
1. `gh release view <old-codec>-vX.Y.Z --json body --jq .body` to capture current notes.
2. Prepend the deprecation banner from pre-flight Gate 3 to the existing notes body.
3. `gh release edit <old-codec>-vX.Y.Z --notes-file -` (or `--notes "$(...)"`) to apply.

Do NOT delete old releases. Do NOT unlist. Do NOT touch tags. Only edit the notes body.

---

## Phase 11: SLIP-0173 PR (post-release follow-up)

After vX.0.0 ships:
1. Fork `satoshilabs/slips`.
2. Edit `slip-0173.md` to add the new HRP entry.
3. Open PR with rationale: "Registering HRP `<NEW_HRP>` for the Mnemonic Descriptor format (BIP draft at `<repo URL>`). Mainnet-only; no testnet variant currently planned."
4. Track in `design/FOLLOWUPS.md`.

This is defensive — registering the HRP closes off future collision risk from independent projects.

---

## Execution model

The phases above are ORDERED — each depends on the previous. Do not parallelize across phases.

Within Phase 0 (Discovery) and Phase 1 (Plan), use specialized agents (Explore, Plan).

Within Phases 2–10, use the **superpowers:subagent-driven-development** skill — one implementer subagent per phase, with spec-compliance and code-quality reviewer subagents gating each phase. The phases are small enough that one implementer per phase is the right granularity.

After Phase 10 (last in-repo phase), run a final **code-reviewer agent** over the cumulative diff before merging the worktree branch back to main.

---

## Gates (must pass after every phase that touches code)

1. `RUSTUP_TOOLCHAIN=stable cargo build --workspace --all-targets`
2. `RUSTUP_TOOLCHAIN=stable cargo test -p <new-codec>` — full test suite green
3. `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets -- -D warnings`
4. `RUSTUP_TOOLCHAIN=stable cargo fmt --check`
5. After Phase 6: `gen_vectors --verify crates/<new-codec>/tests/vectors/v0.1.json` and `v0.2.json` — both PASS
6. After Phase 8: BIP file renders cleanly (mediawiki preview if possible)

A failed gate halts the phase. Fix in place; do not move forward with red gates.

---

## Escape hatches

If the rename gets bogged down (e.g., discovery surfaces 500+ touch points and the plan estimate balloons), STOP and reconsider. Options:

- **Defer the rename**: ship a non-renaming v0.3.0 with v0.3 features, schedule rename for v0.4.0 or v1.0.0.
- **Partial rename**: rename only the HRP and binary (user-facing surface), keep crate/lib internals on old name. Reduces diff at the cost of long-term inconsistency.
- **Different new name**: if collision vet surfaces an issue mid-rename, restart from pre-flight with a different target.

The decision log should record which option was chosen and why if execution diverges from the plan.
