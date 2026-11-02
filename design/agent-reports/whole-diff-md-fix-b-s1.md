**⚠ 1 Important: R-5 newly refuses `l:`/`u:` (or_i with a `0` child) and `and_n` inside a branch, which is the policy compiler's standard output.**

# Whole-diff review: md bug hunt FIX-B, slice 1 (design and threat)

Diff `b2bfdd63..0141ea57`, branch `fix/bughunt-b`. Lens: no fix changes a
correct descriptor, address, key or card; every new refusal fails closed with
the FIX doc's text; nothing valid is newly refused. Probes ran in a detached
scratch worktree at 0141ea57 and a base build at b2bfdd63, both under
`/scratch/cache/bughunt/md/whole-diff-s1/` (binaries
`target-{base,head}/debug/md`). This is the first whole-diff round, so no
finding comes from an earlier fix commit.

## Per-fix verdicts

| fix | verdict | evidence |
|---|---|---|
| R-2 | PASS | `validate.rs` `walk_older` mask `0xFFFF \| (1<<22)` recurses Children, Variable and Tr. A valid time-based `older` passes the mask (read). The gate's row 02 passes. |
| R-4 | PASS | CLI: path component 536870911 gives the same address on base and head (`tb1q2n50pa7qye2wm8jwthn9r3vvgld3k6dkdqudap`; origin case `tb1q6rz28mcfaxtmd6v789l9rrlrusdprr9pqcpvkl`). 536870912 is refused on head with exit 1 and `md: codec error: varint value 536870912 exceeds single-extension range (max 2^29 - 1)`. The check sits before any `ChildNumber` construction, so the dropped `from_*_idx(..).unwrap_or` cannot map a value to another child. |
| R-5 | **FAIL (I-1)** | It refuses the graded nested disjunction (`wsh(and_v(v:pk(@0),or_d(pk(@1),older(10))))`: base exit 0, head exit 2). It also refuses two valid non-disjunctions; see I-1. Top-level controls `or_d(pk,and_v(v:pkh,older))` and `andor(pk,older,pk)` are unchanged (exit 0 on both). |
| R-7 | PASS | `canonicalize.rs` 159-188: one chain over the four TLV index lists, after `check_placeholder_bounds` and before any indexing, returns `PlaceholderIndexOutOfRange{idx,n:d.n}`. No valid card can carry an index >= n, so nothing valid is refused. |
| R-8 | PASS | Encoder pre-checks: unknown tag in 0x04-0x1f, strictly increasing, no duplicate of a known tag. These are the same invariants the decoder enforces, so no round trip a decoder accepted is lost. The BIP diff adds only the `TlvUnknownTagOutOfRange { tag }` row, which matches. |
| R-16 | PASS | `md decode 'md1Źq9dsssqrqq20vuhf9agdphs'`: base decodes it (exit 0, `wpkh(@0/<0;1>/*)`). Head exits 1 with `md: codec error: codex32 decode error: character 'Ź' not in codex32 alphabet`, exactly the FIX doc line 178 text. ASCII input is unchanged because the guard is `!c.is_ascii()` only. |

None of the 9 FIX-A `#[ignore]` rows hides a FIX-B row (confirmed).

## Important

### I-1: R-5 refuses `or_i`/`andor` nodes with a `0` alternative (`l:`, `u:`, `and_n`), which are not disjunctions

**Where:** `crates/md-codec/src/policy_shape.rs:646`, in `collect`:
`Tag::AndOr | Tag::OrB | Tag::OrC | Tag::OrD | Tag::OrI => false,`.

**What goes wrong:** md has no tag for the `l:`/`u:` wrappers or for
`and_n`. They are stored as their expansions:
- `l:X` = `or_i(0,X)`
- `u:X` = `or_i(X,0)`
- `and_n(X,Y)` = `andor(X,Y,0)`

The new arm treats all three as disjunctions. A `0` alternative can never be
satisfied, so each one is really a single path. The result is that
`policy_shape` reports `complete == false` and `skeleton()` refuses. This
happens even though base computed a correct key that matches the tree.

The condition is not unusual. `thresh(2,pk(A),s:pk(B),sln:older(N))` is what
the policy compiler normally emits for "2 of {A, B, timelock}". Observed:

```
md encode --path "m/48'/1'/0'/2'" "wsh(thresh(2,pk(@0/<0;1>/*),s:pk(@1/<0;1>/*),sln:older(144)))"
  -> md1ypfds3cf2qqvzdq3zsu2nnxyjwqqqqqjq6wtjp0p4p6y4d
md shape-key md1ypfds3cf2qqvzdq3zsu2nnxyjwqqqqqjq6wtjp0p4p6y4d
  base: exit 0  wsh(thresh(2,pk(@0/<0;1>/*),s:pk(@1/<0;1>/*),s:or_i(0,n:older(older-blocks#1))))[[]][]NotTaproot
  head: exit 2  md: shape-key: the policy walk met a node it does not understand; no key can be computed from a partial decomposition

md encode --path "m/48'/1'/0'/2'" "wsh(and_v(v:pk(@0/<0;1>/*),and_n(pk(@1/<0;1>/*),older(144))))"
  -> md1ypfds3cf2qqvzfnfg49tsqqqqys3q2rtwl0d8kt3g0
md shape-key md1ypfds3cf2qqvzfnfg49tsqqqqys3q2rtwl0d8kt3g0
  base: exit 0  wsh(and_v(v:pk(@0/<0;1>/*),andor(pk(@1/<0;1>/*),older(older-blocks#1),0)))[[]][]NotTaproot
  head: exit 2  (same refusal text)
```

The same refusal reaches `md compose`/`descriptor` (the verdict notice
prints "no verdict") and `descriptor_route::skeleton_key_of_text`. No
coordinator table row (`coordinator/table.rs`) contains `or_i(0` or an
`andor(..,0)`, so no existing registry verdict is lost.

**Likelihood test:** it fails closed, but it needs only one condition, a
canonical compiler output. That keeps it Important.

**FIX-doc gap:** the gap is in FIX doc line 83 and the grade's R-5 ruling
(`grade-bughunt-md.md`:48-53). They list the five tags without deciding
whether a node with a `0` alternative counts as a disjunction. That is an
"implementer must choose" gap.

**Fix text** (insert in `collect`, immediately before the new
`Tag::AndOr | Tag::OrB | ... => false,` arm and its comment):

```rust
        // `l:X` = or_i(0,X), `u:X` = or_i(X,0) and `and_n(X,Y)` =
        // andor(X,Y,0): the `0` alternative is never satisfiable, so these
        // are not disjunctions; walk them like any conjunction.
        Tag::OrI | Tag::AndOr
            if matches!(&n.body, Body::Children(c)
                if (n.tag == Tag::OrI && c.iter().any(|x| x.tag == Tag::False))
                    || (n.tag == Tag::AndOr && c.get(2).is_some_and(|x| x.tag == Tag::False))) =>
        {
            let Body::Children(children) = &n.body else {
                return false;
            };
            children.iter().all(|c| collect(c, br, keys))
        }
```

FIX doc line 83: add "except an `or_i` with a `0` child (`l:`/`u:`) and an
`andor` whose third child is `0` (`and_n`), which `collect` walks as a
conjunction".

**RED row** (append to `crates/md-codec/tests/policy_shape.rs`, add
`use md_codec::tree::{Body, Node};` to its imports):

```rust
/// `sln:older(N)` = `s:or_i(0,n:older(N))` and `and_n(X,Y)` =
/// `andor(X,Y,0)`: one alternative is `0`, so neither is a disjunction and
/// the branch stays complete (whole-diff s1 I-1).
#[test]
fn zero_alternative_or_i_and_andor_stay_complete() {
    let sln = wrap(
        Tag::Swap,
        node2(
            Tag::OrI,
            Node { tag: Tag::False, body: Body::Empty },
            wrap(Tag::NonZero, timelock(Tag::Older, 144)),
        ),
    );
    let thresh = common::thresh_node(
        2,
        vec![keyarg(Tag::PkK, 0), wrap(Tag::Swap, keyarg(Tag::PkK, 1)), sln],
    );
    let shape = policy_shape(&common::descriptor_of(wrap(Tag::Wsh, thresh), 2));
    assert!(shape.complete, "sln:older inside thresh is not a disjunction");
    assert_eq!(shape.branches.len(), 1);
    assert_eq!(shape.branches[0].slots, vec![0, 1]);
    let and_n = node3(
        Tag::AndOr,
        wrap(Tag::Check, keyarg(Tag::PkK, 1)),
        timelock(Tag::Older, 144),
        Node { tag: Tag::False, body: Body::Empty },
    );
    let tree = wrap(
        Tag::Wsh,
        node2(Tag::AndV, wrap(Tag::Verify, wrap(Tag::Check, keyarg(Tag::PkK, 0))), and_n),
    );
    let shape = policy_shape(&common::descriptor_of(tree, 2));
    assert!(shape.complete, "and_n inside and_v is not a disjunction");
    assert_eq!(shape.branches.len(), 1);
    assert_eq!(shape.branches[0].slots, vec![0, 1]);
}
```

Run as printed:
`cargo nextest run --locked -p md-codec -E 'test(/zero_alternative_or_i_and_andor_stay_complete$/)'`
- On 0141ea57 (RED): `FAIL ... panicked at crates/md-codec/tests/policy_shape.rs:621:5: sln:older inside thresh is not a disjunction`, `1 test run: 0 passed, 1 failed`.
- With the fix text (GREEN), `cargo nextest run --locked --workspace`:
  - `1554 tests run: 1554 passed, 13 skipped`.
  - The 14 FIX-B rows still pass, including `bughunt_md_05_nested_or_under_and_marks_incomplete`, as do `mixed_branch_with_or_under_and_v_marks_incomplete` and the new row.
  - The rebuilt `md shape-key` gives both cards above exit 0 with exactly base's keys.
  - The graded case (`md1ypfds3cf2qqvzfnfgc9tsqqqqq2ptmzuguvm83ls`, `and_v(v:pk,or_d(pk,older(10)))`) is still exit 2.

`cargo fmt --check` passes on the production fix text. The RED row as printed needs `cargo fmt --all`, which only reflows it. `cargo clippy --locked --all-targets -p md-codec -- -D warnings` is clean. The count differs from the settled 1591 because of the feature set of this
plain `--workspace` run. Nothing failed.

## Minor

- M-1 `crates/md-cli` shape-key refusal text (and `SkeletonError::IncompleteShape`): R-5 now sends a *known* nested disjunction to "the policy walk met a node it does not understand". That misdescribes why the key is refused. It fails closed. The two conditions are a nested disjunction plus a holder reading the text as a parser defect. Suggest "a branch holds a nested or/andor; no single key describes it".

## Nit

- N-1 `to_miniscript.rs:175-185`: the R-4 check runs per expanded key in `assemble_origin_and_xkey`. An unused baseline origin above 2^29-1, overridden for every placeholder, is therefore not refused by `to_miniscript`. This is from reading only, not run. It is library-only (encode refuses it; the CLI cannot reach it) and produces no wrong result.

C / I / M / N: 0 / 1 / 1 / 1
