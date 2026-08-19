# Execution review — `285b9fc9` "render `v:` as part of the wrapper chain, not its own arm"

Reviewer: independent adversarial exec review (opus), 2026-08-18.
Scope: **one question only** — does `285b9fc9` introduce a defect, and is `v:`
now rendered correctly in every wrapper position it can occupy? No crate-wide
audit, no refactor proposals.

Method: read-only on source. All mutation work ran in a throwaway `git worktree`
(`git worktree remove --force` afterwards; `git status --short` on the repo is
empty and HEAD is `285b9fc9`). Wide-corpus work ran through the *built*
`md` binaries at `285b9fc9` and at its parent `e2288ddf`, so old-vs-new renderer
output is compared by execution, not by reading.

---

## Verdict

**The diff is sound. No Critical and no Important defect in the renderer.** I
built the parent commit's binary alongside the fixed one and diffed rendered
output over **485 distinct `v`-bearing wrapper shapes that `md encode` accepts**
(exhaustive over wrapper-chain prefixes of length 1–5 drawn from `csadjnv`,
crossed with 14 base fragments and 6 containers, in both `wsh` and `tr`
contexts). **439 of the 485 changed. Every one of the 439 old outputs is
unparseable** (`separator ':' occurred multiple times`) and **every one of the
485 new outputs re-encodes to a byte-identical md1 wire** — which is the
decode → render → parse → re-encode identity, not merely "it parses". The 46
unchanged shapes are exactly the `v:`-on-a-non-wrapper cases the commit message
claims are untouched. Letter order (`vdv:`, `snj:`) is pinned and correct.
Error paths are strictly better than before (no partial output). I independently
re-ran the gates: **782 passed, 0 failed, 2 ignored**; clippy `-D warnings`
clean — matching the commit message.

The one finding worth a gate is not in the renderer, it is in the test the
commit adds to justify itself: **the property test's clause 2 (the "re-parse"
check) is unreachable-as-a-failure**, mutation-proven. The commit message's
central claim — that clause 2 makes emitting an unreadable string "impossible to
pass, *whatever the shape* — including shapes nobody has thought of yet" — is
false. The test is still an 8-entry hand-picked corpus, structurally the same
kind of artifact whose blindness the commit message correctly diagnoses.

---

## Findings

### Important

**I1 — The property test's clause 2 can never fail; the commit's stated reason
for choosing a property test over a 16th hand-written case is unsound.**

`crates/md-cli/src/format/text.rs:236` runs `parse_template(&rendered, …)` only
*after* `crates/md-cli/src/format/text.rs:230` has asserted `rendered == t`, and
`t` was already parsed successfully at `text.rs:224`. So whenever control
reaches line 236, the argument is a string known to parse; when the renderer is
wrong, line 230 panics first and line 236 never executes.

Failure scenario, executed: with the old standalone `Verify` arm restored, the
renderer emits `wsh(and_v(v:j:pk(@0/<0;1>/*),pk(@1/<0;1>/*)))`. The panic comes
from `text.rs:230:13` (`assertion left == right failed: renderer did not
reproduce its input canonically`), never from `text.rs:234:17`. Deleting clause
2 entirely (mutation G) produces a *byte-identical* failure — it is not
load-bearing. Deleting clause 1 instead (mutation H) makes clause 2 fire
(`renderer emitted a template that does NOT re-parse`), proving the clause is
live code that is simply gated to death by the assertion above it.

Consequence: the coverage of this test is exactly its `CASES` list
(`text.rs:207`) and nothing more. A shape nobody thought of is *not* covered,
contrary to `text.rs:196-201` and the commit message. `file:line` —
`crates/md-cli/src/format/text.rs:230` and `:236`.

Not a defect in the fix. It is a false claim about a safety net, in a repo where
this exact class ("a corpus assembled shape-by-shape only covers the shapes
somebody thought of") is what let the original bug survive at two sites. The
remedy is small — e.g. drop the `assert_eq!` from the loop and check
`render(parse(render(parse(t)))) == render(parse(t))` plus re-parse, or add a
second CASES list of *non-canonical* spellings whose render legitimately differs
from the input — but prescribing it is not this review's job.

### Minor

**M1 — Stale comment inside the changed function still says "six wrapper tags"
and omits `Verify`.** `crates/md-codec/src/render.rs:361-363` reads
`// The single dispatch arm at render_node guarantees node.tag is one of / //
the six wrapper tags (Check/Swap/Alt/DupIf/NonZero/ZeroNotEqual), so the / //
first iteration of the loop below always assigns a non-None letter.` There are
now **seven**, and the list is missing the tag this commit added. The doc
comment at `render.rs:332-334` and the `debug_assert!` at `render.rs:367-380`
were both updated; this one was not. Harmless today, but it is the comment a
future reader consults to learn the invariant, and it now under-states it.

**M2 — The multi-letter `pk`-shorthand branch is exercised by no test.**
`crates/md-codec/src/render.rs:412` (`if prefix.ends_with('c') && matches!(…)`)
with a non-empty `prefix_no_c` is the exact path the commit message invokes to
argue "the `v:` directly-on-a-key case is unchanged: `Verify(Check(PkK))`
collapses to prefix `vc` … so the existing pk-shorthand path still emits
`v:pk(K)`". **The claim is true** — I executed it directly at `render_node`
level: `Verify(Check(PkK))` → `v:pk(@0/<0;1>/*)` and `Verify(Check(PkH))` →
`v:pkh(@0/<0;1>/*)`. **But nothing pins it**: weakening the condition to
`prefix == "c"` (mutation D) leaves all 782 tests green.

Mitigating, and why this is Minor not Important: the branch is unreachable from
any v0.30 encoder output. The walker normalizes `Terminal::Check(PkK|PkH)` to a
bare `Tag::PkK`/`Tag::PkH` at `crates/md-cli/src/parse/template.rs:993-1013`, so
`Check` only ever wraps non-key children. Confirmed by execution: `md encode`
maps `vc:pk_k(@0/…)`, `vnc:pk_k(@0/…)` and `vjc:pk_k(@0/…)` onto the *same
wires* as `v:pk`, `vn:pk`, `vj:pk` respectively. The chains that genuinely end
in `c` (`vjnjc:`, `vjjjc:` over `expr_raw_pkh`) land on `RawPkH`, not `PkK/PkH`,
and take the general path. So this is pre-existing legacy/foreign-wire defensive
code, correctly described by the doc comment at `render.rs:342-349`, that the
diff extends a claim onto without adding a pin.

### Nit

**N1 — 5 of the 8 property cases are byte-exact duplicates of existing
hand-written tests.** `text.rs:207` CASES entries duplicate `text.rs:137`
(`snj:` thresh), `:146` (`and_b`+`s:`), `:154` (`or_b`+`s:`), `:57` (`tr` `v:pk`
inheritance) and `:67` (`or_d` recovery). Not vacuous — those five hand-written
tests still go red under mutations B and E — merely redundant. The 3 genuinely
new cases are `vj:pk`, `vn:pk` and the `wsh(and_v(v:pk…))` shorthand guard.

**N2 — Undocumented error-message change.** A structurally-malformed `Verify`
body used to yield `MalformedTree("Verify body must be Children([1])")`; it now
yields `MalformedTree("v: wrapper body must be Children([1])")` (executed, for
0-child, 2-child, and `KeyArg` bodies). Nothing in the tree references the old
string (grep across `*.rs`/`*.md` is empty), and the new path is strictly
better — the old arm could push `"v:"` into `out` before a nested wrapper failed,
the new one leaves `out` empty because letters accumulate in a local `prefix`.
Recorded only because the commit message does not mention it.

### No findings at severity Critical

None. I looked specifically for a shape whose output changed *away* from
correct, and for a chain that re-parses to a different AST; neither exists over
485 shapes.

---

## Mutation results

Baseline before every mutation: `md-codec --test render_template_snapshot`
1 passed; `md-cli --bin md format::text` 17 passed.

| test target | mutation applied | went red? |
| --- | --- | --- |
| property + KAT | **A** (author's, re-run for the panic site): restore the old standalone `Tag::Verify` arm in `render_node` | **RED** — property fails at `text.rs:230:13`, `left: "…v:j:pk(@0/…)…"` vs `right: "…vj:pk(@0/…)…"`. Note *which* line: clause 1, not clause 2 |
| KAT + property + 5 others | **B**: `render.rs:391` `Tag::Verify => Some('v')` → `Some('x')` | **RED** — KAT `[pathological_or_i]` drifts `v:pk`→`x:pk`; property `vj:`→`xj:`; also `roundtrip_tr_and_v_verify_older_inheritance`, `…_or_d_recovery_pattern`, `…_t_or_c_desugars…`, `…_after_absolute_timelock`, `…_sha256/hash160_hash_lock`, `…_thresh…` |
| KAT + property | **C**: `render.rs:396` `prefix.push(c)` → `prefix.insert(0, c)` (reverse letter order) | **RED** — KAT `[wrapper_chain_thresh]` `snj:`→`jns:`; property `vj:`→`jv:`; `roundtrip_wsh_thresh_with_non_key_fragment_child` also red |
| KAT + property + 15 others | **D**: `render.rs:412` `prefix.ends_with('c')` → `prefix == "c"` | **GREEN — all 782 pass.** See finding M2 |
| KAT + 8 md-cli tests | **E**: `render.rs:215` remove `Tag::Verify` from the dispatch arm (falls to the `other =>` catch-all → `MalformedTree`) | **RED** — KAT panics at `render_template_snapshot.rs:141:33`; 8 of 17 `format::text` tests red |
| KAT | **F**: `render_template_snapshot.rs:127` expected `"…vj:pk…"` → `"…vn:pk…"` | **RED** — `[verify_over_nonzero] renderer drifted…`. The new KAT entries do assert |
| property | **G**: mutation A **+ delete clause 2** (`text.rs:235-238`) | **RED, identically to A** (`text.rs:230:13`) → clause 2 contributes nothing |
| property | **H**: mutation A **+ delete clause 1** (`text.rs:230-233`) | **RED at `text.rs:234:17`**, `renderer emitted a template that does NOT re-parse` → clause 2 is live but gated to death by clause 1 |

Also run, not a mutation: two temporary probe tests added to
`md-codec/src/render.rs`'s `mod tests` **in the worktree only**, calling
`render_node` on hand-built `Node`s. Output verbatim:

```
PROBE v(PkK)        = v:pk(@0/<0;1>/*)
PROBE v(PkH)        = v:pkh(@0/<0;1>/*)
PROBE v(c(PkK))     = v:pk(@0/<0;1>/*)
PROBE v(c(PkH))     = v:pkh(@0/<0;1>/*)
PROBE c(PkK)        = pk(@0/<0;1>/*)
PROBE n(v(c(PkK)))  = nv:pk(@0/<0;1>/*)
PROBE v(j(c(PkK)))  = vj:pk(@0/<0;1>/*)
PROBE v(v(PkK))     = vv:pk(@0/<0;1>/*)
PROBE d(v(older))   = dv:older(144)
PROBE v(True)       = v:1
PROBE v(RawPkH)     = v:expr_raw_pkh(0000000000000000000000000000000000000000)
PROBE v(c(RawPkH))  = vc:expr_raw_pkh(0000000000000000000000000000000000000000)
PROBE v(2 kids)     = Err(MalformedTree("v: wrapper body must be Children([1])")) / out=""
PROBE v(0 kids)     = Err(MalformedTree("v: wrapper body must be Children([1])")) / out=""
PROBE v(KeyArg)     = Err(MalformedTree("v: wrapper body must be Children([1])")) / out=""
```

(`nv:` and `vv:` are ill-typed miniscript and unreachable from any parse — they
appear only because the probe fabricates the tree; both were equally garbage
before the fix, so no regression.)

---

## Wrapper-chain coverage

Generated exhaustively over the alphabet `csadjnv`, keeping only prefixes
containing at least one `v`, crossed with base fragments (`pk`, `pk_k`, `pkh`,
`pk_h`, `older`, `after`, `1`, `0`, `multi`, `sha256`, `and_b`, `or_i`,
`expr_raw_pkh`, `and_v`) and containers (`wsh(and_v(F,pk))`,
`wsh(and_b(pk,F))`, `wsh(and_v(v:pk,F))`, `wsh(F)`, `tr(K,and_v(F,pk))`,
`tr(K,F)`). `md encode` filtered out the ill-typed ones.

| sweep | candidates | accepted by `md encode` | NEW ≠ OLD | OLD output re-parses | NEW re-encodes to same wire |
| --- | --- | --- | --- | --- | --- |
| prefix length 1–3 | 11 926 | **179** | **133** | **0 of 133** | **179 of 179** |
| prefix length 4–5 | 243 264 | **306** | **306** | (not re-measured) | **306 of 306** |
| **total** | 255 190 | **485** | **439** | — | **485 of 485** |

Every one of the 133 measured OLD outputs was rejected by
`md encode` with `miniscript parse failed: separator ':' occurred multiple
times`, and all 133 contained a doubled-colon chain. **Zero** NEW renders failed
to re-encode to the *same* md1 wire the render came from.

Chains actually exercised (representative, all `SAME` + `REPARSE-SAME-WIRE`):

- `v:` alone above every non-wrapper: `v:pk`, `v:pkh`, `v:older`, `v:after`,
  `v:multi`, `v:multi_a`, `v:sha256`, `v:hash160`, `v:and_b`, `v:or_b`,
  `v:or_i`, `v:andor`, `v:thresh`, `v:1`, `v:0` — **unchanged from the parent
  commit**, as the commit message claims (these are 46 of the 179).
- `v:` above one wrapper: `vj:`, `vn:`, `vc:` (over `expr_raw_pkh`),
  `vd:`(as `vdv:`).
- `v:` under a wrapper: `dv:older(144)`, `sdv:older(144)`.
- `v` in the middle: `vdv:older(144)`, `vndv:`, `vjdv:`.
- multi-letter: `vnj:`, `vjn:`, `vnnj:`, and at length 4–5 `vnnjj:`, `vjnnj:`,
  `vnnnj:`, `vjnjc:`, `vjjjc:`, `vnnjn:`.
- **Letter order confirmed correct** (outermost letter leftmost): mutation C
  reverses it and turns both the pre-existing `snj:` KAT entry and the new `vj:`
  property case red. `vdv:older(144)` — a chain with `v` at both ends — encodes,
  renders identically, and re-encodes to the identical wire, so the order is
  right in the case where reversal would be invisible in a single-`v` chain.
- **Same-tree, not just parses:** verified for all 485 by re-encoding the render
  and comparing the md1 wire byte-for-byte against the wire the render came
  from. A chain that re-parsed to a *different* AST would produce a different
  wire; none did.

The two new KAT wires were verified independently against the binary:

```
$ md decode md1ypfdsssj5qqcynx5fg2sajgf7wv98ku5s
wsh(and_v(vj:pk(@0/<0;1>/*),pk(@1/<0;1>/*)))
$ md decode md1ypfdsssj5qqcynx53g2smkjr65kprh73r
wsh(and_v(vn:pk(@0/<0;1>/*),pk(@1/<0;1>/*)))
$ md encode --group-size 0 --path bip48 'wsh(and_v(vj:pk(@0/<0;1>/*),pk(@1/<0;1>/*)))'
md1ypfdsssj5qqcynx5fg2sajgf7wv98ku5s
$ md encode --group-size 0 --path bip48 'wsh(and_v(vn:pk(@0/<0;1>/*),pk(@1/<0;1>/*)))'
md1ypfdsssj5qqcynx53g2smkjr65kprh73r
```

---

## Angles examined and cleared

1. **Any OTHER rendering change?** Cleared by execution, not inspection: 485
   shapes diffed old-binary vs new-binary. The only changes are the 439
   doubled-colon repairs; all 46 non-wrapper `v:` cases are byte-identical to
   the parent. No shape silently changed. The `v:`-directly-on-a-key claim holds
   (`Verify(Check(PkK))` → `"vc"` → `v:pk(K)`, probed directly).
2. **`Verify` over `PkH`, and bare `Verify(PkK)`.** `Verify(Check(PkH))` →
   `v:pkh(K)` ✅. **Bare `Verify(PkK)`** — which is what the v0.30 walker
   actually emits (`parse/template.rs:993-1013` normalizes `Check(PkK)` away) —
   renders `v:pk(K)` and re-encodes to the same wire ✅. Bare `Verify(PkH)` →
   `v:pkh(K)` ✅. All three are correct and re-parse.
3. **Letter ORDER.** Correct and pinned; see the coverage section. Verified by
   same-tree comparison (re-encoded wire equality), not by parse success alone.
4. **Chain that parses but means something different.** Cleared for all 485 by
   the render → parse → re-encode → wire-equality check. 0 mismatches.
5. **Exhaustiveness.** 255 190 candidates generated, 485 accepted by
   `md encode`'s typechecker, all round-trip. No shape fails.
6. **Vacuous or weakened tests.** One real hit (I1: clause 2 is dead), one
   redundancy (N1). No pre-existing test became unable to fail — mutations B, C,
   E and F each turn the relevant pre-existing tests red.
7. **The `t:` desugar test** (`text.rs:167-174`). Still correct and still
   asserting: it goes red under mutations B and E, and it still asserts a
   canonical form (`and_v(or_c(…),1)`) that differs from its input (`t:or_c(…)`).
   Its `v:pk(@2)` sits directly on a key, so it takes the shorthand path this
   diff leaves untouched — which is why it passes unchanged.

**Gates re-run independently** (not taken from the commit message):
`cargo test --locked --features md-cli/cli-compiler` → **782 passed, 0 failed,
2 ignored**. `cargo clippy --locked --all-targets --features
md-cli/cli-compiler -- -D warnings` → clean.

---

## Open / could not determine

- **The 2 ignored tests.** The suite reports `2 ignored`; the commit message
  says only "782 passed, 0 failed". I did not identify which tests are ignored
  or whether either touches the renderer. Out of the one question's scope, but
  it is an unstated number.
- **Length-4/5 OLD parseability.** For the 306 length-4/5 shapes I measured that
  NEW ≠ OLD and that all 306 NEW renders re-encode to the same wire, but I did
  not separately confirm that all 306 OLD outputs are unparseable (I confirmed
  it for all 133 of the length-1–3 set). Every OLD output I inspected in that
  band carried a doubled colon, but I am not asserting 306/306 from a sample.
- **The deliberately-skipped runtime output contract.** The commit's stated
  reason — that md-codec cannot validate its own `@N` output because placeholder
  templates are not miniscript-parseable — is sound as far as *parsing* goes. I
  note without asserting a conclusion that `crates/md-codec/src/to_miniscript.rs`
  already contains `node_to_miniscript::<Ctx>` (`:483` handles `Tag::Verify`),
  which builds a typed miniscript AST from the `Node` tree given a key list; a
  dummy-key substitution could in principle typecheck the tree without going
  through text. Whether that is worth building, and whether it would catch this
  class (a *rendering* bug, not a tree bug — it would not), I did not evaluate.
  Per the brief this is a comment on the stated reasoning, not a finding.
- I did not evaluate the `n:` before `0` corner documented at `render.rs:350-353`
  beyond observing that `vn:0` and `vdv:0` encode, render and re-encode
  consistently. That corner predates this diff.

---

*Read-only on source. Mutations ran in a temporary worktree at
`285b9fc9`/`e2288ddf`, since removed; `git status --short` on
`/scratch/code/shibboleth/descriptor-mnemonic` is empty and HEAD is `285b9fc9`.*
