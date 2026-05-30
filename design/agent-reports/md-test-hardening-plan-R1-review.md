# R1 ARCHITECT REVIEW — `IMPLEMENTATION_PLAN_md_test_hardening.md`

Opus feature-dev:code-reviewer. R1 of the mandatory pre-impl gate. R0 was RED (2C/3I/5M); the controller folded all findings. This pass confirms each fold resolved its finding AND introduced no new drift, then re-verifies against LIVE SOURCE on branch `md-codec-test-hardening`. Every API/field/variant/arity/validation-rule the folds depend on was grep-verified against the actual files.

**Tooling note:** No Write/Edit tool available to this review agent; returned to the controller and persisted here verbatim.

## Headline confirmations (against source, file:line)

- **C1 RESOLVED.** `descriptor_from_tree` now collects `referenced_indices` → builds `perm: old→rank` (BTreeSet sorted-rank) → calls `renumber_tree` → derives `n=set.len()` → drops `set.insert(0)`.
  - **(a) Compiles + clippy-clean.** `*index = perm[&*index]` where `index:&mut u8` uses `BTreeMap`'s `Index<&u8>` (via `K:Borrow<Q>`); `&*index:&u8` is a reborrow REQUIRED by the index signature (removing it → type error), so `clippy::needless_borrow` does NOT fire. Immutable index-borrow yields `u8` (Copy), ends before the assign. `MultiKeys`/`Tr`/`Children`/`Variable` arms mirror live `remap_indices` (canonicalize.rs:102-139). `if !*is_nums` deref'd correctly. Clean under `-D warnings`.
  - **(b) Every arm emits contiguous `0..n` post-renumber.** single_sig/sh_wpkh {0}; multisig/sh_wsh/sh_sortedmulti `(0..n)`; tr_multi_a {0}∪{1..n-1}; tr_taptree {0}∪arbitrary→renumbered. Sorted-rank perm is intentional — `canon()` re-canonicalizes to document-order; the renumber only needs the set to equal `0..n` to kill `PlaceholderIndexOutOfRange` (canonicalize.rs:174→255) and `PlaceholderNotReferenced` (canonicalize.rs:184).
  - **(c) No empty-set→n=0 path.** `referenced_indices` Tr-arm inserts `key_index` when `!is_nums`. Every shape has `is_nums:false` keypath@0, so `set ⊇ {0}`, `n≥1`. tr_taptree all-`Older` leaves still has keypath@0. No `KeyCountOutOfRange`.

- **C2 RESOLVED.** `ChunkSetInconsistent` genuine UNIT variant (error.rs:257-259). T2e `Err(Error::ChunkSetInconsistent)` no braces → compiles. Count-mismatch routing CONFIRMED: restamp `cs[0]` count+1 → `expected_count=h0.count` (mutated), loop compares other chunks (unchanged) → `ChunkSetInconsistent` (chunk.rs:343-350), NOT `ChunkSetIncomplete`.

- **I1 RESOLVED.** `UNCORRECTABLE_5ERR` const + build-verify doc. T2i `if let Ok((got,_)) = … { assert_ne!(got, d) }` — compiles WITHOUT `Error: PartialEq`; needs only `Descriptor: PartialEq + Debug` (encode.rs:16 `derive(Debug,Clone,PartialEq,Eq)`). T2d note covers miscorrection: re-verify guard (chunk.rs:561-570) checks "a codeword" not "the original"; note instructs pattern-swap if it miscorrects.

- **I2 RESOLVED.** `assert!(idx < chars.len(), …)`, `idx=3+pos`. HRP="md" (codex32.rs:15) → "md1"=3 chars → data part. T2b/T2c/T2d positions stay in range. Fires loudly on over-run.

- **I3 RESOLVED.** 6 Divergent cosigners × 15 components, wsh(sortedmulti k=2,n=6,indices 0..6). Re-estimated: values 1..515 → per-path ≈116..229 bits; Σ6 ≈1216 payload bits → 152 bytes → ceil(1216/320)=**4 chunks** (`SINGLE_STRING_PAYLOAD_BIT_LIMIT=320=64*5`, chunk.rs:219; div_ceil chunk.rs:248-249). ≥3 (T2f), ≥2 (T2h). n=6/indices(0..6)/k=2 valid SortedMulti; divergent_path(6,3) yields exactly 6.

- **M1 RESOLVED + VERIFIED SAFE (highest-risk fold).** `sh_sortedmulti` = `wrap(Tag::Sh, multikeys(SortedMulti,…))`, explicit_origin=true. md ACCEPTS bare `sh(sortedmulti)`: (1) encode validation (encode.rs:69-77) = placeholder/multipath/taptree only, NO structural gate on Sh's child; (2) `canonical_origin`→`None` catch-all (canonical_origin.rs:65-75); (3) decode TopLevel allow-list admits `Sh` (decode.rs:36-44); (4) `validate_explicit_origin_required` (validate.rs:182-207) passes with n non-empty depth-3 paths (mirrors unit test validate.rs:604). Does NOT make P1 panic — NOT a new Critical. `tr(NUMS,taptree)` de-scope documented (empty-set→n=0 edge; existing unit tests cover NUMS-skip).

- **M2/M3 RESOLVED.** `PathDecl{n,…}` n-derived matches `Descriptor.n`. `SECP_G_COMPRESSED` fully deleted (zero repo refs); Tech Stack states "No bitcoin/pubkey dep."

## New-drift scan
- 7-arm `prop_oneof!` compiles; all arms `.prop_map(…)→Strategy<Value=Descriptor>`, `.boxed()`. Uniform.
- `prop::sample::select(vec![Tag::Multi,Tag::SortedMulti])` — `Tag` derives Clone (tag.rs:14). select satisfied. `prop::sample::select` in scope via prelude.
- Test-filter preserved: T2i `t2i_one_chunk_over_t_never_returns_original` substring-matches `t2i`.
- `use md_codec::error::Error;` still USED by T2e/f/g (Task 1.3). Not dead.
- T2f routing intact: `cs[1].index=0` passes `index<count`; sort→gap (chunk.rs:360-367) finds `0!=1` → `ChunkIndexGap`.
- `assert!` mixes inline `{pos}` + positional `{}` for `chars.len()` (method call) — no `uninlined_format_args`.

## CRITICAL — None.
## IMPORTANT — None.

## MINOR
**M1-R1 (carry-forward, non-blocking)** — `restamp_chunk_header` remains an `unimplemented!` Step-1a trial-build, correctly flagged with a mandatory identity-round-trip proof; public surface sufficient (`codex32::{unwrap_string,wrap_payload}`, `bitstream::{BitReader,BitWriter}`, `ChunkHeader::{read,write}` all pub; `wrap_payload` recomputes BCH). Execution-time item, not a plan defect.

**M2-R1 (advisory)** — no TLV-bearing arm ⇒ `validate_xpub_bytes` (validate.rs:216) + the canonicalize TLV-remap lockstep (canonicalize.rs:221-232, the F4-class #1 surface per SPEC §2.1) get no proptest coverage. Deliberate tradeoff for deleting `SECP_G_COMPRESSED` / avoiding a bitcoin dev-dep; the flat Divergent-path permutation IS exercised. Worth a one-line FOLLOWUP ("md proptest TLV-remap coverage gap"); does NOT block this test-only ship.

## VERDICT: GREEN (0C / 0I / 2M)

All R0 findings correctly resolved with no new drift. The two MINORs are an execution-time trial-build (already flagged) and an advisory TLV-coverage gap (deliberate scope tradeoff). GREEN — cleared for implementation.
