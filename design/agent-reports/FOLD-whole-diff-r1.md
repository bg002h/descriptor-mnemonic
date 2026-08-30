# FOLD-whole-diff-r1 — folding REVIEW-converter-whole-diff-r1

**Branch:** `impl/converter-c4` · **Worktree:** `/scratch/code/worktrees/converter-c4`
**Report folded:** `design/agent-reports/REVIEW-converter-whole-diff-r1.md` (1C / 5I / 11M / 3N)
**Date:** 2026-08-30

**Result: every Critical and every Important is closed, each with a row that
fails without its fix, one commit per finding.** 9 of 14 Minors/Nits fixed
inline, 3 filed with owning phases, 2 already discharged by the I3 fold.

Commits, in order:

| commit | finding |
| --- | --- |
| `cce1355f` | C1 — the T row refuses BIP-388 shape (1) |
| `f751cf2d` | I1 — the `--key` bracket path agrees with the EFFECTIVE source, or refuses |
| `ae084c25` | I2 — step 1 of the seating pipeline folds CASE |
| `8f736edc` | I3 — `md encode` refers a descriptor to `md decompose` (+ N1) |
| `972d010f` | I4 — the T-row flags refuse on the seating route, and on the phrase route |
| `53cdfb10` | I5 — replace the row that could not fail; probe mk by FLAG |
| `0a79c8bf` | RECORDS — the CHANGELOG's two contested sentences, re-measured |
| `b052a5e3` | MINORS — 9 fixed, 3 filed, 2 already done |

---

## EXIT GATE

Run at the fold tip, `cargo build --bin md` fresh:

```
$ cargo nextest run --locked
     Summary [   0.858s] 1069 tests run: 1069 passed, 2 skipped
$ cargo clippy --locked --all-targets -- -D warnings
     (no output)   clippy_exit=0
$ cargo fmt --check
     (no output)   fmt_exit=0
$ cargo doc --no-deps
     Generated target/doc/md/index.html and 1 other file   doc_exit=0
$ bash scripts/matrix-identity-check.sh
matrix identical across 4 homes, 6 lines each:
sha256: 0527deee4a60b60385b07d7bfda8d5987672d2d5195ced56db0780836a5baab1
```

**Row-scoped run over the new rows.** 22 rows were added and 1 replaced in
place, so 23 are addressed by the filter:

```
$ cargo nextest run --locked -E 'test(t_row_) or test(v_patheff_) or test(v_sflag_)
    or test(v_dup_a_case_variant) or test(v_dup_an_all_uppercase)
    or test(dedupe_folds_case) or test(the_referred_command_actually_reads)
    or test(v_d_rt_the_recorded_mint_commands)'
     Summary [   0.017s] 23 tests run: 23 passed, 1048 skipped
```

Expected 23 = 6 (C1) + 5 (I1) + 7 (I4) + 2 (I2 integration) + 1 (I2 unit)
+ 1 (I3) + 1 (I5, replaced in place). **Count matches.** The suite total is the
same arithmetic seen from the other side: the review's gate reproduction
measured **1047** at `9d0c30dc`, and 1047 + 22 new = **1069**.

Also run, not part of the gate but load-bearing for I5:

```
$ bash crates/md-cli/tests/fixtures/decompose/generate.sh
wrote .../crates/md-cli/tests/fixtures/decompose/v-d-rt.txt
$ git status --short crates/md-cli/tests/fixtures/decompose/v-d-rt.txt
     (empty — byte-identical)
```

---

## PER FINDING

### C1 (Critical) — the T row composed a BIP-388-forbidden `sortedmulti(2,X,X,Y)`

**The fix.** `cmd::build::build_descriptor` now calls
`refuse_key_reuse_across_slots` on the BUILT descriptor, after
`apply_path_override_per_slot`. Detection is
`md_codec::validate::validate_no_duplicate_key_slots` — the engine's own
validator, whose only call site was `encode.rs:120`. New `CliError::KeyReuse`
(exit 1, a content refusal like `Seat` and `Decompose`, not `BadArg`'s exit 2).
The BIP-388 citation moved to `crates/md-cli/src/bip388.rs` and is now shared
byte-for-byte with the S row's message.

**Deviation from the brief, with its reason.** The brief said to put the check
in `resolve_keys_fingerprints_and_precedence`, "which holds every parsed key".
It holds the keys but **not the use-sites**, and the use-site is what makes the
check correct rather than merely strict. Measured on the T row *before* the fix:

```
$ md descriptor --template "wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))" --key @0=X --key @1=X
wsh(multi(2,…/<0;1>/*,…/<2;3>/*))#3sxca8l0        exit 0
$ md encode      "wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))" --key @0=X --key @1=X --path …
md1fqrhyps…                                        exit 0
```

BIP 388 permits that shape, `md encode` mints it, and
`duplicate_key_slots.rs::one_key_at_two_different_use_sites_is_not_a_duplicate`
has pinned the boundary since F-218. A payload-only comparison over the parsed
`--key` values has no such boundary and would newly refuse it — replacing the
review's "three verbs, three answers" with a fourth answer. Calling the codec
validator cannot drift from what `md encode` decides.

**Two existing fixtures pinned the forbidden wallet** and are repaired, not
exempted: `cmd_address::address_mainnet_wsh_multi_2of2_receive_0` and
`cmd_address_json::snapshot_wsh_2of2_mainnet_receive_0` both used ONE xpub for
`@0` and `@1` ("degenerate but structurally valid" — it is shape (1)). Both now
use two distinct cosigner accounts. The first still derives its golden
independently with rust-bitcoin; the new snapshot address
`bc1qpa7l8h70m9vjty6580zfhvkmu8xgu5g5gt7c86agydecu0yppd2s29csg0` cross-checks
against the **v-ce1 seating route**, which composes the same 2-of-2 from mk1
cards and prints that same address 0.

**One row C1 falsified.** `one_key_at_two_different_use_sites_is_not_a_duplicate`
compared `SPLIT_SITE`'s address against `SAME_SITE`'s; `md address` now refuses
the latter, so its `assert_ne!` would have passed comparing an address to the
empty string. The contrast is redrawn on a single slot.

**Rows** (6 new; the 3 refusal rows FAILED before the fix):

```
FAIL t_row_one_key_in_two_slots_is_refused_by_descriptor
FAIL t_row_one_key_in_two_slots_is_refused_by_address
FAIL t_row_one_key_in_two_slots_is_refused_through_the_origin_notated_key
PASS t_row_three_distinct_keys_still_compose                 (control)
PASS t_row_one_key_at_two_disjoint_use_sites_still_composes  (control)
PASS t_row_the_same_slot_supplied_twice_keeps_first_wins     (pins today)
```

`--key @0=X --key @0=Y` keeps today's measured behaviour, which is **FIRST-WINS**
— measured at `9d0c30dc`, byte-identical to `--key @0=X` alone (`#jywr5cj9`).
One placeholder cannot be two cosigners, so it is not shape (1). `md encode`'s
template admission is untouched; the md-side question stays filed.

**Re-run of the review's reproduction:**

```
$ md descriptor --template "wsh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))" \
    --key "@0=$X" --key "@1=$X" --key "@2=$Y"
md: key reuse refused: @0 and @1 were given the SAME extended public key at the same
use-site, so `md descriptor` would emit a policy that names 3 cosigners and that ONE of
them can satisfy alone — forbidden by BIP 388 ("the public keys obtained by deserializing
elements of the key information vector must be pairwise distinct"; its security note adds
that reusing pubkeys can be insecure in miniscript wallet policies). UNSUPPORTED here, not
a malformed input: supply one distinct key per slot. `md encode` and `md decompose` already
refuse this wallet, so a card minted from it could never be read back.
exit=1

$ md address --template "wsh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))" \
    --key "@0=$X" --key "@1=$X" --key "@2=$Y"
md: key reuse refused: @0 and @1 … `md address` would emit a policy that names 3 cosigners
and that ONE of them can satisfy alone — forbidden by BIP 388 …
exit=1

$ md descriptor --template "wsh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*))" \
    --key "@0=[73c5da0a/48'/0'/0'/2']$X" --key "@1=[73c5da0a/48'/0'/0'/2']$X"
md: MISMATCH: @0: origin-notated --key states path `48'/0'/0'/2'`, but nothing supplies a
path for @0 …
exit=1
```

**Stated plainly:** the third command — the review's origin-notated spelling —
now stops at the **I1** gate rather than the reuse gate, because its template is
pathless and I1 refuses a bracket path no source can carry. Both are refusals at
exit 1 and no wallet is emitted either way. The C1 row for that spelling
therefore uses an inline-origin template, so it measures reuse and not path
source; both spellings are covered.

### I1 (Important) — the bracket path silently discarded, or silently overridden

**The fix.** `resolve_keys_fingerprints_and_precedence` now takes `path` and
compares the bracket against whichever source **wins** for that slot — exactly
what `apply_path_override_per_slot` decides: inline where the slot declared one,
else `--path`. Disagreement with `--path` refuses, naming the slot, both paths
and which source won. A bracket path with **no** winning source refuses too,
naming inline and `--path` as the channels, rather than emitting an origin md
knows is incomplete.

SPEC P1's sentence is extended to the effective-path wording in the same commit,
marked as a measured post-GREEN fold citing I1. The widening the fold did **not**
take is filed: `design/FOLLOWUPS.md` →
`descriptor-key-bracket-path-as-a-last-resort-source`, owning phase the
post-converter md-cli mini-cycle, **operator decision** (per F-417: widening an
accepted surface is not an implementer's call).

**Rows** (5 new; 3 FAILED before the fix):

```
FAIL v_patheff_bracket_path_disagreeing_with_shared_path_refuses
FAIL v_patheff_bracket_path_with_no_winning_source_refuses_instead_of_truncating
FAIL v_patheff_the_divergent_origin_journey_refuses_rather_than_emitting_false_origins
PASS v_patheff_bracket_path_agreeing_with_shared_path_succeeds       (control)
PASS v_patheff_a_fingerprint_only_bracket_is_unaffected              (control)
```

**Re-run of the review's reproduction, both manifestations:**

```
$ TPL=$(md decode md15zfdsssjjtvyyw2fdssj54qqxppcgsc97v883w6pfw0za5z5u79mg9qp3wxzvhhu3l6n)
$ echo "$TPL"
wsh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))
$ md descriptor --template "$TPL" \
    --key "@0=[73c5da0a/48'/0'/0'/2']xpub6DkFAXWQ2…" \
    --key "@1=[73c5da0a/48'/0'/1'/2']xpub6Dzhyrn…" \
    --key "@2=[73c5da0a/48'/0'/2'/2']xpub6EGx8sP…"
md: MISMATCH: @0: origin-notated --key states path `48'/0'/0'/2'`, but nothing supplies a
path for @0: the template declares no inline origin for it and no --path was given. The
--key bracket path is never itself a source, so md would compose @0 with no origin path at
all and render it `[73c5da0a]` — BIP-380 for "this key IS master 73c5da0a", which it is
not, and a signer handed that origin derives the wrong child. State the path where md can
use it: inline in the template (`@0/48'/0'/0'/2'/…`), or with --path when every slot
lacking an inline origin shares it.
exit=1

$ md descriptor --template "wsh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))" \
    --key "@0=[73c5da0a/48'/0'/0'/2']X0" --key "@1=[73c5da0a/48'/0'/1'/2']X1" \
    --key "@2=[73c5da0a/48'/0'/2'/2']X2" --path "48'/0'/0'/2'"
md: MISMATCH: @1: origin-notated --key path `48'/0'/1'/2'` disagrees with --path
`48'/0'/0'/2'`. The template declares no inline origin for @1, so --path is the source that
WINS for this slot and the descriptor would carry `48'/0'/0'/2'`; the --key bracket path is
never itself a source. Agreement is required — neither side silently overrides the other.
exit=1
```

Before: `#ra3r4ldv` with three `[73c5da0a]` origins, and `#963szlwy` with three
`[73c5da0a/48'/0'/0'/2']`, both exit 0.

### I2 (Important) — case-variant double scan defeated step 1 and misdiagnosed itself

**The fix.** `seat::input::dedupe_strings` compares on a separator- **and**
case-normalised key. The string KEPT is the one as supplied (first appearance
wins), so a MIXED-case string is still the decoder's to reject — bech32 forbids
mixed case within one string and that ruling belongs to `decode_string`, not to
step 1. `seat::run` passes md1 phrases through the same function
(`seat/mod.rs:113`), so the md1 half of a doubled drawer scan gains the same
tolerance.

SPEC A3(a)'s and P2's "byte-identical" wording, the implementation plan's
restatement, and the module doc that quotes the pipeline are all updated in the
same commit, marked as a measured post-GREEN fold citing I2. The four-home
matrix is untouched (sha unchanged).

**Rows** (3 new; 1 FAILED before the fix):

```
FAIL v_dup_a_case_variant_double_scan_still_seats
PASS v_dup_an_all_uppercase_card_set_seats_identically       (control)
PASS seat::input::tests::dedupe_folds_case_and_keeps_the_first_spelling  (unit)
```

**Re-run of the review's reproduction:**

```
$ md descriptor $POL --from-mk1 $A --from-mk1 $B --from-mk1 $C --from-mk1 $D \
    --from-mk1 $E --from-mk1 $(echo $A | tr a-z A-Z)
wsh(multi(2,xpub661MyMwAqRbcGQnC8zMGwRc4EXYJCgXrxx9kXtw1RXqu4TcW26PxHqssgp6sU4N5CR5o9QcUZ
PG31fzPeUHEVPkFit1WpVZTmqZLzpvZG2s/<0;1>/*,xpub661MyMwAqRbcG5161axqKvt7Kx7XBe4pWwNMvgbwdf
fMtvzPnXA85ToGs3EtpEVAAYf9PggopL6xt7ySJw5Kc7ELWVcwopEjYXVHaHy6tFz/<0;1>/*))#9uzthz8n
exit=0
```

Byte-identical to the byte-identical-double-supply control (`#9uzthz8n`).
Before: the chunk-set `373cd` refusal telling the operator to re-mint a good
card.

### I3 (Important) — `md encode`'s stale referral (and N1)

**The fix.** The concrete-descriptor arm of
`parse::template::no_placeholders_message` now prints the two `md decompose`
spellings that do the job. The **BlueWallet arm is unchanged and still correct**
— `decompose` takes a descriptor, not a `Key: value` export — and its `--as`
provenance note is re-scoped to that arm. Swept the tree: the only remaining
live `me sysw pack` referral is that one. N1 (the test module's stale header
prose) rides the same edit.

**Rows** (2 changed, both FAILED before the fix; 1 new):

```
FAIL cli_f420_descriptor_referral::concrete_descriptor_is_refused_and_sent_to_md_decompose
FAIL parse::template::lex_tests::f420_concrete_descriptor_refers_to_md_decompose
NEW  cli_f420_descriptor_referral::the_referred_command_actually_reads_that_descriptor
```

The new row is what keeps the referral honest rather than merely different: it
RUNS the command the message prints, on the descriptor that drew the message,
and asserts it yields the `@i` template `md encode --template` wants.

**Re-run:**

```
$ md encode "wpkh([4bbaa801/84'/0'/0']xpub6CUGRUonZSQ4T…/<0;1>/*)"
md: template parse error: this is a concrete wallet descriptor (it carries a real extended
key), not an md1 template — `md encode` takes a template whose keys are `@i` placeholders.
md reads descriptors with `md decompose`:
    md decompose <DESCRIPTOR> --emit commands   # the mint commands, ready to run
    md decompose <DESCRIPTOR> --emit template   # or just the @i template for --template
exit=1
```

### I4 (Important) — flags silently ignored on the seating route

**The fix, in the clap graph rather than at runtime**, so the conflict is
structural on every entrance and no route can forget it:
`conflicts_with_all = ["phrases", "from_mk1", "from_mk1_file", "seats"]` on
`--key`, `--fingerprint` and `--path`, on both `md descriptor` and
`md address`. Nothing legitimate is lost: all three already
`requires = "template"` and `--template` already `conflicts_with = "phrases"`,
so no valid invocation pairs them with any of the four. The declaration only
makes the existing rule actually fire.

**Scope extended beyond the finding, deliberately, with its measurement.** The
review scoped I4 to the new seating route. Measuring the sibling route:
`md descriptor <keyed md1 card> --key @0=X` composed **byte-identically** to the
same command without the flag (v-d-rt keyed card, `#7jrylug2`, exit 0) — the
same defect, the same dead `requires`, one route over. Closing only the new half
would have left `--key` discarded in silence on a route an operator reaches just
as easily, and it costs one word in the same declaration.

**Rows** (7 new; 5 FAILED before the fix — the phrase-route row measured
separately by removing `"phrases"` from the list and re-running: `1 failed`):

```
FAIL v_sflag_key_on_the_seating_route_refuses_on_both_verbs
FAIL v_sflag_fingerprint_on_the_seating_route_refuses_on_both_verbs
FAIL v_sflag_path_on_the_seating_route_refuses_on_both_verbs
FAIL v_sflag_the_file_channel_refuses_identically
FAIL v_sflag_the_phrase_route_refuses_the_template_flags_too
PASS v_sflag_the_seating_route_itself_is_unaffected      (control)
PASS v_sflag_the_phrase_route_itself_is_unaffected       (control)
```

Each refusal row asserts the message names BOTH sides of the conflict and that
**nothing was composed** (no `wsh(`, no `bc1`) — a refusal that still printed a
wallet would be the defect wearing an error message.

**Re-run of the review's reproduction, all three forms plus `md address`:**

```
$ md descriptor <policy card> --from-mk1 A B C D E \
    --path "48'/0'/0'/2'" --fingerprint "@0=73c5da0a" --fingerprint "@1=73c5da0a"
error: the argument '[PHRASES]...' cannot be used with:
  --path <PATH>
  --fingerprint <@i=HEX>
Usage: md descriptor --from-mk1 <STRING> <PHRASES|--template <TEMPLATE>>
exit=2

$ md descriptor <policy card> --from-mk1 A B C D E --key "@0=xpub6DkFAXWQ2dHxq…"
error: the argument '[PHRASES]...' cannot be used with '--key <@i=XPUB|@i=[fp/path]XPUB>'
exit=2

$ md address <policy card> --from-mk1 A B C D E --key "@0=xpub6DkFAXWQ2dHxq…"
error: the argument '[PHRASES]...' cannot be used with '--key <@i=XPUB|@i=[fp/path]XPUB>'
exit=2
```

All three printed `#9uzthz8n` at exit 0 before.

### I5 (Important) — the vacuous mk assertion, and the mk determination

**The vacuous row is REPLACED, not deleted.**
`v_d_rt_mk_encode_keys_accepted_the_emitted_file` grepped the fixture's own
comment header for two literal strings, so it passed whether or not mk accepted
anything, and one assertion's failure message said "exit code" while the
substring it sought was provenance prose.

Deleting it would have left a real gap. SPEC Acceptance 1(c) is already
established three ways — `generate.sh` RUNS the real `mk encode --keys` and
aborts unless it exits 0; `v_d_rt_emissions_still_match_what_mk_consumed` pins
the key file byte for byte; `v_d_rt_round_trip_equality_through_the_split_set`
seats mk's minted cards and asserts round-trip equality. What **nothing** covered
is the header's RECORDED COMMANDS: every other row consumes the fixture's
SECTIONS and never its header, so a drift in `--emit commands` would turn the
header into a stale record of an unrepeatable run, silently.

`v_d_rt_the_recorded_mint_commands_are_still_the_ones_decompose_emits` extracts
both recorded routes from the header and compares them against a live
`md decompose --emit commands`, using the same two awk extractions `generate.sh`
performs, translated to Rust.

**It can fail — proved by mutation, both directions:**

```
mutate one recorded line in the fixture header      -> 1 test run: 0 passed, 1 failed
mutate the emitter in src/cmd/decompose.rs          -> 1 test run: 0 passed, 1 failed
```

Both reverted; the fixture is byte-identical to `HEAD`.

**The pointer grep earned its keep.**
`acceptance_walks::walk_c_leg_c_rows_still_exist_in_cmd_decompose_roundtrip`
failed on the rename — exactly the rot it exists to catch — and is updated.

#### THE mk DETERMINATION

**The reproduction path is ALIVE. The review's second half is falsified.** The
review measured `mk` on `$PATH` rejecting `--keys` and concluded `generate.sh`
would fail. Run it:

```
$ bash crates/md-cli/tests/fixtures/decompose/generate.sh
wrote .../v-d-rt.txt
$ git status --short .../v-d-rt.txt      # empty: byte-identical
```

`generate.sh` already defaults `MK` to
`/scratch/code/shibboleth/mnemonic-key/target/debug/mk` and symlinks it first on
`PATH`, so the bare `mk` inside decompose's route-2 command resolves to the
pinned binary. The generator was never the broken part; **path pinning was
already in place**, and its own header already said why.

**What IS stale is the INSTALL, and the version string cannot detect it:**

| binary | date | `--version` | `mk encode --keys` |
| --- | --- | --- | --- |
| `/home/bcg/.cargo/bin/mk` | Aug 14 | `mk 0.13.0` | `error: unexpected argument '--keys' found` |
| `/scratch/code/shibboleth/mnemonic-key/target/debug/mk` | Aug 30 | `mk 0.13.0` | listed in `mk encode --help` as `--keys <FILE>` |

So: **the flag did not change — the PATH install predates it.** And because
`crates/mk-cli/Cargo.toml` has not been bumped since `--keys` landed, both print
`mk 0.13.0`; a `--version` gate would pass on the binary that fails.

The probe added to `generate.sh` therefore tests for the **FLAG**, not the
version, and fires by name for an operator who overrides `MK`:

```
$ MK=/home/bcg/.cargo/bin/mk bash crates/md-cli/tests/fixtures/decompose/generate.sh
MK=/home/bcg/.cargo/bin/mk does not support 'mk encode --keys' — it is too old.
Version strings do not discriminate (both spellings print mk 0.13.0);
build the sibling repo and point MK at its target/debug/mk.
exit=1
```

The determination is recorded in the generator's header as well as here.

---

## RECORDS — the CHANGELOG re-measured

Both contested sentences are now true against the fixed tree, reworded to
exactly what holds rather than to what the folds intended.

**1. "Both converter directions refuse it, at three points."** False at
`9d0c30dc`, and *still* not true as written after C1, because the sentence
conflated BIP 388's two shapes. Re-measured at the fold tip:

| shape | route | result |
| --- | --- | --- |
| one xpub, two DISTINCT slots, same use-site | `md descriptor --template` | exit 1 key reuse refused |
| " | `md address --template` | exit 1 key reuse refused |
| " | seating card-set check 2 | exit 1 "SAME extended public key" |
| " | door check 2 (identical origins) | exit 1 |
| " | `md decompose` | exit 1 "pairwise distinct" |
| one xpub, two DISJOINT use-sites | `md descriptor` / `md encode` | **exit 0** (BIP 388 permits) |
| one PLACEHOLDER twice, SAME path | `md descriptor --template` | **exit 0** — md's inversion |
| " | seating door check 1 | exit 1 "same placeholder at more than one position — @0 (2 positions)" |
| " | `md decompose` | exit 1 rule (2), non-disjoint |
| one PLACEHOLDER twice, DISJOINT | `md descriptor --template` | exit 1 "inconsistent path/multipath/hardening" |
| " | `md decompose` | exit 1 naming **md** as the limit, not the BIP |

The entry now says **four** points, scopes the claim to the reachable shape,
states that disjoint use-sites are not reuse and still compose, and names the
placeholder inversion as pre-existing and FILED
(`md-repeated-placeholder-inverts-bip388`) instead of implying it is closed.

**2. "a disagreeing fingerprint or path is never silently overridden."** True of
the fingerprint; true of the `--key` bracket path only after I1; and **still not
true** of one pair the sentence swept in — `--path` against an inline template
origin is a declared PRECEDENCE and the inline origin wins silently (measured:
`--path 84'/0'/0'` with inline `48'/0'/0'/2'` composes
`[deadbeef/48'/0'/0'/2']` at exit 0, which is
`v_precedence_inline_path_wins_over_conflicting_shared_path` working as
specified). The entry now separates the agreement rules from the precedence rule
and names which is which.

Two bullets were added for surface changes the entry did not carry: the T-row
flags now refuse (I4), and `md encode` refers a descriptor to `md decompose`
(I3).

---

## MINORS TRIAGE — 11 Minors + 3 Nits

| # | disposition | what |
| --- | --- | --- |
| M1 | **fixed** | `decompose`/`descriptor` added to the three shipped-surface enumerations: `crates/md-cli/README.md`'s Subcommands table, the BIP draft's "Released with the md CLI binary" list, root `README.md`'s verb line (which was stale pre-diff too) |
| M2 | **fixed** | `md address`'s row now lists three input modes and the origin-notated `--key` |
| M3 | **fixed** | "Mirrors `md encode --path`" on `descriptor`/`address` → same VALUE grammar, NOT the same rule (encode replaces wholesale, these fill per slot). `md verify --path` keeps the phrase; it really does mirror |
| M4 | **fixed** | `seat/disposition.rs`'s "TRUE binding" scoped to CE-1's accidental threat model, with the adversarial-minter boundary stated |
| M5 | **filed** | `all-features-suite-is-red-and-ungated-by-ci` → post-converter md-cli mini-cycle. Re-measured: `--all-features` = 1105 passed, **1 failed**; CI (`ci.yml:48`) runs no `--all-features`, so the whole `cli-compiler` surface is unrun |
| M6 | **filed** | `md-decompose-does-not-read-stdin` → post-converter md-cli mini-cycle. Both halves re-measured; the refusal also does not name `--in` |
| M7 | **fixed** | BRAINSTORM decision 2's retracted S→K premise now carries the C4 measurement |
| M8 | **fixed** | `README.md:128` "the mint commands for T, S and K" → measured `grep -c '^# ── route'` = **2**; there is no T mint command |
| M9 | **fixed** | "the converter makes moving between them cheap" removed from README and SPEC — false in both directions (S→K blocked and filed, K→S a declared non-goal) |
| M10 | **fixed** | `seat/mod.rs`'s "Output forms" no longer lists a keyed card, which contradicted the retraction 16 lines below inside one doc comment. SPEC/PLAN copies stay — there D→K keeps it true |
| M11 | **fixed** | `IMPLEMENTATION_PLAN:228` "all four copies flip to ✓" now names the one cell that did not |
| N1 | **done (I3)** | the F-420 test module header, same edit as I3 |
| N2 | **filed** | `sibling-toolkit-md-manual-lockstep-for-the-converter` → operator's call. The file is in another repo; the flag list in the entry is enumerated from THIS tree |
| N3 | **fixed** | `README.md:134` named three matrix homes; there are four, and `matrix-identity-check.sh` checks all four |

Not in the numbered list, from the review's addendum sweep: the FOLLOWUPS entry
`md-verify-against-flag-for-cross-form-comparison` said "254 characters" against
its own arithmetic (1,901 − 1,648 = 253 = 11 × 23). Corrected to **253** with the
working shown.

The review's noted FOLLOWUPS gap — "I3 is filed nowhere" — is moot: I3 is fixed
rather than deferred.

---

## DEVIATIONS FROM THE BRIEF, with reasons

1. **C1's check lives in `build_descriptor`, not in
   `resolve_keys_fingerprints_and_precedence`.** The resolver holds the parsed
   keys but not the use-sites, and a payload-only comparison would newly refuse
   the BIP-legal disjoint form that `md descriptor` and `md encode` both accept
   today (measured, above). Calling the codec's own validator on the built
   descriptor keeps `md descriptor` and `md encode` from ever disagreeing.

2. **The C1 refusal's wording is A3-style but the C1 origin-notated ROW uses an
   inline-origin template.** The review's own spelling is pathless, and after I1
   that stops at the path gate first — the row would have passed for the wrong
   reason. Both spellings refuse; the row measures the one it is named for.

3. **I4's fix extends past the seating route to the phrase route.** Same defect,
   same dead `requires` clause, measured on the v-d-rt keyed card, one word in
   the same declaration. Stated and rowed rather than done quietly.

4. **I5's generator fix is a FLAG probe, and the review's "the reproduction path
   is dead" claim is refuted rather than folded.** `generate.sh` already pinned
   `mk` by path; running it reproduces the fixture byte-identically. What was
   asked for (pin by path) was already true, so what landed is the sanity probe
   — tuned to the flag, because the version string provably does not
   discriminate.

5. **`v_d_rt_mk_encode_keys_accepted_the_emitted_file` was replaced rather than
   deleted.** The brief allowed deletion if redundant. It is redundant *as
   written*, but its slot covers something no sibling does — the fixture
   header's recorded commands — so the slot was refilled with a row that binds
   them and that fails under mutation in both directions.
