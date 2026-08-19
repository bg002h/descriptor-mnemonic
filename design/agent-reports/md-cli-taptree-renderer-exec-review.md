# Independent adversarial execution review — `cf139508` (`md-cli` taptree renderer)

- **Reviewer:** independent agent, read-only on code (one report file written; no source edited, nothing committed).
- **Subject:** `cf1395088a5c440997c3d8188ec184975468e948`, one file, `crates/md-cli/src/compile.rs` (+199 / −7).
- **Question asked:** does this commit introduce a defect, and is `render_tr_template` correct for **every** taptree that can actually reach it?
- **Method:** all experiments ran in a throwaway `git worktree` at `cf139508` (created, used, `git worktree remove --force`d; main repo verified `git status --porcelain` empty afterwards). Every number below is pasted tool output, not inference.

---

## Verdict

**The diff is sound. Zero Critical and zero Important defects found.** The new renderer was checked against an *independent* recursive oracle over **23,714 distinct proper binary taptree topologies** — every shape with 1 through 11 leaves — and matched byte-for-byte on all of them, plus the two BIP-341 maximum-depth (128) spines, left- and right-heavy. A separate non-regression probe over all 2,056 shapes with 1–9 leaves established the property that actually matters for a wallet tool: of those, **9 were parseable under the old `Descriptor::to_string()` path and the new renderer reproduces all 9 byte-identically (0 divergences); the other 2,047 were malformed before and all 2,047 now re-parse.** So the change strictly repairs and never rewrites. Six independent mutations of the renderer each killed 4–8 tests, so the new tests genuinely assert; both assertions of the tripwire are live; and `compile_strips_descriptor_checksum` is **not** vacuous (it dies under the mutation that reintroduces `to_string()`), though its coverage has narrowed to the keypath-only branch. The four findings below are all Nits: an unbalanced parenthesis in an unreachable error string, a dead fallback branch, a now-stale doc comment, and an untested (because unreachable) error branch. None gates.

---

## Findings

### Critical
**None.**

### Important
**None.**

### Minor
**None.**

### Nit

**N1 — The "unclosed nodes" error message has an unbalanced parenthesis.**
`crates/md-cli/src/compile.rs:182`. Measured:

```
$ python3 -c "s=open('crates/md-cli/src/compile.rs').read().split(chr(10)); line=s[181]; print(repr(line)); print('open parens:', line.count('('), 'close parens:', line.count(')'))"
'            "internal error: taptree left {} node(s) unclosed; refusing to emit a malformed template (partial: tr({internal},{out})",'
open parens: 3 close parens: 2
```

`(partial:` and `tr(` both open; only one `)` closes. Failure scenario: none reachable (see N4) — if it ever printed, the operator would see `... (partial: tr(@4,{pk(@3)` with no closing paren. Cosmetic, in code that cannot execute.

**N2 — The non-`Tr` fallback branch is unreachable dead code.**
`crates/md-cli/src/compile.rs:142-148`. Proven, not assumed:

- `render_tr_template` has exactly one call site — `compile.rs:97` — confirmed by grepping the whole `crates/` tree; the only other hits are the `fn` definition at `:137` and doc/test-name text at `:265`, `:287`, `:292`, `:309`.
- That call site passes the result of `Policy::compile_tr`, whose body returns `Descriptor::new_tr(...)` (`vendor/miniscript/src/policy/concrete.rs:246`), and `new_tr` is `Ok(Descriptor::Tr(Tr::new(key, script)?))` (`vendor/miniscript/src/descriptor/mod.rs:246-248`). It can only ever be the `Tr` variant.

Mutation M9 (deleting the error branch entirely) and the fact that no test exercises the fallback both corroborate. **The behaviour is nevertheless correct if it were reachable** — it is the verbatim pre-diff checksum-stripping code, so nothing was lost. Recording it so a future reader knows the branch is defensive, not load-bearing.

**N3 — `compile_strips_descriptor_checksum`'s doc comment now describes a mechanism that no longer runs on the path it tests.**
`crates/md-cli/src/compile.rs:388-397`. The comment says *"The split_once strip MUST drop it."* On the tap path there is no longer a `split_once` strip — the guarantee now comes from `format!` never emitting a `#` (`:153`, `:187`). The test is **not vacuous** — mutation M7 (`return Ok(format!("tr({internal})"))` → `return Ok(desc.to_string())`) turns it red — but what it now guards is "the keypath-only branch does not fall back to `to_string()`", which is a different claim from the one written above it. Note also that it exercises only the `tap_tree() == None` branch: mutation M8, which appends a bogus `#deadbeef` to the **tree** output, leaves this test green while killing 8 others. The tree path is covered, just by different tests.

**N4 — The unclosed-nodes error branch is asserted by nothing.**
`crates/md-cli/src/compile.rs:180-185`. Mutation M9 removed the entire branch and **all 14 tests stayed green**. That is expected and acceptable — the branch is unreachable by construction (see "Angles examined", angle 2) — but it means the diff's one deliberate departure from upstream PR #953 carries no test of its own. It also cannot be tested through the public surface, since `TapTree`'s `depths_leaves` field is private (`vendor/miniscript/src/descriptor/tr/taptree.rs:30`) and every constructor preserves properness. Flagging as a Nit rather than proposing a fix.

**N5 — The tripwire's exact-string assertion also pins `compile_tr`'s key-extraction and Huffman leaf ordering, not only the `Display` bug.**
`crates/md-cli/src/compile.rs:306-310`. `"tr(@4,{{pk(@3),pk(@2),pk(@1),pk(@0)}})"` encodes three independent upstream behaviours: which key `extract_key` promotes to internal, the `BinaryHeap` tie-break order inside `with_huffman_tree` (`vendor/miniscript/src/policy/concrete.rs:955-978`), and the `fmt_helper` flattening bug (`vendor/miniscript/src/descriptor/tr/taptree.rs:87-113`). A miniscript release that changed leaf ordering *without* fixing #953 would fire the tripwire with the message "check whether the miniscript pin moved past PR #953" — a false alarm.

Two things hold this down to a Nit rather than a Minor: (a) the message says *"check whether"*, which is correctly hedged rather than asserted; (b) `render_tr_template_pins_every_topology_class` (`:264-283`) pins the **same** policy through the **same** compiler, so an ordering change fires both tests and the pair disambiguates — whereas a genuine #953 landing fires only the tripwire. And I measured the one resolution that is actually reachable today: under **miniscript 13.1.0**, which is what a build *without* `--locked` resolves to, all 14 tests including the tripwire still pass (output in "Angles examined", angle 7). So the hazard is hypothetical at the current pin.

---

## Mutation results

Applied one at a time to a pristine copy of `compile.rs` at `cf139508`, then `cargo test --locked --offline -p md-cli --features md-cli/cli-compiler --bin md compile::tests`. Baseline before any mutation: **14 passed, 0 failed.**

| # | Test(s) targeted | Mutation applied | Went red? |
|---|---|---|---|
| M0 | — (baseline) | comment appended to `let mut out = String::new();` | no — 14 passed, 0 failed (control) |
| M1 | renderer | `while let Some(2) = child_counts.last()` → `Some(3)` (subtree never closes) | **YES — 4 failed** (`compile_or_chain_four_keys_tap_reparses`, `compile_thresh_1_of_5_tap_reparses`, `render_tr_template_pins_every_topology_class`, `compile_caterpillar_shapes_still_reparse`) |
| M2 | renderer | deleted `if !child_counts.is_empty() { out.push(','); }` (drop the separating comma) | **YES — 4 failed** (same four) |
| M3 | renderer | closing `out.push('}')` → `out.push('{')` | **YES — 4 failed** (same four) |
| M4 | renderer | descend loop `child_counts.len() < depth` → `<= depth` | **YES — 8 failed** (adds `compile_or_two_keys_tap`, `compile_and_pk_pk_tap_auto_nums`, `compile_thresh_2_of_3_tap_auto_nums`, `compile_or_pk_and_pk_older_tap`) |
| M5 | renderer | removed the parent-increment inside the close loop | **YES — 4 failed** (same four as M1) |
| M6 | renderer | removed the post-leaf `*c += 1` increment | **YES — 4 failed** (same four as M1) |
| M7 | `compile_strips_descriptor_checksum` | keypath-only `Ok(format!("tr({internal})"))` → `Ok(desc.to_string())` (reintroduces `#checksum`) | **YES — 3 failed**, including `compile_strips_descriptor_checksum` (also `compile_pk_tap_keypath_only`, `compile_pk_tap_explicit_nums_extract_still_wins`) → **the checksum test is NOT vacuous** |
| M8 | `compile_strips_descriptor_checksum` | tree output `format!("tr({internal},{out})")` → `...#deadbeef` | checksum test stayed **green**; 8 other tests went red → checksum test does not cover the tree branch (N3) |
| M9 | unclosed-nodes error branch | deleted the whole `if !child_counts.is_empty() { return Err(...) }` block | **NO — 14 passed, 0 failed** → branch is asserted by nothing (N4) |
| M10 | tripwire (assert 1) | expected string `tr(@4,{{pk(@3),pk(@2),pk(@1),pk(@0)}})` → `tr(@0,{{pk(@3),pk(@2),pk(@1),pk(@4)}})` | **YES — 1 failed**, only the tripwire |
| M11 | tripwire (assert 2) | `parse_template(upstream, …).is_err()` → `.is_ok()` | **YES — 1 failed**, only the tripwire |

Every mutation of live renderer logic (M1–M6) was caught. The two survivors, M8 and M9, are exactly the two Nits recorded above.

---

## Angles examined and cleared

**1. Correctness for topologies the tests do not cover — CLEARED, exhaustively.**
Built every proper binary tree shape with *n* leaves via `TapTree::leaf` / `TapTree::combine`, rendered through the real `render_tr_template`, and compared against an **independent** oracle written directly from the recursive shape (`format!("{{{a},{b}}}")`) with no reference to the algorithm under test:

```
n=1: 1 shapes OK      n=7:   132 shapes OK
n=2: 1 shapes OK      n=8:   429 shapes OK
n=3: 2 shapes OK      n=9:  1430 shapes OK
n=4: 5 shapes OK      n=10: 4862 shapes OK
n=5: 14 shapes OK     n=11: 16796 shapes OK
n=6: 42 shapes OK
TOTAL shapes checked: 23714
```

All 23,714 matched byte-for-byte. Separately, all shapes up to 8 leaves were round-tripped through `crate::parse::template::parse_template` and every one parsed. Deep right-spine and deep left-spine at the BIP-341 maximum: `max depth = 128, leaves = 129`, rendered length 1316, brace depth never negative, ends at 0, `maxd == 128` — both directions. `TapTree::combine` itself rejects depth 129 (`vendor/miniscript/src/descriptor/tr/taptree.rs:40-49`), so 128 is genuinely the ceiling and the renderer reaches it.

*Why exhaustive coverage to 11 leaves settles the general case:* the algorithm's only state is `child_counts`, and the invariant is that after each leaf's close loop the next leaf's depth is always `>= child_counts.len()` (the top of the stack is a node at depth `len-1` awaiting its right child at depth `len`), so `child_counts.len() > depth` cannot arise for any pre-order sequence from a proper binary tree — the `while child_counts.len() < depth` descend at `:163` therefore always lands exactly. `Vec<u8>` counters are bounded at 2 by the close loop, so no overflow at any depth; `usize::from(leaf.depth())` widens a `u8` (`taptree.rs:200`), so no truncation.

**2. Can the error branch fire, and is returning an error right there? — CLEARED; unreachable, and it does not turn a working case into a failure.**
It fired **zero times** across 23,714 shapes + both 128-deep spines. Structurally: `TapTree` wraps a private `depths_leaves: Vec<(u8, Arc<Miniscript>)>` (`taptree.rs:30`) with no public writer; the only constructors are `leaf` (one entry at depth 0), `combine` (concatenates two proper trees and increments every depth — `taptree.rs:40-49`), `translate_pk` (length- and depth-preserving), and the crate-private `TapTreeBuilder` used only by the descriptor parser at `tr/mod.rs:376`. A `grep` for `depths_leaves` outside `taptree.rs` returns nothing. All four preserve properness, so an unclosed stack is unconstructible. Returning `Err` is therefore inert; the cost is N1 and N4, both cosmetic. It is also strictly safer than #953's silent assumption: were the invariant ever broken, this refuses rather than emitting a malformed template.

**3. The non-`Tr` fallback — CLEARED as dead code; behaviour would still be correct.** See N2 for the proof chain (single call site → `compile_tr` → `new_tr` → `Descriptor::Tr`). Its checksum-stripping body is the unmodified pre-diff code.

**4. Leaf and key rendering with `Pk = String` — CLEARED.**
- *Leaves cannot contain braces.* Swept a 12-fragment corpus (`multi_a`, `and_v`, `or_d`, `thresh`, `andor`, `or_i`, `j:`, `n:` wrappers, …) through `Miniscript::<String, Tap>::to_string()`; none emitted `{` or `}`. Miniscript fragments use `name(args)` syntax only; brace syntax belongs to the descriptor tree layer, which is exactly what this function now owns.
- *Leaves CAN contain commas, and that is harmless.* Built a balanced (previously broken) 4-leaf tree from comma-bearing leaves and rendered:
  `tr(50929b…03ac0,{{multi_a(2,@0,@1,@2),and_v(v:pk(@3),older(144))},{multi_a(1,@4,@5),thresh(2,pk(@6),s:pk(@7),s:pk(@8))}})` — re-parses via `parse_template`, exact string matched. Commas inside parens do not confuse the parser.
- *Policy keys cannot inject braces or commas.* Every injection attempt was rejected by miniscript's own policy parser before reaching the renderer, e.g. `or(pk(a{b}),pk(@1))` → ``parse: illegal `{` at position 8 (Taproot branches not allowed here)``; `thresh(1,pk({A,B}),…)` → same; `pk(x})` → ``parse: `(` (position 11) closed by `}` (position 13)``. Realistic keys pass through intact: `pk([aabbccdd/48h/0h/0h/2h]xpub6E/<0;1>/*)` and `@0/**` both render correctly.
- *The internal key is not validated here* — passing `unspendable_key = ","` yields `tr(,,multi_a(2,@0,@1,@2))`. **Not a finding against this diff:** it is byte-for-byte the pre-diff behaviour (`Tr`'s own `Display` at `vendor/miniscript/src/descriptor/tr/mod.rs:416-427` writes `write!(f, "tr({},{})", key, s)` with no escaping either), and it is unreachable from the CLI — `validate_unspendable_key_nums_only` (`crates/md-cli/src/main.rs:280-294`) rejects anything but the NUMS literal, and it guards both dispatch sites (`main.rs:331` and `main.rs:404`).

**5. Did a pre-existing test become vacuous? — CLEARED (no), with one narrowing.** `compile_strips_descriptor_checksum` survives all six renderer mutations but **dies under M7**, so it still has a live purpose. Its coverage narrowed to the keypath-only branch (M8 proves it) and its doc comment went stale — recorded as N3. The other pre-existing exact-string tests (`compile_or_two_keys_tap`, `compile_or_pk_and_pk_older_tap`, `compile_thresh_2_of_3_tap_auto_nums`, `compile_and_pk_pk_tap_auto_nums`, `compile_pk_tap_keypath_only`, `compile_pk_tap_explicit_nums_extract_still_wins`) all still pass unmodified and all still fail under at least one mutation — they are the byte-level non-regression guard, and they hold.

**6. Do the new tests actually assert? — CLEARED.** M1–M6 each killed 4–8 tests; M10 and M11 each killed the tripwire alone, one per assertion. No new test survived every mutation.

**7. Tripwire stability — CLEARED at the current pin; residual hazard recorded as N5.** Measured the one realistic drift: `cargo update -p miniscript --precise 13.1.0` (the version a build *without* `--locked` resolves to, since `Cargo.toml:18` states `"13.0.0"` as a caret requirement) then re-ran the module — `test result: ok. 14 passed; 0 failed`. So 13.1.0 changes neither the `Display` bug nor `compile_tr`'s output, and the documented un-locked-resolution caveat does not destabilise the tripwire today.

**8. Non-regression — CLEARED, and this is the strongest single result.** For every proper binary shape with 1–9 leaves, compared the old path (`desc.to_string()` minus `#checksum`) against the new renderer, classifying by whether the old output was accepted by `parse_template`:

```
old-parseable: 9   old-BROKEN: 2047   divergences on old-parseable: 0
```

Every case the old code got right, the new code reproduces byte-identically. Every one of the 2,047 it got wrong now parses. Only 9 of 2,056 shapes (the caterpillars, one per leaf count) ever worked — which independently corroborates the commit message's framing that a plain 1-of-5 taproot wallet was simply uncompilable.

**9. Blast radius of the diff — CLEARED.** The `SegwitV0` arm (`compile.rs:69-82`) is untouched by the hunk (`@@ -94,17 +94,97 @@` starts after it); `compile_segwitv0_pk` passes unchanged. `render_tr_template` is `#[cfg(feature = "cli-compiler")]`-gated transitively through the module, so the default build is unaffected.

---

## Open / could not determine

- **Exhaustive shape coverage stops at 11 leaves (23,714 shapes) plus the two 128-deep spines.** Shapes with 12+ leaves were not enumerated (Catalan growth). The invariant argument in angle 1 covers them and the two maximum-depth spines probe the only dimension exhaustive enumeration misses, but this is reasoning on top of measurement rather than pure measurement, and I state it as such.
- **The unclosed-nodes error branch (`compile.rs:180-185`) was never executed** — not by any test, and not by any of the 23,716 trees I constructed. Its message string has therefore never been rendered, which is how N1 survived. I could not construct an input that reaches it through miniscript's public API, and I did not resort to `unsafe` or a vendored patch to force it.
- **I did not re-run the author's full-workspace gate** (`781 passed / 0 failed`, clippy, feature-off build) — the brief listed those as settled and nothing I found gives reason to doubt them. What I ran is the `compile::tests` module (14 tests) plus my own probes, all under `--locked --offline`.
- **PR #953's actual upstream source was not read.** I verified the *vendored 13.0.0* `fmt_helper` bug directly (`vendor/miniscript/src/descriptor/tr/taptree.rs:87-113`, the `last_depth`-delta loop) and verified the local algorithm against an independent oracle, so the local code's correctness does not rest on #953 being faithfully transcribed. The claims "merged 2026-05-25" and "in no release through 13.1.0" were taken as given per the brief; I did confirm empirically that 13.1.0 is still broken.
