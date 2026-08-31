# REVIEW — mdcli-mini whole-diff adversarial execution review, r1

**Date:** 2026-08-31
**Scope:** `git diff 6c981e52..14ecde84` (P1–P6 implementation + P7a sweep; 67 files,
+6464/−793). Worktree `/scratch/code/shibboleth/descriptor-mnemonic-mdcli-mini`,
branch `mdcli-mini`, HEAD `14ecde84`.
**Normative artifacts:** `design/SPEC_mdcli_mini.md` (GREEN),
`design/IMPLEMENTATION_PLAN_mdcli_mini.md` (GREEN). Spec-design questions treated as
settled; the question here is whether the IMPLEMENTATION is faithful, sound, and free of
introduced regressions.
**Reviewer:** independent, no authorship of the diff.

## Lenses run

1. **False-PASS hunting** in the new rows, with live mutation (break the code, observe
   the suite, revert).
2. **"A diff falsifies text it never touches"** — tree-wide sweep for comments, shipped
   diagnostics, release notes and tests the cycle made false but did not edit.
3. **Funds-safety adversarial pass** on the admission changes: can a refused shape still
   reach a minted card or a derived address through a path no row covers; can N3's
   bracket arm invert precedence; can R9's relaxed `ArgGroup`s reach seating with
   inconsistent state.
4. **The recorded judgment calls**, one at a time: P3's verify-single-warn, P5's timelock
   validation on `emit_md1_card`, P6's existence-based FILE/string routing, P2's tenth-site
   layer pin, P6/R6's anchored desugar.
5. **Wording sweep** of every new rendered line: the never-"invalid" rule, citation
   accuracy against BIP 388's own text, and the read-side warn body.

## What I executed

- `./scripts/phase-gate.sh` — **exit 0, all six steps.** nextest `--all-features`
  1180 run / 1180 passed / 2 skipped; `cargo test --workspace --doc` ok (0 doctests);
  clippy `--all-targets --all-features -D warnings` clean; `cargo fmt --check` clean;
  `RUSTDOCFLAGS="-D warnings" cargo doc … --all-features` clean; the
  `display-grouping-vectors.tsv` checksum pin OK.
- `MD=$PWD/target/debug/md bash crates/md-cli/tests/fixtures/seating/generate.sh` →
  exit 0, `git status --porcelain crates/md-cli/tests/fixtures/` **empty**. Plan P2 step 7's
  "`git diff` clean over every fixture written after the frozen V-R5M1 block" verified
  independently, including the frozen-fixture existence assert.
- Fetched `bip-0388.mediawiki` from `bitcoin/bips` master and read the "Additional rules"
  section (lines 189–199) and the invalid-policy list (lines 300–312) verbatim.
- ~45 live CLI probes: all five Family-1 rendered lines and R-N1d; the three card-input
  refusals; the read-side warn on both frozen fixtures; every N3 precedence arm; all four
  R9 guards on both verbs; `--verify-against` on all three input modes plus five
  pathological arguments; eleven `/**`-adjacent decompose spellings; decompose stdin
  (`-`, empty, garbage); the order-sensitivity of the replaced `keyed_tr_multi_a` vector.
- **One mutation, full-suite**: reordering two adjacent statements in
  `cmd/build.rs::build_descriptor`. Result recorded in I2. Tree reverted; `git status`
  clean; binary rebuilt and re-probed to confirm the shipped behavior is safe.

## Overall

The core of the cycle is sound. The N1 classifier is a genuine single implementation with
the disposition as a parameter, and the row set proves it structurally (the card-path rows
`assert_eq!` against the same constants as the template rows). The C1 placement constraint
is honoured — nothing new entered `encode_payload`'s validator set, and both frozen plates
read at exit 0 on all four reading verbs. N3's precedence cannot invert (see below). R9's
relaxed groups cannot reach seating with inconsistent state. The three replaced corpus
vectors preserve their stated roles, verified by measurement rather than by reading the
comment. The findings below are one wrong normative citation, one funds-shaped coverage
gap, one dropped advisory on a new minting surface, and two bodies of shipped text the
diff silently falsified.

---

# CRITICAL

None.

---

# IMPORTANT

## I1 — R-N1a's rendered line cites the wrong BIP 388 rule, on four surfaces

`crates/md-cli/src/parse/reuse.rs:272-280`, `Finding::SamePathExpression::message()`.

The line renders (measured, `md encode "wsh(sortedmulti(2,@0/<0;1>/*,@0/<0;1>/*))"`):

> `md: unsupported: @0 appears at 2 use sites in this template with the same path
> expression, so ONE key would fill every one of them. That is forbidden by BIP 388
> ("the public keys obtained by deserializing elements of the key information vector
> must be pairwise distinct"), whose forbidden-example list names sh(multi(1,@0/**,@0/**))
> — "Repeated keys with the same path expression". …`

**The quoted rule is not the rule this shape breaks.** BIP 388's "Additional rules"
(fetched from `bitcoin/bips` master, 2026-08-31) are three unnumbered paragraphs:

- line 191: "A wallet policy must have at least one key placeholder and the corresponding key."
- line 193: "The public keys obtained by deserializing elements of the key information vector must be pairwise distinct" — the sentence quoted above.
- line 195: "If two `KEY` are `KP/<M;N>/*` and `KP/<P;Q>/*` for the same key placeholder `KP`, then the sets `{M, N}` and `{P, Q}` must be disjoint."

For `sh(multi(1,@0/**,@0/**))` the key information vector holds **one** element, so
pairwise-distinctness over it is vacuously satisfied. What the shape violates is the
**disjointness** rule (line 195): `@0/**` ≡ `@0/<0;1>/*` twice, `{0,1}` and `{0,1}` are not
disjoint. BIP 388's own annotation of the adjacent invalid example
(`sh(multi(1,@0/<0;1>/*,@0/<1;2>/*))` → "Non-disjoint multipath expressions") places both
examples in the disjointness family. The spec's own R-N1a row names only the invalid-example
list as the authority; the implementation added a rule citation the spec did not mandate,
and picked the wrong one.

**It contradicts md's own shipped analysis.** `crates/md-cli/src/decompose/mod.rs:243-251`
states the split correctly — "Two occurrences of the SAME key expression are one placeholder
used twice — rule (2), which is satisfied only when the multipath sets are disjoint" — and
the D-row refusal at `:311-315` cites the disjointness rule *together with* the same invalid
example. The unification in P3.3b also **regressed** the seating door check: its previous
wording cited both rules ("…must be pairwise distinct", and two key expressions on one
placeholder must have disjoint multipath sets") and the replacement kept only the
inapplicable half.

**Blast radius.** Four surfaces render it — `md encode`, `md descriptor --template`,
`md address --template` (all `parse/template.rs:2673`), the card path
(`cmd/build.rs:96`), and the seating door check (`seat/satisfy.rs:158`) — plus the WARN
rendering on `decode`/`inspect`/`bytecode`/`verify`. Three test files pin the wrong text
verbatim: `tests/n1_admission_taxonomy.rs:120-125` (`MSG_N1A`),
`tests/seating_vectors.rs:692-700`, `crates/md-cli/src/seat/satisfy.rs:506`, and
`tests/cmd_encode.rs:1013` / `tests/sortedmulti_a_taproot_leaf.rs:150` assert its prefix.

**Failure construction.** An operator reads the refusal, opens BIP 388, and finds that the
rule md quoted is satisfied by their template — the tool's stated ground for refusing is
checkably false. `bip388.rs`'s own doc comment names this class: "a drifted quotation is a
false record about a normative document."

**Direction.** Cite the disjointness rule for R-N1a (R-N1b already states it correctly in
prose at `reuse.rs:281-289`) and keep the invalid-example quotation, which is right;
update the four pinned strings in the same commit. *(R-N1d's use of the pairwise-distinct
rule is substantively correct — two placeholders put the key in the vector twice — and is
unaffected; see N1 for its numbering label.)*

## I2 — the funds-shaped ordering in `build_descriptor` is pinned by nothing; a two-statement reorder emits `multi_a(2,X,X)` and an address at exit 0 with the suite green

`crates/md-cli/src/cmd/build.rs:66-72`.

```rust
apply_path_override_per_slot(&mut descriptor, args.path, &inline_declared, &bracket_sourced)?;
refuse_key_reuse_across_slots(&descriptor, args.cmd)?;
```

`refuse_key_reuse_across_slots`'s own doc comment states the order is load-bearing:

> Runs AFTER `apply_path_override_per_slot` so it inspects the descriptor exactly as it is
> about to be rendered — and so that a slot whose origin only `--path` supplies is
> expandable when the check runs (`expand_per_at_n` raises `MissingExplicitOrigin`
> otherwise, which makes the validator a silent no-op).

**Executed.** I swapped exactly those two statements and ran the whole suite:

```
cargo nextest run --locked --all-features --no-fail-fast
Summary  1180 tests run: 1180 passed, 2 skipped
```

**1180/1180 green.** With that binary:

```
$ md descriptor --template "tr(@0/<0;1>/*,multi_a(2,@1/<0;1>/*,@2/<0;1>/*))" \
    --key @0=<K1> --key @1=<K2> --key @2=<K2> --path "48'/0'/0'/2'"
tr(xpub…/<0;1>/*,multi_a(2,xpub661MyMwAqRbcG5161…/<0;1>/*,xpub661MyMwAqRbcG5161…/<0;1>/*))#sytk8gy9
note: stdout is watch-only — public keys only, cannot spend
   exit 0

$ md address --template "<the same>" … --count 1
bc1pj9zvvs3el6ved40pa3588nlrv4p2gkzve4jcq90phrpqterw9nksnun7y8
   exit 0
```

That is a 2-of-2 leaf **one** key satisfies alone, plus a derived receive address — exactly
the class `REVIEW-converter-whole-diff-r1` C1 closed (`sortedmulti(2,X,X,Y)` at exit 0) —
reopened by a two-line reorder that no test notices. The unmutated binary refuses both
(verified after revert), so **this is a coverage gap, not a live defect.**

**Why the existing rows miss it.** Every duplicate-key row in
`tests/duplicate_key_slots.rs` uses a **canonical** wrapper (`wsh(multi(…))`,
`wsh(sortedmulti(…))`), where `expand_per_at_n` succeeds with or without the origin
override, so the ordering is invisible to them. The gap is only reachable through a
**non-canonical wrapper whose origin comes solely from `--path` or from N3's bracket** —
`tr` with a script tree, `wsh(or_i(…))`, and the miniscript wrappers generally — and there
is no such row. This cycle edited that exact call site (`apply_path_override` →
`apply_path_override_per_slot`, and N3 changed when it runs: the function now proceeds
where it previously early-returned), so the untested invariant is one this diff touched.

**Direction.** Add one row per verb: `md descriptor` / `md address --template
"tr(@0/<0;1>/*,multi_a(2,@1/<0;1>/*,@2/<0;1>/*))" --key @1=X --key @2=X --path <P>` must
exit 1 with the `key reuse refused` line and print no `bc1`/`tr(` — the mutation above is
the check that the row can fail.

## I3 — `--emit md1`, the new minting surface, drops `md encode`'s legacy-P2SH footgun advisory

`crates/md-cli/src/cmd/descriptor.rs::emit_md1_card` carries `md encode`'s
`validate_relative_timelocks` gate across, with the right reason stated inline:

> THE AUTHORING GATE, for the same reason `md encode` runs it before it mints … This is a
> minting surface too, and which command engraved the plate makes no difference to what it
> claims.

That reasoning was applied to one of the five things `md encode` does around a mint.
`cmd/encode.rs:204-225` also emits, on every mint: `emit_legacy_p2sh_advisory` (F-A4),
`emit_pathless_advisory` (P1.2), `emit_unseatable_template_advisory` (F-227),
`emit_unhardened_origin_note` (F-410), and `emit_engraving_card`. `emit_md1_card` emits
only `emit_output_class_advisory`.

**Executed** (F-A4, an `sh(sortedmulti(…))` policy seated from two mk1 cards minted with
the sibling `mk` binary):

```
$ md descriptor <sh(sortedmulti) policy card> --from-mk1 <c1> --from-mk1 <c2> --emit md1
md1fvgjgps9q2tvyyy5jmpprj5qqcx8ppg4cyja6p372zc9gwrh7h9q2hyxs7s5yl6smtcshpj0jjh50sdt
… (4 chunks)                                                       exit 0
stderr: chunk-set-id · seating notes · address-0 note · watch-only note
$ grep -c "legacy P2SH" <that stderr>   → 0

$ md encode "sh(sortedmulti(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/1'/2'/<0;1>/*))" --key … --fingerprint …
$ grep -c "legacy P2SH" <that stderr>   → 1
    warning: sh(multi)/sh(sortedmulti) is legacy P2SH multisig — susceptible to third-party
    txid malleability, the 520-byte redeemScript limit caps you near ~15 keys, and it gets
    no segwit witness discount; prefer wsh(...) or sh(wsh(...))
```

Two commands, one wallet, one plate; one warns and the other does not. F-227 and F-410 are
genuinely inapplicable (`--emit md1` produces a keyed card, and F-410 keys off
`args.template`), but F-A4 is reachable and proven absent, and `emit_pathless_advisory`
reads `descriptor.unresolved_origin_indices()` — a property of the seated card, so it is
the same class.

**Direction.** Call `emit_legacy_p2sh_advisory(&descriptor.tree, …)` and
`emit_pathless_advisory(&descriptor, …)` from `emit_md1_card`, with a row asserting the
F-A4 line on the `sh(sortedmulti)` seating above.

## I4 — the `[Unreleased]` CHANGELOG section now states the opposite of what the cycle ships, and the cycle added no entry of its own

`CHANGELOG.md` was not touched by the diff. Its `## md-cli [Unreleased] — the wallet-form
converter` section (line 7 onward) is the release note this work will ship under, and two
of its statements are now false:

- **line 113-115:** "One key at two DISJOINT use-sites (`<0;1>` beside `<2;3>`) is **not**
  reuse: it derives a different child at every index, BIP 388 permits it, and it still
  composes and still encodes."
  Measured today: `md encode "wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))" --key @0=X --key @1=X`
  → **exit 1**, R-N1d. It composes and encodes nowhere; this is precisely the shape P2
  refuses at `encode`, `descriptor --template`, `address --template`, and both card inputs.
- **line 117-127:** "`md descriptor` refuses the BIP-LEGAL disjoint form (`@0/<0;1>/*`
  beside `@0/<2;3>/*` → "@0 appears with inconsistent path/multipath/hardening") **while
  composing the BIP-FORBIDDEN same-path form**. That predates this work and **is not
  changed by it**; … md's template admission is filed as an md-side question
  (`design/FOLLOWUPS.md`, `md-repeated-placeholder-inverts-bip388`)."
  Measured: the same-path form refuses (R-N1a, exit 1); the quoted message is no longer
  what md prints for either shape; and P7a **closed**
  `md-repeated-placeholder-inverts-bip388` in `design/FOLLOWUPS.md:2131` citing P2+P3.
  Three claims, all false, in the paragraph that describes the exact behaviour this cycle
  inverted.

Separately, the repo's convention is one stacked `## md-cli [Unreleased] — <title>` section
per change-set (there are five at lines 7/133/139/145/190), and **this cycle added none** —
so `--emit md1`, `--verify-against`, the N1 admission refusals, `--from-mk1`'s arity, N3's
bracket source, and decompose's `/**` and `-` are undocumented in the file whose header
says "All notable changes to `md-codec` and `md-cli` are documented in this file."

**Direction.** Correct the two paragraphs above in the converter section (they describe a
tree that no longer exists) and add this cycle's `[Unreleased]` section; the plan's P7
close-out lists a cross-repo manual pass but no CHANGELOG step, which is how it was missed.

## I5 — `md decompose`'s shipped disjoint-sets refusal quotes a message `md encode` no longer prints

`crates/md-cli/src/decompose/mod.rs:331-339`, a rendered operator-facing line, untouched by
the diff and falsified by it:

```
the key expression `<X>` appears at 2 positions with DISJOINT multipath sets (…). BIP 388
permits that shape — this is not a BIP violation — but md's template surface is narrower:
`md encode` refuses one placeholder at two positions ("@N appears with inconsistent
path/multipath/hardening"), so decompose would be handing you a template md itself cannot
ingest. UNSUPPORTED here. …
```

Measured today, `md encode "wsh(multi(2,@0/<0;1>/*,@0/<2;3>/*))" --key @0=X`:

```
md: unsupported: @0 appears at use sites with DISJOINT multipath sets — <0;1> and <2;3>.
The WALLET is legal under BIP 388 … Keep this wallet as a descriptor instead:
me sysw pack --as descriptor --in <your export file>
```

The quoted string is gone from that path — N1's classifier now fires ahead of
`resolve_placeholders`, which is where "inconsistent path/multipath/hardening" lived
(`parse/template.rs:782`; the new call at `:2673` is deliberately upstream of it). So the
D row tells the operator to expect a diagnostic md will not produce.

Two more sites carry the same now-stale measurement and would mislead the next reader:
`decompose/mod.rs:254-257` (doc comment, "measured 2026-08-30, SPEC A3's 'measured scope
note'") and `crates/md-cli/tests/cmd_decompose.rs:487-492` (test comment).

**Second-order, same line:** the D row and the T row now refuse the *same wallet* (one key
at two disjoint path sets) with different guidance — the T row hands the runnable escape
`me sysw pack --as descriptor --in <your export file>` that R-N1c/R-N1d were mandated to
name, the D row says "Give each position its own key, or engrave this wallet by another
route." Worth reconciling while the line is being corrected.

**Direction.** Replace the quoted message with the R-N1c line (or drop the quotation and
say "md's template surface refuses one placeholder at two positions"), and update the two
comments. `design/SPEC_wallet_form_converter.md:237` carries the same sentence but is
covered by the mdcli-mini spec's explicit "no edit to the shipped converter spec" ruling.

---

# MINOR

## M1 — the N3-specific spelling of "a slot with no path from any source still refuses" has no row

SPEC N3's third vector row and plan P4 step 1's third row. Rows 1 and 2 exist
(`tests/cli_p1_origin_key.rs::v_n3_divergent_origin_wallet_composes_and_equals_inline_pasted_origins`,
`::v_patheff_bracket_path_disagreeing_with_shared_path_refuses`). Row 3 is covered only by
the **pre-existing** `tests/cli_path_override_reaches_noncanonical.rs::address_without_path_still_refuses_a_noncanonical_wrapper`,
which uses bare keys and therefore never enters N3's new code path.

The variant N3 could plausibly have broken — a bracket sourcing *some* slots while another
has none, which now makes `apply_path_override_per_slot` proceed where it used to
early-return — is unpinned. Measured correct today:

```
$ md descriptor --template "tr(@0/<0;1>/*,pk(@1/<0;1>/*))" --key "@0=[73c5da0a/48'/0'/0'/2']<K1>" --key "@1=<K2>"
md: codec error: non-canonical wrapper requires explicit origin for @1, but none provided
   exit 1
```

**Direction.** Add that invocation as row 3.

## M2 — `--emit md1` produces an artifact with no transcribe-ready form

`md descriptor … --emit md1` has no `--out`, no `--group-size`, no `--separator`, and
prints no engraving card (`md --help`, `md descriptor --help`, verified). `md encode`
supplies all four, and the engraving card is described in its own source as "the thing a
human actually transcribes onto a plate". The S→K cell exists precisely because `md encode`
**cannot** mint this card (the depth-3/4 rule), so there is no second command to get the
grouped form from — the matrix cell is flipped ✓ while the journey it names ends at an
ungrouped string and a shell redirect at the default umask.

**Direction.** Either route `--emit md1` through the same post-mint block (which also fixes
I3), or record the omission as a follow-up with its owning phase.

## M3 — `--verify-against` with an empty FILE reports the error under the wrong flag name

```
$ md descriptor <card…> --verify-against /tmp/empty.txt
md: --in /tmp/empty.txt: no md1 strings in this file. An EMPTY file is what a FAILED
upstream command leaves behind -- check the command that wrote it.   exit 2
```

`resolve_verify_against` reuses `cmd::read_md1_inputs`, whose message is written for
`--in`. The operator passed `--verify-against` and is told about a flag they did not use.
(Exit 2 rather than a verdict is correct — no false signal.)

## M4 — the read-side warning ends with a mint/compose remedy while nothing was refused

The brief's known cosmetic, judged. On `md decode <R-N1a card>` the line renders:

```
md: warning: @0 appears at 2 use sites … md declines to mint or compose this shape:
give each distinct key its own placeholder.
```

**Judgment: it does not materially mislead** — the prefix is `md: warning:`, the exit code
is 0, and the decoded template is on stdout, so no reader can take it for a refusal. The
identical body is also the observable proof of the single-source rule, which the rows
`assert_eq!` on, and that is worth more than the polish. Two clauses are nonetheless
inapplicable at that moment: "md declines to mint or compose this shape" (md declined
nothing here) and the authoring remedy, offered to someone holding a plate they cannot
re-author. R-N1c/R-N1d additionally tell a plate-holder to "Keep this wallet as a
descriptor". The operator's real question at that moment — *is my plate still readable and
my wallet still spendable?* — is left unanswered.

**Direction (optional).** A disposition-aware final clause only, leaving the body identical
— e.g. WARN appends "This card still reads and its wallet is unaffected; md will not mint
or compose the shape again." If taken, `as_warning()` in
`tests/n1_admission_taxonomy.rs:580` has to stop deriving the warn line from the refusal
line by prefix-strip.

## M5 — `spend_equal`'s "unchanged bit for bit" is stronger than proven

`crates/md-cli/src/seat/compose.rs:225-228`: "reordering the checks inside
`spend_equal_verdict` does not change which pairs are equal". The reorder moved
`expand_per_at_n(a)?` / `(b)?` **ahead** of the structural byte compare, so a pair whose
expansion fails now returns `Err` where it previously returned `Ok(false)` after the
structure check. Unreachable through the CLI today (a `MissingExplicitOrigin` descriptor
cannot be minted, so `--verify-against` cannot be handed one), but the claim as written
covers the error case and does not hold for it. Length mismatch is safe: differing `n`
always changes `comparison_form`'s bytes, so the `if ea.len() == eb.len()` skip still
reaches `Structure`.

---

# NIT

- **N1 — "rule (1)".** R-N1d's line says `rule (1) requires "…pairwise distinct"`. BIP 388
  does not number its additional rules; counting them, the pairwise-distinct sentence is
  the **second** (the first is "A wallet policy must have at least one key placeholder and
  the corresponding key"). The label matches the repo's pre-existing convention
  (`build.rs:293`, `decompose/mod.rs:244`), so it is consistent rather than new — but an
  operator who opens the BIP counts a different rule. The *substance* of R-N1d's citation
  is correct (two placeholders do repeat the key in the vector); only the label is off.

- **N2 — burndown closure dates.** `design/FOLLOWUPS.md`'s new closure notes are dated
  `2026-08-30` (e.g. "✓ CLOSED by P4 (2026-08-30)") while the rulings they cite are the
  2026-08-31 brainstorm walk, and the report headers date the phases 2026-08-31.

- **N3 — doctests are not widened.** `.github/workflows/ci.yml:48` and
  `scripts/phase-gate.sh:33` run `cargo test --workspace --doc` **without**
  `--all-features`, so a doctest in a feature-gated module would not run. Harmless today
  (0 doctests measured), but the plan's "P6 adds prime doctest sites" did not happen and
  the latent hole the doctest line was added to close is still open on the feature axis.

- **N4 — `--verify-against` existence routing on a colliding name.** `Path::new(arg).is_file()`
  wins over "looks like an md1 string". Constructed the collision (an md1 string used as a
  filename, cwd containing it): the file is read, its contents fail to decode, exit 1 with
  `md: codec error: … does not start with HRP md1`. **Safe direction — never a verdict** —
  but the error does not say a file was read, so the operator sees md reject the string
  they pasted. Also true for a mistyped path: it is treated as a literal md1 string and
  draws a decode error rather than "no such file". P6's judgment call is sound; only the
  message hides which branch ran.

- **N5 — `count_occurrences`'s `Body::Tr` internal-key arm is unexercised for R-N1a.** Every
  fixture that triggers a card-path Family-1 finding puts the repeat in `Body::MultiKeys`
  (`r-n1a-keyed.txt`, `v-r5m1.txt`); no card exercises `tr`'s bare internal-key index. The
  function moved verbatim from `seat/satisfy.rs` (diff confirms byte-identical arms), so
  this is inherited coverage rather than new, but the arm now serves a wider predicate than
  it did.

---

## Explicitly checked and CLEAN (do not re-derive)

- **N3 precedence cannot invert.** `bracket_sourced` is written only in the `None` arm of
  `match &shared_path`, and only after `inline_paths.get(&ok.i)` has `continue`d
  (`cmd/build.rs:213-247`). A slot can therefore enter that map only when *both* higher
  sources were silent — so `apply_path_override_per_slot`'s merge arms could be reordered
  without changing behaviour. Confirmed live on all five arms (inline-wins, `--path`-wins,
  bracket-sources, bracket-vs-`--path` disagreement, bracket-vs-inline disagreement).
- **The same-xpub/different-declared-origin/same-use-site shape is refused**, and by the
  right check: `reuse::classify` correctly leaves it to the codec floor
  (`validate_no_duplicate_key_slots` compares `(xpub, use_site_path)` and ignores origin),
  which fires. Measured through the N3 bracket path at exit 1.
- **P3's verify-single-warn judgment holds.** `cmd/verify.rs` reaches `println!("OK")` only
  when `encode_payload(decoded) == encode_payload(expected)` byte-for-byte, so a card whose
  shape the template does not carry cannot reach exit 0 — Family 1 is a property of the
  tree and Family 2 of the `Pubkeys` TLV, both inside the compared payload. No input dodges
  both warn sites.
- **P5's timelock validation cannot refuse a needed seating result.** The seated descriptor's
  `tree` is the policy card's tree unchanged, and that card can only exist if `md encode`
  already passed the same `validate_relative_timelocks` on the same tree. It is also
  load-bearing rather than redundant: `encode_payload` (`md-codec/src/encode.rs:100-120`)
  does **not** run it.
- **R6's anchored desugar does not silently corrupt.** Probed `/**`, `/<0;1>/**`, `/**/0`,
  `/**'`, `/0/**`, `/*/**`, `/***`, two-key and mixed spellings, with and without an origin
  bracket: only the two BIP-388 forms rewrite (`…/**` and `…/0/**`), everything else reaches
  rust-miniscript unchanged and refuses. `/**` and `/<0;1>/*` decompose to the identical
  template. The checksum is verified against the text as written, before the rewrite.
- **R9's relaxed groups cannot reach seating with inconsistent state.** All four guard rows
  reproduced on both verbs; `template` stays mutually exclusive with `phrases` and
  `from_mk1` via its own `conflicts_with`, unaffected by `multiple(true)`;
  `check_from_mk1_arity` runs on the **merged** `--from-mk1` + `--from-mk1-file` list, so
  guard 3 cannot be evaded through the file spelling; guard order (2 before 3) is pinned by
  the exact-line rows.
- **The three replaced corpus vectors keep their roles, measured not asserted.**
  `keyed_tr_multi_a`'s leaf is still order-sensitive (written vs reversed give different
  addresses; `sortedmulti_a` gives one address for both), and the five origins of
  `keyed_wsh_timelock_hashlock` are pairwise distinct with `@4` correctly under the second
  master. `md vectors` output matches the committed corpus, and the generator re-run is
  byte-clean.
- **`md compile` cannot open a mint path**, and `md vectors` is fail-closed through the
  single named `parse_template` call, with a row that says what it does not prove.
- **The anti-over-refusal controls are real.** The clean-card control asserts *no* `md: `
  line on five verbs; the same-fingerprint/different-accounts control composes; the
  single-use-site control composes.
- **`gui-schema` is clap-derived** and already reports `--emit`, `--verify-against` and the
  new `--from-mk1` arity — no stale hand-written schema.

COUNTS: 0C / 5I / 5M / 5N
