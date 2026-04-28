# Rename Decision Log: `wdm` → `md` (v0.3.0)

**Workflow**: see `design/RENAME_WORKFLOW.md`.
**Trigger date**: 2026-04-27.
**Target release**: v0.3.0.
**SemVer rationale**: pre-1.0 breaking-change axis is the second component (`0.X`); rename is wire-format-breaking → bump from 0.2.x → 0.3.0.

---

## User decisions (confirmed 2026-04-27)

1. **Acronym expansion**: "Mnemonic Descriptor" (was "Wallet Descriptor Mnemonic"). Drops the "Wallet" prefix; reorders descriptor/mnemonic.
2. **Crate name change**: YES (`wdm-codec` → `md-codec`).
3. **Repo name**: KEEP `descriptor-mnemonic` (GitHub URL unchanged).
4. **SemVer target**: 0.3.0 (next minor; pre-1.0 breaking-axis convention).
5. **Past releases**: KEEP intact (no unlist, no delete) but EDIT release notes to prepend a deprecation banner pointing at v0.3.0.
6. **HRP collision**: VETTED CLEAN (see Pre-flight Gate 1 below).

---

## Pre-flight Gate 1 — HRP collision vet

Performed 2026-04-27.

### SLIP-0173 registry

Fetched `https://raw.githubusercontent.com/satoshilabs/slips/master/slip-0173.md`. Result: **`md` is NOT registered.** Closest 2-character HRPs:

| HRP | Coin | Status |
|---|---|---|
| `mm` | Miden mainnet | distinct |
| `mtst` | Miden testnet | distinct (4 chars) |
| `my` | Myriad mainnet | distinct |

### Other HRP namespaces checked

- **Lightning**: `lnbc`, `lntb`, `lnbcrt`, `lnsb`, `lno`, `lni`, `lnr` — distinct.
- **Liquid sidechain**: `ex`, `lq`, `el`, `tlq`, `ert` — distinct.
- **Codex32 (BIP 93)**: `ms` — distinct (and the spiritual neighbor whose 2-char style we're matching).
- **Nostr (NIP-19)**: `npub`, `nsec`, `note`, `nevent`, `nprofile`, `naddr`, `nrelay` — distinct.
- **Cosmos chain HRPs**: enumerated via web search; none match.

### Web search

Queries:
- `"md1" bech32 address cryptocurrency` — no relevant hits
- `bech32 HRP "md" human readable part registered` — no in-use prefix surfaced

### Verdict

**CLEAN.** No collision. Proceed.

### Defensive follow-up

File a SLIP-0173 PR registering `md` for "Mnemonic Descriptor" once v0.3.0 ships. Tracked as new FOLLOWUPS entry: `slip-0173-register-md-hrp`. Closes off independent-project collision risk.

---

## Pre-flight Gate 2 — Decision matrix

| Item | Old | New | Notes |
|---|---|---|---|
| HRP | `wdm` | `md` | enters polymod via HRP-expansion |
| HRP-expansion bytes | `[3, 3, 3, 0, 23, 4, 13]` (length 7) | `[3, 3, 0, 13, 4]` (length 5) | recompute per BIP §"Checksum" item 1; HRP-expansion length = `2*len(HRP) + 1` |
| Crate package | `wdm-codec` | `md-codec` | `Cargo.toml [package] name` |
| Crate dir | `crates/wdm-codec/` | `crates/md-codec/` | `git mv` |
| Library name | `wdm_codec` | `md_codec` | `Cargo.toml [lib] name` |
| Binary name | `wdm` | `md` | end-user shell command |
| Bin source dir | `src/bin/wdm/` | `src/bin/md/` | `git mv` |
| BIP filename | `bip/bip-wallet-descriptor-mnemonic.mediawiki` | `bip/bip-mnemonic-descriptor.mediawiki` | `git mv` |
| BIP title | "Wallet Descriptor Mnemonic" | "Mnemonic Descriptor" | first heading + abstract |
| Acronym | WDM | MD | constants, doc text, type-name prefixes |
| Tag prefix | `wdm-codec-vX.Y.Z` | `md-codec-vX.Y.Z` | new tags only |
| Generator string | `"wdm-codec 0.X"` | `"md-codec 0.3"` | family-stable for v0.3.x |
| Repo name | `descriptor-mnemonic` | `descriptor-mnemonic` | UNCHANGED per user decision |

### HRP-expansion derivation (verification)

Per BIP 173 §"Bech32" HRP-expansion procedure:
- For each HRP character `c`: emit `ord(c) >> 5`
- Emit a single zero byte
- For each HRP character `c`: emit `ord(c) & 31`

**For `md` (0x6d, 0x64):**
- `ord('m') = 0x6d = 109`; `109 >> 5 = 3`
- `ord('d') = 0x64 = 100`; `100 >> 5 = 3`
- emit `0`
- `109 & 31 = 13`
- `100 & 31 = 4`

Result: `[3, 3, 0, 13, 4]`. Length 5.

**For `wdm` (legacy verification):**
- `ord('w') = 0x77 = 119`; `119 >> 5 = 3`
- `ord('d') = 0x64 = 100`; `100 >> 5 = 3`
- `ord('m') = 0x6d = 109`; `109 >> 5 = 3`
- emit `0`
- `119 & 31 = 23`
- `100 & 31 = 4`
- `109 & 31 = 13`

Result: `[3, 3, 3, 0, 23, 4, 13]`. Length 7. (Matches existing BIP text.)

---

## Pre-flight Gate 3 — Past-release deprecation banner

To prepend to each existing GitHub Release body for `wdm-codec-v0.2.0` / `v0.2.1` / `v0.2.2` / `v0.2.3`:

```markdown
> ⚠️ **DEPRECATED — superseded by `md-codec-v0.3.0`.** This release uses HRP `wdm` and crate name `wdm-codec`, both of which were renamed in v0.3.0 to `md` and `md-codec` respectively. The format is now called "Mnemonic Descriptor" (was "Wallet Descriptor Mnemonic"). **Wire format incompatibility:** strings produced by this release start with `wdm1...` and will not validate against v0.3.0 decoders, which expect `md1...` strings. Pin to this tag only for historical compatibility; new work should target [`md-codec-v0.3.0`](https://github.com/bg002h/descriptor-mnemonic/releases/tag/md-codec-v0.3.0) or later. Repository URL is unchanged.
```

The repo URL stays the same — only the crate inside changes name.

---

## Wire-format break details

This rename changes the polymod input prefix (HRP-expansion goes from 7 bytes to 5 bytes) and the HRP letters themselves. Therefore:

- Every committed test vector becomes invalid against the new HRP. Both `v0.1.json` and `v0.2.json` are regenerated. New SHAs (will be captured here at execution time).
- The family-stable generator string from v0.2.1 (`"wdm-codec 0.2"` → byte-identical regen across v0.2.x) does NOT carry across the rename. New generator string `"md-codec 0.3"` produces a new family-stable SHA promise for v0.3.x.
- BIP §"Test vectors" SHA reference must be updated.
- No engraved cards in the wild are believed to exist (pre-release, BIP draft status), so user-facing migration burden is zero. If any pre-release adopters exist, they need to re-engrave.

### MIGRATION.md scope (to be drafted in Phase 8)

New section `v0.2.x → v0.3.0`:
1. **Wire format**: HRP changed from `wdm` to `md`. Strings starting with `wdm1...` are no longer valid v0.3.0 inputs; re-encode from descriptor source if needed.
2. **Crate name**: `Cargo.toml` dependency `wdm-codec = "..."` → `md-codec = "0.3"`.
3. **Library import**: `use wdm_codec::...` → `use md_codec::...`. Type names mostly stable except those with `Wdm` prefix (e.g., `WdmBackup` → `MdBackup`).
4. **CLI command**: `wdm encode ...` → `md encode ...`. Subcommand surface and flags unchanged.
5. **Test vector SHAs**: `v0.1.json` and `v0.2.json` SHA pins both change. Conformance suites must update.
6. **Repository URL**: unchanged.

---

## Open items at decision-log freeze

To be filled during execution:
- `<TODO: new v0.1.json SHA after Phase 6 regen>`
- `<TODO: new v0.2.json SHA after Phase 6 regen>`
- `<TODO: discovery agent's "surprises" callout — anything not in this decision log that the rename will break>`
- `<TODO: final touch-point count from discovery vs. plan estimate>`

---

## Execution status

- [x] Pre-flight Gate 1 (HRP vet) — complete 2026-04-27
- [x] Pre-flight Gate 2 (decision matrix) — complete 2026-04-27
- [x] Pre-flight Gate 3 (deprecation banner drafted) — complete 2026-04-27
- [ ] Phase 0 (Discovery agent dispatch) — pending
- [ ] Phase 1 (Plan agent dispatch) — pending
- [ ] Phases 2–10 (subagent-driven-development execution) — pending
- [ ] Phase 11 (SLIP-0173 PR — post-release) — pending

This file is the canonical reference for all rename-related decisions. If discovery or execution surfaces a question not answered here, STOP and update this file before proceeding.
