# SPEC — LANE2-walker: backport `Terminal::RawPkH` arm to md-cli's `walk_miniscript_node`

**Repo:** `descriptor-mnemonic` (md-cli only). **Default branch:** `main`. **Source SHA verified against:** `f18a027` (origin/main, re-grepped at write time).
**SemVer:** md-cli MINOR `0.9.2 → 0.10.0`; md-codec **NO bump** (stays `0.39.0` — wire `Tag::RawPkH=0x21` + `Body::Hash160Body` + decode already exist).
**Ship:** md-cli direct-to-`main` FF + tag `descriptor-mnemonic-md-cli-v0.10.0` + crates.io publish.

---

## ⚠️ FOUR recon corrections (empirically proven; the plan/test MUST follow THESE, not the recon JSON)

I built the arm in a throwaway local build (worktree at `f18a027`) and ran it against the live md-cli (miniscript `=13.0.0`). FOUR claims in the recon JSON / earlier draft are **factually wrong** and would mislead a TDD implementer:

1. **Test entry point is NOT `parse_template`.** A literal-hash RawPkH descriptor has **no `@N` placeholder**, so `parse_template("wsh(c:expr_raw_pkh(<hash>))", &[], &[])` fails EARLY at `resolve_placeholders` (template.rs:411) with **`"template contains no @i placeholders"`** — it never reaches the walker. **Use the lower-level path** `MsDescriptor::<DescriptorPublicKey>::from_str(...)` + `walk_root(&d, &km)`, exactly like the sibling `wpkh_root` / `pkh_root` tests in `mod root_tests` (template.rs:1267-1282). (Empirically confirmed: probe printed `parse_template ERR: template contains no @i placeholders`.)

2. **The walked tree is `Wsh → Check → RawPkH`, NOT `Wsh → RawPkH`.** The `c:` (Check) wrapper at the walker's `Terminal::Check` arm (template.rs:939-960) only short-circuit-collapses inner `PkK`/`PkH`; for a `RawPkH` child it falls through to `Tag::Check` → `Body::Children([Node{tag: RawPkH, ...}])`. So the assertion must descend **two** levels: `Wsh.body.Children[0]` is `Tag::Check`; `Check.body.Children[0]` is `Tag::RawPkH`. (Empirically confirmed: probe printed `L1 tag=Check`, `L2 tag=RawPkH body=Hash160Body([0;20])`.)

3. **The render round-trip target is `wsh(c:expr_raw_pkh(<hex>))` with NO `#<csum>`** — not the recon's `wsh(expr_raw_pkh(<hash>))#<csum>`. `descriptor_to_template` emits the `c:` prefix and produces **no checksum**. (Empirically confirmed: probe printed `RENDER back=Ok("wsh(c:expr_raw_pkh(0000…0000))")`.)

4. **(CRITICAL — added by R0; see Change 1)** Adding the `Terminal::RawPkH` arm makes the existing catch-all `_ => Err(...)` at template.rs:1130-1132 **PROVABLY UNREACHABLE**, which under the repo's `-D warnings` CI gate is a **HARD COMPILE ERROR**, not a warning. `Terminal::RawPkH` is the **sole** uncovered variant (27 arms present, RawPkH is the 28th and last), and `miniscript::Terminal` is **not** `#[non_exhaustive]` (verified at miniscript-13.0.0 `decode.rs:90` — no attribute above `pub enum Terminal`). So the arm closes the last gap and kills the wildcard. **The arm MUST be paired with `#[allow(unreachable_patterns)]` on the catch-all** — see Change 1 for the empirically-proven remedy. The earlier draft's Verification step 5 ("no new lint") and "Regression risk: VERY LOW" prose were both **wrong on this point** and are corrected below.

These corrections are the whole point of the single-author re-grep gate; the arm body itself is exactly as the recon describes.

---

## Change 1 — Add the `Terminal::RawPkH` walker arm **AND** silence the now-unreachable catch-all

**File:** `crates/md-cli/src/parse/template.rs`

**Current behavior:** `walk_miniscript_node` (template.rs:903-1133) has **27** `Terminal` arms but **no `Terminal::RawPkH`**. A `Check(RawPkH)` child therefore falls into the catch-all `_ => Err(CliError::TemplateParse("unsupported miniscript fragment: {ms}"))` at template.rs:1130-1132. Today `MsDescriptor::from_str("wsh(c:expr_raw_pkh(<hash>))")` PARSES (display `wsh(expr_raw_pkh(...))#2ey4n0ax`) but `walk_root` errors `unsupported miniscript fragment: expr_raw_pk_h(...)`. The arm **IS reachable** via `Check(RawPkH)` — disproving the FOLLOWUPS "descriptor-unreachable" claim.

### Edit 1a — insert the arm

Insert directly **after** the `Terminal::Hash160(h)` arm (currently template.rs:1060-1063, the last of the Sha256/Hash256/Ripemd160/Hash160 hash-terminal cluster) and **above** the single-arity-wrapper cluster (`Terminal::Swap` at 1068). Mirrors toolkit `parse_descriptor.rs:739-742` **verbatim**:

```rust
        Terminal::RawPkH(h) => Ok(Node {
            tag: Tag::RawPkH,
            body: Body::Hash160Body(h.to_byte_array()),
        }),
```

`to_byte_array()` needs `bitcoin::hashes::Hash` — **already imported** at the top of the fn (template.rs:907 `use bitcoin::hashes::Hash;`). The `Tag` and `Body` **types** are already in scope via the module-level `use md_codec::tag::Tag;` (template.rs:767) and `use md_codec::tree::{Body, Node};` (template.rs:768) — enabling `Tag::RawPkH` / `Body::Hash160Body` path access with **no new import** (the enum VARIANTS are reached through those type paths, not separately imported).

**Anchor uniqueness check (for the Edit):** the `Terminal::Hash160(h) => Ok(Node { tag: Tag::Hash160, body: Body::Hash160Body(h.to_byte_array()) }),` block is unique in the file — insert after it.

### Edit 1b — ⚠️ MANDATORY: silence the now-unreachable catch-all (the CRITICAL fold)

`Terminal::RawPkH` was the **LAST** uncovered `Terminal` variant. With the arm in place, **all 28** `Terminal` variants are matched explicitly, so the `_ =>` catch-all at template.rs:1130 becomes **provably unreachable**. `miniscript::Terminal` is **not** `#[non_exhaustive]` (miniscript-13.0.0 `decode.rs:90`), so the compiler fires `unreachable_patterns` — and under the repo's CI gate (`RUSTFLAGS: "-D warnings"` at `ci.yml:10` **and** `cargo clippy --workspace --all-targets -- -D warnings` at `ci.yml:47`) that becomes a **HARD COMPILE ERROR** (`error: unreachable pattern … could not compile`), not a warning.

**Empirically reproduced** in a throwaway worktree at `f18a027`: with the arm but WITHOUT the `#[allow]`, `RUSTFLAGS="-D warnings" cargo build -p md-cli` FAILS:
```
error: unreachable pattern
1134 |         _ => Err(CliError::TemplateParse(format!(
     |         ^ ...and 24 other patterns collectively make this unreachable
     = note: `-D unreachable-patterns` implied by `-D warnings`
     = help: to override `-D warnings` add `#[allow(unreachable_patterns)]`
```
(The compiler's own `help:` line names the exact fix.)

**REMEDY (empirically proven GREEN):** Insert `#[allow(unreachable_patterns)]` directly **above** the `_ => Err(CliError::TemplateParse(...))` catch-all at template.rs:1130. Final shape:

```rust
        // Other miniscript fragments — TemplateParse error until BIP 388 templates need them.
        #[allow(unreachable_patterns)]
        _ => Err(CliError::TemplateParse(format!(
            "unsupported miniscript fragment: {ms}"
        ))),
```

**Rationale for keeping the now-dead arm** (rather than deleting it): preserves the graceful runtime `TemplateParse` error path for any *future* miniscript variant (e.g., if a later `miniscript` pin adds a `Terminal` variant — at which point the `#[allow]` becomes a no-op again and the arm re-activates). Deleting the catch-all instead would be brittle across miniscript upgrades. With the `#[allow]` I confirmed all gates GREEN: `RUSTFLAGS="-D warnings" cargo build -p md-cli` clean, `cargo clippy -p md-cli --all-targets -- -D warnings` clean.

**SCOPE FENCE:** Add **only** the RawPkH arm (+ the `#[allow]`). Do **NOT** add a `SortedMultiA` Terminal arm — `miniscript 13.0.0` has **no `Terminal::SortedMultiA` variant** (the 28 variants at `decode.rs:90-157` are: True, False, PkK, PkH, RawPkH, After, Older, Sha256, Hash256, Ripemd160, Hash160, Alt, Swap, Check, DupIf, Verify, NonZero, ZeroNotEqual, AndV, AndB, AndOr, OrB, OrD, OrC, OrI, Thresh, Multi, MultiA). sortedmulti at descriptor root is already handled via `WshInner::SortedMulti`/`ShInner::SortedMulti` in `walk_wsh_inner`/`walk_sh` (template.rs:877/894). RawPkH is the **sole** missing Terminal arm. This closes the entire residual encode-side walker gap; the toolkit slug's "all 24 v0.3 arms + sortedmulti_a" framing is over-scoped relative to reality.

---

## Change 2 — Add the walk-side round-trip TDD test (write BEFORE the arm; RED → GREEN)

**File:** `crates/md-cli/src/parse/template.rs`, in **`#[cfg(test)] mod root_tests`** (template.rs:1262-1303, `use super::*` already present at 1264; `use std::str::FromStr` at 1265). Add **after** `build_multi_node_rejects_k_above_thirty_two` (closes at 1302), before the module's closing `}` at 1303.

**TDD ordering caveat (from the CRITICAL fold):** This test is RED for the *right* reason only when the catch-all still exists and compiles. The intended RED→GREEN flow:
- **Step A (RED):** Write Change 2 test FIRST, with **neither** Edit 1a (arm) nor Edit 1b (`#[allow]`) applied. `cargo test -p md-cli --bin md walk_rawpkh` compiles (catch-all is live, no unreachable lint) and the test FAILS at runtime with `"unsupported miniscript fragment"` — proving the gap.
- **Step B (GREEN):** Apply Edit 1a **and** Edit 1b **together** (the arm cannot land without the `#[allow]` or the crate won't compile under `-D warnings`; locally `cargo build` without `-D warnings` only *warns*, but CI is `-D warnings` — so always apply both as one atomic change). Re-run → GREEN.

```rust
    /// v0.10.0 — `Terminal::RawPkH` walker arm. `wsh(c:expr_raw_pkh(<hash>))`
    /// PARSES through miniscript 13.0.0's descriptor parser and walks to
    /// `Wsh → Check → RawPkH` (the `c:`/Check wrapper does NOT collapse for a
    /// RawPkH child — only for PkK/PkH). Closes the encode half of the round-trip
    /// whose render half already ships (md-cli format/text.rs:182 Tag::RawPkH arm).
    /// NOTE: this form has no `@N` placeholder, so it CANNOT go through
    /// `parse_template` (that path refuses "template contains no @i placeholders");
    /// the walker is reached via `MsDescriptor::from_str` + `walk_root`, mirroring
    /// `wpkh_root`.
    #[test]
    fn walk_rawpkh_wsh_check_emits_rawpkh_node() {
        let s = "wsh(c:expr_raw_pkh(0000000000000000000000000000000000000000))";
        let d = MsDescriptor::<DescriptorPublicKey>::from_str(s).expect("parse");
        let km = std::collections::BTreeMap::<String, u8>::new();
        let root = walk_root(&d, &km).expect("walk_root must succeed post-RawPkH-arm");
        assert_eq!(root.tag, Tag::Wsh);
        // Wsh → Check → RawPkH
        let check = match &root.body {
            Body::Children(ch) => ch[0].clone(),
            _ => panic!("Wsh body must be Children"),
        };
        assert_eq!(check.tag, Tag::Check);
        let rawpkh = match &check.body {
            Body::Children(ch) => ch[0].clone(),
            _ => panic!("Check body must be Children"),
        };
        assert_eq!(rawpkh.tag, Tag::RawPkH);
        assert!(matches!(rawpkh.body, Body::Hash160Body(h) if h == [0u8; 20]));
    }
```

**Run targeted:** `cargo test -p md-cli --bin md walk_rawpkh` (note: md-cli is a **binary** crate, bin name `md` — use `--bin md`, NOT `--lib`).

**Footgun to avoid (recon-flagged + re-confirmed):** Do **not** write the input as bare `wsh(expr_raw_pkh(...))` — miniscript rejects it as `non-T miniscript` (it's type K, can't top a B-requiring body); only the `c:`-wrapped form parses.

**(Optional, NOT required)** A full `descriptor_to_template` render round-trip is feasible but adds `md_codec::origin_path` import noise; the existing render-side test already pins the render half — see Open Questions. If added, assert output `== "wsh(c:expr_raw_pkh(0000…0000))"` (no `#csum`).

---

## Change 3 — Update the now-stale render-side test doc-comment

**File:** `crates/md-cli/src/format/text.rs`

**Current behavior:** The doc-comment on `render_bare_rawpkh_emits_expr_raw_pkh` (text.rs:686-693) asserts: *"The walker doesn't emit `Tag::RawPkH` from any BIP 388 wallet-policy input (no `Terminal::RawPkH` walker arm), so this path is unreachable via `parse_template`."* After Change 1, **the first half is false** — the walker now emits `Tag::RawPkH` for `wsh(c:expr_raw_pkh(...))`. (The "unreachable via `parse_template`" half remains technically true only because that specific *placeholderless* form has no `@N` and `parse_template` rejects it for that reason — but the *walker* is reachable via `walk_root`.)

**Exact edit:** Replace the stale sentence (text.rs:686-689, the lines reading `/// v0.4.3 — bare \`Tag::RawPkH\` rendering. The walker doesn't emit` … `/// \`parse_template\`.`) with corrected text, e.g.:

```rust
    /// v0.4.3 — bare `Tag::RawPkH` rendering, unit-pinned at the `render_node`
    /// level. Since v0.10.0 the walker DOES emit `Tag::RawPkH` (for the parseable
    /// `wsh(c:expr_raw_pkh(<hash>))` form → `Wsh → Check → RawPkH`); the walk half
    /// is pinned by `walk_rawpkh_wsh_check_emits_rawpkh_node` in `parse::template`.
    /// This test still constructs the Node directly to pin the bare-node rendering
    /// invariant in isolation (no `@N` placeholder is involved, so the full
    /// `parse_template` pipeline — which requires placeholders — is not the entry
    /// point here).
```

The test body (text.rs:695-710) and its assertion (`expr_raw_pkh(0000…)`) are **unchanged** — the bare-node `render_node` form (no `wsh(`/`c:`/checksum) is still correct.

---

## Change 4 — CHANGELOG entry

**File:** `CHANGELOG.md` (single combined md-codec+md-cli file at repo root; **no per-crate CHANGELOG**). Newest-at-top, prefix convention `## md-cli [X.Y.Z] — DATE`. Insert a new block **above** the current top entry `## md-cli [0.9.2] — 2026-06-21` (line 8):

```markdown
## md-cli [0.10.0] — <DATE>

**SemVer-MINOR — `walk_miniscript_node` gains the final missing `Terminal::RawPkH` arm; widens the accepted input set (a previously-erroring valid descriptor now encodes). `md-codec` UNTOUCHED (wire `Tag::RawPkH`/`Body::Hash160Body` + decode already existed; the render half already shipped in v0.4.3).**

### Added

- `parse::template::walk_miniscript_node` now translates `Terminal::RawPkH(h)` → `Node { tag: Tag::RawPkH, body: Body::Hash160Body(h.to_byte_array()) }`, completing the encode half of the `expr_raw_pkh` round-trip. `wsh(c:expr_raw_pkh(<20-byte-hash>))` (the only parseable shape — bare `expr_raw_pkh(...)` is a non-T and rejected by miniscript) now walks to `Wsh → Check → RawPkH` and `descriptor_to_template` renders it back to the string `wsh(c:expr_raw_pkh(<hex>))`. RawPkH was the LAST missing `Terminal` arm in md-cli's walker (27 already present, now 28 — full encode-side parity). Since this was the final uncovered variant, the walker's `_ =>` catch-all is now unreachable; it is retained behind `#[allow(unreachable_patterns)]` so a future miniscript variant still hits a graceful `TemplateParse` error. No new flag/option/subcommand/output-shape → no manual-mirror, no toolkit cross-tool-differential cascade (pinned at frozen tag v0.7.1). `md-codec` stays `=0.39.0`.

### Scope note

- This closes the **descriptor → walk → Node → render-string** loop (`descriptor_to_template` → `wsh(c:expr_raw_pkh(<hex>))`). It does **NOT** make a full `descriptor → md1 → descriptor` codec round-trip work: md-codec's md1→miniscript DECODE path (`to_miniscript.rs:614`) STILL intentionally refuses `Tag::RawPkH` ("not constructible through miniscript's public API"). A full codec round-trip is out of scope and not claimed. Closes md FOLLOWUP `terminal-rawpkh-walker-arm-missing` and toolkit FOLLOWUP `walker-backport-to-md-cli`.
```

---

## Change 5 — Version bump (two sites; both required by md release ritual)

- `crates/md-cli/Cargo.toml:3` — `version = "0.9.2"` → `version = "0.10.0"`.
- `Cargo.lock:480` — under `name = "md-cli"` (block at Cargo.lock:479-481), `version = "0.9.2"` → `version = "0.10.0"`. (Edit the lockfile directly OR run `cargo build -p md-cli` to regenerate; verify only the md-cli version line moved.)
- **md-codec Cargo.toml: do NOT touch** (stays `0.39.0`).

---

## Change 6 — FOLLOWUP flips (cross-repo lockstep, in the shipping commit)

### 6a. md repo — `descriptor-mnemonic/design/FOLLOWUPS.md` (entry at 174-181)

Flip `terminal-rawpkh-walker-arm-missing` and **correct the false rationale**:
- **`- **Status:** \`open\`` (line 180) → `- **Status:** \`resolved\` (2026-06-23, md-cli v0.10.0).`**
- Replace the **`- **Why deferred:**`** paragraph (line 179) — its claim *"md-cli's `Descriptor::from_str` … rejects `expr_raw_pkh(...)` upstream of the walker"* is **empirically false**. New text: *"Resolution corrects a false premise: the arm is NOT descriptor-unreachable. `wsh(c:expr_raw_pkh(<hash>))` PARSES through md-cli's miniscript 13.0.0 pin (display `wsh(expr_raw_pkh(...))#…`) and walks to `Terminal::Check(Terminal::RawPkH(<hash>))` → caught by the walker's `Check` arm which recurses into the (previously-missing) `RawPkH` arm. Bare `expr_raw_pkh(...)` is rejected only as a non-T; the `c:` wrapper makes it type B and parseable. Added the walker arm + a `MsDescriptor::from_str`/`walk_root` round-trip test (the form is placeholderless so `parse_template` is not its entry point). Because RawPkH was the LAST uncovered `Terminal` variant, the walker's catch-all `_ =>` is now unreachable and is retained behind `#[allow(unreachable_patterns)]` (required — `-D warnings` would otherwise hard-fail the build)."*
- Also fix the **`- **Where:**`** sentence (line 177): the clause *"The encoder therefore cannot produce `Tag::RawPkH` from any input, even hypothetically"* is now resolved — append/replace with a note that the arm now exists.
- Add **`- **Companion:** \`mnemonic-toolkit/design/FOLLOWUPS.md\` — \`walker-backport-to-md-cli\`** (the entry currently has no Companion line; the cross-repo rule requires one).

### 6b. toolkit repo — `mnemonic-toolkit/design/FOLLOWUPS.md` (entry at 1552-1559)

Flip `walker-backport-to-md-cli` and **narrow the over-scoped claim**:
- **`- **Status:** \`open\`` (line 1558) → `- **Status:** \`resolved\` (2026-06-23, md-cli v0.10.0).`**
- Correct the **`- **What:**`** sentence (line 1556): the claim *"all 24 v0.3-NEW `Terminal` arms … md-cli's walker … rejects all of these"* is stale — md-cli's walker already had 27 Terminal arms; **RawPkH was the SOLE missing one**. New text: *"Reality at md `f18a027`: md-cli's `walk_miniscript_node` already covered 27 `Terminal` arms (hash terminals, timelocks, all wrappers, AND/OR/AndOr/Thresh, Multi/MultiA, True/False); `Terminal::RawPkH` was the ONLY missing arm — backported in md-cli v0.10.0, achieving full encode-side walker parity (28/28). NOTE: no `Terminal::SortedMultiA` exists in miniscript 13.0.0 (sortedmulti is handled at descriptor-inner level, not as a Terminal), so 'sortedmulti_a' was never a Terminal-arm gap."*
- Add **`- **Companion:** \`descriptor-mnemonic/design/FOLLOWUPS.md\` — \`terminal-rawpkh-walker-arm-missing\`**.
- (Residual note, optional) the toolkit's own `arm_raw_pkh` `#[ignore = "RawPkH is descriptor-unreachable …"]` stub at `parse_descriptor.rs:2777` carries the same false belief; un-ignoring it is OUT OF SCOPE for this md-only fork (would force a toolkit commit) — mention as a residual if desired.

---

## Files touched (summary)

| File | Change |
|---|---|
| `crates/md-cli/src/parse/template.rs` | Change 1 (arm + `#[allow(unreachable_patterns)]`) + Change 2 (walk-side test) |
| `crates/md-cli/src/format/text.rs` | Change 3 (stale doc-comment) |
| `crates/md-cli/Cargo.toml` | Change 5 (version) |
| `Cargo.lock` | Change 5 (md-cli version line) |
| `CHANGELOG.md` | Change 4 |
| `descriptor-mnemonic/design/FOLLOWUPS.md` | Change 6a |
| `mnemonic-toolkit/design/FOLLOWUPS.md` | Change 6b (cross-repo; commit in toolkit repo) |

**Stage paths explicitly — no `git add -A`.**

---

## Verification sequence (the G1-B "HOW") — CI gates each step re-fires

The descriptor-mnemonic CI gates (`.github/workflows/ci.yml`) re-fired here:
- `ci.yml:10` — `RUSTFLAGS: "-D warnings"` (build/test warnings = errors)
- `ci.yml:47` — `cargo clippy --workspace --all-targets -- -D warnings`
- `ci.yml:65` — `RUSTDOCFLAGS: "-D warnings"` (doc gate)
- fmt gate — `cargo fmt --all --check`

1. **RED first:** write Change 2 test, run `cargo test -p md-cli --bin md walk_rawpkh` → MUST fail at runtime with `"unsupported miniscript fragment"` (proves the gap). (Compiles fine here — the catch-all is still live and reachable.)
2. **Apply Change 1 as one atomic edit (BOTH 1a arm + 1b `#[allow]`):** the arm without the `#[allow]` will compile-error under `-D warnings` (`error: unreachable pattern … could not compile`) because RawPkH is the last uncovered variant. Re-run `cargo test -p md-cli --bin md walk_rawpkh` → GREEN.
3. Apply Change 3 (doc-comment) → `cargo test -p md-cli --bin md render_bare_rawpkh` still GREEN (body unchanged).
4. **Full suite:** `cargo test --workspace --all-targets` and `cargo test --workspace --doc` → GREEN (re-fires `ci.yml` with `-D warnings`).
5. **Lint (the CRITICAL re-fire):** `cargo clippy --workspace --all-targets -- -D warnings` → **clean ONLY with the `#[allow(unreachable_patterns)]` from Edit 1b**; without it this step FAILS (`error: unreachable pattern`). `cargo fmt --all --check` → clean (format the arm). (Empirically proven: with `#[allow]`, both `RUSTFLAGS="-D warnings" cargo build -p md-cli` and `cargo clippy -p md-cli --all-targets -- -D warnings` Finished clean.)
6. **Doc:** `cargo doc --workspace --no-deps --document-private-items` (RUSTDOCFLAGS=-D warnings) → clean.
7. Bump version (Change 5), regenerate/verify `Cargo.lock`, CHANGELOG (Change 4), FOLLOWUP flips (6a/6b).
8. **Ship:** commit → FF to `main` → tag `descriptor-mnemonic-md-cli-v0.10.0` → `cargo publish -p md-cli` (crates.io; md-codec already published at 0.39.0).
9. **No toolkit follow-on commit** beyond the 6b FOLLOWUP flip — cross-tool-differential.yml and manual.yml both stay pinned at md-cli v0.7.1 and remain GREEN (no new clap flag, no re-fire).

---

## Regression risk: VERY LOW — with ONE compile-gate caveat

Additive-only on the accept side — converts one catch-all `Err` into a successful encode; no existing accept path changes (RawPkH always errored before, so nothing that worked can break). No test asserts RawPkH/`expr_raw_pkh` is rejected (the `unsupported miniscript fragment` rejection tests in `v017_v1_encode_acceptance.rs` cover and_v/older, not RawPkH).

**THE ONE non-trivial caveat (CRITICAL fold):** because RawPkH is the **last** uncovered `Terminal` variant and `miniscript::Terminal` is **not** `#[non_exhaustive]` (miniscript-13.0.0 `decode.rs:90`), the arm makes the `_ =>` catch-all **unreachable**, which `-D warnings` (`ci.yml:10`/`ci.yml:47`) turns into a HARD COMPILE ERROR. **Edit 1b's `#[allow(unreachable_patterns)]` is MANDATORY, not optional** — without it, CI cannot pass. Empirically verified both directions in a throwaway worktree at `f18a027`: (a) arm-only → build & clippy FAIL with `unreachable pattern`; (b) arm + `#[allow]` → fmt/clippy/build all clean.

**Wire/decode/render attribution (corrected — crate labels fixed):**
- **md-codec** RawPkH sites (UNTOUCHED): `tag.rs:84` (variant), `tag.rs:133` (`Tag::RawPkH => (0x21, None)` encode), `tag.rs:197` (`0x21 => Ok(Tag::RawPkH)` decode), `tree.rs:318` (`Tag::Ripemd160 | Tag::RawPkH =>` 160-bit body decode), and `to_miniscript.rs:614` (which **REFUSES** construction — see Scope note in Change 4).
- **md-cli** render/JSON sites (the render half already ships): `crates/md-cli/src/format/text.rs:182` (`Tag::RawPkH =>` render arm) and `crates/md-cli/src/format/json.rs:207` (`Tag::RawPkH => JsonTag::RawPkH`). **These are in md-CLI, NOT md-codec** (md-codec has no `json.rs` and no text-render RawPkH arm — corrects the earlier draft's mislabel; the line numbers were correct, the crate label was wrong).

So this completes an existing half-loop (encode + render-string), no new wire surface, no md-codec bump. The arm body is byte-identical to the toolkit's proven arm at `parse_descriptor.rs:739-742`.

---

## Open Questions

1. **Full `descriptor_to_template` render round-trip test — add or skip?** The walk-side test (Change 2) plus the existing render-side test (text.rs `render_bare_rawpkh_emits_expr_raw_pkh`, doc-comment corrected by Change 3) jointly pin both halves. A combined round-trip test would add `md_codec::origin_path` import noise for marginal coverage. Recommendation: SKIP (the two unit tests already cover both directions); flagged so the implementer doesn't add it reflexively.
2. **Toolkit `arm_raw_pkh` `#[ignore]` stub un-ignore (toolkit `parse_descriptor.rs:2777`)** carries the same now-false "descriptor-unreachable" belief. OUT OF SCOPE for this md-only fork (would force a toolkit code commit, not just the 6b FOLLOWUP flip). Track as a residual if desired.
