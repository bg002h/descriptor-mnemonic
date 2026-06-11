# R0 Review — Cycle B proptest expansion (round 4)

Reviewer: Fable 5 architect agent (a7b3f517492649bb4), 2026-06-11.
Target: design/BRAINSTORM_proptest_fragment_domain_expansion.md (R3 fold) @ 69b7a74.
Targeted re-check of the round-3 folds ([I1″] + optional Bdu-resolution minor) per the round-3 "Path to GREEN". Rounds 1–3 findings not re-litigated.
Persisted verbatim per CLAUDE.md convention.

## Verdict: GREEN

## Critical
- none

## Important
- none

## Minor
- `pk(@i)` at :211 carries the disambiguating annotation "(bare PkK; renders c:pk_k)" but `pk_h(@i)` has no parallel "(renders c:pk_h, i.e. string `pkh`)" note. The generator builds AST trees (Tag::PkH) and renders via `to_miniscript_descriptor`, so the implementation path is unambiguous — but anyone hand-writing STRING fixtures (e.g. the golden self-test cells) could write the literal `pk_h(...)`, which is type K, not B, and fails typecheck ("sub-fragment 0 has type K rather than B" — I tripped this myself in the round-4 scratch before correcting to `pkh`). One parenthetical would close it. Non-blocking.
- (Carried from round 3, no action needed: strategy-map budget assertion double-encodes; microseconds.)

## Checks

1. **[I1″] resolved exactly as specified — YES.** Folded text at :228–232: "W: **`s:pk(@i)` only** for swap (`s:` requires `Bo` — exactly one input — and `pk` is the only single-input Bdu leaf; round 3 proved `s:pkh`/`s:multi`/`s:multi_a` all fail typecheck with \"SWAP … does not take exactly one input\"), and `a:<any single-node Bdu leaf>` (`a:pkh`/`a:multi`/`a:multi_a` all proven Ok). [I1″]". This is verbatim the round-3 prescription (s: restricted to the only `o` Bdu leaf; a: stays general over single-node leaves; recursion excluded by "single-node"), with the empirical rationale and finding marker carried in-text.

2. **Bdu-resolution clause present and unambiguous — YES.** :233–234: "`Bdu` in the B-productions above denotes the ACTIVE CONTEXT's pool (Segwitv0/Legacy vs Tap, next bullet). [round-3 minor]" — placed immediately after the W production, inside the grammar bullet, tagged to its origin. The optional minor was adopted, not just acknowledged.

3. **End-to-end implementer read of the typed-grammar section (:196–260) — no remaining ambiguity, and one genuinely NEW emittable shape class found and PROVEN.** The folded `a:<any single-node Bdu leaf>` arm, combined with the context-split pools, makes `a:<hashlock>` emittable in Segwitv0/Legacy W positions (Segwit/Legacy Bdu includes hashlocks; rounds 1–3 explicitly proved only `a:pk`/`a:pkh`/`a:multi`/`a:multi_a`). Proven sane by scratch test against pinned miniscript 13.0.0 — all Ok: `wsh(and_b(pk,a:sha256(h)))`, `wsh(thresh(2,pk,a:hash160(h),s:pk))`, `sh(and_b(pk,a:hash256(h)))`, `sh(thresh(1,pk,a:ripemd160(h)))`, `wsh(thresh(2,pkh,a:sha256(h),s:pk,a:pkh))` (all four W-arm kinds mixed), plus tap composition pins `tr(NUMS,and_b(pk,a:pkh))` and `tr(NUMS,thresh(1,pk,a:multi_a(1,…)))`. Tap W remains inside the proven set by construction (tap Bdu is keys-only, so a: spans only `pk`/`pk_h`/`multi_a` there — all round-3-proven). No other production changed; the remaining grammar (and_v/and_b/or_i/or_d/andor/thresh, tap sanity rules (a)–(c), per-production TDD proof clause at :246–248) reads internally consistent and matches the round-1/2/3 evidence logs. Nothing the grammar can emit is outside the proven-or-TDD-gated set.

4. **Status header correctly reflects the round history — YES.** :3–9: "Status: R3 (round-3 findings folded; awaiting round-4 R0). Source SHA: `69b7a74`" with round 1 "RED: 3C/5I … markers `[C1]`…`[M4]`, `[Q5]`/`[Q6]`", round 2 "RED: 1C/2I … `[C1′]`, `[I1′]`, `[I2′]`, `[M1′]`–`[M3′]`", round 3 "YELLOW: 0C/1I … `[I1″]` + Bdu-resolution clause" — all verified against the persisted reviews' actual verdict lines and finding-marker counts (round 1: C1–C3, I1–I5, M1–M4, Q5/Q6).

## Evidence log

- Tree state: HEAD `69b7a74`; `git status` clean except the 4 expected untracked files (spec + three persisted reviews). Scratch file `crates/md-codec/tests/r4_scratch.rs` created, run via `cargo test -p md-codec --test r4_scratch -- --nocapture`, deleted after; nothing committed.
- Scratch results (pinned miniscript 13.0.0, `Descriptor::<DescriptorPublicKey>::from_str` — P6 step-3's oracle): the 7 shapes in check 3 all `Ok`. First scratch run had one failure — `wsh(thresh(2,pk_h(…),…))` → `TypeCheck(… sub-fragment 0 has type K rather than B)` — which is my STRING authoring error (bare `pk_h` is the K-typed unchecked fragment; the spec's `pk_h(@i)` denotes AST Tag::PkH, rendering as checked `pkh` = B, exactly as round 3's walk used). Corrected to `pkh(` → Ok. This is the source of the Minor above, not a spec defect.
- Round-history cross-check: round-1 review header "Verdict: RED" with markers [C1][C2][C3]/[I1]–[I5]/[M1]–[M4]/[Q5][Q6]; round-2 "Verdict: RED" 1C/2I; round-3 "Verdict: YELLOW" 0C/1I — all match the spec's :4–9 history lines.
- Fold-scope check: `s:` restriction and Bdu clause are the only deltas in the grammar bullet vs the round-3-reviewed text; W feeds `and_b(B, W)` and `thresh(k, Bdu, W…)` only, so the fold's blast radius is fully covered by the scratch shapes plus round 3's pre-proven set.

GREEN: 0 Critical / 0 Important. The spec is cleared for implementation. Per convention, persist this review verbatim to `design/agent-reports/cycle-b-proptest-expansion-r0-round4-review.md` and flip the spec status line to R4/GREEN before the TDD phases begin (the per-production empirical-proof clause at :246–248 remains the implementation-time gate for any production added later).
