**⚠ 3 Important (R-5, R-7, R-8: plants survive the full md-codec suite)**

# Whole-diff review, md FIX-B slice 2 (tests-can-fail)

Reviewer: Sonnet 5.5, medium. Target 0141ea57 (scratch worktree
`/scratch/cache/bughunt/md/whole-diff-s2/w`, plants via `../plant.sh`, each
plant run against the 14 FIX-B rows by the FIX §5 filter; baseline 14/14 pass).
Previous-round fix commit: this is the first review of 0141ea57 (build by the
FIX-B builder; FIX review r1 folded earlier).

## Plants that the rows catch (reason confirmed from the failing row)
- R-2 remove the `Body::Variable` arm: row 02 FAILS.
- R-4 `>` -> `>=`: 04f control FAILS. Threshold `> 2^30`: 04f FAILS. Origin
  check removed: 04c FAILS. Alt check removed: 04b, 04f FAIL. Only first alt
  checked: 04f FAILS. Only non-first alts: 04b FAILS. Reported value
  clamped: 04b, 04c FAIL.
- R-5 whole arm back to `true`: policy_shape row and 05 FAIL.

## Findings

### I-1 R-5: four of the five disjunction tags are untested (plant survives the whole md-codec suite)
`crates/md-codec/src/policy_shape.rs:644-646`. Plant: move one tag out of
the `=> false` arm back into the `AndB | AndV | ...` recursing arm.
Observed (14 FIX-B rows, and again the full `cargo nextest run -p md-codec`,
672 tests): dropping `AndOr`, `OrB`, `OrC` or `OrI` passes 14/14 and 672/672;
only dropping `OrD` fails (rows 05 and `mixed_branch_with_or_under_and_v_marks_incomplete`,
both use `or_d`). So a regression that again merges a nested `andor`/`or_b`/`or_c`/`or_i`
into one branch (the graded wrong-summary defect, a wrong k-of-n shown to the holder)
cannot fail any test, while FIX §2.4 and the new doc paragraph promise all five.
Fix: add this row to `crates/md-codec/tests/bughunt.rs` after row 05 (it
needs no new helper; it uses `desc`, `node`, `key` already there). Run as
printed in the scratch copy: PASS on 0141ea57; each of the five single-tag
drop plants FAILS it (`... under and_v must mark the summary incomplete`),
output observed above.

```rust
/// #5, every disjunction tag R-5 lists (not only `or_d`).
#[test]
fn bughunt_md_05b_every_nested_disjunction_tag_marks_incomplete() {
    for tag in [Tag::AndOr, Tag::OrB, Tag::OrC, Tag::OrD, Tag::OrI] {
        let kids = if tag == Tag::AndOr {
            vec![key(Tag::PkK, 1), key(Tag::PkK, 2), key(Tag::PkK, 3)]
        } else {
            vec![key(Tag::PkK, 1), key(Tag::PkK, 2)]
        };
        let d = desc(
            4,
            node(
                Tag::Wsh,
                Body::Children(vec![node(
                    Tag::AndV,
                    Body::Children(vec![
                        node(Tag::Verify, Body::Children(vec![key(Tag::PkK, 0)])),
                        node(tag, Body::Children(kids)),
                    ]),
                )]),
            ),
            TlvSection::new_empty(),
        );
        let shape = md_codec::policy_shape::policy_shape(&d);
        assert!(
            !shape.complete && shape.branches.is_empty(),
            "{tag:?} under and_v must mark the summary incomplete: {shape:?}"
        );
    }
}
```
Test name to add to the FIX §5 filter and TRACE: `bughunt_md_05b_every_nested_disjunction_tag_marks_incomplete`.

### M-1 R-4 first-offending order is untested
`to_miniscript.rs` guard chain order (origin then alts; FIX §2.5, architect ruling). Plant: `alt_values.chain(origin_values)` passes 14/14. No row has an origin component and an alt both above 2^29-1 with different values, nor two keys. Fails closed, only the reported value differs. Fix: in 04f add a descriptor with origin value 536870913 and alt 536870912 and assert `VarintOverflow{value:536870913}` on every entry.

### I-2 R-7: idx == n and three of the four TLV lists are untested
`canonicalize.rs:185` (`if idx >= d.n`) is one chain over use_site_path_overrides, fingerprints, pubkeys, origin_path_overrides (the single-chain implementation is sound: one check, the stated error, `n: d.n`). Plants run against the 14 rows: `idx > d.n` at line 185 passes 14/14 and also the full suite of 672 (an idx == n entry reaches the permutation code and panics instead of the typed refusal); removing the pubkeys, usp or origin_path_overrides link of the chain also passes 14/14 (pubkeys removal passes the full suite). Only the fingerprints link is covered (row 07 fails when it is removed). Failure: a regression to `>` or a dropped list re-opens the R-7 panic on a hostile card (a refusal that does not refuse), unnoticed.
Fix: add to `crates/md-codec/tests/bughunt.rs` and to the FIX §5 filter and TRACE the row below (full body in `/scratch/cache/bughunt/md/whole-diff-s2/bughunt_new.rs`, `bughunt_md_07b_every_tlv_list_idx_equal_n_refused`: four cases, each list holding one entry with idx 2 on a descriptor with n = 2, under `catch_unwind`, asserting `Some(Err(PlaceholderIndexOutOfRange { idx: 2, n: 2 }))`).
Observed: PASS on 0141ea57; FAIL under `idx > d.n` (`use_site_path_overrides: idx == n`); fails under removal of each of the four chain links.

### I-3 R-8: range boundaries and known tag 0x03 are untested
`tlv.rs:98` (`(0x04..=0x1f).contains(tag)`). Plants: lower bound `0x03..=`, `0x00..=`-style widening, and upper bound `..=0x20` all pass 14/14 and the full suite. Accepting known tag 0x03 (origin_path_overrides) as an unknown entry writes a second record that aliases a known tag: a wrong-card path. Rows 08a-c use only mid-range tags.
Fix: add `bughunt_md_08d_unknown_tlv_tag_range_boundaries` (body in `bughunt_new.rs`): tags 0x00, 0x03, 0x20 give `Err(TlvUnknownTagOutOfRange { tag })` with 0 bits written; tags 0x04 and 0x1f give `Ok(())`.
Observed: PASS on 0141ea57; FAIL under `0x03..=0x1f` and under `0x04..=0x20`.

## Other checks
- policy_shape guards `nkeys as usize == br.slots.len()` (lines 466, 480) planted to `true`: `mixed_branch_without_a_lock_still_reports_zero_threshold` fails (line 480 plant) and `a_duplicate_index_multi_reports_no_threshold_in_either_spelling` fails (both); the expectation changes do not hide these.
- The nine FIX-A ignores: all FIX-B rows live in md-codec; none of the 14 is ignored.
- R-2, R-16 (codex32.rs and chunk.rs, with and without prefix), R-4 (all single-path drops) and R-8 duplicate plants are caught by their rows.

## Fold accounting
First review of 0141ea57; no previous round's fix commit to attribute (0 of the C/I/M/N).
C / I / M / N = 0 / 3 / 1 / 0
