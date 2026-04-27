# Phase 3 Decision Log

Living document of decisions made during autonomous execution of Phase 3 (Framing layer — bytecode header + path declaration). Each entry: what was decided, why, alternatives considered, where to verify in code.

## Open questions for review

(Populated as design questions arise that I picked a default for. Empty = no open questions.)

---

## Decisions made

### D-1 (Phase 3 scope): defer plan's Tasks 3.5, 3.6, and 3.9 out of Phase 3

**Context**: `design/IMPLEMENTATION_TASKS_v0.1.md` lines 2135–2145 list nine subtasks for Phase 3. Three of them either don't fit the framing-layer scope or were already absorbed elsewhere.

**Decision**: Phase 3 executes Tasks 3.1, 3.2, 3.3, 3.4, plus a combined "path declaration framing" task (plan's 3.7 + 3.8 merged because the encoder and decoder for the `Tag::SharedPath` framing share enough surface that splitting them adds review overhead without separation benefit). The remaining tasks are deferred:

- **Plan's 3.5 (IndexMap for path emission ordering)** → deferred to Phase 5. The BIP v0 spec (line 224 of `bip/bip-wallet-descriptor-mnemonic.mediawiki`) explicitly assumes a single shared path for all placeholders, so emission ordering of *multiple* paths is a v1+ concern. If the task was meant in the narrower sense of "IndexMap<placeholder_index, DerivationPath>", that data structure is owned by the parsed `WalletPolicy` (Phase 5), not by the framing layer.
- **Plan's 3.6 (encoder arm for `Tag::Placeholder`)** → already done in Phase 2 via D-7 (Phase 2 encoder emits `0x32 <single-byte index>` directly through the `placeholder_map` mechanism per D-4; the wire format described by 3.6 is exactly what Phase 2 produces).
- **Plan's 3.9 (end-to-end test `wsh(pk(@0/**))` → bytecode → `WalletPolicy` → identical canonical string)** → deferred to Phase 5. The test requires `WalletPolicy::from_str` and `WalletPolicy::to_canonical_string`, which are scheduled for Tasks 5.1–5.3. Phase 3 ships round-trip tests at the framing-primitive level (header round-trip, path round-trip, declaration round-trip) but cannot reach the WalletPolicy boundary.

**Rationale**: D-6 established the framing layer as Phase 3's scope ("header byte + path declaration"). The plan's 3.5/3.6/3.9 either pre-suppose later phases or duplicate completed work. Re-scoping now keeps Phase 3 focused on exactly the surface D-6 mandates and avoids fake "completed" tasks that get re-opened in Phase 5.

**Alternatives considered**:
- **Execute all nine tasks as listed**: rejected — 3.6 has nothing to do (would produce a no-op commit), and 3.5/3.9 would either block on Phase 5 prerequisites or require building partial Phase 5 surface inside Phase 3 (scope creep).
- **Merge Phase 3 into Phase 5**: rejected — the framing primitives (header type, path codec, declaration codec) are independently testable and reviewable; bundling them with WalletPolicy parsing would inflate review surface and lose the natural Phase 2/3/5 staging.

**Verify**: Phase 3 commit log will contain commits for 3.1 → 3.2 → 3.3 → 3.4 → 3.5' (path declaration framing) only. Plan's 3.5/3.6/3.9 will be cross-referenced from the Phase 5 plan when it executes (or this decision will be cited as a closure note).

---

(More decisions appended as Phase 3 progresses.)

---

## Tasks completed

| Task | Commit (feat / fix) | Status notes |
|------|---------------------|--------------|
| 3.1 BytecodeHeader | `e7c8f23` (no fix needed) | `#[non_exhaustive]` struct + `from_byte`/`as_byte`/`v0`; new `BytecodeErrorKind::ReservedBitsSet { byte, mask }`; version-check-priority documented and tested via `0x14`; 14 new tests (169 → 183). Code-review nits (`&self` vs `self` on Copy; `v0` → consider `new_v0`; `const fn`; `Display` for diagnostics) deferred to Task 3.5' integration pass per reviewer suggestion. |
| _style cleanup_ | `994eb24` | Pure `cargo fmt` over Phase 2 leftovers (`decode.rs`, `encode.rs`, `varint.rs`, `encoding.rs`, `lib.rs`). No semantic changes. CI fmt-check gating is still scheduled for Task 5.10. |
| 3.2 Path dictionary | `26603c3` (no fix needed) | `LazyLock<[(u8, DerivationPath); 13]>`; 13 BIP entries verified character-by-character; `0x16` testnet gap preserved; `0xFE`/`0xFF` correctly excluded; 5 new tests (183 → 188). Code-review follow-ups: (a) `&'static DerivationPath` return — re-examine if Task 3.4's `decode_path` needs owned values (likely cleanest to clone explicitly at the call site); (b) consider adding a `path_to_indicator` regression test for `m/44'/0'/0'/0` (prefix-mismatch boundary); (c) two redundant `use std::str::FromStr` imports could be hoisted. None of these block. |
| 3.3 encode_path | `081feb4` (no fix needed) | `pub fn encode_path(&DerivationPath) -> Vec<u8>`; dictionary fast-path returns `[indicator]`; explicit form emits `[0xFE, LEB128(count), LEB128(2c|2c+1)…]`; child arithmetic in u64 via `u64::from(*index)` (overflow-safe at BIP32 max `2^31-1`); reuses `varint::encode_u64`; matches sibling pattern of `encode_template` returning `Vec<u8>` from public API; 8 new tests including `m/100` multi-byte LEB128 + max-hardened boundary `m/2147483647'`; (188 → 196). Follow-up: test `encode_unknown_path_uses_explicit_form` partially re-implements decoding — rewrite as round-trip once Task 3.4 lands (done in 3.4 bonus cleanup). |
| 3.4 decode_path + Cursor extract | `e5f5a1a` (refactor) + `987496f` (feat); no fix needed | Cursor moved from `decode.rs` private struct to new `bytecode/cursor.rs` as `pub(crate)`; `pub(crate) fn decode_path(&mut Cursor) -> Result<DerivationPath, Error>` reverses encode_path; explicit form decodes LEB128 count+children with `n & 1`/`n >> 1` split; bound `n > 0xFFFF_FFFF` (= max valid encoded value at BIP32 max hardened `2^31-1`) prevents `u32` cast truncation; new `BytecodeErrorKind::InvalidPathComponent { encoded: u64 }`; reserved indicators reuse `UnknownTag(b)`; 7 new tests + Task 3.3 test simplified to round-trip; (196 → 203). `#[allow(dead_code)]` on `decode_path` until Task 3.5' wires it. Follow-ups (all addressed in 3.5'): cursor field visibility, dead_code migration, count cast → `try_from`, `n>>1` cast → `try_from`, truncation-test tightening. |
| 3.5' Path declaration framing | `bdeb639` (no fix needed) | `pub fn encode_declaration(&DerivationPath) -> Vec<u8>` emits `[Tag::SharedPath.as_byte()] ++ encode_path(path)`; `pub(crate) fn decode_declaration(&mut Cursor) -> Result<DerivationPath, Error>` does 3-way `Tag::from_byte` dispatch (`Some(SharedPath)` → `decode_path`; `Some(other)` → new `UnexpectedTag { expected: u8, got: u8 }`; `None` → `UnknownTag`); chose `u8`/`u8` for `UnexpectedTag` to avoid coupling errors to `#[non_exhaustive] Tag`; `#[allow(dead_code)]` migrated from `decode_path` to `decode_declaration` (single transitive root pending P5); 8 new tests including pinned wire layouts `[0x33, 0x01]` (dict) and `[0x33, 0xFE, 0x02, 0x59, 0x00]` (explicit `m/44'/0`) + `0xC0` unknown-tag rejection + `[0x33]` truncation; (203 → 211). All 5 cleanup follow-ups from 3.4 review applied in this same commit. Code-review follow-ups (defer to P5 framing pass): (a) save `count_offset` before reading varint so 32-bit-only `usize::try_from` failures point at varint start, not after; (b) add boundary test for explicit path with ≥128 components (multi-byte LEB128 *count*). |

---

## Phase 3 closure

Phase 3 is feature-complete as of `bdeb639` (211 lib tests passing, clippy `-D warnings` clean, fmt clean). The framing primitives — `BytecodeHeader` + `encode_path`/`decode_path` + path dictionary + `encode_declaration`/`decode_declaration` — are all implemented, tested, and ready for Phase 5 to compose into `encode_bytecode` / `decode_bytecode`.

The `#[allow(dead_code)]` on `decode_declaration` is the single remaining pending-integration marker; it goes away when Phase 5's framing wrapper calls into it.

**Deferred to later phases (per D-1 above):**
- Plan's Task 3.5 (IndexMap path emission ordering) → Phase 5 (WalletPolicy)
- Plan's Task 3.6 (placeholder encoder arm) → already done in Phase 2 via D-7
- Plan's Task 3.9 (end-to-end test through WalletPolicy) → Phase 5

**Code-review follow-ups (status as of 2026-04-27):**
- ✅ `count_offset` diagnostic offset — addressed in `bdc0c3f` (path.rs followups bucket)
- ✅ Multi-byte explicit-path component-count boundary test — addressed in `bdc0c3f`
- ✅ Task 3.1 `BytecodeHeader` review nits (Copy receiver, `v0` → `new_v0`, `const fn`, module-doc cleanup, bit-2 comment) — addressed in `df7bb4e`
- ✅ Task 3.2 prefix-mismatch regression test, hoisted `use std::str::FromStr` — addressed in `bdc0c3f`
- ⏸ `decode_declaration_from_bytes(&[u8]) -> Result<(DerivationPath, usize), Error>` — still deferred (no v0.1 consumer; revisit if Phase 5 framing wrapper finds the Cursor-shaped API inconvenient)
