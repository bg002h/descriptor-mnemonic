# FIX-B: md bug hunt, codec fixes (small-fix track)

Repo: descriptor-mnemonic. Crate: `md-codec` only. Base: `b2bfdd63` (branch
`fix/bughunt-b`). Inputs: grade `design/agent-reports/grade-bughunt-md.md`
(rulings R-2, R-4 codec half, R-5, R-7, R-8, R-9, R-16), triage
`design/agent-reports/bughunt-triage-md.md`. Review class: B (address and
descriptor derivation). No CLI flag, no printed CLI text of its own and no
manual change (the toolkit manual is untouched; see "Build order" for the CLI
texts that change as a consequence).

Toolchain: `export PATH=$HOME/.cargo/bin:$PATH`; cargo 1.85.0; never
`--release`.

Architect ruling: `design/agent-reports/architect-md-fixdoc-stops.md`
(R-4 refuses every value above 2^29-1 on every path; all alternatives are
checked).

## 1. Build order

1. FIX-B builds and merges first.
2. FIX-A bases on FIX-B's merged head.
3. FIX-A rows whose expected output depends on FIX-B:
   - `crates/md-cli/tests/bughunt.rs` row `bughunt_md_04_*` (`md encode` with
     an alt of 2147483648): with FIX-B applied the codec refuses with
     `md: codec error: varint value 2147483648 exceeds single-extension range (max 2^29 - 1)`
     unless FIX-A's CLI refuses first with its own text.
   - Any `md address` / `md descriptor` row FIX-A adds for #4 with a value
     above 2^29-1: the codec now refuses those (`Error::VarintOverflow`,
     text as above with the value), exit 1, where unmodified code prints an
     address and exits 0. This includes [2^29, 2^31), e.g. 536870912.

## 2. Spec delta (exact texts)

### 2.1 BIP error taxonomy

`bip/bip-mnemonic-descriptor.mediawiki`, table at lines 929-958. After line
956 (`| <code>TlvOrderingViolation { prev, current }</code> || TLV entries in non-ascending tag order.`)
and its separator line 957 (`|-`), insert these two lines:

```
| <code>TlvUnknownTagOutOfRange { tag }</code> || An unknown TLV entry's tag is outside the reserved range 0x04–0x1F (encoder refuses before writing).
|-
```

No other BIP text changes. Line 459 (VarintOverflow above 2^29-1) and line
752 (lenient decoders) already state R-4 and R-9.

### 2.2 policy_shape honesty contract

`crates/md-codec/src/policy_shape.rs` module doc, "# The honesty contract"
paragraph (lines 16-25). After line 25 (ending "invariant exists to
prevent.") insert a `//!` blank line and then this paragraph:

```
//! A disjunction (`andor`, `or_b`, `or_c`, `or_d`, `or_i`) inside a branch
//! that the top-level split leaves whole also makes `complete` false: listing
//! it would need a cross product of branches.
```

### 2.3 New error variant

`crates/md-codec/src/error.rs`, inserted immediately before the
`TlvOrderingViolation` variant (its doc comment is line 116):

```rust
    /// An unknown TLV entry whose tag is outside the reserved range 0x04..=0x1f.
    #[error("unknown TLV tag 0x{tag:02x} is outside the reserved range 0x04..=0x1f")]
    TlvUnknownTagOutOfRange {
        /// The offending tag.
        tag: u8,
    },
```

Display examples: tag 0x21 → `unknown TLV tag 0x21 is outside the reserved range 0x04..=0x1f`;
tag 0x01 → `unknown TLV tag 0x01 is outside the reserved range 0x04..=0x1f`.

### 2.4 Behaviour per ruling, with the exact production change

| ruling | behaviour after FIX-B | site (unmodified line) |
|---|---|---|
| R-2 | `older(n)` under `thresh` (`Body::Variable`) is truncation-checked: returns the existing `Error::RelativeTimelockTruncated` | `validate.rs` `walk_older`, line 238: add an arm `Body::Variable { children, .. } => { for c in children { walk_older(c, consensus_bits)?; } }` before the `Body::Tr` arm |
| R-4 | any origin component, or any use-site multipath alternative, whose value is > 2^29-1 is refused with the existing `Error::VarintOverflow { value }` by every `to_miniscript` entry (`to_miniscript_descriptor`, `to_miniscript_descriptor_multipath`, `derive_address` and their `_with_network` forms) | `to_miniscript.rs` `assemble_origin_and_xkey`, line 172, immediately before `let path = origin_path_to_derivation(&e.origin_path);` (line 175); see 2.5 |
| R-5 | `collect` returns `false` on `AndOr`, `OrB`, `OrC`, `OrD`, `OrI` inside a branch, so the shape is `complete == false` and `branches` is empty. Top-level splitting is unchanged | `policy_shape.rs` `collect`, line 637: remove the five tags from the `Tag::AndB \| Tag::AndV \| ...` arm and add before it `// A disjunction inside a branch would need a cross product of` / `// branches; the summary refuses it rather than show a wrong one.` / `Tag::AndOr \| Tag::OrB \| Tag::OrC \| Tag::OrD \| Tag::OrI => false,` |
| R-7 | every TLV entry idx (in `use_site_path_overrides`, `fingerprints`, `pubkeys`, `origin_path_overrides`, in that order) must be `< d.n`; the first that is not returns the existing `Error::PlaceholderIndexOutOfRange { idx, n: d.n }`; no panic | `canonicalize.rs` `canonicalize_placeholder_indices`, immediately after `check_placeholder_bounds(&d.tree, d.n)?;` (line 165) |
| R-8 | `TlvSection::write` checks `unknown` in order: a tag outside `0x04..=0x1f` returns `TlvUnknownTagOutOfRange { tag }`; a tag equal to an earlier unknown entry's tag returns the existing `TlvOrderingViolation { prev: tag, current: tag }`. Both before anything is written | `tlv.rs` `write`, immediately after the `let Self { .. } = self;` destructure (line 94), before `// Collect entries, sort by tag.` |
| R-9 | none (row inverted; decoders stay lenient, BIP :752) | none |
| R-16 | a non-ASCII char never aliases a codex32 symbol: it reaches the existing invalid-character refusal, same text as an ASCII non-alphabet char | `codex32.rs` `char_to_symbol` (line 72): first statement `if !c.is_ascii() { return None; }`. `chunk.rs` `parse_chunk_symbols`, line 525: after the whitespace/`-` skip, `if !c.is_ascii() { return Err(Error::Codex32DecodeError(format!("chunk {chunk_index}: character {c:?} not in codex32 alphabet"))); }` |

R-8 loop, exact:

```rust
        for (i, (tag, _, _)) in unknown.iter().enumerate() {
            if !(0x04..=0x1f).contains(tag) {
                return Err(Error::TlvUnknownTagOutOfRange { tag: *tag });
            }
            if unknown[..i].iter().any(|(t, _, _)| t == tag) {
                return Err(Error::TlvOrderingViolation { prev: *tag, current: *tag });
            }
        }
```

Add one line to `write`'s `# Errors` doc list:
`/// - [`Error::TlvUnknownTagOutOfRange`] if an unknown entry's tag is outside 0x04..=0x1f; [`Error::TlvOrderingViolation`] if two unknown entries share a tag.`

### 2.5 R-4 guard, exact

All alternatives are checked, including on the single-path entries; the first offending value is reported (key order, then origin components, then alternatives).

In `assemble_origin_and_xkey`, before `let path = ...`:

```rust
    // Every origin component and every use-site alternative must fit the md1
    // varint (BIP: values above 2^29 - 1 are rejected with VarintOverflow);
    // a larger in-memory value is refused, never mapped to another child.
    let origin_values = e.origin_path.components.iter().map(|c| c.value);
    let alt_values = e.use_site_path.multipath.iter().flatten().map(|a| a.value);
    if let Some(value) = origin_values
        .chain(alt_values)
        .find(|&v| v > MAX_VARINT_VALUE)
    {
        return Err(Error::VarintOverflow { value });
    }
```

Above the `/// Translate an `OriginPath` into a `bip32::DerivationPath`.` doc
line of `origin_path_to_derivation` (line 349), add:

```rust
/// Largest value an md1 LP4-ext varint carries (BIP: 2^29 - 1).
const MAX_VARINT_VALUE: u32 = (1 << 29) - 1;
```

Then, because the guard makes every value fit, replace the silent fallbacks
with direct construction:
- `origin_path_to_derivation`: `ChildNumber::from_hardened_idx(c.value).unwrap_or(ChildNumber::Hardened { index: c.value })`
  → `ChildNumber::Hardened { index: c.value }`, and
  `ChildNumber::from_normal_idx(c.value).unwrap_or(ChildNumber::Normal { index: c.value })`
  → `ChildNumber::Normal { index: c.value }`.
- `build_descriptor_multi_public_key`, the multipath alt map (line 258):
  `ChildNumber::from_hardened_idx(a.value).unwrap_or(ChildNumber::Hardened { index: a.value })`
  → `ChildNumber::Hardened { index: a.value }`.

The grade's hint "origin_path_to_derivation becomes fallible" is met by the
single guard instead: both callers (`assemble_origin_and_xkey` at lines 221
and 251) pass through it first, so behaviour is identical and the signature
stays.

## 3. RED rows

All in `crates/md-codec/tests/bughunt.rs` (committed with this doc) except
the last, in `crates/md-codec/tests/policy_shape.rs`. Run on unmodified
`b2bfdd63` after the architect fold (controller re-run of the §5 filter,
`/scratch/cache/bughunt/md/fixb-red2.log`: 14 run, 1 passed (the inverted
#9 row), 13 failed). Expected GREEN: all pass.

| row | what it asserts (GREEN) | observed on unmodified code |
|---|---|---|
| `bughunt_md_02_thresh_older_truncation_checked` | `older(210000)` inside `thresh` → `Err(RelativeTimelockTruncated{..})` | FAIL: `older(210000) inside thresh must be RelativeTimelockTruncated, got Ok(())` |
| `bughunt_md_04b_codec_to_miniscript_alt_2_31_varint_overflow` | alt 2147483648: every entry → `Err(VarintOverflow { value: 2147483648 })` | FAIL: left `Ok("wpkh(xpub6BemYiVNp19Zz9Bw6kmmfXR2LEFukA1hnhSZrXgE2AUJvNLW8a87gg72bQLi4RfGHcKcR4ojrEFgFJgNCXcjVYSH75YmvhTZ7qh9FCrxv3a/2147483648/*)#tsc5uuuc")`, right `Err(VarintOverflow { value: 2147483648 })` |
| `bughunt_md_04c_codec_to_miniscript_origin_2_31_varint_overflow` | origin component 2147483648: every entry → `Err(VarintOverflow { value: 2147483648 })` | FAIL: left `Ok("wpkh(xpub69mdgvyMbhUa8NZU4mK1MPegY1Ly3RuFioKAUfoZME7QVTPh6HcmtzXRqq4sKVFoLutTmzr2bC3cZqC4LArSS8LM2RirENAMJ7MhCEpkcti/0/*)#8yz96sz2")` |
| `bughunt_md_04f_codec_unselected_alt_above_2_29_varint_overflow` | origin 84'/0'/0', alts `[0, 536870912]`: every entry → `Err(VarintOverflow { value: 536870912 })` (chain 0 selects alt 0, so the overflowing alt is unselected); control alts `[0, 536870911]`: `to_miniscript_descriptor(&d, 0)`, `to_miniscript_descriptor_multipath` and `derive_address(0, 0, Bitcoin)` all `Ok` | FAIL: chain 0 returns `Ok` (no guard) |
| `bughunt_md_05_nested_or_under_and_marks_incomplete` | `complete == false`, no branches | FAIL: `complete: true`, one branch, slots [0,1], lock OlderBlocks 10 |
| `bughunt_md_07_canonicalize_out_of_range_tlv_index_no_panic` | `Err(PlaceholderIndexOutOfRange { idx: 5, n: 2 })` | FAIL: panic at `canonicalize.rs:137:16` `index out of bounds: the len is 2 but the index is 5` |
| `bughunt_md_08_unknown_tlv_tag_above_31_not_silently_truncated` | `Err` text `unknown TLV tag 0x21 is outside the reserved range 0x04..=0x1f`, nothing written | FAIL: left `Ok(())` |
| `bughunt_md_08b_unknown_tlv_tag_duplicating_known_tag_not_written` | `Err` text `unknown TLV tag 0x01 is outside the reserved range 0x04..=0x1f` | FAIL: left `Ok(())` |
| `bughunt_md_08c_duplicate_unknown_tlv_tag_refused` (new) | `Err(TlvOrderingViolation { prev: 5, current: 5 })` | FAIL: left `Ok(())` |
| `bughunt_md_09_varint_noncanonical_extension_accepted_per_bip` (inverted) | `read_varint` → `Ok(5)` | PASS (as required) |
| `bughunt_md_16_single_string_non_ascii_alias_refused` | `decode_md1_string("md1Źq9dsssqrqq20vuhf9agdphs")` → `codex32 decode error: character 'Ź' not in codex32 alphabet`; control `'b'` → same wording | FAIL: left `None` (decoded the card) |
| `bughunt_md_16b_chunk_non_ascii_alias_refused` | `chunk::reassemble` with an aliased chunk 0 → `codex32 decode error: character 'Ŧ' not in codex32 alphabet` (no chunk prefix); control `'b'` same wording | FAIL: left `None` for `md1Ŧwyleqqpqptvyyqqcqqe7vaxqflrlrvl` |
| `bughunt_md_16c_correct_chunks_non_ascii_alias_refused` (new) | `chunk::correct_chunks` → `codex32 decode error: chunk 0: character 'Ŧ' not in codex32 alphabet`; control `'b'` → `codex32 decode error: chunk 0: character 'b' not in codex32 alphabet` | FAIL: left `None` |
| `mixed_branch_with_or_under_and_v_marks_incomplete` (policy_shape.rs) | `wsh(and_v(v:older(10), or_d(multi(2,@0,@1,@2), c:pk_k(@3))))` → `!complete`, `branches.is_empty()` | FAIL: panic `an or under and_v cannot be listed as branches` |

CLI consequence of R-16 (no CLI row; FIX-B adds no CLI test). On unmodified
`b2bfdd63`, `md decode md1Źq9dsssqrqq20vuhf9agdphs` prints the card
`wpkh(@0/<0;1>/*)` and exits 0, while `md decode md1bq9dsssqrqq20vuhf9agdphs`
prints `md: codec error: codex32 decode error: character 'b' not in codex32 alphabet`
and exits 1. After FIX-B the alias prints
`md: codec error: codex32 decode error: character 'Ź' not in codex32 alphabet`
and exits 1.

## 4. Tests that change

All committed with this doc; the builder edits no tests except adding the §5 ignore attributes.

- `crates/md-codec/tests/bughunt.rs`:
  - row 05 renamed (R-5) to `bughunt_md_05_nested_or_under_and_marks_incomplete`;
  - row 09 inverted and renamed (R-9) to
    `bughunt_md_09_varint_noncanonical_extension_accepted_per_bip`,
    `assert_eq!(v, Ok(5))`;
  - row 07 tightened to the exact `PlaceholderIndexOutOfRange { idx: 5, n: 2 }`;
  - rows 08 and 08b tightened to the exact R-8 Display texts;
  - row 16 tightened to the exact text plus the `'b'` control;
  - row 16b rewritten for `reassemble` (no chunk prefix);
  - rows 04b, 04c, 04f, 08c, 16c added (04b/04c cover the grade's
    "codec `to_descriptor`" row: no function of that name exists, so they
    run `to_miniscript_descriptor`, `to_miniscript_descriptor_multipath` and
    `derive_address`).
- `crates/md-codec/tests/policy_shape.rs`:
  - `mixed_branch_with_extra_key_beside_a_multi_reports_zero_threshold` is
    replaced by `mixed_branch_with_or_under_and_v_marks_incomplete` (same
    tree; R-5 makes the old assertion wrong);
  - `mixed_branch_without_a_lock_still_reports_zero_threshold` keeps its
    assertions and now carries the guard-mutation note: replacing
    `nkeys as usize == br.slots.len()` (`policy_shape.rs` lines 462 and 476)
    with `true` makes it FAIL (checked on the scratch fix).

- `crates/md-cli/tests/bughunt.rs`, `compose_bughunt.rs`, `input_bughunt.rs`: the ignore attribute above. (§5 step 1; FIX-A removes or replaces them.)

No other test changes. A scratch application of exactly section 2 passed
every md-codec test, including the
`proptest_to_miniscript.proptest-regressions` seed `v = 2147483648`.

## 5. Gate

Run once each, output to a file, then grep:

0. `cargo fmt --all` (§2's inline Rust texts are not rustfmt-formatted; this reflows the R-8 `return Err(...)` and the R-16 `chunk.rs` statement without changing them).
1. `cargo nextest run --locked --workspace --all-targets --all-features --no-fail-fast`
2. `cargo test --locked --workspace --doc --all-features`
3. `cargo fmt --all --check`
4. `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`
5. The FIX-B rows by exact name; the matched count must be **14** (13
   bughunt rows + 1 policy_shape row; `cargo nextest list` on `b2bfdd63`
   matched 13; 04f is the 14th), all passed:

```
cargo nextest run --locked --workspace -E 'test(/(^|::)bughunt_md_(02_thresh_older_truncation_checked|04b_codec_to_miniscript_alt_2_31_varint_overflow|04c_codec_to_miniscript_origin_2_31_varint_overflow|04f_codec_unselected_alt_above_2_29_varint_overflow|05_nested_or_under_and_marks_incomplete|07_canonicalize_out_of_range_tlv_index_no_panic|08_unknown_tlv_tag_above_31_not_silently_truncated|08b_unknown_tlv_tag_duplicating_known_tag_not_written|08c_duplicate_unknown_tlv_tag_refused|09_varint_noncanonical_extension_accepted_per_bip|16_single_string_non_ascii_alias_refused|16b_chunk_non_ascii_alias_refused|16c_correct_chunks_non_ascii_alias_refused)$/) | test(/(^|::)mixed_branch_with_or_under_and_v_marks_incomplete$/)'
```

Expected workspace result for step 1: every test passes. FIX-B adds `#[ignore = "owed to FIX-A (md bug hunt row NN)"]` (NN = the row number), on the line after `#[test]`, to the nine md-cli rows its base already carries and that FIX-A owns: `crates/md-cli/tests/bughunt.rs` rows 01, 03, 04, 10, 11, 12, 14, `crates/md-cli/src/seat/compose_bughunt.rs` row 06 and `crates/md-cli/src/seat/input_bughunt.rs` row 13. Step 1 then reports 1591 passed, 13 skipped. FIX-A removes the ignore from rows 01, 03, 04, 10, 11, 12 and 14 as its RED step and keeps 06 and 13 ignored. Any failure is a FIX-B FAIL. Then the mutation gate per the coordinator's rules.
