architect md-fixdoc-stops: 4 answered; 1 prior rulings changed

Brief: `coordinator/briefs/architect-md-fixdoc-stops.md`. Inputs: FIX-B doc on
`fix/bughunt-b` 8ffe0fce, FIX-A doc and report on `fix/bughunt-a` 04398106,
grade R-4, BIP mediawiki:459 and :752. The changed ruling is R-4's sentence
"Values in [2^29, 2^31) are unchanged ...". It contradicts the rest of R-4,
so it is struck and replaced (Q1).

My own measurements: `md` built from fix-a 04398106. Its `crates/*/src` is
identical to b2bfdd63 (`git diff --stat b2bfdd63 HEAD -- crates/*/src` is
empty). Fixture XPUB_A is the one in `crates/md-cli/tests/bughunt.rs`.

| alt | encode | address --count 1 | descriptor |
|---|---|---|---|
| 536870911 | exit 0 | exit 0 `bc1qd4366caaldhw2sx6970x8zqn5f9s8yzjm84kt2` | exit 0 `…/<536870911;1>/*)#rjkyn63n` |
| 536870912 | exit 1 `md: codec error: varint value 536870912 exceeds single-extension range (max 2^29 - 1)` | exit 0 `bc1qghsy27425xapuv6gltah6uf2q9w5usrjucr5uy` | exit 0 `…#akxwxgma` |
| 2147483648 | exit 1, varint text | exit 0 `bc1q0qgl7sdpw0vvdn3g0kha052hkjugcm8zujqhea` | exit 0 `…#26hvecv9` |
| 4294967296 | exit 1 ``md: template parse error: @0 multipath alt `4294967296` is not a bare unsigned integer`` | same | same |

## Q1 (S-1, both docs): R-4's range. Answer: (i), refuse every value > 2^29-1 on every path

**Corrected R-4 sentence.** It replaces "Values in [2^29, 2^31) are unchanged: address and descriptor accept them, and encode keeps its varint refusal.":

> Values in [2^29, 2^31) pass the lexer. The codec refuses them on every path (encode, address, descriptor, and the library entries `to_miniscript_descriptor[_with_network]`, `to_miniscript_descriptor_multipath[_with_network]` and `Descriptor::derive_address`) with the existing `Error::VarintOverflow { value }` (BIP mediawiki:459), exit 1.

**Error and text for each path** (all CLI refusals exit 1 with empty stdout):

| input | where it is refused | variant | text |
|---|---|---|---|
| CLI alt in [2^31, 2^32-1] | lexer (FIX-A) | `CliError::TemplateParse` | ``md: template parse error: @0 multipath alt `<n>` is hardened; hardened derivation is impossible on a watch-only (xpub) card`` |
| CLI alt in [2^29, 2^31) | codec guard (FIX-B §2.5); encode refuses at `write_varint` as it does today | `CliError::Codec(Error::VarintOverflow)` | `md: codec error: varint value <n> exceeds single-extension range (max 2^29 - 1)` |
| CLI alt > 2^32-1 | lexer (unchanged) | `CliError::TemplateParse` | see Q3 |
| library: origin component or alt > 2^29-1 | codec guard | `Error::VarintOverflow { value }` | `varint value <n> exceeds single-extension range (max 2^29 - 1)` |

One exception is unchanged. In `derive_address`, a *hardened* alt still gets `HardenedPublicDerivation`, because `has_hardened_use_site` runs first (derive.rs:105).

**Why (i).** The BIP states it at :459 ("Component values up to 2^29 − 1 are representable; larger values are rejected with Error::VarintOverflow { value }"). It is the only VarintOverflow line in the BIP (grep). Line :752 is about how decoders treat non-minimal encodings, not about range, so it does not bear on this.

Under (ii), `md address` would keep printing addresses for a template that `md encode` refuses, which means no card can ever back those addresses. (ii) also needs a new variant, because `VarintOverflow`'s Display names 2^29-1 (error.rs:325). (i) needs no new variant, and its guard is already written and measured in FIX-B §2.5.

No md1 card can reach the guard: the decoder cannot produce a value above 2^29-1 (varint.rs). So the guard only changes library-built `Descriptor`s and CLI templates.

The struck sentence was a ruling defect, not a choice. Every md1 path routes through `to_miniscript` (`address` at cmd/address.rs:82, `descriptor` at cmd/descriptor.rs:206/210), so both halves of R-4 could not hold at once.

**Rows.**

- **FIX-A adds two md-cli rows** in `crates/md-cli/tests/bughunt.rs`:
  - `bughunt_md_04d_address_alt_above_2_29_refused_as_varint_overflow`
    - Command: `address --template 'wpkh(@0/<536870912;1>/*)' --key @0=XPUB_A --count 1`.
    - Expect: exit 1, stdout empty, stderr == `md: codec error: varint value 536870912 exceeds single-extension range (max 2^29 - 1)\n`.
    - Control in the same row: `<536870911;1>`, expect exit 0 and stdout == `bc1qd4366caaldhw2sx6970x8zqn5f9s8yzjm84kt2\n`.
    - Unmodified code (measured): exit 0, `bc1qghsy27425xapuv6gltah6uf2q9w5usrjucr5uy`.
  - `bughunt_md_04e_descriptor_alt_above_2_29_refused_as_varint_overflow`
    - Command: `descriptor --template 'wpkh(@0/<536870912;1>/*)' --key @0=XPUB_A`.
    - Expect: exit 1, stdout empty, stderr == the same line as 04d.
    - Control in the same row: `<536870911;1>`, expect exit 0 and stdout ending `/<536870911;1>/*)#rjkyn63n\n`.
    - Unmodified code (measured): exit 0, `…#akxwxgma`.
  - Both rows go GREEN on FIX-B's head, before any FIX-A production code is written.
  - Why I expect stderr to be exactly that one line: nothing is printed before the codec call. address.rs has no `eprintln`, and descriptor.rs prints its first one at :229, after :206/:210. FIX-A's builder confirms this at RED-on-B.
- **FIX-B adds one codec row** in `crates/md-codec/tests/bughunt.rs`. It also covers Q2.
  - Name: `bughunt_md_04f_codec_unselected_alt_above_2_29_varint_overflow`.
  - Setup: `wpkh_with_key`, origin 84'/0'/0', alts `[0, 536870912]` (non-hardened).
  - Expect: `assert_all_entries_refuse(&d, || Error::VarintOverflow { value: 536_870_912 })`. Chain 0 selects alt 0, so the alt that overflows is the one not selected.
  - Control: alts `[0, 536870911]`. `to_miniscript_descriptor(&d, 0)`, `to_miniscript_descriptor_multipath(&d)` and `d.derive_address(0, 0, Network::Bitcoin)` are all `Ok`.
  - It is RED on unmodified code: chain 0 has no guard, so it returns `Ok`. It is GREEN under §2.5.
  - It kills three mutations: guard at >= 2^31 (fails), guard on the selected alt only (fails), and `>=` in place of `>` (the control fails).
  - Rows 04b and 04c alone pass under both (i) and (ii), as the FIX-B doc says.

**Folds.**

- **FIX-B:**
  - Delete STOP S-1; §2.5 stands as written.
  - Add row 04f to §3, §4 and the §5 filter (`|04f_codec_unselected_alt_above_2_29_varint_overflow`). The matched count becomes **14**.
- **FIX-A:**
  - §2.3: replace "Values in [2^29, 2^31) are not touched by FIX-A (see STOP S-1 ...)" with the corrected R-4 sentence.
  - §1 item 3: "rows depending on FIX-B: 04d and 04e (their expected text is the codec's; GREEN on FIX-B's head)". Item 4 then expects 10 failing and 04d/04e passing on B's head.
  - §3: add rows 04d and 04e.
  - §6 filter: `(01|03|04|04b|04c|04d|04e|10|11|11b|12|14)`, expected **12 run, 12 passed**.
  - Delete STOP S-1.
- **Grade:** R-4's sentence is replaced as above.

Nothing here is Brian's alone.

## Q2 (FIX-B S-2): which value is reported, and are unselected alts checked

**Answer.** Every entry checks every origin component and every alternative of every key, including on the single-path entries. The first offending value is reported, in this order:

1. key order, as `expand_per_at_n` yields the keys (@0 first);
2. within one key, origin components in path order, then alternatives in path order.

This is FIX-B §2.5 exactly as written.

**Why.** A descriptor with any value above 2^29-1 has no md1 card (BIP :459). Refusing it on chain 0 while serving it on chain 1 would show the holder receive addresses for a wallet whose change chain refuses. Checking every alt also matches the multipath entry and the encoder, and §2.5 already does it with one guard in `assemble_origin_and_xkey` (to_miniscript.rs:161), which every key passes through.

The reporting order only decides which number appears in a refusal; it cannot produce a wrong result. So I add no row for the order.

**Row.** `bughunt_md_04f_codec_unselected_alt_above_2_29_varint_overflow` (Q1) pins "unselected alts are checked". Without it, a selected-alt-only guard passes every row.

**Folds.** FIX-B: delete STOP S-2, and add one sentence to §2.5: "All alternatives are checked, including on the single-path entries; the first offending value is reported (key order, then origin components, then alternatives)." FIX-A: none. Nothing is Brian's alone.

## Q3 (FIX-A S-3): `<4294967296;1>`. Answer: keep today's text

**Exact text:** ``md: template parse error: @0 multipath alt `4294967296` is not a bare unsigned integer``, exit 1, stdout empty. It is the same on encode, address and descriptor (measured above).

**Why.** 2^32 is not a BIP-32 child index at all, so calling it "hardened" would be false. Today's text refuses it, and §2.3's `u32`-parse disjunct leaves it untouched, so this costs no spec text. Correctness is a tie, and I picked the smaller spec.

**Row.** No new name. Row `bughunt_md_04_multipath_hardened_range_refused_as_hardened` gains a second assertion: `encode 'wpkh(@0/<4294967296;1>/*)'` gives exit 1, stdout empty, and stderr == the text above plus `\n`. This pins the choice against a u64-based lexer. The row stays RED on unmodified code through its first assertion.

**Folds.** FIX-A: in §2.3, drop "unless STOP S-3 rules otherwise"; add the assertion to row 04 in §3; delete STOP S-3. Nothing is Brian's alone.

## Q4 (FIX-A S-4): the private RED rows 06 and 13

**Answer.** FIX-A marks both with `#[ignore]`. It is the first FIX that can leave the workspace fully green, and both rows live in md-cli, FIX-A's crate. Deleting the rows would lose the reproductions. The attributes go directly above each `#[test]`:

- `crates/md-cli/src/seat/compose_bughunt.rs:11`: `#[ignore = "deferred-minor md-bughunt-06-comparison-form-mint-policy (design/FOLLOWUPS.md)"]`
- `crates/md-cli/src/seat/input_bughunt.rs:6`: `#[ignore = "deferred-minor md-bughunt-13-dedupe-display-separators (design/FOLLOWUPS.md)"]`

The repo already uses the `#[ignore = "<reason>"]` form (partition.rs:380, bch_exhaustive_sweep.rs:183).

**FOLLOWUPS entries.** These go in descriptor-mnemonic `design/FOLLOWUPS.md` under "## Open items", using the file's own format:

```
### `md-bughunt-06-comparison-form-mint-policy` — a pre-F-218 duplicate-slot card gets no comparison form
- **Surfaced:** md bug hunt, finding #6 (grade-bughunt-md.md, M).
- **Where:** `crates/md-cli/src/seat/compose_bughunt.rs` (`bughunt_md_06_comparison_form_independent_of_mint_policy`, `#[ignore]`d).
- **What:** verify/seating refuses (exit 1, mint-policy error) a card minted before F-218 (38cc2fb5) that seats one xpub in two slots; checking an existing card should not depend on today's mint policy. Fails closed; no wrong verdict.
- **Why deferred:** owned by the toolkit migration of verify and seating; that migration's spec owes a row asserting a verdict on such a card. Un-ignore the row there.
- **Status:** open. **Owner:** toolkit migration of verify and seating.
- **Tier:** cross-repo

### `md-bughunt-13-dedupe-display-separators` — a hyphen-grouped and an unbroken paste of one mk1 card are not deduped
- **Surfaced:** md bug hunt, finding #13 (grade-bughunt-md.md, M).
- **Where:** `crates/md-cli/src/seat/input_bughunt.rs` (`bughunt_md_13_dedupe_strings_strips_display_separators`, `#[ignore]`d).
- **What:** `--from-mk1` intake does not strip display separators before deduping. Downstream md fails closed: the same card twice exits 1 ("not a decodable mk1 string: invalid data-part length: 135"; FIX-A §3 probe).
- **Why deferred:** Minor, fails closed; seating code moves with the toolkit migration of verify and seating. Un-ignore the row there.
- **Status:** open. **Owner:** toolkit migration of verify and seating.
- **Tier:** cross-repo
```

**#13's owner.** I gave #13 the same owner as #6 because both rows are in md-cli's `seat::` module, which is the seating code that migrates. Q-B, recorded in the grade as Brian's alone, only changes this owner line: if Q-B is answered "no", it becomes "next md bug hunt". The ignore and the Minor grade stand either way, because the probe shows md fails closed. The ignore does not depend on Q-B.

**Gate.** FIX-A §6 changes "The workspace nextest cannot be green while ..." to "The workspace nextest is fully green: rows 06 and 13 are `#[ignore]`d per §2.x, with the FOLLOWUPS entries `md-bughunt-06-…` and `md-bughunt-13-…`."

**Folds.**

- **FIX-A:**
  - Add a §2 subsection with the two attributes and the two FOLLOWUPS entries; both go in FIX-A's GREEN commit.
  - Add both files to §4.
  - Change the §6 sentence as above.
  - Delete STOP S-4.
- **FIX-B:** in §5, change "`input_bughunt.rs` 13 (Q-B, FIX-A)" to "`input_bughunt.rs` 13 (Minor, deferred; FIX-A `#[ignore]`s 06 and 13)". On FIX-B's head both rows still fail, as §5 already expects.

Brian's alone: only Q-B, which is already listed in the grade. It does not block S-4.

## Notes

- Row names now collide across crates: md-codec `04b`/`04c` versus md-cli `04b`/`04c`. That is harmless, because FIX-A's filter is `-p md-cli` and FIX-B's filter uses full names. The new letters (FIX-A 04d/04e, FIX-B 04f) avoid adding any more collisions.

Counts: 0 Critical, 0 Important, 0 Minor, 0 Nit; 4 questions answered; 1 prior ruling changed (R-4 sentence); rows added 3 (FIX-A 04d, 04e; FIX-B 04f) plus 1 assertion in FIX-A row 04; folds: FIX-A 4 STOPs, FIX-B 2 STOPs + §5 line; Brian's alone 0 new (Q-B pre-existing, non-blocking).
