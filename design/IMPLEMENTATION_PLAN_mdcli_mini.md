# IMPLEMENTATION PLAN — post-converter md-cli mini-cycle

Status: DRAFT p3 — plan reviews r1 (2C/7I/7M/3N), r2 (0C/2I/4M/2N,
18/19+C1 completed per r3) and r3 (0C/1I/1M/2N) are all folded;
mechanical fold-check pending. Spec:
`SPEC_mdcli_mini.md`, **GREEN at `b8a64938`** (R0 loop closed
2026-08-31 at 0C/0I over four rounds). Baseline revision for
staleness re-validation: **`b8a64938`** — every citation in this plan
was verified against that tree, and every measurement in it was run
this session. The plan contains NO speculative code blocks, so there
is nothing for a build-gate script to extract; its machine-checkable
surface is commands and citations, all already run.

## The gate (every phase; closes `phase-gate-omits-cargo-doc`)

From P1 onward (P1 itself widens CI to match):

```
cargo nextest run --locked --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo fmt --check
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --all-features
```

plus, same gate, every run (r1 I3 — the gate must name every CI job
or the command that runs them):

```
cargo test --workspace --doc          # nextest does NOT run doctests
( cd design && sha256sum -c display-grouping-vectors.tsv.sha256 )   # conformance-vector pin, ci.yml:73
```

**P1 commits `scripts/phase-gate.sh`** running all six lines, and
every later phase runs the script, not a from-memory list. The
script states its blind spot plainly: the freebsd and musl
compile/test jobs (ci.yml:95+) AND the windows/macos legs of the
test matrix (ci.yml:31-49 runs three OS contexts; a local run
reproduces one — r2 M-a) are CI-only gates a local run cannot
reproduce — the push-ritual staging run covers them
before anything reaches `main`. The burndown row for
`phase-gate-omits-cargo-doc` closes by citing the script's commit.

**P1's own final commit runs the WIDENED lines (r1 I4):** the
narrow form exists only for re-validating work that predates P1 —
P1's deliverable is the all-features-green suite, so its gate must
observe it.

Machine-verified at baseline: clippy and doc are exit 0 WITH
`--all-features`; nextest `--all-features --no-fail-fast` fails only
the fired tripwire P1 deletes (1106 run / 1105 passed / 1 failed / 2
skipped); the workspace has 0 doctests today (the doctest line is a
latent-hole closure, and P6 adds prime doctest sites). Every phase's
implementer runs the gate before its final commit and pastes the
summary lines into the commit message.

**Acceptance-4 obligation, binding on EVERY phase (r1 I6):** every
diagnostic any phase introduces or rewrites gets a
RENDERED-stderr-line row (from the `md:` prefix onward), not a body
substring. The binding rule is the sentence above — EVERY introduced or
rewritten diagnostic, no fixed count (r2 M-c). The current
enumeration, for orientation only: R-N1a/b/c/origin(/hardening if
reachable) and R-N1d (P2); the two card-input refusals and the
read-side warning (P3); the R9 refusals on BOTH verbs (P4); the two
`--emit md1` mode refusals (P5); the SPEND-EQUAL/NOT verdict lines,
the garbage-argument decode error, and the rewritten decompose
refusal (P6).

**A plan's GREEN expires.** Before dispatching each phase's
implementer, the controller re-validates this plan against the tree,
scoped to *"what did the last phase falsify here?"* — not a fresh
audit.

## Phase order and rationale

P1 first so every later phase runs the widened gate. P2/P3 are the
normative core (risk-set; spec N1). P4–P6 are surface work on ground
P2/P3 stabilise. P7 closes out. One implementer per phase; the
controller folds small post-review fixes inline rather than spawning
more agents.

### P1 — R5: the all-features closure

1. Delete `render_tr_template` and the fired tripwire
   (`crates/md-cli/src/compile.rs:338`,
   `upstream_display_is_still_broken_delete_local_renderer_when_this_fails`);
   render via upstream `Display`. The tripwire IS the failing test —
   after the deletion the all-features suite must be fully green.
   **Disambiguation already run (r1 M3, plan-r1 reviewer):** the
   tripwire fails ALONE while
   `render_tr_template_pins_every_topology_class` passes — the
   repo's own documented test (`compile.rs:329-333`) for "genuine
   upstream PR #953, not an ordering change". Do not re-derive.
   **Checksum note (r1 M4):** upstream `Descriptor::to_string()`
   appends a BIP-380 checksum the deleted `format!` build never had;
   `render_tr_template_pins_every_topology_class` compares exact
   checksum-free strings and is the gate for the strip — keep it
   green, and RENAME it (r1 N3: its name cites the deleted
   function).
2. Widen `.github/workflows/ci.yml`: `--all-features` on the test
   job (`cargo test --workspace --all-targets`), the clippy job
   (line 65) and the doc job (line 93), in the same commit that
   widens the gate above. Sequencing (1) before (2) keeps CI green
   throughout (never-skip-jobs).
3. Sweep for orphans of the deletion: `compile.rs`
   imports/helpers (clippy `--all-features` now sees them) AND the
   comment naming `render_tr_template` at
   `crates/md-codec/tests/bitcoind_differential.rs:671` (r1 M5).

Closes FOLLOWUPS `all-features-suite-is-red-and-ungated-by-ci`.

### P2 — N1 mint/compose refusals (template path)

The classifier, per the spec's placement constraints: input is
(i) the per-placeholder occurrence list and (ii) the resolved
per-`@i` key bindings; each predicate has ONE implementation;
per-verb disposition (refuse/warn) is a parameter; NOTHING new inside
`encode_payload`'s validator set; the codec floor
(`validate_no_duplicate_key_slots` at `encode.rs:120`) and the S-row
`check_no_repeated_xpub` stay as shipped.

**A third shipped implementation, named and RULED (r2 I-2; ground
corrected per r3 I-1):** `check_no_repeated_placeholder`
(`seat/satisfy.rs:188`, called at `seat/mod.rs:134` as the first
door check on the seating path) already refuses Family-1 shapes on
`md descriptor`'s card input. The ground for unifying is the spec's
SINGLE-SOURCE rule alone: `descriptor`'s card input is on the
REFUSE mint/compose surface, so P3.1's new card-input Family-1
refusal would otherwise be a SECOND implementation of the predicate
already shipped at `satisfy.rs:188`. Reachable-domain facts (r3,
measured): the policy at the door check is KEYLESS by construction
(`seat/mod.rs:122-130` refuses wallet-policy cards first), so
Family 2 is undetectable there in principle; and the wire cannot
carry one placeholder at two different triples (`md encode` refuses
"inconsistent path/multipath/hardening"), so the check's reachable
domain is EXACTLY R-N1a — where its shipped wording is correct.
Ruling: P3 UNIFIES it — the door check becomes an invocation of the
shared classifier (per-verb disposition) IN PLACE: its position is
order-normative (`seat/mod.rs:100-106`: "each of those refusals is
accurate only where it sits") and MUST NOT move past A3 to obtain
key bindings it does not need. Its wording becomes the taxonomy's
R-N1a message (the only reachable row), and the two sites pinning
the old wording (`seating_vectors.rs:679-687`,
`satisfy.rs:530-548`) update in the same commit. It is NOT in the
stays-as-shipped set; `check_no_repeated_xpub` and the codec floor
still are.

TDD order — rows first, red before green:

1. **Fixture mint FIRST, from the baseline binary — full commands,
   flags included (r1 I7: the keyless spelling mints a one-chunk
   template card that composes nothing):**

   ```
   md encode "wsh(sortedmulti(2,@0/<0;1>/*,@0/<0;1>/*))" \
     --key @0=<KEY 1 xpub> --path "48'/0'/0'/2'" --group-size 0
     # → chunk-set-id 0xed813, 2 chunks — the KEYED R-N1a card
   md encode "wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))" \
     --key @0=<KEY 1 xpub> --key @1=<KEY 1 xpub> \
     --path "48'/0'/0'/2'" --group-size 0
     # → chunk-set-id 0x00ee4, 4 chunks — the R-N1d delta card
   ```

   Baseline procedure (r1 M7): `git worktree add <dir> b8a64938 &&
   cargo build -p md-cli` — and verified benign: the spec's
   `ed2fe9c2` and this plan's `b8a64938` differ by nothing under
   `crates/`, `scripts/` or `.github/`, and P1 touches nothing
   on the `md encode` path (r2 M-d: its edits are a feature-gated
   compile module, CI config, and test-file text), so a post-P1 tree
   mints byte-identical cards. Commit the strings as fixtures with
   provenance headers in the V-BOUND-REF pattern
   (`crates/md-cli/tests/fixtures/seating/generate.sh` — full path,
   r1 M2). After this phase no shipped binary can mint them — the
   fixtures are the read-side and card-input rows' input forever.
2. Family-1 rows: R-N1a refusal at `encode`, `descriptor
   --template`, `address --template`; R-N1b; R-N1c full rendered
   line (new error variant — NOT `CliError::TemplateParse` — body
   names BIP-legality, one-path-per-key/F-417, and the runnable
   escape `me sysw pack --as descriptor --in <your export file>`);
   R-N1-origin (names the origin axis; cites no BIP rule);
   R-N1-hardening reachability probe — pin the row if reachable past
   the single-site hardened-wildcard refusal, otherwise record
   unreachability in the phase report.
3. R-N1d delta rows: template-path refusal with full rendered line
   (attributes the pairwise-distinct violation to the SPELLING's key
   vector; wallet-expressible-as-descriptor; same escape; never the
   shipped same-use-site wording). Flip
   `duplicate_key_slots.rs::one_key_at_two_different_use_sites_is_not_a_duplicate`
   and `::t_row_one_key_at_two_disjoint_use_sites_still_composes`
   to refusal rows.
4. Must-COMPOSE control: same fingerprint, different accounts,
   different xpubs — composes (the operator-probe row; catches a
   fingerprint-keyed misimplementation).
5. Correct the stale comments: `cmd/build.rs:280-283` ("BIP 388
   permits it") and `md-codec/src/validate.rs:353-355`.
6. **R-N1a blast-radius dispositions (r1 C1) — enumerated; the
   implementer improvises none of them.** Measured at baseline, all
   exit 0 today and refused after this phase:
   - The three MANIFEST vectors carrying refused shapes are
     **REPLACED, not deleted**, each preserving its stated coverage
     role with a BIP-legal shape: `keyed_tr_sortedmulti_a` and
     `keyed_tr_multi_a` (internal key repeated in the leaf at the
     identical triple) give the INTERNAL KEY a placeholder that
     appears nowhere in the leaf (r2 N-a: the leaf placeholders are
     already distinct today) — the order-sensitivity role
     (`keyed_tr_multi_a`'s comment: the ONLY order-sensitive tap
     leaf, mutation-proven) survives either way, verified by r2:
     the leaf keeps two distinct keys in written order; `keyed_wsh_timelock_hashlock` (@1/@2 repeated
     across the two spending clauses) gets fresh placeholders in the
     recovery clause. **Rust-primary lockstep:** the corpus change
     lands here with the phase; the Go port's sync is flagged in the
     phase report and follows, never leads.
   - The binding tests (r2 I-1 — `template_roundtrip.rs` and
     `json_snapshots.rs` SKIP all three vectors via their
     `force_chunked` continue and were never red):
     `crates/md-cli/tests/vector_corpus.rs:15` (`diff -r`s
     `md vectors` output against the committed corpus) and
     `crates/md-cli/tests/conformance_vectors_roundtrip.rs:36`.
     Both go red until the COMMITTED CORPUS is regenerated:
     `md vectors --out crates/md-codec/tests/vectors` rewrites the
     15 files ({the 3 vector names} × {template, bytes.hex,
     phrase.txt, descriptor.json, conformance.json}). Those files
     are the cross-language artifact vendored into the Go port
     byte-for-byte (`crates/md-cli/tests/corpus_origin_consistency.rs:11-14`) — THIS is
     where the Rust-primary lockstep bites: regeneration lands with
     this phase, the Go vendor sync is flagged in the phase report
     and follows. Constraint on the replacements: never bind one
     `[fingerprint/path]` origin to two different xpubs
     (`crates/md-cli/tests/corpus_origin_consistency.rs` reads the conformance JSONs).
     Checked clear by r2, do not re-derive: the
     `display-grouping-vectors.tsv` checksum pin holds zero MANIFEST
     content; no insta snapshot carries `keyed_*`; `wire_golden.rs`
     pins a different vector; the BIP mediawiki names none of the
     three.
   - **`md vectors` invocation point, ruled:** the classifier sits
     where `md vectors`' `parse_template` path hits it, so the
     generator REFUSES a future forbidden-shape vector fail-closed.
   - `sortedmulti_a_taproot_leaf.rs:77/:139`: rewrite the templates
     to distinct placeholders (the leaf-admission behavior under
     test does not need reuse); `:106`'s message assertion updates
     to whichever refusal now fires first, verified by running it.
   - `cli_unhardened_origin_note.rs:134` and
     `cli_keyed_excess_origin_note.rs:169`: the repeated-placeholder
     template is an incidental vehicle — rewrite to `@0`/`@1`,
     preserving the note under test.
7. **The seating-fixture generator survives (r1 C2):**
   `crates/md-cli/tests/fixtures/seating/generate.sh`'s V-R5M1 block
   mints an R-N1a-shaped template (measured exit 0 today; refused
   after this phase, and under `set -e` the script would die AFTER
   truncating `v-r5m1.txt`, leaving every later fixture
   unregenerated). Disposition: regenerate `v-r5m1.txt` one final
   time from the baseline binary, convert its block to an
   existence-assert with a frozen-by-design provenance note, and
   re-run the generator end-to-end in this phase's gate —
   `git diff` clean over EVERY fixture written after that block is
   the check (r2 M-b: that is 17 files, not r1's nine).
8. **`md compile` determination, recorded and pinned (r1 I1):**
   probed at baseline — `md compile 'thresh(2,pk(@0),pk(@0),pk(@1))'
   --context segwitv0` (and `and`/`or` forms, and `--context tap`)
   all refuse with "Policy contains duplicate keys". The refusal is
   rust-miniscript's, pinned locally by nothing — add the row (a
   duplicate-key policy refuses at `md compile`) so an upstream bump
   cannot silently open a mint path for a refused shape.

### P3 — N1 card paths + read side

1. Card-input composing refusals: `descriptor` and `address` refuse
   the P2 fixture cards (both the R-N1a card and the R-N1d delta
   card) — the r3 I-1 rows; measured today both read at exit 0
   through the card branch (`build.rs:69-77`, no reuse check).
2. Read-side rows on both fixture cards: `decode`, `inspect`,
   `bytecode`, `verify` complete at exit 0 WITH a warning naming the
   BIP-388 violation (Acceptance 5's newly-refuses scope). `verify`'s
   `--template` argument warns rather than refuses.
3. Seating: the V-BOUND-REF different-paths sibling row (same xpub
   declared at two paths refuses at `check_no_repeated_xpub` —
   measured 2026-08-31, previously unpinned).
3b. **The door-check unification (r3 M-1 — the deliverable ruled in
   P2's preamble, restated here where its implementer reads):**
   `check_no_repeated_placeholder` becomes an in-place invocation of
   the shared classifier with the R-N1a rendered line;
   `seating_vectors.rs:679-687` and `satisfy.rs:530-548` update in
   the same commit; the refusal does not move past A3.
4. T-row/S-row parity is now row-pinned from both sides (spec r3
   M-1): the delta wallet refuses at T (P2) and S (shipped);
   the legitimate same-seed wallet composes at T (P2 control) and
   seats at S (shipped control).

Closes FOLLOWUPS `md-repeated-placeholder-inverts-bip388`.

### P4 — N3 bracket-as-source + R9 arity (the descriptor/address input surface)

1. N3: the last-resort arm in
   `resolve_keys_fingerprints_and_precedence` plus the per-slot
   override via `apply_path_override_per_slot`. Rows: the
   different-accounts wallet composes AND equals the
   inline-origins composition byte-for-byte; a disagreeing bracket
   with `--path` present still refuses; a slot with no path from any
   source still refuses.
2. R9 — **on BOTH verbs (r1 I2): `--from-mk1` is declared twice,
   `main.rs:400` (`Descriptor`) and `:560` (`Address`), each under
   its own required input group.** `num_args = 1..` on both; BOTH
   guards on both verbs (mk1-prefix in the positional → names
   `--from-mk1`; md1-prefix among `--from-mk1` values → names the
   positional); the flag-first ordering must produce the symmetric
   guard's diagnostic, not clap's missing-required error — mechanics
   are this phase's to choose under that outcome. Rows: the four
   below on `descriptor`, plus at minimum the two guard rows
   duplicated on `address`: positional-first composes; flag-first
   trailing-md1 → named refusal; mk1-in-positional → named refusal;
   `--from-mk1` with no policy card anywhere → named refusal
   (nothing falls through to seating with an empty policy). Guard
   scope note (r1 M6): the md1-prefix guard applies to the
   `--from-mk1` values and the positional ONLY — P5's literal
   `--emit md1` value must not trip it, and P5's rows run after P4
   and would catch it.

Closes FOLLOWUPS `descriptor-key-bracket-path-as-a-last-resort-source`
and `from-mk1-arity-spills-card-strings-into-the-md1-positional`.

### P5 — N2: `--emit md1`

1. Emission from the seating result on `md descriptor` with
   `--from-mk1` OR `--from-mk1-file` (r1 M1 — both are admissible
   per spec; `collect_mk1` at `main.rs:891` merges them, and one row
   uses the `--from-mk1-file` spelling, the one the FOLLOWUPS
   journey recommends for a 30-card set); carries the origin
   metadata learned from seating; depth rule on `md encode --key`
   untouched.
2. Input-mode refusals: `--emit md1` with `--template` refuses
   naming `md encode`; on a keyed-card positional refuses as a
   re-emit, by name.
3. Oracle rows: PRIMARY — byte-identity against `md encode` invoked
   with inline per-slot origins + per-slot `--fingerprint @i=HEX`
   matching the policy declarations (measured executable at
   baseline); SECONDARY — `spend_equal` and address-0 equality
   against the keyed fixture; plus one row pinning a seating refusal
   surviving under `--emit md1`.
4. Matrix: flip the S→K cell in all 4 homes in the same commit;
   `scripts/matrix-identity-check.sh` green in the gate run.

Closes FOLLOWUPS `md-cannot-mint-a-keyed-card-from-a-split-set`.

### P6 — R3 + R6 + R7 (small riders)

1. R3 `--verify-against <md1|FILE>` on `md descriptor`: wires
   `spend_equal` (delete its `#[allow(dead_code)]` and the
   now-false comment); output per spec; **exit 0 equal / 5 not / 1-2
   errors**. **The flag must NOT inherit the T-row flag pattern
   (r1 I5): every other value flag on this verb is
   `requires = "template"` + `conflicts_with_all = [phrases,
   from_mk1, …]`, which would make the flag unusable on exactly the
   two card modes the FOLLOWUP exists for.** Admissible on all three
   composing input modes. Rows: equal cross-form pair (0) — spelled
   as the SPLIT set via `--from-mk1` verified against the keyed
   card; a keyed-card POSITIONAL composition with `--verify-against`
   (0) — the mode row; one-xpub-off (5, names the values half);
   origins-differ (0, EQUAL); garbage argument (1, decode error, no
   verdict).
2. R6: one desugar core serving both spellings (generalise off the
   `@`-anchor or two thin front-ends over one component — no
   keep-in-sync regex pair); `--help` names `/**`; row: `/**`
   decomposes identically to its `/<0;1>/*` rewrite.
3. R7: `-` on decompose's positional (≡ `--in /dev/stdin`); the
   not-a-descriptor refusal names `--in` and `-`.

Closes FOLLOWUPS `md-verify-against-flag-for-cross-form-comparison`,
`md-decompose-rejects-double-wildcard-input`,
`md-decompose-does-not-read-stdin`.

### P7 — close-out

1. R8: the FOLLOWUPS entry `md-decompose-has-no-json-output`
   already states the trigger in nearly the spec's words (r1 N2) —
   VERIFY it suffices, add only a one-line "parked by the mini-cycle
   walk ruling 2026-08-31" closure citing the brainstorm; do not
   duplicate the trigger text.
2. Cross-repo docs pass in `bg002h/mnemonic-toolkit`:
   `docs/manual/src/40-cli-reference/42-md.md` catches up on the
   converter cycle's surface plus this cycle's
   (`--verify-against`, `--emit md1`, bracket-source, `-` on
   decompose, `/**`, R9 arity); `tests/lint.sh flag-coverage` green
   there. Also closes FOLLOWUPS
   `sibling-toolkit-md-manual-lockstep-for-the-converter` (ruled
   into this phase by the operator, 2026-08-31).
3. FOLLOWUPS reconciliation sweep over ALL ELEVEN burndown rows
   (r1 N1): the eight originals, the walk-discovered R9 entry,
   `phase-gate-omits-cargo-doc` (closes citing P1's
   `scripts/phase-gate.sh` commit) and `sibling-toolkit-…` (closes
   citing this phase's docs-pass commit); each closure cites a
   commit.
4. **Whole-diff independent adversarial review (mandatory,
   non-deferrable)** over the full cycle diff before merge; report
   persisted; fold loop to 0C/0I.
5. Push via `scripts/push-via-staging.sh`; `main` frozen for the
   window.

## Follow-up burndown (slug → owning phase)

| slug | phase |
| --- | --- |
| `all-features-suite-is-red-and-ungated-by-ci` | P1 |
| `md-repeated-placeholder-inverts-bip388` | P2+P3 |
| `descriptor-key-bracket-path-as-a-last-resort-source` | P4 |
| `from-mk1-arity-spills-card-strings-into-the-md1-positional` | P4 |
| `md-cannot-mint-a-keyed-card-from-a-split-set` | P5 |
| `md-verify-against-flag-for-cross-form-comparison` | P6 |
| `md-decompose-rejects-double-wildcard-input` | P6 |
| `md-decompose-does-not-read-stdin` | P6 |
| `md-decompose-has-no-json-output` | P7 (parked, trigger recorded) |
| `sibling-toolkit-md-manual-lockstep-for-the-converter` | P7 |
| `phase-gate-omits-cargo-doc` | closed by this plan's gate |

## Review protocol

Plan R0 to 0C/0I before P1's implementer dispatches. Rust-primary:
every admission change lands here with vectors; no Go port leads.
Reviewer tiers sonnet/opus; agents persist their own reports to
`design/agent-reports/`; persist-before-fold, two commits, gate
output in the fold commit's message.
