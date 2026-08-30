# IMPL — converter C3 (`md decompose`, the D row)

**Phase:** C3 of the wallet-form converter (plan §3 C3; SPEC "P3 — the concrete
descriptor becomes an entrance").
**Worktree:** `/scratch/code/worktrees/converter-c3`, branch `impl/converter-c3`,
branched from `impl/converter-c2` (`c940529c`).
**Final SHA:** `ccd5b537`.
**Not pushed. Worktree left in place.**

## Commits

| SHA | What |
| --- | --- |
| `d2e3c6fe` | `md decompose` — the walker, the emissions, and seven P3 refusals |
| `e0fca5b4` | V-D-RT — the acceptance-grade round trip, both routes, both halves |
| `ccd5b537` | followups: two C3 findings, neither gating, each with its owning phase |

`git diff --stat impl/converter-c2..HEAD` — 12 files, **2,458 insertions, 0
deletions**. Nothing existing was rewritten; the phase is additive apart from
five wiring lines (`cmd/mod.rs`, `error.rs`, `main.rs`, `help_examples.rs`).

## What landed

### The subcommand

```
md decompose <DESCRIPTOR>... [--in FILE] [--emit WHAT] [--network NET]
```

`--emit` is a clap `ValueEnum`: `all` (default), `template`, `keys`,
`fingerprints`, `descriptor`, `commands`.

* **template** — the keyless BIP-388 template, md's admitted surface, `'`
  hardened spelling, **no checksum**. Measured 2026-08-30: md's template parser
  computes the BIP-380 checksum over the SYNTHETIC-substituted form, so a
  template carrying its own checksum draws `invalid checksum abcdefgh; expected
  75nw0edv`. A suffix here would be one md refuses, so the emission uses
  `{:#}` (the alternate formatter upstream's `write_checksum_if_not_alt`
  skips).
* **keys** — BIP-380 origin-notated records, one per line, a valid
  `mk encode --keys` file. Keys are **as parsed**: true depth, child number and
  origin from the input, never a re-serialised depth-0 form.
* **fingerprints** — `--fingerprint @i=HEX` per slot that states one.
* **descriptor** — the canonicalised concrete descriptor with a **recomputed**
  checksum. An `h`-spelled input comes back `'`-spelled with a different
  checksum (`#tpdwkkds`), pinned by a unit row.
* **commands** — both mint routes as runnable command lines.
* **all** — template, keys and fingerprints under `#` headers.

### The walker (`src/decompose/walk.rs`, 371 lines)

Fresh, as SPEC P3 r1 M5 requires. It translates
`Descriptor<DescriptorPublicKey>` → `Descriptor<String>` through
`miniscript::Descriptor::translate_pk` — structural substitution, never string
surgery on a descriptor — and renders with `{:#}`.

**Placeholder numbering is by first appearance in the canonical rendering, not
`for_each_key` order.** Measured 2026-08-30: `for_each_key` on `tr(K,pk(L))`
yields `[L, K]` — the leaf before the internal key — so numbering from it would
relabel every taproot wallet.
`walk::tests::taproot_internal_key_is_slot_zero` fails if the ordering step is
dropped.

Two fail-closed guards on the emitted template: it must contain no
`xpub`/`tpub`/`ypub`/`zpub`/`vpub`/`upub` substring (the keyless half must be
keyless), and its `@` count must equal the key count.

### Refusals, each by name

| Family | Behaviour |
| --- | --- |
| V-D-CHKSUM | Bare and correctly-checksummed descriptors accepted. A wrong checksum names **both** the supplied and the computed value and the remedy ("drop the `#…` suffix … md recomputes"). The expected value is computed with `miniscript::descriptor::checksum::Engine`, not scraped from the parser's error text, so the message cannot drift with upstream wording. Not the F-420 class. |
| V-D-JSON | `{` or `[` at position 0. **Exact, not heuristic**: a KEY expression may start with `[`, but a whole descriptor never is one (`Descriptor::Bare` still wraps a miniscript). Tested BEFORE the pair and checksum paths so a multi-line blob draws the right message. Names `listdescriptors`, the `"desc"` field, and the receive/change combination. |
| V-D-PAIR | Two descriptors → the `<0;1>` remedy, phrased conditionally ("if that is what these are") so it never falsely asserts the two are a pair. Reachable from argv (the positional is `Vec<String>`) and from `--in FILE`. A **single** fixed-path descriptor still decomposes. |
| V-D-DEPTH | Depth **or** child-number disagreement with the origin path refuses, naming mk's constraint. |
| V-D-NOORIG | Bare key line, preceded by a `#` NOT-mk-mintable comment (mk skips `#` lines), plus a stderr note. Template and descriptor emissions unaffected. `--emit commands` refuses, naming the keys, the reason, and the emissions that do work. |
| V-D-REUSE | BIP 388 rule (1): the same extended key under **different** key expressions. |
| V-D-SHAPE2 | BIP 388 rule (2): one key expression at two positions with **non-disjoint** multipath sets. |

Every refusal says "forbidden by BIP 388" / "UNSUPPORTED" and never calls the
input invalid. `assert_bip388_wording` in the test file asserts all three halves
together so no row can forget one.

Two refusals beyond the roster, both needed for soundness and both named:

* a **raw (non-extended) public key** — an md slot seats an xpub and an mk1
  card carries one, so there is nothing to decompose;
* a **network mismatch** — otherwise decompose would print `md encode --key`
  commands md's own version-byte check refuses, with nothing naming why.

## Measurements that shaped the design

All taken 2026-08-30 against this tree, with the real binaries.

1. **`for_each_key` order is not textual order.** `tr(K,pk(L))` yields
   `[L, K]`. Numbering must come from the rendering.
2. **`sanity_check` does not cover BIP 388 rule (1).** The same xpub under two
   different origins returns `Ok(())`, because the two `DescriptorPublicKey`
   values differ. It *does* report `RepeatedPubkeys` for two identical key
   expressions. So the rule-(1) check is md's own; the rule-(2) case would have
   been caught upstream but must refuse in decompose's own words.
3. **`{:#}` suppresses the checksum**; plain `{}` appends one computed over the
   template text, which md would reject.
4. **md's split re-composition loses every origin without `--fingerprint` at
   mint time.** `md encode <keyless template>` then
   `md descriptor --from-mk1-file` renders bare xpubs; add
   `--fingerprint @i=…` and the `[73c5da0a/48'/0'/0'/2']` brackets come back.
   That makes the fingerprint emission load-bearing, not decoration, and it is
   why route 2 of `--emit commands` carries the flags.
5. **`mk encode --keys` refuses an origin-less record**
   (`expected BIP-380 origin notation \`[fingerprint/path]xpub\``) and a
   depth-inconsistent one
   (`xpub origin-path mismatch: xpub depth 4 / child 2' vs origin_path depth 3 /
   last Some(Hardened { index: 0 })`). Both messages are quoted in decompose's
   own refusals.
6. **`mk encode` requires a stub binding** (`at least one of --policy-id-stub or
   --from-md1 is required`), which is why route 2 is a two-command bridge with
   `--from-md1-set policy.md1`.
7. **rust-miniscript does not parse `/**`** at the pinned rev — filed, see
   Follow-ups.

## Two defects the tests caught

Both real, both in code that looked right.

1. **`--emit commands` was not runnable.** Every md template carries `'` as its
   hardened marker, so `'{template}'` closed the quote at the first hardened
   step and the shell saw a different template than the one printed — a false
   claim in the output itself. Fixed with POSIX `'\''` quoting.
   `emit_commands_route1_line_actually_runs` now **executes** the emitted line
   through `sh -c` with `md` on `PATH` and asserts an md1 artifact comes out.
2. **The V-D-RT round-trip rows were testing the fixture, not the code.** They
   re-minted from the *fixture's* copy of the template. A mutation dropping the
   origin-path segment from `build_template` failed only **3 of 30** `v_d_*`
   tests, just one of them in the round-trip file. The rows now mint from the
   **live** emissions, and the same mutation fails **8 of 30**, 6 of them in
   that file. Recorded in the module doc.

## V-D-RT: the acceptance-grade round trip

**Fixture:** `crates/md-cli/tests/fixtures/decompose/v-d-rt.txt`, generated by
`generate.sh` in the same directory.

**Provenance.** The wallet is
`wsh(sortedmulti(2, K1/<0;1>/*, K2/<0;1>/*, K3/<0;1>/*))` where K1..K3 are
records 1–3 of `tests/fixtures/pathological/keys.txt` — the pinned
depth-consistent fixture SPEC Acceptance 1(c) names (r2 M5). The generator
reads those records from the file, and
`v_d_rt_fixture_is_the_pinned_wallet_from_the_first_three_keys_txt_records`
re-derives the descriptor from the same file and asserts equality, so the
provenance is machine-checked rather than asserted in a header.

**The generator runs decompose's own instructions.** It extracts the two mint
routes `--emit commands` prints and `eval`s them verbatim, with `md` and `mk`
symlinked onto `PATH` in a scratch directory. Both exit codes are captured and
written into the fixture header (route 1 = 0, route 2 = 0). So the committed
artifacts are evidence that the emitted commands **run**.

**Determinism verified, not assumed.** Two consecutive runs produced
byte-identical files:
`sha256 93d14a22d010dbcf0a4fd8ee682bd655418c048818af7092086ecc0c483765a1`.

**The two relations, both halves asserted separately.** `Facts` is computed in
the test from rust-miniscript directly — independent of `src/decompose` — as
(structure with keys replaced by `@n`, per-slot chain-code‖point hex, per-slot
use-site, per-slot origin).

* SPEND-EQUALITY = structure + values + use-sites, origins **excluded**.
* ROUND-TRIP-EQUALITY = spend-equality **and** origins equal exactly.

"xpub VALUE" is the chain code and point, not the 111-character serialisation.
r1 C3, re-measured here: md's `Pubkeys` TLV carries 65 bytes and md-codec
reconstructs a **depth-0** xpub, so the round trip renders
`xpub661MyMwAqRbc…` where the input had `xpub6DkFAXWQ2dHxq…`. Same wallet,
different string. `assert_round_trip_equal` asserts that inequality
**deliberately**, so if md ever carried depth on the wire someone must decide
rather than tighten the relation by accident.

**The two negatives** are the mutation guard on the relation itself: a swapped
key breaks spend-equality (structure and origins still match, so only the value
half can catch it); an altered origin leaves spend-equality intact and breaks
round-trip-equality. Without them `spend_equal` could return `true`
unconditionally and every other row would pass.

**Independent oracle:** both routes derive the same address 0 as the input
wallet, derived by md itself through the `--template` + `--key` channel with no
card in the loop (SPEC B2).

## Deviations, with reasons

1. **mk runs in the fixture generator, not inside the test.** The brief said
   "run them; mk binary BY PATH". mk is a sibling repo's binary; this repo's CI
   runs `cargo test --workspace --all-targets`
   (`.github/workflows/ci.yml:48`) and never builds it, so a test shelling out
   to mk would either red CI or skip — and a skipped gate prints ok. Instead
   the real `mk encode --keys keys.txt --from-md1-set policy.md1` ran here over
   the file `md decompose --emit keys` produced, its cards are committed with
   the command and exit code in the header, and
   `v_d_rt_emissions_still_match_what_mk_consumed` asserts byte-for-byte in CI
   that today's emission is still that file. Emission drift fails in CI; mk
   drift fails a re-run of the generator. This is the pattern
   `tests/fixtures/seating/generate.sh` already uses, for the same reason.
   *The brief's intent is served; its literal placement is not.*
2. **The BIP-388-LEGAL disjoint-multipath repetition also refuses**, with its
   own message. `wsh(sortedmulti(2,K/<0;1>/*,K/<2;3>/*))` has disjoint sets and
   BIP 388 permits it, but md's template surface refuses `@0` at two positions
   ("@N appears with inconsistent path/multipath/hardening" — SPEC A3's own
   measured scope note), so emitting that template would hand the operator
   something md cannot ingest. The refusal names **md** as the limit, not the
   BIP, and says explicitly that BIP 388 permits the shape. Pinned by
   `v_d_shape2_disjoint_sets_refuse_naming_mds_narrower_template_surface`. This
   is an extra refusal the roster does not list; it lives in the `v_d_shape2`
   family.
3. **`--network` was added** (mirrors every other md verb). Not named in the
   plan. Without it a testnet descriptor produces `md encode --key` commands md
   then refuses on version bytes, with nothing naming the reason.
4. **`--emit descriptor` exists.** SPEC P3's V-D-NOORIG bullet says "the
   template and descriptor outputs still work", so a descriptor emission is
   implied; it is the natural home for SPEC "Canonicalisation"'s recompute-the-
   checksum rule.
5. **`--emit commands`' route 2 includes a `cat > keys.txt <<'MDKEYS'`
   heredoc.** That is shell, not `md`/`mk` — but `mk encode --keys` needs a
   file, and an unrunnable snippet would be worse. The section is labelled.
6. **No `--json`.** Filed as a Nit rather than invented; see Follow-ups.

## Follow-ups filed (commit `ccd5b537`)

* `md-decompose-rejects-double-wildcard-input` — **Minor**, owning phase
  *post-converter md-cli mini-cycle (C4 may absorb it)*. rust-miniscript at the
  pinned rev `ff4732e` does not parse BIP-389's `/**` at all, so md is
  asymmetric: `md encode "wpkh(@0/**)"` desugars and works
  (`parse/template.rs::desugar_double_wildcard`, pinned by
  `tests/cli_bip388_double_wildcard.rs`) while
  `md decompose "wpkh(<key>/**)"` refuses with `at derivation index '**':
  invalid child number format`. The fix is a local desugar on decompose's
  input, but it **widens** the D-row input boundary SPEC P3 stated, so C3 filed
  it rather than taking it.
* `md-decompose-has-no-json-output` — **Nit**, owning phase *post-converter
  md-cli mini-cycle*.

Neither gates. Plan §6's three existing follow-ups were reconciled on entry:
none is owned by C3 (`md-repeated-placeholder-inverts-bip388` is explicitly
re-tagged away from this cycle; `stub-keyed-wallet-binding-at-mint` is
mint-side; the C4 items are C4's).

## Exit gate — outputs verbatim

```
$ cargo nextest run --locked
     Summary [   0.837s] 1038 tests run: 1038 passed, 2 skipped
  exit 0

$ cargo clippy --locked --all-targets -- -D warnings
    Finished `dev` profile [optimized + debuginfo] target(s) in 0.05s
  exit 0

$ cargo fmt --check
  (no output)
  exit 0

$ cargo test --workspace --doc --locked        # CI runs this too
  test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  exit 0
```

Row-scoped run over the eight `v_d_*` families:

```
$ cargo nextest run --locked -E 'test(v_d_)'
     Summary [   0.021s] 30 tests run: 30 passed, 1010 skipped
  exit 0
```

**Expected vs matched — 8 roster families (plan §4), 8 matched, none short:**

| Family (roster row) | Expected | Matched |
| --- | --- | --- |
| `v_d_rt` (V-D-RT) | ≥1 | **11** |
| `v_d_depth` (V-D-DEPTH) | ≥1 | **3** |
| `v_d_noorig` (V-D-NOORIG) | ≥1 | **3** |
| `v_d_reuse` (V-D-REUSE) | ≥1 | **2** |
| `v_d_shape2` (V-D-SHAPE2) | ≥1 | **3** |
| `v_d_json` (V-D-JSON) | ≥1 | **2** |
| `v_d_pair` (V-D-PAIR) | ≥1 | **3** |
| `v_d_chksum` (V-D-CHKSUM) | ≥1 | **3** |
| **total** | **8 families** | **8 families / 30 tests** |

Every family carries more than one test because each roster row needed both a
refusing case and a non-refusing negative half (or, for V-D-SHAPE2, the
overlapping *and* the equal *and* the disjoint set cases, which a
set-equality implementation would not separate).

**Suite growth, measured per file:** the C2 tip (`c940529c`) runs **987**
tests; C3 runs **1038**, so C3 adds **51**. Of those, 30 are the `v_d_*` rows
above (19 in `cmd_decompose.rs`, 11 in `cmd_decompose_roundtrip.rs`) and 21 are
not: 6 happy-path/runnability rows in `cmd_decompose.rs`, 14 unit rows in
`src/decompose/` (`--bin md -E 'test(decompose::)'`), and
`help_examples::decompose_example_matches_actual_output`. Per-file counts:
`cmd_decompose` 25, `cmd_decompose_roundtrip` 11, `decompose::` units 14,
`help_examples` 2 → 3.

## Snapshot surfaces (plan D3) — checked directly, nothing committed moved

* **`md gui-schema`** gains a `decompose` entry, derived from clap, not from a
  hand-maintained list: `--in` (path), `--emit`
  (dropdown `[all, template, keys, fingerprints, descriptor, commands]`),
  `--network` (dropdown), positional `DESCRIPTORS` (repeating, not required).
  `tests/cmd_gui_schema.rs` asserts a **contains** list plus the two negative
  canaries (`gui-schema` and `help` absent), not set equality, so it is
  unaffected. **No gui-schema JSON is committed anywhere in the tree** —
  grepped across `*.json`, `*.yml`, `*.sh`, `*.md`; the only hits are prose.
* **`md gen-man --out`** gains `md-decompose.1`. `tests/cmd_gen_man.rs`
  generates into a tempdir and asserts a contains-set plus the zero-`*-help*.1`
  canary, so nothing committed moves.
* **`md --help`** gains a `decompose` line. `md decompose --help` carries an
  EXAMPLES block, and `tests/help_examples.rs` now runs it —
  `check_example("decompose")` executes the printed command and compares its
  stdout against the help text, so the example cannot rot.
* **`git status` shows zero modified files under `tests/snapshots/`.** No insta
  snapshot needed regeneration; none was regenerated.

## What C3 did NOT do

* No review was dispatched. Plan §3 C3's exit names "the same scoped
  independent review pattern as C2's (sonnet, mechanical: do the eight rows
  assert what the spec's P3 bullets demand; any false-PASS shape)" — that is
  the controller's to run before C4.
* No matrix cells flipped, no CHANGELOG entry, no README note (plan §3 C4
  items 2–3).
* No `--from-mk1` / seating-engine change; C2's surface is untouched.
* No mk code change, no md-codec change, no wire-format change.
