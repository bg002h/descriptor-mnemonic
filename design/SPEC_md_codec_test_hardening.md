# SPEC — md-codec test-hardening (themes 1/2/3 from the constellation survey)

**Status:** DRAFT — pre-R0
**Repo / branch:** `descriptor-mnemonic`, default branch **`main`**, crate `crates/md-codec`, edition 2024
**Source ground-truth SHA:** `ca4591b` (origin/main at authoring; line numbers grep-verified against it)
**Recon:** `mnemonic-toolkit/cycle-prep-recon-codec-test-hardening-themes-1-2-3.md` (recon cycle **2 of 3**, mk→md→ms)
**Design provenance:** constellation survey → cycle-prep strict-gate recon → brainstorm → **two opus architect consults** (one on the theme-1 `Descriptor`-strategy fork + the canonicalization finding; one adjudicating the theme-2 5–8-error assertion). Every load-bearing claim independently grep-verified.

---

## §1 Problem / context

md-codec encodes wallet **descriptors/policies** as `md1` (codex32-family BCH, chunked). It is the constellation's **largest grammar surface** (~4.8k test fns / ~170k LOC) but has **no property/fuzz testing** of the encode↔decode bijection, BCH correction tested only in the data part at one length, the per-chunk re-verify guard untested with an aliasing pattern, the cross-chunk reassembly-validation branches untested, and no indel reject-contract for the toolkit's `repair --md1 --max-indel` oracle. This is the **md slice** of the survey's three themes. **Test-only**, no version bump unless a guard test goes red (then md-codec fix-bump + toolkit git-dep pin refresh — see §6).

Mirrors the just-shipped **mk-codec** cycle, but md differs materially: (a) the bijection property is **canonical-fixpoint, not naive** (§2.1 — the headline); (b) md's guard is a **per-chunk polymod re-verify**, not mk's cross-chunk hash; (c) the indel oracle entry is `reassemble(&[&str])` (string API), unlike mk's `decode`.

---

## §2 Verified ground truth (SHA `ca4591b`)

- `Descriptor` (`encode.rs:17`) derives `PartialEq, Eq` (`:16`); all transitively-owned types (`Node`/`Body`/`Tag`/`PathDecl`/`TlvSection`/`UseSitePath`/`OriginPath`) derive structural `PartialEq, Eq` — no manual impls, no float, TLVs are order-sensitive `Vec<(u8,_)>` (canonicalize sorts them ascending, so equality is order-deterministic **post-canonicalize**). The fixpoint `==` is a true structural identity.
- Public API (`lib.rs`): `encode::{Descriptor, encode_payload, encode_md1_string}`, `decode::{decode_payload, decode_md1_string}`, `chunk::{reassemble, decode_with_correction, split, derive_chunk_set_id, ChunkHeader, CorrectionDetail}`, `canonicalize::canonicalize_placeholder_indices`.
- **BCH capacity is t=4** for the regular code `BCH(93,80,8)`: `decode_regular_errors` rejects `deg == 0 || deg > 4` (`bch_decode.rs:416`). The `TooManyErrors { bound: 8 }` (`chunk.rs:529/543/568`) carries the **display value 8** (singleton bound 2t), NOT the correctable count. Checksum width = **13 symbols**, regular code only (long dropped) (`codex32.rs:18`, `bch_decode.rs:12`).
- Per-chunk **polymod re-verify guard** at `chunk.rs:559-570` (recompute residue after correction → `TooManyErrors` if `!= 0`). `decode_with_correction(&[&str])` (`chunk.rs:492`) is the ONLY public path that exercises per-chunk correction + the guard end-to-end. `reassemble` does a **hard** `bch_verify_regular` (no correction, `codex32.rs:114-119`) and `decode_md1_string` never corrects.
- Cross-chunk validation branches: `ChunkSetInconsistent` (`chunk.rs:348`), `ChunkSetIncomplete` (`:351`), `ChunkIndexGap` (`:362`), `ChunkSetIdMismatch` (`:382`). Derived csid = **top 20 bits** of the encoding-id hash (`chunk.rs:175-179`).

### §2.1 THE canonicalization finding — `decode(encode(d)) == d` is FALSE for non-canonical `d`

`encode_payload` (`encode.rs:65-67`) clones and canonicalizes internally before emitting:
```
let mut d_canonical = d.clone();
canonicalize_placeholder_indices(&mut d_canonical)?;   // permutes tree indices, Divergent paths, all 4 TLV maps
```
The decoder does NOT re-canonicalize; it **rejects** non-canonical wires with `PlaceholderFirstOccurrenceOutOfOrder` (`validate.rs:30`, from `decode.rs:56`). So a hand-built `Descriptor` whose first-occurrence isn't ascending encodes to the *canonical* wire and decodes to a *different* (canonical) descriptor. `canonicalize_placeholder_indices` (`canonicalize.rs:168`) is the **sole** normalizer (every other step in `encode_payload` is a pure validator that rejects, not rewrites). Existing fixpoint pattern to lift: `round_trip_canonicalize_encode_decode_canonicalize` (`canonicalize.rs:954`).

**Consequence:** the bijection property MUST be canonical-fixpoint (§3 P1), and a second property over deliberately-non-canonical inputs (§3 P2) is the highest-value latent-bug catcher (the `remap_indices` + Divergent inverse-permutation + four TLV `remap_tlv_vec` calls that must permute in lockstep — the direct analog of toolkit bug F4).

---

## §3 Theme 1 — property/fuzz harness (`tests/proptest_roundtrip.rs` + `tests/common/mod.rs`)

Add `proptest = "1"` to `crates/md-codec/Cargo.toml` `[dev-dependencies]` (per-crate; mirror mk-codec's just-shipped choice). Pin `proptest!` `cases`.

### §3.1 The `Descriptor` strategy — option (c) (architect-adjudicated)
A shared `tests/common/mod.rs` exposes `descriptor_strategy()`:
- **Templated common shapes** (`prop_oneof!`), each parameterized: single-sig `wpkh(@0)`/`pkh(@0)`/`tr(@0)` (n=1, kiw=0 edge); `sh(wpkh(@0))`; `wsh(multi(k,@0..))` / `wsh(sortedmulti(...))` (n∈1..=8, k∈1..=n); `sh(wsh(sortedmulti(...)))`; `sh(sortedmulti(...))` (legacy P2SH — `canonical_origin==None`, forces the explicit-origin path); `tr(@0, multi_a(k,..))` / `tr(@0, sortedmulti_a(..))`; `tr(@0, <taptree>)`; `tr(<NUMS>, <taptree>)`.
- **Bounded-recursion `tr()` taptree sub-strategy** (`prop_recursive`, depth ≤ 3, ≤ 4 leaves): internal nodes `TapTree{Children(2)}`; leaves drawn ONLY from the permitted allow-list (`pk_k`/`pk_h`/`multi_a`/`sortedmulti_a` + `and_v`/`or_d`/`older`/`after`/`sha256` wrappers) so NO `prop_filter` churn and no forbidden-leaf generation (`validate.rs:164 is_forbidden_leaf_tag`).
- **Varied params:** `n` biased to the kiw-width boundaries {1,2,3,4,5,8,9} (`encode.rs:37-41` ⟷ `decode.rs:26` duplicated `⌈log₂n⌉`); `k`; **non-canonical index orderings** (emit `MultiKeys` indices like `[2,0,1]` so canonicalization is exercised); `path_decl` ∈ {elided-when-canonical, Shared, Divergent(len==n)}; `use_site_path` ∈ {standard_multipath, 2..=9-alt multipath} (`use_site_path.rs:43-45`); TLV maps ∈ {none, fingerprints, valid-pubkey wallet-policy, origin overrides}.
- **Pubkeys derived from a fixed valid compressed key** (e.g. secp G), NOT random `[u8;65]` (`validate_xpub_bytes` rejects ~50% otherwise, `validate.rs:221`). Indices constructed to cover `0..n` by design (sidesteps the global `PlaceholderNotReferenced` constraint without a repair pass; the canonicalization in P1 absorbs the permutation).

**Architect explicitly ruled out the full recursive `Arbitrary<Node>` (option b):** ~400-600 LOC re-implementing `read_node` + `validate.rs` + `canonicalize.rs` + the `canonical_origin` complement; high tautology risk ("fuzzer and SUT share a bug"). Option (c) ≈ 200 LOC, low-medium risk.

### §3.2 Properties
- **P1 — canonical-fixpoint bijection:** `let c = canonicalize(d.clone())?; decode_payload(encode_payload(&c)?…)? == c` AND `encode_payload(d) == encode_payload(c)` byte-equal (pins the encoder's internal canonicalization against the explicit one).
- **P2 — canonicalize-is-normalizer (F4-class catcher):** over deliberately-scrambled descriptors (non-canonical indices + populated `Divergent` paths + multiple populated TLV maps simultaneously), assert `encode_payload(d) == encode_payload(canonicalize(d))` byte-equal AND `decode_payload(encode_payload(d)) == canonicalize(d)`. This hits the #1 bug surface (`canonicalize.rs` permutation lockstep), currently covered only for hand-picked n≤3.
- **P3 — decode panic-freedom:** arbitrary `&[u8]` → `decode_payload`; arbitrary `&str` → `decode_md1_string`, `chunk::reassemble`; never panic (return `Ok`/`Err`). Folds in the unencodable-input contract: `Some(vec![])` TLVs / empty TLV entries return a typed `Err` (`EmptyTlvEntry`, `error.rs:144`), never a panic.
- **P4 — string-level round-trip (architect completeness add):** `decode_md1_string(encode_md1_string(canonicalize(d))?)? == canonicalize(d)`. This is a DISTINCT surface from P1: the full codex32 path adds ≤4 trailing-zero-bit symbol-alignment padding (`codex32.rs:86-91`) that `decode_md1_string` absorbs via TLV-rollback (`decode.rs:74-82`) — a padding/rollback regression is invisible to the payload-level P1.
- **P5 — chunk round-trip:** `reassemble(split(&canonicalize(d))?…)? == canonicalize(d)` (the chunked surface; `chunking.rs` only spot-checks 3 fixed descriptors).

### §3.3 Notes (architect)
- md does NOT collapse `Divergent([p,p,p])` to `Shared(p)` (that's the toolkit's job, F4) — P2 must NOT expect collapse; `Divergent` round-trips as `Divergent` (`encode.rs:82`).
- `proptest-regressions/` → add `**/proptest-regressions/` to `descriptor-mnemonic/.gitignore` (the mk lesson; nested per-test-file path).

---

## §4 Theme 2 — BCH adversarial (`tests/bch_adversarial.rs`)

Drive via **`decode_with_correction(&[&str])`** (the only public path hitting per-chunk correction + the re-verify guard). t=4; cap deterministic correction at 4 errors; `bound:8` is a display constant (pin `==8`, annotate "8 = singleton bound 2t, NOT the correctable count").

| ID | Cell | Assert |
|---|---|---|
| T2a | 1–4-error correction across **3 descriptor lengths** (`small`/`deep`/near-cap data-parts) | `Ok(original)` + `details.len()==count` |
| T2b | corrupt 1–4 symbols **inside the trailing 13-symbol checksum region** (current corpus only hits the data part) | `Ok(original)` (BCH corrects checksum-symbol errors identically) |
| **T2c** | **randomized 5–8-error sweep**, one chunk, ~300 seeds × 3 lengths (seeded in-test xorshift, no `rand` dep) | **`!= Ok(original)`** — see §4.1 |
| T2d | one **deterministic hand-picked** 5-error pattern (mirror existing `five_error_too_many`) | `is_err()` = `TooManyErrors { bound: 8 }` |
| T2e | `reassemble` count/header mismatch (re-stamped or spliced chunks) | `Err(ChunkSetInconsistent)` (`chunk.rs:348`) |
| T2f | `reassemble` duplicate/gapped index | `Err(ChunkIndexGap)` (`chunk.rs:362`) |
| **T2g** | `reassemble` headers agree on csid but reassembled payload derives a different csid (re-stamp every header with a foreign csid) | `Err(ChunkSetIdMismatch{..})` (`chunk.rs:382`) — deepest/highest-value branch |
| T2h | multi-chunk: 2 different chunks each ≤4 errors | `Ok(original)`, `details` aggregates across chunks |
| T2i | multi-chunk: one chunk >t in a valid 3-chunk set | `Err(TooManyErrors{chunk_index:..})` — **atomic abort, no partial output** (plan §1 D28, `chunk.rs:11-13`) |

Helper: `restamp_chunk_header(chunk, mutate) -> String` (parse → mutate `ChunkHeader` → re-wrap) for T2e/T2f/T2g.

### §4.1 The T2c assertion — `!= Ok(original)`, NOT `is_err()` (TWO-architect-adjudicated)
A randomized 5–8-error pattern can be **miscorrected** to a *different valid codeword* C′ (received±4 landing in C′'s radius-4 decode sphere; reachable since `d(C,C′)` can be 9..12 ≥ the min distance 9). `decode_regular_errors` returning `Some` ALWAYS yields a valid codeword (no syndrome re-check beyond `deg≤4`), so the re-verify guard (`chunk.rs:561`) confirms "*a* codeword," **never "*the original*"** — a true miscorrection passes it and returns `Ok(C′≠C)` with a ~2⁻²⁶ per-trial residual (single-chunk; multi-chunk adds the 20-bit csid wall → ~2⁻⁴⁶, but a probabilistic margin must not be encoded as a hard `is_err()`). Therefore `is_err()` is **flaky** over a randomized sweep; `!= Ok(original)` is the robust invariant (holds unconditionally — a ≥5-error corruption never silently returns the original). This is the same call the mk cycle made for the same code geometry. The deterministic hand-picked T2d pattern keeps `is_err()` (verified-once is non-flaky). The "5–8 errors can't reach another codeword" argument is the **perfect-code fallacy** applied to a non-perfect (d=9) code — rejected.

---

## §5 Theme 3 — indel reject-contract (`tests/indel_reject_contract.rs`)

Entry: **`md_codec::chunk::reassemble(&[&str])`** (md takes `&str`, unlike mk's `decode`). The toolkit's `Md1IndelOracle` (`mnemonic-toolkit/crates/mnemonic-toolkit/src/repair.rs:1028`, `:1043` calls `reassemble`, doc `:962`/`:1024` "does NOT self-correct") relies on `reassemble` failing closed on a length-changed string.

`reassemble` does a **hard `bch_verify_regular`** (no correction, `codex32.rs:114-119`), so an insert/delete almost always fails the verify → `Err`. Assertion strength = **`is_err()`** (safe — no correction path, so no miscorrection; double-walled by the cross-chunk csid for the rare BCH-passing case).

| ID | Cell | Assert |
|---|---|---|
| T3a | insert one alphabet char mid-data-part (sweep positions × sampled chars) | `is_err()` |
| T3b | delete one symbol mid-data-part (sweep positions) | `is_err()` |
| T3c | delete below the 13-symbol checksum length (deterministic out-of-band) | `Err(Error::Codex32DecodeError(s)) where s.contains("too short")` (`codex32.rs:122`) — md's analog of mk's `InvalidStringLength` |
| T3d | multi-chunk: indel in one chunk; also assert `!= Ok(original)` as a regression tripwire if `unwrap_string` ever softened to self-correct | `is_err()` (+ `!= Ok(original)` belt) |

---

## §6 SemVer / branch / lockstep
- **Branch:** `main` (md-codec's default).
- **SemVer:** test-only ⇒ **no version bump** (`proptest` is a dev-dep). Commit to `main`. **Iff** a guard test goes red and we fix inline → md-codec PATCH/MINOR fix-bump + its own R0 + refresh the `mnemonic-toolkit` git-dep pin to md-codec. (Decision locked at brainstorm: fix clear/contained bugs inline, defer big/ambiguous ones via `#[ignore]` + FOLLOWUP; surface either way.)
- **Lockstep:** none — crate-internal tests, no CLI/manual/GUI surface change.

---

## §7 Test inventory
Theme 1: `tests/common/mod.rs` (`descriptor_strategy` + helpers) + `tests/proptest_roundtrip.rs` (P1–P5). Theme 2: `tests/bch_adversarial.rs` (T2a–T2i). Theme 3: `tests/indel_reject_contract.rs` (T3a–T3d). Gate: `cargo test -p md-codec` green; `cargo +stable clippy -p md-codec --all-targets -- -D warnings` clean; `cargo +stable fmt --check --all` clean (CI parity — edition 2024; local nightly rustfmt ≠ CI stable, the mk lesson).

---

## §8 Phased plan
- **Phase 0** — `proptest` dev-dep + `.gitignore`; `tests/common/mod.rs` strategy + `proptest_roundtrip.rs` P1–P5. **Trial-compile the `prop_recursive` taptree typing + the Form-A property's borrow/clone shape FIRST** (architect caveat — the recursion-strategy generics are the one proptest-API-friction risk). P1/P2 may surface a real canonicalization bug → SPEC §6.
- **Phase 1** — Theme 2 (T2a–T2i). T2c/T2g are the likely bug-finders.
- **Phase 2** — Theme 3 (T3a–T3d). File any deferred FOLLOWUPs.
- **Phase 3** — full verify + end-of-cycle opus R0 + ship to `main`.

---

## §9 R0 agenda (what the architect must stress)
1. The §2.1 canonicalization finding + the P1/P2 framing (is the fixpoint assert sound; does P2 actually reach the permutation-lockstep bug surface?).
2. **The §4.1 T2c assertion** — re-confirm `!= Ok(original)` (not `is_err()`) is correct against the decoder + BCH theory (two architects already adjudicated B; R0 should sanity-check the transcription).
3. The §3.1 strategy generates only encodable+canonicalizable descriptors (no spurious encode-error in P1); valid pubkeys; the kiw-boundary `n` distribution.
4. P4/P5 (string + chunk round-trips) correctly target distinct surfaces.
5. The cross-chunk re-stamp helper (T2e–T2g) is constructible from the public `split`/`ChunkHeader` surface.
6. Nothing mis-scoped; the `tr()` taptree bounded recursion is the right ambition (not full `Arbitrary<Node>`).

---

## §10 Source citations (verified at `ca4591b`)
`encode.rs:16-17,37-41,65-92` (PartialEq, kiw, internal canonicalize); `decode.rs:26,56,74-82` (kiw dup, placeholder validate, rollback); `canonicalize.rs:168,954` (sole normalizer, fixpoint test); `validate.rs:30,164,221` (non-canonical reject, forbidden-leaf, xpub bytes); `bch_decode.rs:12,416` (13-symbol checksum, deg>4); `chunk.rs:175-179,305,348/351/362/382,492,524-570` (csid width, reassemble, cross-chunk branches, decode_with_correction, correction loop + re-verify guard); `codex32.rs:18,86-91,114-126` (checksum width, padding, hard verify, too-short); `use_site_path.rs:43-45`; `tree.rs` (Node/Body/Tag + tr taptree wire). Consumer: `mnemonic-toolkit/crates/mnemonic-toolkit/src/repair.rs:962,1028,1043`.
