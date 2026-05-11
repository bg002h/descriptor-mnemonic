# v0.31.0 — code review r1 (2026-05-10)

**Working tree:** dirty at HEAD `7169973`; not yet committed.

**Scope:** md-codec v0.31.0 — single atomic patch bundling 3 FOLLOWUP resolutions + release mechanics. Per user directive: all reviewer findings (incl. Low/Nit) fold inline; zero new FOLLOWUPs.

**Files reviewed:** `crates/md-codec/src/{decode, error, derive}.rs`, `crates/md-cli/src/format/json.rs`, `crates/md-codec/tests/{address_derivation, chunking}.rs`, `bip/bip-mnemonic-descriptor.mediawiki`, `crates/md-codec/Cargo.toml`, `crates/md-cli/Cargo.toml`, `CHANGELOG.md`, `design/FOLLOWUPS.md`, `Cargo.lock`.

---

## Critical (block ship)

None.

## Important (must fix before ship)

None.

## Low (fixed inline)

### L-1 — `decode_rejects_non_canonical_root_tag` test comment misleading (FIXED INLINE)

- **Where:** `crates/md-codec/src/decode.rs:100-101` (pre-fix).
- **What:** Test comment said "populated path_decl satisfies `validate_explicit_origin_required`" — true but misleading: the TopLevel check fires BEFORE `validate_explicit_origin_required` runs, so the path_decl validity isn't load-bearing for why the test passes. A reader investigating could mis-trace the validator chain.
- **Fix (applied):** Rewrote the comment to clarify the TopLevel check short-circuits above the downstream validators; path_decl is populated to mirror a realistic shape, not because it's necessary for the test outcome.
- **Re-test:** `cargo test -p md-codec --lib decode_rejects_non_canonical_root_tag` — passes.

## Nit

None above the confidence threshold.

---

## Correctness checks (all passed)

1. **TopLevel check.** `decode.rs:36-44` — check fires AFTER `read_node`; allow-list is exactly `{Sh, Wsh, Wpkh, Pkh, Tr}`; raises `OperatorContextViolation { tag: tree.tag, context: ContextKind::TopLevel }`. Comment cites SPEC §11 + the wrapper-tag vs canonical-shape distinction per architect r1 I-1. ✓
2. **`decode_rejects_non_canonical_root_tag`.** Constructs `Tag::AndV`-rooted Descriptor; encode succeeds (encoder has no root-tag check); decode rejects with the exact expected error. ✓
3. **`JsonTag` mirror.** All 36 variants present (cross-checked against `tag.rs`); `#[serde(rename_all = "PascalCase")]`; `From<&Tag>` with 36 explicit arms (no wildcards); line 209 replaced with `JsonTag::from(&n.tag)`; 0 leftover `format!("{:?}", ...)` invocations on enums in json.rs. ✓
4. **PascalCase pin test.** All 12 mixed-case variants pinned (PkK, PkH, RawPkH, MultiA, SortedMultiA, OrI, DupIf, NonZero, ZeroNotEqual, TapTree, False, True). ✓
5. **TapTree JSON coverage.** `tr_with_taptree_serializes_taptree_string` constructs a real `Body::Tr { tree: Some(TapTree { ... }) }` node, serializes, asserts `"TapTree"` appears. Regression coverage for `JsonTag::TapTree`. ✓
6. **error.rs doc-comment.** `OperatorContextViolation`'s doc no longer references the FOLLOWUP id; describes TopLevel as live + TapLeaf/MultiBody as intentionally not wired. ✓
7. **chunking.rs fallout fix.** `multi_chunk_descriptor` bare `Tag::SortedMulti` root wrapped in `Tag::Wsh` with `Body::Children([SortedMulti])`. Wrap is sound: split/reassemble round-trip remains the test's purpose; inner SortedMulti's chunk-spanning behavior unchanged. ✓
8. **JsonTag visibility.** `pub(crate)` matches surrounding pattern (`JsonNode` is also `pub(crate)`). ✓
9. **BIP draft prose.** Additive sentence at `bip-mnemonic-descriptor.mediawiki:866` row; cites live fire site at `decode_payload`, allow-list, TapLeaf coverage, MultiBody structural unreachability. No "no live fire site" language removed (none existed). ✓
10. **CHANGELOG.** `## md-codec [0.31.0] — 2026-05-10` entry with Added/Changed/Removed/Workspace sections; all 3 FOLLOWUPs cited as resolved; format consistent with v0.30.0 entry. ✓
11. **FOLLOWUPs resolved.** All 3 entries marked `Status: resolved (v0.31.0: ...)` with phase reference. ✓
12. **Cargo.toml + Cargo.lock + dep sync.** md-codec at 0.31.0; md-cli dep spec at 0.31.0; md-cli's own version 0.4.3 unchanged. ✓
13. **Zero new FOLLOWUPs.** Confirmed via `grep '^### `' design/FOLLOWUPS.md` count: same as pre-edit. No new entries appended. ✓

---

## Verdict

**Ship.** 0C/0I/1L (fixed inline)/0N. All architect-r1 + reviewer-r1 findings folded into the commit. Zero new FOLLOWUPs filed (user directive satisfied). Ready to commit + annotated tag `md-codec-v0.31.0` + push.
