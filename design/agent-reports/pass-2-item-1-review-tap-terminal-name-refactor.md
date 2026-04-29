# Pass 2 item 1 — review: tap_terminal_name refactor (terminal_to_tag delegation)

**Status:** DONE
**Commit:** (uncommitted at review time — reviewing working-tree changes; will be committed by controller post-review)
**File(s):**
- `crates/md-codec/src/bytecode/encode.rs`
- `crates/md-codec/src/bytecode/decode.rs`
- `crates/md-codec/src/bytecode/tag.rs` (read for cross-reference, not modified)
**Role:** reviewer (code-quality)

## What was reviewed

The refactor rewrites `tap_terminal_name` in encode.rs to delegate to `tag_to_bip388_name` in decode.rs via a new `terminal_to_tag` adapter, eliminating the parallel hand-maintained operator-name tables described in FOLLOWUPS entry `v0-5-tap-terminal-name-and-tag-to-bip388-name-parallel-tables`. The review checked: (1) correctness of the `terminal_to_tag` mapping against `tag.rs`; (2) robustness of the `SortedMultiA` fallback; (3) test coverage of the new test; (4) project-convention compliance; (5) defensive catch-all duplication; (6) anything else worth flagging.

## Findings

### Check 1 — `terminal_to_tag` mapping correctness

All 27 arms were verified against `tag.rs`. No swap errors detected. Specific spot-checks for the high-risk groups:

Wrappers: `Alt→Tag::Alt (0x0A)`, `Swap→Tag::Swap (0x0B)`, `Check→Tag::Check (0x0C)`, `DupIf→Tag::DupIf (0x0D)`, `Verify→Tag::Verify (0x0E)`, `NonZero→Tag::NonZero (0x0F)`, `ZeroNotEqual→Tag::ZeroNotEqual (0x10)` — all correct.

OR family: `OrB→Tag::OrB (0x14)`, `OrC→Tag::OrC (0x15)`, `OrD→Tag::OrD (0x16)`, `OrI→Tag::OrI (0x17)` — all correct, in canonical BIP 388 order.

No issues found.

### Check 2 — SortedMultiA fallback robustness

The safety net is sound. The test asserts `terminal_to_tag(&sma).is_none()`. If `Tag::SortedMultiA` is added in a future miniscript upgrade and `terminal_to_tag` is updated to return `Some(Tag::SortedMultiA)` for `Terminal::SortedMultiA`, that assertion fails (expected `None`, got `Some(...)`), forcing the developer to revisit both `terminal_to_tag` and the `tap_terminal_name` fallback match arm. The guard works as described.

There is one subtle point: the fallback in `tap_terminal_name` is a match with arm `Terminal::SortedMultiA(_) => "sortedmulti_a"`. If a future `Tag::SortedMultiA` exists but `terminal_to_tag` is NOT updated to return it (a developer updates the Tag enum without touching `terminal_to_tag`), the diagnostic would still silently serve the hardcoded literal string rather than consulting `tag_to_bip388_name`. However, this scenario requires `terminal_to_tag` to be stale while `tag_to_bip388_name` is updated — a two-file inconsistency that is exactly the hazard the refactor was designed to fix. The `is_none()` assertion in the test catches this: if `Tag::SortedMultiA` is added, `terminal_to_tag` will need updating (the match is exhaustive today and the `#[allow(unreachable_patterns)]` wildcard will still compile but the correct fix is to add the new arm), and the test assertion will fail, prompting review. Confidence: the protection is adequate given the non-`#[non_exhaustive]` status of `Terminal`.

No issues found.

### Check 3 — Test adequacy

The test `tap_terminal_name_delegates_to_tag_to_bip388_name` enumerates 30 `(Terminal, Tag)` pairs and asserts byte-identical names for each, then separately asserts the `SortedMultiA` `None` + `"sortedmulti_a"` fallback.

Coverage against the Tag enum (excluding top-level wrappers `Wsh/Tr/Wpkh/Sh/Pkh/Bare/TapTree` and reserved/framing tags `0x24–0x33/0x35`):

All 27 Terminal-to-Tag pairs that exist are exercised:
- Constants: `True`, `False`
- Keys: `PkK`, `PkH`, `RawPkH`
- Multisig: `Multi`, `SortedMulti`, `MultiA`
- Timelocks: `After`, `Older`
- Hashes: `Sha256`, `Hash256`, `Ripemd160`, `Hash160`
- Wrappers: `Alt`, `Swap`, `Check`, `DupIf`, `Verify`, `NonZero`, `ZeroNotEqual`
- Logical: `AndV`, `AndB`, `AndOr`, `OrB`, `OrC`, `OrD`, `OrI`, `Thresh`

`SortedMultiA` (no Tag) is separately exercised at the bottom of the test.

One observation worth noting (not a blocking issue): `Terminal::Multi` and `Terminal::SortedMulti` appear in the test with a `Tap`-context type annotation. These are Segwitv0-only operators in practice — miniscript's type system prevents them from appearing in actual Tap-context parsed expressions. However, `Terminal` is generic and the Rust type system allows constructing them as `Terminal<DescriptorPublicKey, Tap>` directly (no `from_ast` call on the outer terminal). The test correctly exercises the `terminal_to_tag` mapping for these variants, which `tap_terminal_name` will see if a Tap-context terminal somehow carries them. This is fine; the test is testing the name-mapping function, not miniscript's type rules.

No gaps in coverage. Test is tight.

### Check 4 — Project conventions

`terminal_to_tag` carries its weight. The function eliminates the maintenance hazard without introducing layering that outlives its purpose. The rustdoc on `tap_terminal_name` correctly explains the delegation and the `SortedMultiA` wrinkle without restating what the code obviously does. No "see Task X" plan-task references. The visibility change on `tag_to_bip388_name` (`pub(crate)`) is the minimal change needed to allow cross-module access. No over-engineering detected.

### Check 5 — Catch-all defensiveness (duplication)

`terminal_to_tag` has `#[allow(unreachable_patterns)] _ => return None`. `tap_terminal_name` has `_ => "<unknown-terminal>"` in its post-`if let` match. These serve different roles:

- The wildcard in `terminal_to_tag` returns `None` for any Terminal variant added by a future miniscript upgrade that the function doesn't yet know about. Without it, a new variant would cause `terminal_to_tag` to be the error site (non-exhaustive match compile error). With it, a new variant silently falls through to `None`, and `tap_terminal_name`'s own `_ => "<unknown-terminal>"` arm surfaces it to callers as `"<unknown-terminal>"` rather than delegating via `tag_to_bip388_name`. The `#[allow(unreachable_patterns)]` keeps the guard in place against a future upgrade.

- The wildcard in `tap_terminal_name` is the last-resort fallback for the case where `terminal_to_tag` returns `None` for a variant that is neither `SortedMultiA` nor a future allocated variant.

The duplication is not removable cleanly: removing the wildcard from `terminal_to_tag` would leave it non-exhaustive if Terminal gains new variants (compile error without `#[non_exhaustive]` on Terminal); removing the wildcard from `tap_terminal_name` would leave a case where a new variant returns `None` from `terminal_to_tag` and isn't handled by the `SortedMultiA` arm, producing a compile error. Both guards are necessary for the layered defense. No issue.

### Check 6 — Other

Nothing warrants flagging at the ≥80 confidence threshold. The decode.rs change is strictly limited to adding `pub(crate)` to `tag_to_bip388_name` and updating its rustdoc — both are correct and minimal.

## Critical issues

None.

## Important issues

None.

## Follow-up items

The FOLLOWUPS entry `v0-5-tap-terminal-name-and-tag-to-bip388-name-parallel-tables` is resolved by this refactor. The controller should update its Status field to `resolved <SHA>` once the commit lands. No new FOLLOWUPS entries warranted by this review.
