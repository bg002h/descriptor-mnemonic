# BRAINSTORM — md-codec proptest fragment/domain/nesting expansion (stress Cycle B)

Status: R4 **GREEN (0C/0I)** — cleared for implementation. Source SHA:
`69b7a74` (origin/main, 2026-06-11). Reviews (persisted verbatim):
design/agent-reports/cycle-b-proptest-expansion-r0-round1-review.md (RED:
3C/5I — folded; markers `[C1]`…`[M4]`, `[Q5]`/`[Q6]`),
…-round2-review.md (RED: 1C/2I — folded; markers `[C1′]`, `[I1′]`, `[I2′]`,
`[M1′]`–`[M3′]`), …-round3-review.md (YELLOW: 0C/1I — folded; marker
`[I1″]` + Bdu-resolution clause), and …-round4-review.md (GREEN: 0C/0I;
non-blocking pk_h-string-fixture minor folded).
Program context: Cycle B of the 6-cycle stress-testing program run from the
mnemonic-toolkit repo (Cycle A = toolkit backup→restore proptest, shipped
toolkit @ 9d3da6c). This cycle lives HERE, in the wire-owning repo.

## Problem statement

`crates/md-codec/tests/common/mod.rs::descriptor_strategy` (lines 198–280)
generates only: single-sig (wpkh/pkh/tr-keypath), sh(wpkh), wsh(multi |
sortedmulti), sh(wsh(sortedmulti)), sh(sortedmulti), tr+multi_a, and tr
taptrees whose leaves are PkK / PkH / MultiA(1) / `Older(1..=65535)`
(common/mod.rs:73–81). Consequences:

1. **Six leaf fragments are never generated**: `After`, `Sha256`, `Hash256`,
   `Ripemd160`, `Hash160` never appear at all; `Older` appears only inside tr
   taptrees and only in `1..=65535` (the full wire domain is `u32` —
   tree.rs:142–144 writes all 32 bits raw, tree.rs:275–277 reads them back;
   no decode-side range validation exists).
2. **No miniscript nesting is generated**: the combinators (`AndV`, `AndB`,
   `AndOr`, `OrB`, `OrC`, `OrD`, `OrI`, `Thresh`) and wrappers (`Check`,
   `Verify`, `Swap`, `Alt`, `DupIf`, `NonZero`, `ZeroNotEqual`) have wire
   round-trip coverage only via hand-written unit cells in tree.rs, never
   property coverage, and never inside `wsh(...)`/`sh(...)` at the Descriptor
   level (the current strategy puts NO general miniscript under wsh/sh).
3. **The proptest never exercises `to_miniscript`**: proptest_roundtrip.rs
   P1–P5 stop at AST equality. The v0.32 generic converter
   (src/to_miniscript.rs) — the thing address derivation and the toolkit's
   restore path ultimately depend on — has zero property coverage. The
   restore campaign (toolkit v0.54.0–v0.54.2) and the Cycle-A find
   (`bundle-accepts-sortedmulti-in-combinator-restore-cannot`) both show this
   layer is where real funds-relevant bugs live.

Goal (from program charter): broaden to all 6 less-common fragments × full
value domains × nesting, through the encode→codex32→decode→to_miniscript
round-trip.

## Layer constraints (recon, grep-verified at 69b7a74; round-1 R0 re-verified
every citation empirically, incl. scratch experiments against the PINNED
miniscript)

**Pinned versions (Cargo.lock):** miniscript **13.0.0** (crates.io), bitcoin
0.32.8, proptest 1.11.0. All miniscript-behavior claims below were verified
against 13.0.0 by the round-1 reviewer, not assumed.

**Decode-side validation** (decode.rs:56–69) is structural only:
- Root tag ∈ {Sh, Wsh, Wpkh, Pkh, Tr} (`matches!` at decode.rs:36–44). [M1]
- Placeholder usage: every `@i < n` referenced; first occurrences in
  pre-order must be ascending (validate.rs:17–37). The existing
  `canon()` helper (common/mod.rs:283–287 →
  `canonicalize_placeholder_indices`) normalizes generated trees.
- Taptree leaves: forbidden tags = {Wpkh, Tr, Wsh, Sh, Pkh, Multi,
  SortedMulti} (`is_forbidden_leaf_tag`, validate.rs:164–169). [M1]
  Everything else is a permitted leaf — including timelocks, hashlocks,
  combinators, wrappers, MultiA, SortedMultiA, RawPkH, True/False. Note
  `walk_tap_tree_leaves` does NOT recurse into non-TapTree leaves, so
  Multi/SortedMulti nested inside a combinator leaf also pass decode.
- Explicit origin required for non-canonical wrappers (validate.rs:182+);
  xpub point validity when TLV pubkeys present (validate.rs:216+).
- `k ≤ n` for Multi-family and Thresh is enforced at DECODE
  (tree.rs:229–231, 241–243) but NOT at encode (tree.rs:90–121 checks only
  1..=32 ranges) → k > n encodes but cannot decode. The generators keep
  k ≤ n; separately, P8 pins this encoder-side gap loudly (see below). [Q5]
- `n` must be 1..=32: `PathDecl::write` rejects n = 0 and n > 32
  (origin_path.rs:105–112, `KeyCountOutOfRange`). A keyless tree (all
  True/False/timelock/hashlock leaves) derives n = 0 and ENCODE-fails —
  every strategy must guarantee ≥ 1 key-bearing node by construction. [C3]
- Recursion depth < 128 (tree.rs:167, MAX_DECODE_DEPTH).
- Payload size: `split()` hard-errors above 64 chunks = 20,480 payload bits
  (chunk.rs:250–251). Measured: n=32 + 32 pubkeys + 32 fingerprints + 32
  divergent 3-deep paths + minimal tree = 19,855 bits = 63 chunks — ~600
  bits of headroom. Strategies carry an explicit payload budget (below). [I3]
- **No timelock-range, hash-value, or miniscript-type validation.** Full
  `u32` / `[u8;32]` / `[u8;20]` domains MUST round-trip at the wire layer.

**to_miniscript layer** (src/to_miniscript.rs) additionally requires:
- TLV pubkeys present and on-curve (`expand_per_at_n` →
  `Error::MissingPubkey` otherwise; to_miniscript.rs:72). Current strategy
  emits `TlvSection::new_empty()` (common/mod.rs:194) → every current
  generated descriptor would fail this leg. The typed tier must attach real
  xpubs (pattern: tests/address_derivation.rs:30–43 `account_xpub_bytes`
  + :81–82 `t.pubkeys = Some(vec![(0, xpub_bytes)])`; `bitcoin::bip32` via
  the regular `bitcoin` dep — no new dev-deps). [M3]
- Type-valid miniscript: `Miniscript::from_ast` (to_miniscript.rs:478) runs
  `Type::type_check` + `check_global_validity` ONLY — it does NOT reject
  sigless/malleable ("non-sane") expressions. Context rules enforced there:
  Multi rejected in Tap, MultiA rejected in Segwitv0/Legacy. [C1 evidence]
- **Tr-only reparse sanity (the round-1 make-or-break find):** pinned
  miniscript's `Descriptor::from_str` (descriptor/mod.rs:1047–1062) carries
  a Tr-ONLY compat branch running `sanity_check()` + per-leaf
  `ext_check(ExtParams::sane())`. So a tr() descriptor with any non-sane
  leaf (sigless spend path, repeated pubkeys, mixed height/time locks in a
  path) CONSTRUCTS and RENDERS fine but FAILS reparse. wsh/sh have no such
  branch (proven: `wsh(or_i(older(1),older(2)))` reparses Ok). The T-tier
  tap grammar must therefore be sane-by-construction (below). [C1]
- Per-context resource limits enforced by `from_ast`/threshold builders:
  Legacy `pk_cost ≤ 520` (multi ≤ 15 keys; general sh() miniscript must
  stay small); Segwitv0 multi ≤ 20 keys (`MAX_PUBKEYS_PER_MULTISIG`) and
  `pk_cost ≤ 3600`; Tap multi_a fine to 32 (limit 999). [I2]
- Consensus-valid locktimes, MEASURED domains (round-1 corrected the
  draft's wrong BIP-68-mask claim) [C2]:
  - `AbsLockTime::from_consensus` (after): valid **1..=0x7FFF_FFFF**;
    0 and ≥ 0x8000_0000 → clean Err at the constructor call
    (to_miniscript.rs:429–435), never a panic.
  - `RelLockTime::from_consensus` (older): valid iff **non-zero AND bit 31
    clear**. Out-of-BIP-68-mask values like 0x10000 and 0x00410000 are
    ACCEPTED by miniscript (its known leniency — the reason toolkit
    v0.53.9 added its own mask gate). P6 pins these as Ok loudly with a
    comment citing toolkit v0.53.9; P7's rel-bad set is {0} ∪ {bit-31 set}.
- Known unsupported shapes (LOUD refuse, all pre-filed):
  - `Tag::SortedMulti` as miniscript leaf (must be sole wsh/sh child) —
    to_miniscript.rs:417–422.
  - `Tag::SortedMultiA` anywhere — rust-miniscript v13 has no Terminal
    (to_miniscript.rs:423–428; FOLLOWUP
    `md-codec-sortedmulti-a-to-miniscript-rendering-gap`).
  - `Tag::RawPkH` — not constructible via public API (to_miniscript.rs:453–457).
  - `Tag::Check` over a NON-bare-key child double-wraps and errors
    (shape-C; to_miniscript.rs:304–323; FOLLOWUP A2). `Check` over bare
    PkK/PkH is accepted via the 0.35.1 idempotence arm.

## Proposed design

Test-only change in md-codec (new/extended strategies in
`tests/common/mod.rs`, new properties in `tests/proptest_roundtrip.rs` plus
a new `tests/proptest_to_miniscript.rs`). NO-BUMP (like Cycle A): no library
code changes, no new dependencies (`proptest` already a dev-dep; `bitcoin`,
`miniscript` are regular deps importable from integration tests). [M3]

Both new strategies REUSE `descriptor_from_tree`'s collect-referenced-
indices → renumber-to-contiguous-0..n logic (common/mod.rs:166–196), which
is what makes the existing `canon()` `.expect` safe; any new root-builder
goes through it. [M4]

### Tier 1 — wire-domain strategy (W): full domains, arbitrary nesting

New `wire_descriptor_strategy()` generating decode-valid (per the structural
rules above) but NOT necessarily type-valid trees:

- Leaf pool, split into KEY-BEARING (PkK/PkH(key), Multi/SortedMulti/
  MultiA/SortedMultiA with k ≤ n) and KEYLESS (`After`/`Older` over the
  FULL u32 domain biased to boundaries {0, 1, 0xFFFF, 0x10000, 0x0040FFFF,
  0x00410000, 499_999_999, 500_000_000, 0x7FFFFFFF, 0x80000000, u32::MAX}
  ∪ any::<u32>(); `Sha256`/`Hash256` × any [u8;32]; `Ripemd160`/`Hash160`/
  `RawPkH` × any [u8;20]; `True`; `False`).
- Recursive combinator layer (prop_recursive): unary wrappers {Check,
  Verify, Swap, Alt, DupIf, NonZero, ZeroNotEqual} × arity-2 {AndV, AndB,
  OrB, OrC, OrD, OrI} × AndOr(3) × Thresh(k ≤ children ≤ 4 in recursive
  position).
- **≥ 1 key guarantee [C3]:** every generated descriptor tree conjoins at
  least one key-bearing node by construction — the root builder pairs the
  recursive subtree with a key-bearing leaf (e.g. via a top-level arity-2
  combinator) OR the recursive generator forces one designated branch to
  bottom out in the key-bearing pool. For tr roots, the guarantee is
  carried by the TAPTREE: the designated key-bearing branch is a taptree
  leaf drawn from the key-bearing pool, or the internal key is a non-NUMS
  `@0` (a NUMS-internal-key tr with an all-keyless taptree would derive
  n = 0 and encode-fail). [M1′]
- **Payload budget [I3][I1′]:** recursion depth ≤ 4, fan-out ≤ 4, and
  ≤ 24 total nodes per tree (hash leaves are 262 bits ⇒ tree worst case
  ≈ 6.3k bits). Node count does NOT bound n (a single `Body::MultiKeys`
  leaf carries up to 32 indices, and pubkey TLVs cost ≈ 528 bits/key —
  round-2 proved a within-caps overflow at 65 chunks), so the budget is
  enforced on the ACTUAL ENCODING: the strategy's final map runs
  `encode_payload(&canon(d))` and asserts `total_bits ≤ 18,000` (margin
  under the 20,480 cliff), failing loudly on drift. Belt-and-braces: the
  W tier caps n ≤ 8 whenever pubkeys/fingerprints TLVs are attached.
- Roots: `wsh(W)`, `sh(W)`, `sh(wsh(W))`, `tr(@0 | NUMS,
  taptree-of-W-leaves)` (taptree leaves filtered against the forbidden-tag
  list). SortedMulti-under-combinator-inside-wsh IS generated — decode
  permits it (no validator inspects non-sole-child SortedMulti), it is
  wire-round-trippable, and it is exactly the Cycle-A engrave-but-can't-
  restore shape: W-tier keeps it, P7 classifies it as a clean
  to_miniscript refusal.
- **TLV randomization [Q6]:** the W tier randomizes Shared vs Divergent
  path-decls and occasionally attaches `origin_path_overrides`,
  `fingerprints`, and `pubkeys` TLVs — this exercises
  `canonicalize_placeholder_indices`' TLV-permutation arm
  (canonicalize.rs:221–232), currently property-uncovered. Constraints:
  origin paths non-empty (satisfies `validate_explicit_origin_required`),
  `use_site_path_overrides` keep a uniform alt-count
  (validate.rs:117–138), attached pubkeys are valid curve points (reuse
  the T-tier key material), and TLV bits count against the payload budget.
- Properties: run the existing P1 (canonical fixpoint), P2 (normalizer),
  P4 (string round-trip), P5 (chunk round-trip) over the W strategy.
  Hash-literal leaves are 166–262 bits each so nested W trees materially
  exercise multi-chunk split/reassemble for the first time under property
  coverage.

### Tier 2 — typed strategy (T) + to_miniscript round-trip leg

New `typed_descriptor_strategy()` producing type-correct-by-construction
miniscript trees WITH TLV xpubs attached:

- Key material: derive **32** account xpubs from the standard abandon
  mnemonic once (`OnceLock`), reusing the address_derivation.rs pattern;
  attach `tlv.pubkeys` for all `@i`. [I5]
- **Per-context caps [I2][I3]:** T-tier n ≤ 16 when attaching full TLV
  (payload budget); Legacy (`sh(miniscript)`) multi ≤ 15 keys and nesting
  depth ≤ 2 with ≤ 6 keys total (pk_cost 520); Segwitv0 (`wsh`) multi
  ≤ 20 keys, depth ≤ 4 (pk_cost 3600); Tap multi_a ≤ 16 (n-budget, not a
  miniscript limit).
- Typed grammar (B-type root per context), productions verified against
  pinned miniscript 13.0.0 in round 1:
  - B: `pk(@i)` (bare PkK; renders c:pk_k), `pk_h(@i)` (bare PkH; renders
    c:pk_h = string `pkh(...)` — when hand-writing STRING fixtures use
    `pkh`, never the K-typed literal `pk_h`), `multi(k,…)`
    (Segwitv0/Legacy), `multi_a(k,…)` (Tap), `sha256/hash256/ripemd160/
    hash160(h)`, `after(v)`/`older(v)` (valid domains per above,
    boundary-biased), `and_v(v:B, B)`, `and_b(B, W)`, `or_i(B, B)`,
    `or_d(Bdu, B)`, `andor(Bdu, B, B)`, `thresh(k, Bdu, W…)`.
  - Bdu (dissatisfiable+unit subset), CONTEXT-SPLIT [C1′]:
    - Segwitv0/Legacy Bdu: `pk(@i)`, `pk_h(@i)`, `multi`, hashlocks, and
      recursively **`or_d(Bdu, Bdu)`** (right arm must also be du —
      round-1 fix [I1]).
    - Tap Bdu: **keys only** — `pk(@i)`, `pk_h(@i)`, `multi_a`,
      recursively `or_d(Bdu_tap, Bdu_tap)`. NO hashlocks (a hashlock in a
      dissatisfiable position creates a sigless or malleable spend path —
      round 2 proved `tr(X,or_d(sha256(h),pk(Y)))` fails reparse with
      SiglessBranch and `tr(X,andor(sha256(h),pk(Y),pk(Z)))` with
      Malleable).
    - Timelocks are NOT Bdu in any context (not unit / not
      dissatisfiable-safe).
  - W: **`s:pk(@i)` only** for swap (`s:` requires `Bo` — exactly one
    input — and `pk` is the only single-input Bdu leaf; round 3 proved
    `s:pkh`/`s:multi`/`s:multi_a` all fail typecheck with "SWAP … does
    not take exactly one input"), and `a:<any single-node Bdu leaf>`
    (`a:pkh`/`a:multi`/`a:multi_a` all proven Ok). [I1″]
  - `Bdu` in the B-productions above denotes the ACTIVE CONTEXT's pool
    (Segwitv0/Legacy vs Tap, next bullet). [round-3 minor]
- Contexts: `wsh(T_segwit)`, `sh(T_legacy)`, `tr(NUMS-or-@0,
  taptree(T_tap…))` with context-restricted pools (multi vs multi_a).
- **Tap sanity-by-construction [C1][C1′]:** because reparse runs the
  Tr-only sanity branch, the INVARIANT is: **every T-tier tap leaf must
  pass `sanity_check` — signature-bearing AND non-malleable** (round 2:
  sig-bearing alone is insufficient; `andor(sha256,pk,pk)` is all-paths-
  sig-bearing yet Malleable). Concretely:
  (a) the tap-leaf B-pool drops bare timelock/hashlock leaves and sigless
  `or_i`/`thresh` arms; hashlocks and timelocks appear in tap leaves ONLY
  under `v:` inside `and_v(v:<lock|hash>, <sig-bearing B>)` (proven sane:
  `and_v(v:sha256(h),pk)` constructs, reparses, derives); the tap Bdu pool
  is keys-only per the grammar above. Each tap production retained at
  implementation time must have an empirical sanity proof (round-1/2
  evidence logs, or a new red→green check during TDD);
  (b) each `@i` is referenced AT MOST ONCE across the entire tr descriptor
  (internal key included) — repeated-pubkey check is whole-descriptor;
  (c) one timelock class per tr descriptor (relative-height XOR
  relative-time; absolute-height XOR absolute-time; relative + absolute
  TOGETHER is fine — round 2 proved `and_v(v:older(144),
  and_v(v:after(700000),pk))` reparses Ok). Note this per-descriptor XOR
  is DELIBERATELY stricter than miniscript's actual per-spend-path
  same-class rule (rel-height in leaf A + rel-time in leaf B would be
  legal); the coverage trade is intentional — do not "fix" it without
  re-deriving the per-path rule. [M2′] wsh/sh tiers are NOT
  sanity-constrained (reparse doesn't sanity-check them; sigless wsh
  shapes are legitimate extra coverage and are kept).
- **P6 (the new leg)**: for `d` in T: let `c = canon(d)`;
  1. `to_miniscript_descriptor(&c, 0)` MUST succeed (no filtering — a
     failure is a generator bug or a codec bug, both RED);
  2. wire round-trip: encode→string→chunks→decode == `c` (ties the leg to
     the wire);
  3. **reparse fixed-point as semantic equality [I4]:**
     `miniscript::Descriptor::from_str(&rendered.to_string())` MUST
     succeed AND equal the constructed `Descriptor` (PartialEq, verified
     present in pinned 13.0.0) — catches Display/parse asymmetries. This
     `from_str` leg — rust-miniscript's parser — is the genuinely
     INDEPENDENT oracle in P6 [I2′];
  4. **end-to-end derivation succeeds [I2′]:** `c.derive_address(0, 0,
     Network::Bitcoin)` (method; returns `Address<NetworkUnchecked>`,
     `.assume_checked()` before comparing) succeeds and equals
     `reparsed.at_derivation_index(0)` → `.address(Network::Bitcoin)`.
     NOTE: `derive_address` (derive.rs:92–133) is itself
     `to_miniscript_descriptor` + at_derivation_index + address, so given
     step 3 the EQUALITY is implied — this step's marginal value is only
     that the full derivation pipeline errors nowhere. Independence of the
     ADDRESS oracle is anchored instead by golden address literals in the
     self-test cells (below).
  (The draft's "to_miniscript(decoded) == to_miniscript(original)" oracle
  was vacuous given P1 + determinism and is dropped. [I4])
- **P7 (clean-error leg)**: wire-valid-but-miniscript-invalid inputs →
  `to_miniscript_descriptor` returns `Err` (never panics), AND the wire
  round-trip (P1/P4/P5-equivalent assertions) is still exact. Input
  classes: SortedMultiA leaf; RawPkH leaf; SortedMulti under a combinator;
  shape-C `Check(<non-bare-key>)`; `after(0)` / `after(≥0x8000_0000)`;
  `older(0)` / `older(bit-31-set)`; Segwitv0/Legacy `multi` with
  21..=32 keys [I2]. (NOT in this set: out-of-BIP-68-mask older values —
  miniscript accepts them; pinned Ok in P6. [C2])
- **P8 (encoder-side clean-error + k>n gap pin) [Q5]:** property that
  `encode_payload` returns clean `Err` (never panics) on out-of-range
  k/n/children counts; PLUS a loud characterization cell pinning that
  k > n (both ≤ 32) currently ENCODES successfully while decode rejects
  `KGreaterThanN` — an encoder-side engrave-but-can't-restore gap, same
  family as the Cycle-A find. File FOLLOWUP
  `encode-accepts-k-greater-than-n` (companion in toolkit repo per
  cross-repo convention) rather than silently constraining the generator.

### Anti-vacuity (Cycle-A discipline)

- `generator_covers_all_fragments`: sample the W and T strategies with a
  fixed-seed `TestRunner` and explicit weights on rare alternatives [M2];
  assert every target Tag appears ≥ once per strategy within the sample
  budget (W: all tags incl. the 6 fragments + all combinators/wrappers;
  T: all to_miniscript-supported tags), and that the boundary timelock
  constants actually appear.
- Permanent oracle self-test cells: fixed known-good descriptors through
  P6 (e.g. `wsh(and_v(v:pk(@0),older(144)))`,
  `wsh(andor(pk(@0),older(4096),pk(@1)))`, a tr with a sane
  `and_v(v:sha256(h),pk(@1))` leaf, and the miniscript-leniency pins
  `older(0x10000)`/`older(0x00410000)` rendering Ok with the v0.53.9
  citation), each pinning a **golden address literal** (the
  address_derivation.rs golden-vector pattern) — this is what anchors the
  address oracle independently of the converter under test [I2′]; fixed
  known-bad through P7 (SortedMultiA leaf, `after(0)`,
  `older(0x80000000)`, shape-C Check, 21-key Segwitv0 multi). [C2]
- No `prop_filter` on the T strategy (correct-by-construction); any filter
  on W carries a rejection budget (`max_global_rejects` like Cycle A).

### Scope exclusions

- Malformed/adversarial WIRE input (bit-flips, truncation) — Cycle C
  (cargo-fuzz).
- Cross-tool md1 emission differential — Cycle D.
- Fixing any bug the harness finds (file FOLLOWUPs; fix in its own cycle
  unless funds-safety dictates otherwise). This includes the P8 k>n
  encoder gap.
- md-codec library code changes (incl. the SortedMultiA rendering gap and
  shape-C Check) — pre-filed FOLLOWUPs, not this cycle.

## Resolved decisions (round-1 R0 answers, adopted)

1. Typed grammar over full type-system generator; productions as repaired
   above ([I1] or_d recursion, [C1] tap sanity, [I2] caps).
2. P6 oracle = constructed-vs-reparsed semantic equality + address
   differential; original-vs-decoded rendering comparison dropped as
   vacuous. [I4]
3. File layout: strategies in `tests/common/mod.rs`; W properties join
   `proptest_roundtrip.rs`; P6/P7/P8 in new `proptest_to_miniscript.rs`
   with `#![cfg(feature = "derive")]` (address_derivation.rs precedent).
4. 256 cases/property; OnceLock xpub derivation; W-tier node budget keeps
   P5 wall-clock sane.
5. P8 added; k>n encoder gap pinned loudly + FOLLOWUP. [Q5]
6. W-tier randomizes shared/divergent/overrides/fingerprints/pubkeys TLVs
   under the stated constraints. [Q6]
