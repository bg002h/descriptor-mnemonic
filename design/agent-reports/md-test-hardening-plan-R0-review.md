# R0 ARCHITECT REVIEW — `IMPLEMENTATION_PLAN_md_test_hardening.md`

Opus feature-dev:code-reviewer. Mandatory pre-impl gate. Reviewed against LIVE SOURCE in `crates/md-codec/` on branch `md-codec-test-hardening`. Every API/field/variant/arity the plan depends on was grep-verified against the actual files (not the plan's snapshot block).

**Tooling note:** No `Write`/`Edit` tool available to the review agent; review returned to the controller and persisted here verbatim.

## Headline verification (confirmed against source, file:line)

- **Construction primitives all CORRECT.** `Descriptor{n,path_decl,use_site_path,tree,tlv}` (encode.rs:16-28); `Body::{Children(Vec<Node>), Variable{k,children}, MultiKeys{k,indices}, Tr{is_nums,key_index,tree:Option<Box<Node>>}, KeyArg{index}, Hash256Body, Hash160Body, Timelock(u32), Empty}` (tree.rs:18-73); `PathDecl{n,paths}` + `PathDeclPaths::Shared(OriginPath)|Divergent(Vec<OriginPath>)` (origin_path.rs:81-96); `OriginPath{components}`, `PathComponent{hardened,value}` (origin_path.rs:18-50); `Tag::{Wpkh,Pkh,PkK,PkH,Tr,Wsh,Sh,TapTree,Multi,SortedMulti,MultiA,Older}` (tag.rs:15-89); `UseSitePath::standard_multipath()` (use_site_path.rs:58); `TlvSection::new_empty()` (tlv.rs:43). All field/variant names in the plan's `common/mod.rs` + fixtures are exactly right.
- **Public API arities CORRECT.** `encode_payload(&Descriptor)->Result<(Vec<u8>,usize)>` (encode.rs:65); `decode_payload(&[u8],usize)` (decode.rs:15); `encode_md1_string`/`decode_md1_string(&str)` (encode.rs:114, decode.rs:79); `chunk::{split(&Descriptor)->Result<Vec<String>>, reassemble(&[&str]), decode_with_correction(&[&str])->Result<(Descriptor,Vec<CorrectionDetail>)>, ChunkHeader}` (chunk.rs:235/305/492/21); `canonicalize_placeholder_indices(&mut Descriptor)->Result<()>` (canonicalize.rs:168). All re-exported at lib.rs:41-56.
- **`restamp_chunk_header` surface SUFFICIENT (item 6 — NOT Critical).** `codex32::wrap_payload`/`unwrap_string` are `pub` (codex32.rs:67,92); `bitstream::{BitReader,BitWriter}` `pub`; `ChunkHeader::{read,write}` `pub` with `pub` fields `version,chunk_set_id,count,index` (chunk.rs:21-67). `wrap_payload` recomputes BCH (codex32.rs:70), so re-stamped chunks pass hard verify and the mutated header reaches the cross-chunk branches. Step-1a identity-round-trip guard is the right mitigation.
- **§4.1 T2c `!= Ok(original)` CORRECT.** `decode_regular_errors` returns `None` only for `deg==0||deg>4` (bch_decode.rs:416); a `deg≤4` spurious locator yields `Some(valid codeword)` and the re-verify guard (chunk.rs:560-570) confirms "a codeword," not "the original." `!= Ok(original)` robust; `is_err()` would be flaky.
- **Theme 3 CORRECT.** `reassemble`→`unwrap_string`→`bch_verify_regular` hard-verify (codex32.rs:115, no correction). T3c: `bch_verify_regular` returns false on `len<13` (bch.rs:77) before the "too short" message (codex32.rs:122) → `Codex32DecodeError("BCH … failed")`; broad `Codex32DecodeError(_)` pin correct.
- **Error variants** (error.rs): `TooManyErrors{chunk_index,bound}` (:400), `ChunkIndexGap{expected,got}` (:272), `ChunkSetIdMismatch{expected,derived}` (:281), `Codex32DecodeError(String)` (:247) exist. **`ChunkSetInconsistent` is a UNIT variant** (:259) — see C2.
- **T2e/f/g branches REACHABLE in the assumed order.** `reassemble` checks count/csid/version → `ChunkSetInconsistent` (chunk.rs:343-350), `ChunkSetIncomplete` (:351), sort + `ChunkIndexGap` (:360-367), decode + derived-csid → `ChunkSetIdMismatch` (:379-386).
- **P3 `decode_payload` debug-assert hazard correctly folded** — `total_bits = bytes.len()*8` pin (I1 from spec-R0). `&str` arms panic-free.
- **edition 2024, resolver 3, `[lints] workspace=true` (`missing_docs="warn"`, `clippy::all="warn"`).** `proptest` NOT yet a workspace dep — Task 0.1 genuinely new.

## CRITICAL

**C1 — `tr_taptree` generates non-contiguous placeholder indices → `canon().expect()` PANICS on most draws; P1/P2 ship broken.** `taptree_strategy(max)` draws leaf indices from `1..=max` over an arbitrary subset; `descriptor_from_tree` computes `n=set.len()` but does NOT renumber the tree — the comment "tree indices are already 0..n by construction" is FALSE for this arm. Taptree referencing `{3,7}` → `set={0,3,7}`, `n=3`, tree references `@7,@3 ≥ n` → `check_placeholder_bounds` (canonicalize.rs:174) → `Err(PlaceholderIndexOutOfRange{idx:7,n:3})` (:255-257) → `canon().expect()` panics. Default outcome, not an edge. Faithful-transcription failure: SPEC §3.1 mandated "`n` DERIVED … renumbered to `0..n`"; plan dropped the renumber half. **Fix:** in `descriptor_from_tree`, build `perm: old→rank_in_sorted_set` and rewrite every `KeyArg.index`/`MultiKeys.indices`/`Tr.key_index` through `perm` (recursive remap mirroring canonicalize.rs:102-139) BEFORE deriving `path_decl`, so emitted set is exactly `0..n`.

**C2 — T2e `matches!(reassemble(&refs), Err(Error::ChunkSetInconsistent { .. }))` is a COMPILE ERROR.** `ChunkSetInconsistent` is a unit variant (error.rs:257-259). Struct-pattern `{ .. }` won't compile → `bch_adversarial.rs` won't build → whole `cargo test -p md-codec` down. **Fix:** `Err(Error::ChunkSetInconsistent)` (no braces). Siblings at 521/531 (`ChunkIndexGap{..}`/`ChunkSetIdMismatch{..}`) ARE struct variants and are correct.

## IMPORTANT

**I1 — T2d/T2i pin a hard error contract on an UNVERIFIED hand-picked 5-error pattern → false-fail risk.** Both corrupt fixed `[1,4,7,10,13]` and assert `is_err()`/`Err(TooManyErrors{..})`. Per §4.1 a 5-error pattern is NOT guaranteed uncorrectable — BM can return a spurious `deg≤4` locator (bch_decode.rs:414-419) → different valid codeword passes re-verify → `Ok(C′)` not `Err`. **Fix:** add build-time "verify the chosen pattern yields the asserted error; substitute until it does" to BOTH; extract a shared verified `UNCORRECTABLE_5ERR` const with a re-verify-if-fixture-changes comment; loosen T2i so it isn't brittle to the chunk-0 miscorrection path.

**I2 — `corrupt_chunk_at` indexes out of bounds (panic) if `pos` exceeds data-part length.** `chars[idx]`, `idx=3+pos`, no guard. Estimate holds for stated fixtures but a future shrink → opaque `index out of bounds`. **Fix:** `assert!(idx < chars.len(), "corrupt pos {pos} past data-part")`; have T2a derive max pos from `chunks[0].chars().count()-3`.

**I3 — `multi_chunk_descriptor` lands on the 2/3-chunk boundary; T2f needs ≥3.** 4 Divergent×15 → payload ≈900 bits / `SINGLE_STRING_PAYLOAD_BIT_LIMIT=320` (chunk.rs:219) → exactly 3. Boundary-fragile. Runtime assert prevents false-pass but **Fix:** make `split(&d).len()` a required Step-1a output AND pre-emptively enlarge (6 cosigners) so T2f isn't boundary-close.

## MINOR

**M1 — Spec shapes `sh(sortedmulti)` (legacy P2SH, `canonical_origin==None`, canonical_origin.rs:65-75) and `tr(<NUMS>,<taptree>)` (is_nums=true, NUMS-skip canonicalize.rs:63,113-114) NOT built.** Coverage gap vs R0-GREEN spec §3.1. **Fix:** add the arms or document de-scope with rationale.

**M2 — `descriptor_from_tree` non-explicit branch hardcodes `PathDecl{n:1}` while `Descriptor.n` is derived.** Matches for current n=1 callers; latent trap. **Fix:** `PathDecl{n, …}`.

**M3 — `SECP_G_COMPRESSED` dead.** No shape generates a pubkeys TLV. **Fix:** add a TLV-bearing arm or delete the const.

**M4 — `missing_docs="warn"` interaction with `pub` helpers in test crates unproven (zero existing integration tests in md-codec).** Low-risk (doesn't fire on bin-style test crates; helpers carry doc comments). **Fix:** confirm `-D warnings` clean at Task 0.2 trial-compile, or allow-list.

**M5 — `prop_recursive` typing is SOUND; trial-compile framing adequate, no concrete fix.** Verified `prop_recursive<R:Strategy<Value=Self::Value>, F:Fn(BoxedStrategy<Self::Value>)->R>`: closure gets `BoxedStrategy<Node>`, `(inner.clone(),inner).prop_map(|(l,r)|Node{…})` is `Strategy<Value=Node>` matching the leaf base. `.boxed()` erases cleanly. Keep the guard; not a latent error.

## VERDICT: RED (2C / 3I / 5M)

Faithfully transcribes the two headline spec calls (canonical-fixpoint P1/P2; `!= Ok(original)` T2c); all APIs/error-variants/cross-chunk-branch ordering/oracle-entry verified against live source. RED driven by C1 (tr_taptree drops mandated renumber → P1/P2 ship broken) + C2 (unit-variant struct-pattern → won't compile). I1–I3 false-fail/latent-panic/boundary-fragility. Fold C1+C2, address I1–I3, sweep minors, re-dispatch R1.
