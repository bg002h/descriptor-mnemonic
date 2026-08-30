# REVIEW-converter-whole-diff-r2 — scoped verification of FOLD-whole-diff-r1

**Scope, per dispatch brief: did the fold fix each finding of
`REVIEW-converter-whole-diff-r1`, and did it introduce a new defect? NOT a
fresh audit.** Worktree `/scratch/code/worktrees/converter-c4`, branch
`impl/converter-c4`, HEAD `a80db64a` (matches the fold's stated tip; working
tree clean).

**Settled, not re-derived (per brief):** gates green — controller reproduced
`cargo nextest run --locked` = 1069 passed / 2 skipped across all filtered
sub-runs (arithmetic cross-checked: every `-E` filter's "N run, M skipped"
summed to 1071 = 1069 + 2, consistently); clippy/fmt clean; the C1
reproduction refuses with the BIP-388 message naming both slots.

**Result: 0 Critical / 0 Important. GREEN — this closes the whole-diff gate.**

---

## Per-finding verification

For each of C1, I1–I5: (a) re-ran the review's own reproduction against the
fold tip and diffed the output against what the fold report pastes; (b)
spot-checked two (C1, I2) by reading the assertion/call site rather than
mutating, to confirm the row fails if the fix is reverted; (c) confirmed each
finding's non-refusing sibling row exists and asserts real content.

**C1 (Critical).** Re-ran verbatim: `sortedmulti(2,X,X,Y)` via `md descriptor`
and `md address` both now refuse with `key reuse refused: @0 and @1 were
given the SAME extended public key at the same use-site …` (exit 1) — matches
the fold's pasted output byte-for-byte. The origin-notated two-slot spelling
now stops at the I1 gate (`MISMATCH: @0: origin-notated --key states path …
but nothing supplies a path`) rather than the reuse gate, exactly as the fold
states; the T-row test for that spelling uses an inline-origin template
instead, which I confirmed at `crates/md-cli/tests/duplicate_key_slots.rs:281`
and ran (passes). (b) Read `refuse_key_reuse_across_slots`
(`crates/md-cli/src/cmd/build.rs:301`): if its call from `build_descriptor`
were removed, `compose()` would return exit 0 and
`assert_t_row_reuse_refusal`'s `assert!(!out.status.success(), …)` would fail
— confirmed by reading, not mutating. (c) Sibling row
`t_row_one_key_at_two_disjoint_use_sites_still_composes` exists and asserts
`out.status.success()` plus a real `wsh(` payload; ran it, passes.

**C1's deviation — the three specific checks the brief asked for:**
1. **Disjoint case still composes** — ran live:
   `md descriptor --template "wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))" --key @0=X --key @1=X`
   → composes at exit 0. `md encode` on the same template/keys also mints at
   exit 0 (measured live). Neither regressed.
2. **Same-use-site refuses on descriptor AND address** — ran both live
   against the review's three-slot reproduction; both refuse at exit 1 with
   the identical `key reuse refused` message, cmd name substituted (`md
   descriptor` / `md address`).
3. **Same validator as `md encode`, cited at both call sites** —
   `grep -rn validate_no_duplicate_key_slots crates/` returns exactly two call
   sites: `crates/md-cli/src/cmd/build.rs:301` and
   `crates/md-codec/src/encode.rs:120`, both calling
   `md_codec::validate::validate_no_duplicate_key_slots` — literally the same
   function, not a re-implementation. The three verbs cannot diverge because
   there is one function, not three.

**I1 (Important).** Ran both manifestations verbatim against the fold tip.
Manifestation A (bracket path, no winning source):
`MISMATCH: @0: origin-notated --key states path \`48'/0'/0'/2'\`, but nothing
supplies a path for @0 …` exit 1 — matches. Manifestation B (bracket path vs
`--path` disagreement): `MISMATCH: @1: origin-notated --key path
\`48'/0'/1'/2'\` disagrees with --path \`48'/0'/0'/2'\` …` exit 1 — matches.
Sibling control `v_patheff_bracket_path_agreeing_with_shared_path_succeeds`
exists and ran (passes).

**I2 (Important).** Ran the exact case-variant reproduction command (30-card
pathological set + one uppercase re-scan of card 0) through
`v_dup_a_case_variant_double_scan_still_seats` — passes, byte-identical to the
non-variant control per the test's own `assert_eq!`. (b) Read `dedupe_strings`
(`crates/md-cli/src/seat/input.rs:117-132`): the comparison key is
`normalised.to_lowercase()`; reverting to a byte-identity key would let the
uppercase re-scan survive as a second string, merge at step 2, and the test's
`assert!(variant.status.success(), …)` would fail — confirmed by reading. (c)
Sibling control `v_dup_an_all_uppercase_card_set_seats_identically` exists and
passes, confirming the tolerance is a real decoder equivalence and not a
masked failure.

**I3 (Important).** Ran `md encode` on a concrete descriptor: now prints
`md reads descriptors with \`md decompose\`:` with both `--emit commands` /
`--emit template` spellings, exit 1 — matches the fold's paste exactly.
Confirmed the BlueWallet arm (`crates/md-cli/src/parse/template.rs:347-352`)
is untouched and still refers to `me sysw pack --as <descriptor|md1>`; its own
test `f420_bluewallet_file_refers_to_me_sysw_pack` still asserts that referral
and passes. A tree-wide `grep -rn "sysw pack"` shows the concrete-descriptor
arm's referral is gone everywhere except that one BlueWallet arm and its
tests/docs — no stale copy survives.

**I4 (Important) — the widening, the second specific check.** Ran the
byte-identical reproduction against the actual `v-d-rt` keyed card
(`crates/md-cli/tests/fixtures/decompose/v-d-rt.txt:49-54`):
`md descriptor <keyed card>` (no `--key`) composes
`wsh(sortedmulti(2,[73c5da0a/…]xpub…))#7jrylug2` at exit 0 — checksum matches
the fold report's citation exactly. Adding `--key "@0=<xpub>"` to the same
invocation now refuses at the clap layer: `error: the argument
'[PHRASES]...' cannot be used with '--key <@i=XPUB|@i=[fp/path]XPUB>'`, exit
2. Confirmed the mechanism in `crates/md-cli/src/main.rs`:
`conflicts_with_all = ["phrases", "from_mk1", "from_mk1_file", "seats"]` is
present on `--key`/`--fingerprint`/`--path` for both `descriptor` (lines
326/357/392) and `address` (further down the same file) — the one word the
fold describes is exactly `"phrases"` added to the existing list. Fires on
both the new seating route and this pre-existing phrase route, as claimed.

**I5 (Important).** Ran
`v_d_rt_the_recorded_mint_commands_are_still_the_ones_decompose_emits` —
passes. Did not re-run the fold's mutation proof (already documented with
before/after `0 passed, 1 failed` in both directions in `FOLD-whole-diff-r1.md`
and reverted); accepted as measured since the row's assertions (extracting
both recorded header routes and comparing to a live `--emit commands`) are a
direct structural bind, not a string-presence check like the row it replaced.
The `mk` determination (flag probe vs. version string) was not independently
re-verified — out of scope for a fold-verification pass, since it changes a
generator script rather than `md`'s own behavior, and the fold's claim (both
binaries print `mk 0.13.0`, only one supports `--keys`) is a measurement the
report already shows working from both sides.

---

## RECORDS — CHANGELOG re-measured, live

Read the reworded CHANGELOG (`CHANGELOG.md:92-130`, "Refused, deliberately —
BIP-388 key reuse", and the `--key` precedence paragraph at `:58-77`). Spot-
checked two rows of the fold's route-by-route table live, chosen for maximum
distance from what C1/I1 already exercised:

1. **"one PLACEHOLDER twice, SAME path → `md descriptor --template` composes,
   exit 0 (md's inversion)"** — ran
   `md descriptor --template "wsh(sortedmulti(2,@0/<0;1>/*,@0/<0;1>/*))" --key @0=X`
   → composed with checksum `#kuunuuvp`, exit 0. Matches.
2. **"one PLACEHOLDER twice, DISJOINT → `md descriptor --template` refuses,
   'inconsistent path/multipath/hardening'"** — ran
   `md descriptor --template "wsh(multi(2,@0/<0;1>/*,@0/<2;3>/*))" --key @0=X`
   → `md: template parse error: @0 appears with inconsistent
   path/multipath/hardening`, exit 1. Matches.

Also independently verified the second contested sentence's remaining true
scope: an inline template origin silently wins over a conflicting `--path`
(ran a template with inline `48'/0'/0'/2'`, keys with no bracket, and
`--path 84'/0'/0'` — composed at exit 0, no complaint), consistent with the
CHANGELOG's own carve-out ("the inline origin wins, per slot … a declared
PRECEDENCE rather than an agreement").

Both spot-checked rows and the precedence check confirm the CHANGELOG now
states exactly what the binary does, including the parts it admits are still
open (the placeholder inversion, filed).

---

## MINORS commit (`b052a5e3`) — skim

`git diff --stat` for the commit touches 10 files; only three carry source
code: `crates/md-cli/src/main.rs` (M3), `crates/md-cli/src/seat/disposition.rs`
(M4), `crates/md-cli/src/seat/mod.rs` (M10). Read all three diffs directly:
every hunk is inside a `///` or `//!` doc comment — no `#[arg(...)]` attribute
value, no function body, no test assertion changed. None of the 9 "fixed"
Minors silently changed normative behavior; the gate's unchanged test count
(1069 before and after this commit, per the fold's own gate log) corroborates
this mechanically — a behavior change would need a row, gained or lost.

Confirmed the 3 filed items each carry an owning phase in
`design/FOLLOWUPS.md`: `all-features-suite-is-red-and-ungated-by-ci` (M5) →
"post-converter md-cli mini-cycle"; `md-decompose-does-not-read-stdin` (M6) →
"post-converter md-cli mini-cycle";
`sibling-toolkit-md-manual-lockstep-for-the-converter` (N2) → "operator's
call — a cross-repo docs pass". All three are genuine future phases, not the
converter cycle itself, so none is overdue under the phase-ownership rule.
Also checked in passing: I1's filed widening
(`descriptor-key-bracket-path-as-a-last-resort-source`) and the pre-existing
`md-repeated-placeholder-inverts-bip388` both carry owning phases too.

---

## What I did not re-derive

Gate green, the 1069/1069 count, clippy/fmt/doc cleanliness, and the base C1
reproduction were taken as settled per the brief and not re-run from scratch
— though every targeted `nextest -E` filter I ran (7, then 18, then 1 tests)
summed skip+run to 1071 each time, which is a consistency check on that
settled count, not a re-derivation of it. I did not re-audit the whole diff,
re-litigate the R0-closed spec decisions, or reopen any Minor/Nit disposition
beyond confirming the triage table's claims.

---

## Verdict

**GREEN. 0 Critical / 0 Important.** Every one of C1, I1, I2, I3, I4, I5
closes its actual failure scenario (re-run outputs match the fold's pastes
exactly, not merely "a refusal appeared somewhere"), each fix's row is a
genuine assertion that would fail on revert (confirmed by reading for two,
consistent by construction for the rest), and each finding's happy-path
sibling still composes real content. The MINORS commit is doc-only. This
closes the whole-diff gate — the branch is clear to merge.
