# FIX-md-single-print-2026-11: `md encode` and `md descriptor --emit md1` print each md1 string once

Model: Claude Opus 5.5 (claude-opus-5-5), 2026-11-22. Author of this FIX document (not the coordinator).
Base: `origin/main` 5df4cb92 (md-cli 0.20.8). Branch `fix/single-print`.
Folded for review r1 (`coordinator/agent-reports/review-fix-single-print-r1.md`; fold model Claude Sonnet 5.5,
claude-sonnet-5-5, 2026-11-22): I-2 (the `--separator` refusal remedies: sections 0, 1, 3, 4, 5), I-3 (section 8b),
I-4 (section 8), M-3 (`md repair --in` help). `origin/main` was still 5df4cb92 when the fold ran.

## 0. Ruling and scope

Brian 2026-11-22 (typed "B"): reverse D4 of mnemonic-engrave `design/SPEC_constellation_cli_uniformity.md`. `ms`,
`md` and `mk` print their string ONCE, on stdout; the grouped stderr engraving card is dropped everywhere. `mt
encode` and `mnemonic bundle` are the model. The uniformity-spec delta is on mnemonic-engrave branch
`fix/single-print` (commit 4f35eb9f).

In scope: the two md verbs that print the card through `cmd::encode::emit_engraving_card`: `md encode` and `md
descriptor --emit md1`. Out of scope: every other md verb (`md repair` prints its strings once, on stdout, measured
2026-11-22; `md compose`, `md decompose`, `md decode`, `md vectors` print no copy on stderr).

### PROPOSED decisions (architect confirms)

- P1. `--group-size` and `--separator` (on `md encode`, and on `md descriptor` where they still `requires = "emit"`)
  are KEPT and group STDOUT on request, as `mt encode` and `mnemonic bundle` do. `--group-size` default changes
  5 -> 0 (unbroken), so a plain `md encode` prints on stdout exactly what it printed before. `--separator` keeps its
  parser, its refusal texts (the remedy sentences are reworded: `--separator space` alone no longer groups; exact
  texts in section 1, review r1 I-2) and default `space`.
- P2. `--out FILE` receives exactly the bytes stdout would have carried (grouped under `--group-size`), as `mt encode
  --out` does; the single renderer `cmd::encode::render_artifact` serves both verbs and both destinations.
- P3. `--json` ignores the grouping flags (unchanged; measured byte-identical to the base).
- P4. `chunk-set-id: 0x<5 hex>` STAYS on stderr: it is not a copy of the string, and it is what tells an operator
  which chunks belong together. `--policy-id-fingerprint`, the advisories, the seating notes and the output-class
  note are unchanged and keep their order.
- P5. The watch-only note under `--out` (stdout empty, note still says "stdout is watch-only") is not changed here;
  proposed FOLLOWUPS line in `design/FIX_mk_single_print.md` section 9 covers md and mk.

## 1. Spec delta (md-local texts)

`md encode --help`, measured on the GREEN build:

    --out <FILE>
        Write the md1 artifact to FILE instead of stdout, CREATED 0600 (SPEC §6b, F-244) — which a shell redirect cannot do. OVERWRITES an existing file, and tightens its mode. The file holds exactly what stdout would have (grouped only under `--group-size`); the chunk-set-id and the advisories still go to stderr

    --group-size <GROUP_SIZE>
        Group each md1 string on stdout every N characters, for hand engraving (0 = unbroken, the default). `--out` receives the same bytes; `--json` stays unbroken

        [default: 0]

    --separator <SEPARATOR>
        Separator to use with `--group-size`: `space` (keyword) or the literal " ". Whitespace only — `hyphen` and `comma` are no longer accepted

        [default: space]

`md descriptor --help` (`--out`, `--group-size`, `--separator`) and the `--in` help of the reading verbs change the
same way; the exact doc-comment text is in the section-3 diff (`main.rs`; `repair.rs` for `md repair --in`, review
r1 M-3: "so a card copied in grouped form re-ingests", as `md decode --in`).

`--separator` refusals (review r1 I-2; `parse_separator` in `main.rs`, shared by `md encode` and `md descriptor`; exit
2 as before). With the default group size now 0, `--separator space` alone prints unbroken, so the remedy sentences
change (the retired-value text before the sentence is unchanged):

- retired value (`hyphen`, `-`, `comma`, `,`), last sentence: "Use `--group-size 5 --separator space` (or the literal
  " ") for a grouped card, or drop `--separator` for an unbroken one (the default)."
- any other value, last sentence: "Drop `--separator` for an unbroken card (the default), or use `--group-size 5
  --separator space` for a grouped one."

Both remedies run as printed (measured): `md encode 'wpkh(@0/<0;1>/*)' --network signet --group-size 5 --separator
space` prints `md1yq pqqxq q8xtw hw4xw n4qh`, and without `--separator` it prints `md1yqpqqxqq8xtwhw4xwn4qh`, both
exit 0.

`CHANGELOG.md`, under the md-cli Unreleased heading:

    ### Changed
    - **Each md1 string is printed once.** `md encode` and `md descriptor --emit md1` no longer repeat their
      strings on stderr as a grouped engraving card, and the `group size:` / `separator:` lines are gone (Brian
      2026-11-22, reversing D4 of the constellation uniformity spec). `--group-size N` now groups stdout, on
      request; its default is `0`, so a plain `md encode` prints on stdout exactly what it printed before. `--out
      FILE` receives the same bytes stdout would have carried. `chunk-set-id:` stays on stderr. `--json` is
      unchanged.

## 2. Behaviour: exact bytes per case

`T` = `tpubDC9Go1KDateW3gS8VXZ6DD1Xu7PgoTdPcf1MX9Z6qVLiHbaeDJ78swPyuQ8YQY19QjtrzkfkZSXwqCcb7XArtid1iLq8Vy55Ydfm4giZh6X`
(testnet, m/84'/1'/0', fp 73c5da0a). `A` = `encode "wpkh(@0/<0;1>/*)" --key "@0=T" --fingerprint @0=73c5da0a --path
"m/84'/1'/0'" --network signet` (three chunks). `N` = `note: stdout is watch-only — public keys only, cannot spend`,
`NT` = `note: stdout is a keyless descriptor template (no keys)`, `X` = `chunk-set-id: 0x9c25f`.

    S1 = md1fnsjlpqpqpm6j8pqqxqqks883w6pg25z995kj6tfd95kj6tfd952c99a0666r69m
    S2 = md1fnsjlpqfd95kj6tfd95kj6tfd95kj6tfd95kj6tfgzrfa9d853cuvgm24dp0q95f
    S3 = md1fnsjlpqklkqkq4p8rle9rdzqpmvrraa6zjn9fcmd34x5udt9t5qcvmqx6e0y3yrt
    G1 = md1fn sjlpq pqpm6 j8pqq xqqks 883w6 pg25z 995kj 6tfd9 5kj6t fd952 c99a0 666r6 9m
    G2 = md1fn sjlpq fd95k j6tfd 95kj6 tfd95 kj6tf d95kj 6tfgz rfa9d 853cu vgm24 dp0q9 5f
    G3 = md1fn sjlpq klkqk q4p8r le9rd zqpmv rraa6 zjn9f cmd34 x5udt 9t5qc vmqx6 e0y3y rt

Measured on the GREEN build; every exit is 0.

| case | stdout | stderr | before (5df4cb92) |
|---|---|---|---|
| `md A` (multi-chunk) | `S1\nS2\nS3\n` | `X\nN\n` | stderr `X\nG1\nG2\nG3\ngroup size: 5\nseparator: space\nN\n` |
| `md A --group-size 5` (also `--separator space` / `" "`) | `G1\nG2\nG3\n` | `X\nN\n` | stdout `S1..S3`; stderr as above |
| `md A --group-size 0` | `S1\nS2\nS3\n` | `X\nN\n` | stderr `X\nS1\nS2\nS3\ngroup size: 0\nseparator: space\nN\n` |
| `md A --group-size 5 --out F` | empty; F (0600) = `G1\nG2\nG3\n` | `X\nN\n` | F = `S1\nS2\nS3\n`; stderr card as row 1 |
| `md A --group-size 5 --json` | the unchanged pretty JSON (`"chunks"` unbroken, `"schema": "md-cli/2"`) | `N\n` | byte-identical (measured `cmp`) |
| `md encode "wpkh(@0/<0;1>/*)" --network signet` (single string) | `md1yqpqqxqq8xtwhw4xwn4qh\n` | `NT\n` | stderr `md1yq pqqxq q8xtw hw4xw n4qh\ngroup size: 5\nseparator: space\nNT\n` |
| the same with `--group-size 5` | `md1yq pqqxq q8xtw hw4xw n4qh\n` | `NT\n` | stdout unbroken |
| `md descriptor <v-b1-wallet md1 x4> --from-mk1 <its 4 mk1> --emit md1` (fixture `crates/md-cli/tests/fixtures/seating/v-b1-wallet.txt`) | the 4 unbroken md1 lines (unchanged) | `chunk-set-id: 0x7a2e1`, then the 3 seating `note:` lines, then `N` | the 4 grouped lines + `group size: 5` + `separator: space` sat between `chunk-set-id:` and the seating notes |
| the same with `--group-size 5` | the 4 lines grouped by 5 | as the row above | stdout unbroken |
| refusals, exit codes, `requires = "emit"` | unchanged, except the two `--separator` remedy sentences (section 1) | unchanged | — |

## 3. Production changes (exact; the GREEN build measured in section 6)

`cmd/encode.rs`: `separator_name` and `emit_engraving_card` are replaced by `render_artifact` (one md1 per line,
grouped on request), used for stdout and `--out`. `cmd/descriptor.rs`: `emit_md1_card` renders through the same
function and no longer writes the card. `main.rs`: flag docs and defaults. Doc comments that described the card are
rewritten in the same diff.

```diff
diff --git i/crates/md-cli/src/cmd/descriptor.rs w/crates/md-cli/src/cmd/descriptor.rs
index c4c221fe..c8858685 100644
--- i/crates/md-cli/src/cmd/descriptor.rs
+++ w/crates/md-cli/src/cmd/descriptor.rs
@@ -69,15 +69,15 @@ pub struct DescriptorArgs<'a> {
     /// `emit-md1-has-no-transcribe-ready-form`: write the keyed md1
     /// artifact to this file (created 0600) instead of stdout when `emit ==
     /// Some(Emit::Md1)`. Mirrors `md encode`'s `--out` byte for byte
-    /// (`crate::cmd::write_artifact`); the engraving card, the
-    /// chunk-set-id and the advisories are unaffected. `None`/inert when
+    /// (`crate::cmd::write_artifact`); the chunk-set-id and the advisories
+    /// are unaffected. `None`/inert when
     /// `emit` is `None` -- the clap surface's `requires = "emit"` is what
     /// keeps that combination unreachable.
     pub out_file: Option<&'a std::path::Path>,
-    /// mstring display-grouping for the `--emit md1` engraving card, mirroring
+    /// mstring display-grouping for the `--emit md1` artifact, mirroring
     /// `md encode`'s `group_size`/`separator` exactly (same default, same
     /// meaning): insert `separator` every `group_size` chars (0 = unbroken).
-    /// Display only, on stderr. Inert when `emit` is `None`.
+    /// Applies to stdout and `--out`. Inert when `emit` is `None`.
     pub group_size: usize,
     pub separator: char,
     pub json: bool,
@@ -404,19 +404,18 @@ fn check_emit_md1_input_mode(
 /// Mint the seated policy as a keyed md1 card: the SAME transcribe-ready
 /// output contract as `md encode` (`emit-md1-has-no-transcribe-ready-form`).
 ///
-/// The cards come from [`crate::cmd::encode::mint_md1_cards`], and the
-/// engraving card comes from [`crate::cmd::encode::emit_engraving_card`] --
-/// the SAME two functions `md encode` mints and renders through, so N2's
-/// byte-identity oracle is a question about the composed `Descriptor` alone,
-/// and the two minting surfaces cannot drift into two card renderers.
+/// The cards come from [`crate::cmd::encode::mint_md1_cards`], and the text
+/// from [`crate::cmd::encode::render_artifact`] -- the SAME two functions
+/// `md encode` mints and renders through, so N2's byte-identity oracle is a
+/// question about the composed `Descriptor` alone, and the two minting
+/// surfaces cannot drift into two renderers.
 ///
 /// Mirrors `md encode`'s §6a/§6b/§6c contract exactly (measured against the
 /// binary, `design/agent-reports/IMPL-emit-md1-form.md`): stdout is the
-/// canonical artifact, UNBROKEN and nothing else, written to `out_file` when
-/// given (created 0600, `crate::cmd::write_artifact`) instead of stdout; the
-/// chunk-set-id (when chunked), the grouped engraving card, `group size:`
-/// and `separator:` all go to stderr in that order, immediately after the
-/// artifact is written -- exactly where `md encode` puts them. Only after
+/// artifact and nothing else, printed ONCE (grouped only when `--group-size`
+/// asks), written to `out_file` when given (created 0600,
+/// `crate::cmd::write_artifact`) instead of stdout; the chunk-set-id (when
+/// chunked) goes to stderr -- exactly where `md encode` puts it. Only after
 /// that does this diverge from `md encode`'s stderr shape, for
 /// `emit_seating_notes` -- an S-row-only insertion with no `md encode`
 /// counterpart -- before the advisories both surfaces share.
@@ -446,22 +445,13 @@ fn emit_md1_card(
         // artifact and nothing else.
         eprintln!("chunk-set-id: 0x{csid:05x}");
     }
-    let mut body = String::new();
-    for s in &cards {
-        body.push_str(s);
-        body.push('\n');
-    }
+    let body = crate::cmd::encode::render_artifact(&cards, group_size, separator);
     // `md encode --out`'s exact contract: FILE when given (created 0600, a
     // shell redirect cannot do that -- F-244), stdout otherwise.
     match out_file {
         Some(p) => crate::cmd::write_artifact(p, &body)?,
         None => print!("{body}"),
     }
-    // THE ENGRAVING CARD -- `md encode`'s own stderr rendering, single-sourced
-    // (`emit-md1-has-no-transcribe-ready-form`). Same function, same
-    // position relative to the artifact write, same three lines: the grouped
-    // card(s), `group size:`, `separator:`.
-    crate::cmd::encode::emit_engraving_card(&cards, group_size, separator, &mut std::io::stderr());
     emit_seating_notes(seating_notes);
     // F-A4: legacy-P2SH-multisig footgun advisory, carried across from `md
     // encode` (review r1 I3) -- this is a minting surface too, and which
diff --git i/crates/md-cli/src/cmd/encode.rs w/crates/md-cli/src/cmd/encode.rs
index b0a3a730..2d19adc2 100644
--- i/crates/md-cli/src/cmd/encode.rs
+++ w/crates/md-cli/src/cmd/encode.rs
@@ -14,8 +14,8 @@ use md_codec::tree::{Body, Node};
 pub struct EncodeArgs<'a> {
     pub template: &'a str,
     /// P3 §6b — write the artifact to this file (created 0600) instead of
-    /// stdout. Affects the ARTIFACT only: the engraving card, the chunk-set-id
-    /// and every advisory still go to stderr.
+    /// stdout. Affects the ARTIFACT only: the chunk-set-id and every advisory
+    /// still go to stderr.
     pub out_file: Option<&'a std::path::Path>,
     pub keys: &'a [String],
     pub fingerprints: &'a [String],
@@ -189,17 +189,13 @@ pub fn run(args: EncodeArgs<'_>) -> Result<u8, CliError> {
     // the same gate on md's authoring surface.
     md_codec::validate::validate_relative_timelocks(&descriptor.tree)?;
 
-    // P3 §6a + D4. stdout is the canonical artifact, UNBROKEN, and nothing
-    // else; the grouped form -- the thing a human actually transcribes onto a
-    // plate -- moves to the stderr engraving card below. Before this, a
-    // pipeline had to strip a header line AND pass `--group-size 0`, and
-    // `me sysw pack` classified a grouped chunk as "not a form this container
-    // can place" at exit 4.
+    // P3 §6a. stdout is the artifact and nothing else, printed ONCE: unbroken
+    // by default (`me sysw pack` classifies a grouped chunk as "not a form this
+    // container can place" at exit 4), grouped when `--group-size` asks.
     let (cards, chunk_set_id) = mint_md1_cards(&descriptor, args.force_chunked)?;
     if let Some(csid) = chunk_set_id {
         // The chunk-set-id is not deleted -- it is the only thing that tells
-        // an operator which chunks belong to one card. It heads the engraving
-        // card on stderr.
+        // an operator which chunks belong to one card. It goes to stderr.
         //
         // The fixture-file writer in `cmd/vectors.rs` still emits this same
         // text into a FILE and is deliberately untouched: §6a is a rule about
@@ -212,21 +208,11 @@ pub fn run(args: EncodeArgs<'_>) -> Result<u8, CliError> {
     // bearing on it. The file is CREATED 0600 through the shared crate's
     // `write_private`; a shell redirect cannot do that, which is why the flag
     // exists at all (F-244).
-    let mut body = String::new();
-    for s in &cards {
-        body.push_str(s);
-        body.push('\n');
-    }
+    let body = render_artifact(&cards, args.group_size, args.separator);
     match args.out_file {
         Some(p) => crate::cmd::write_artifact(p, &body)?,
         None => print!("{body}"),
     }
-    emit_engraving_card(
-        &cards,
-        args.group_size,
-        args.separator,
-        &mut std::io::stderr(),
-    );
     if args.policy_id_fingerprint {
         let id = compute_wallet_policy_id(&descriptor)?;
         eprintln!(
@@ -310,56 +296,19 @@ pub fn mint_md1_cards(
     }
 }
 
-/// P3 §6c: the keyword for a separator char, for the card's `separator:` line.
+/// The artifact text: one md1 string per line, grouped every `group_size`
+/// characters when asked (0 = unbroken). Printed ONCE, on stdout or to `--out`
+/// (Brian 2026-11-22, reversing D4) -- there is no stderr engraving card.
 ///
-/// The card names the flag VALUE a reader would pass to reproduce it, not the
-/// raw char -- ` ` on a line of its own is invisible, which is the one value
-/// that matters after §6c narrows this to whitespace.
-fn separator_name(sep: char) -> String {
-    match sep {
-        ' ' => "space".into(),
-        '-' => "hyphen".into(),
-        ',' => "comma".into(),
-        other => other.to_string(),
-    }
-}
-
-/// P3 §6c / D4 — the engraving card, on stderr.
-///
-/// §6c rules that `md` and `mk` have no card to move the grouped form to, that
-/// P3 owns the contents, and that the minimum it must carry is the grouped
-/// string itself -- after D4 this is the only place that form exists.
-///
-/// SHAPE follows `ms`'s: plain `label: value` lines, no prefix character,
-/// ending with the tool's existing output-class advisory (emitted by the
-/// caller, after the warnings). `mnemonic bundle`'s `#`-prefixed card is the
-/// other in-constellation precedent and is deliberately NOT followed -- its `#`
-/// mirrors the comment headers on its own stdout, a surface `md` does not have.
-///
-/// The grouped string comes FIRST because it is the thing a human transcribes.
-/// Every chunk of a chunk set is rendered, not just the first: a chunk missing
-/// from the card is a chunk nobody engraves.
-///
-/// There is no `--no-engraving-card` on `md` -- §6c names that flag for `ms`
-/// and `mnemonic` only, and warns that it is what makes "no grouped form
-/// anywhere" possible.
-///
-/// `pub(crate)`, not private: `cmd::descriptor::emit_md1_card` (N2's `--emit
-/// md1`, the S -> K cell) calls this SAME function so the two minting
-/// surfaces cannot drift into two card renderers (the followup this closes,
-/// `emit-md1-has-no-transcribe-ready-form`) -- single-sourced exactly the way
-/// `mint_md1_cards` above already is.
-pub(crate) fn emit_engraving_card<W: std::io::Write>(
-    cards: &[String],
-    group_size: usize,
-    separator: char,
-    stderr: &mut W,
-) {
+/// `pub(crate)`: `cmd::descriptor::emit_md1_card` (`--emit md1`) renders
+/// through this same function, so the two minting surfaces cannot drift.
+pub(crate) fn render_artifact(cards: &[String], group_size: usize, separator: char) -> String {
+    let mut body = String::new();
     for s in cards {
-        let _ = writeln!(stderr, "{}", render_grouped(s, group_size, separator));
+        body.push_str(&render_grouped(s, group_size, separator));
+        body.push('\n');
     }
-    let _ = writeln!(stderr, "group size: {group_size}");
-    let _ = writeln!(stderr, "separator: {}", separator_name(separator));
+    body
 }
 
 /// F-A4: is `tree` a top-level bare legacy P2SH multisig — `sh(multi(...))`
diff --git i/crates/md-cli/src/cmd/repair.rs w/crates/md-cli/src/cmd/repair.rs
index 2cfb0b77..12a35ba2 100644
--- i/crates/md-cli/src/cmd/repair.rs
+++ w/crates/md-cli/src/cmd/repair.rs
@@ -50,7 +50,7 @@ pub struct RepairArgs {
     pub md1_strings: Vec<String>,
 
     /// P3 §6b — read md1 strings from FILE, one per line. Display separators
-    /// are stripped per line, so a card copied off an engraving card
+    /// are stripped per line, so a card copied in grouped form
     /// re-ingests.
     #[arg(long = "in", value_name = "FILE")]
     pub in_file: Option<std::path::PathBuf>,
diff --git i/crates/md-cli/src/main.rs w/crates/md-cli/src/main.rs
index 97c98a8e..f665bf6f 100644
--- i/crates/md-cli/src/main.rs
+++ w/crates/md-cli/src/main.rs
@@ -73,8 +73,10 @@ impl CliNetwork {
 /// is a plate.
 ///
 /// The refusal names BOTH remedies (§6h — remedy text must be executable):
-/// `--separator space` for a grouped card, `--group-size 0` for an unbroken
-/// one. Someone reaching for `hyphen` wanted one or the other.
+/// `--group-size 5 --separator space` for a grouped card (`--separator space`
+/// alone no longer groups: the default group size is 0), dropping
+/// `--separator` for an unbroken one. Someone reaching for `hyphen` wanted one
+/// or the other.
 fn parse_separator(s: &str) -> Result<char, String> {
     match s {
         "space" | " " => Ok(' '),
@@ -82,12 +84,13 @@ fn parse_separator(s: &str) -> Result<char, String> {
             "separator {s:?} is no longer accepted: --separator is whitespace-only across the \
              constellation (SPEC §6c), because `mt` strips whitespace and nothing else on decode, \
              so a hyphen- or comma-grouped card is one `mt` refuses AFTER the plates are cut. \
-             Use `--separator space` (or the literal \" \") for a grouped card, or \
-             `--group-size 0` for an unbroken one."
+             Use `--group-size 5 --separator space` (or the literal \" \") for a grouped card, or \
+             drop `--separator` for an unbroken one (the default)."
         )),
         other => Err(format!(
             "invalid separator {other:?}; --separator is whitespace-only: expected `space` (or \
-             the literal \" \"). Use `--group-size 0` for an unbroken card."
+             the literal \" \"). Drop `--separator` for an unbroken card (the default), or use \
+             `--group-size 5 --separator space` for a grouped one."
         )),
     }
 }
@@ -115,8 +118,9 @@ enum Command {
         in_file: Option<std::path::PathBuf>,
         /// Write the md1 artifact to FILE instead of stdout, CREATED 0600
         /// (SPEC §6b, F-244) — which a shell redirect cannot do. OVERWRITES an
-        /// existing file, and tightens its mode. The stderr engraving card,
-        /// the chunk-set-id and the advisories are unaffected.
+        /// existing file, and tightens its mode. The file holds exactly what
+        /// stdout would have (grouped only under `--group-size`); the
+        /// chunk-set-id and the advisories still go to stderr.
         #[arg(long = "out", value_name = "FILE")]
         out_file: Option<std::path::PathBuf>,
         /// Compile a sub-Miniscript-Policy expression into a template (cli-compiler).
@@ -157,14 +161,14 @@ enum Command {
         /// Force chunked encoding even for short policies.
         #[arg(long)]
         force_chunked: bool,
-        /// Insert a separator every N characters in the ENGRAVING CARD on
-        /// stderr (0 = unbroken card). SPEC §6c. stdout is always the
-        /// unbroken md1 string; --json stays unbroken too.
-        #[arg(long, default_value_t = 5)]
+        /// Group each md1 string on stdout every N characters, for hand
+        /// engraving (0 = unbroken, the default). `--out` receives the same
+        /// bytes; `--json` stays unbroken.
+        #[arg(long, default_value_t = 0)]
         group_size: u16,
-        /// Separator for the engraving card: `space` (keyword) or the literal
-        /// " ". Whitespace only — `hyphen` and `comma` are no longer accepted.
-        /// SPEC §6c.
+        /// Separator to use with `--group-size`: `space` (keyword) or the
+        /// literal " ". Whitespace only — `hyphen` and `comma` are no longer
+        /// accepted.
         #[arg(long, default_value = "space", value_parser = parse_separator)]
         separator: char,
         /// Removed in v0.12.0; now a hard error. md1 is regular-code-only
@@ -199,8 +203,8 @@ enum Command {
         #[arg(num_args = 1..)]
         strings: Vec<String>,
         /// Read md1 strings from FILE, one per line (SPEC §6b). Display
-        /// separators are stripped per line, so a card copied off the
-        /// engraving card re-ingests.
+        /// separators are stripped per line, so a card copied in grouped form
+        /// re-ingests.
         #[arg(long = "in", value_name = "FILE")]
         in_file: Option<std::path::PathBuf>,
         /// Emit a structured JSON object on stdout instead of the
@@ -560,24 +564,24 @@ enum Command {
         /// Write the KEYED md1 artifact to FILE instead of stdout, CREATED
         /// 0600 -- mirrors `md encode --out` (F-244) byte for byte, which a
         /// shell redirect cannot do. OVERWRITES an existing file, and
-        /// tightens its mode. The stderr engraving card, the chunk-set-id
-        /// and the advisories are unaffected. Only meaningful with `--emit
+        /// tightens its mode. The file holds exactly what stdout would have
+        /// (grouped only under `--group-size`); the chunk-set-id and the
+        /// advisories still go to stderr. Only meaningful with `--emit
         /// md1` -- `requires = "emit"` refuses it structurally rather than
         /// silently discarding it on the plain descriptor row (the I4
         /// defect class this cycle keeps naming on this verb).
         #[arg(long = "out", value_name = "FILE", requires = "emit")]
         out_file: Option<std::path::PathBuf>,
-        /// Insert a separator every N characters in the ENGRAVING CARD on
-        /// stderr (0 = unbroken card) -- mirrors `md encode --group-size`
-        /// exactly, same default. stdout is always the unbroken md1 string.
-        /// Only meaningful with `--emit md1`; see `--out`'s note on
-        /// `requires`.
-        #[arg(long, default_value_t = 5, requires = "emit")]
-        group_size: u16,
-        /// Separator for the engraving card: `space` (keyword) or the
-        /// literal " " -- mirrors `md encode --separator` exactly, same
-        /// default. Whitespace only. Only meaningful with `--emit md1`; see
+        /// Group each md1 string on stdout every N characters, for hand
+        /// engraving (0 = unbroken, the default) -- mirrors `md encode
+        /// --group-size` exactly. Only meaningful with `--emit md1`; see
         /// `--out`'s note on `requires`.
+        #[arg(long, default_value_t = 0, requires = "emit")]
+        group_size: u16,
+        /// Separator to use with `--group-size`: `space` (keyword) or the
+        /// literal " " -- mirrors `md encode --separator` exactly. Whitespace
+        /// only. Only meaningful with `--emit md1`; see `--out`'s note on
+        /// `requires`.
         #[arg(
             long,
             default_value = "space",
```

## 4. RED rows

New file `crates/md-cli/tests/single_print.rs` (verbatim below; rustfmt-clean). Row = test function name.

| row | expected after GREEN | measured on base 5df4cb92 |
|---|---|---|
| `encode_prints_each_chunk_once` | stdout `S1..S3` unbroken; stderr exactly `X\nN\n` | FAIL: stderr `X\nG1\nG2\nG3\n...` (the second copy) |
| `encode_group_size_groups_stdout` | `--group-size 5`: stdout `G1\nG2\nG3\n`; stderr `X\nN\n` | FAIL: stdout unbroken |
| `encode_out_writes_the_stdout_bytes` | `--group-size 5 --out F`: stdout empty, F = `G1..G3`, stderr `X\nN\n` | FAIL: stderr carries the grouped card |
| `encode_single_chunk_template_prints_once` | stdout `md1yqpqqxqq8xtwhw4xwn4qh\n`; stderr exactly `NT\n` | FAIL: stderr `md1yq pqqxq q8xtw hw4xw n4qh\ngroup size: 5\nseparator: space\nNT\n` |
| `descriptor_emit_md1_prints_each_chunk_once` | no stderr line starts `md1`, `group size:` or `separator:` | FAIL: a grouped `md1f0 ghpps ...` line on stderr |
| `descriptor_emit_md1_group_size_groups_stdout` | `--group-size 5` stdout = the default stdout regrouped by 5 | FAIL: stdout unbroken |
| `separator_refusal_remedies_run_as_printed` | `--separator hyphen` exits 2 with the new retired-arm sentence; `--group-size 5 --separator space` exits 0 and prints `md1yq pqqxq q8xtw hw4xw n4qh\n`; no `--separator` prints `md1yqpqqxqq8xtwhw4xwn4qh\n`; `--separator x` exits 2 with the new catch-all sentence | FAIL: the refusal still says "Use `--separator space` (or the literal " ") for a grouped card, or `--group-size 0` for an unbroken one." |

Command (run, not pinned): `CARGO_TARGET_DIR=<own> cargo nextest run --locked --build-jobs 6 -j 6 -p md-cli --test
single_print`. Base result: `7 tests run: 0 passed, 7 failed` (log `/scratch/cache/fold-sp-md-red.log`, nextest rc 100, run on
`origin/main` 5df4cb92 with only the new file added). Assertion excerpts from that run:

```text
    thread 'encode_prints_each_chunk_once' panicked at crates/md-cli/tests/single_print.rs:94:5:
      left: "chunk-set-id: 0x9c25f\nmd1fn sjlpq pqpm6 j8pqq xqqks 883w6 pg25z 995kj 6tfd9 5kj6t fd952 c99a0 666r6 9m\nmd1fn sjlpq fd95k j6tfd 95kj6 tfd95 kj6tf d95kj 6tfgz rfa9d 853cu vgm24 dp0q9 5f\nmd1fn sjlpq klkqk q4p8r le9rd zqpmv rraa6 zjn9f cmd34 x5udt 9t5qc vmqx6 e0y3y rt\ngroup size: 5\nseparator: space\nnote: stdout is
     right: "chunk-set-id: 0x9c25f\nnote: stdout is watch-only — public keys only, cannot spend\n"
    thread 'encode_single_chunk_template_prints_once' panicked at crates/md-cli/tests/single_print.rs:126:5:
      left: "md1yq pqqxq q8xtw hw4xw n4qh\ngroup size: 5\nseparator: space\nnote: stdout is a keyless descriptor template (no keys)\n"
     right: "note: stdout is a keyless descriptor template (no keys)\n"
    thread 'descriptor_emit_md1_prints_each_chunk_once' panicked at crates/md-cli/tests/single_print.rs:143:9:
    thread 'descriptor_emit_md1_group_size_groups_stdout' panicked at crates/md-cli/tests/single_print.rs:155:5:
      left: "md1f0ghpps9q2tvyyy5jmpprj5qqcy8ppgtcgu79mg9tnchdq59wpyhwsv0jskp2rsal4egz4eqdccu772e060rs\nmd1f0ghppsf5859p875x67p5s3wem7sgluxl3d2a3syx3m7halwd7s7d5e8l2xm3y3xzfmadfjcjukwzsuw7pydp\nmd1f0ghppsje20ur0anz7jwkzae8efejcxy50llpx82qfmryv7l68w6hzragnj3g5qrl85zeapccg28cpyh2qcaz\nmd1f0ghppse8wq0vdczfyy55tqsd5576trsa3p40nfpd7hs
     right: "md1f0 ghpps 9q2tv yyy5j mpprj 5qqcy 8ppgt cgu79 mg9tn chdq5 9wpyh wsv0j skp2r sal4e gz4eq dccu7 72e06 0rs\nmd1f0 ghpps f5859 p875x 67p5s 3wem7 sglux l3d2a 3syx3 m7hal wd7s7 d5e8l 2xm3y 3xzfm adfjc jukwz suw7p ydp\nmd1f0 ghpps je20u r0anz 7jwkz ae8ef ejcxy 50llp x82qf mryv7 l68w6 hzrag nj3g5 qrl85 zeapc cg28c pyh2q c
    thread 'separator_refusal_remedies_run_as_printed' panicked at crates/md-cli/tests/single_print.rs:180:5:
    error: invalid value 'hyphen' for '--separator <SEPARATOR>': separator "hyphen" is no longer accepted: ... Use `--separator space` (or the literal " ") for a grouped card, or `--group-size 0` for an unbroken one.
    thread 'encode_out_writes_the_stdout_bytes' panicked at crates/md-cli/tests/single_print.rs:115:5:
      left: "chunk-set-id: 0x9c25f\nmd1fn sjlpq pqpm6 j8pqq xqqks 883w6 pg25z 995kj 6tfd9 5kj6t fd952 c99a0 666r6 9m\nmd1fn sjlpq fd95k j6tfd 95kj6 tfd95 kj6tf d95kj 6tfgz rfa9d 853cu vgm24 dp0q9 5f\nmd1fn sjlpq klkqk q4p8r le9rd zqpmv rraa6 zjn9f cmd34 x5udt 9t5qc vmqx6 e0y3y rt\ngroup size: 5\nseparator: space\nnote: stdout is
     right: "chunk-set-id: 0x9c25f\nnote: stdout is watch-only — public keys only, cannot spend\n"
    thread 'encode_group_size_groups_stdout' panicked at crates/md-cli/tests/single_print.rs:102:5:
      left: "md1fnsjlpqpqpm6j8pqqxqqks883w6pg25z995kj6tfd95kj6tfd952c99a0666r69m\nmd1fnsjlpqfd95kj6tfd95kj6tfd95kj6tfd95kj6tfgzrfa9d853cuvgm24dp0q95f\nmd1fnsjlpqklkqkq4p8rle9rdzqpmvrraa6zjn9fcmd34x5udt9t5qcvmqx6e0y3yrt\n"
     right: "md1fn sjlpq pqpm6 j8pqq xqqks 883w6 pg25z 995kj 6tfd9 5kj6t fd952 c99a0 666r6 9m\nmd1fn sjlpq fd95k j6tfd 95kj6 tfd95 kj6tf d95kj 6tfgz rfa9d 853cu vgm24 dp0q9 5f\nmd1fn sjlpq klkqk q4p8r le9rd zqpmv rraa6 zjn9f cmd34 x5udt 9t5qc vmqx6 e0y3y rt\n"
```

The RED file, verbatim:

```rust
//! `md encode` and `md descriptor --emit md1` print each md1 chunk ONCE, on
//! stdout (Brian 2026-11-22, reversing D4 of mnemonic-engrave
//! `design/SPEC_constellation_cli_uniformity.md`; `design/FIX_md_single_print.md`).
//!
//! The grouped stderr engraving card, and its `group size:` / `separator:`
//! lines, are gone; `chunk-set-id:` stays on stderr. `--group-size N` groups
//! STDOUT on request (default 0, ungrouped), as `mt encode` and
//! `mnemonic bundle` do, and `--out FILE` receives exactly the bytes stdout
//! would have carried.

use assert_cmd::cargo::CommandCargoExt;
use std::process::Command;

/// mk-cli's `V15_84_TEST` account xpub (testnet, m/84'/1'/0', fp 73c5da0a).
const TPUB: &str = "tpubDC9Go1KDateW3gS8VXZ6DD1Xu7PgoTdPcf1MX9Z6qVLiHbaeDJ78swPyuQ8YQY19QjtrzkfkZSXwqCcb7XArtid1iLq8Vy55Ydfm4giZh6X";
const WATCH_NOTE: &str = "note: stdout is watch-only \u{2014} public keys only, cannot spend\n";
const TEMPLATE_NOTE: &str = "note: stdout is a keyless descriptor template (no keys)\n";
const CSID: &str = "chunk-set-id: 0x9c25f\n";
const V_B1_WALLET: &str = include_str!("fixtures/seating/v-b1-wallet.txt");

fn md() -> Command {
    Command::cargo_bin("md").unwrap()
}

fn text(b: &[u8]) -> String {
    String::from_utf8(b.to_vec()).unwrap()
}

/// A keyed signet wpkh wallet policy: three chunks.
fn encode(extra: &[&str]) -> std::process::Output {
    let key = format!("@0={TPUB}");
    let mut args = vec![
        "encode",
        "wpkh(@0/<0;1>/*)",
        "--key",
        &key,
        "--fingerprint",
        "@0=73c5da0a",
        "--path",
        "m/84'/1'/0'",
        "--network",
        "signet",
    ];
    args.extend_from_slice(extra);
    md().args(&args).output().unwrap()
}

fn grouped5(s: &str) -> String {
    let c: Vec<char> = s.chars().collect();
    c.chunks(5)
        .map(|g| g.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(" ")
}

fn regroup5(body: &str) -> String {
    body.lines().map(|l| grouped5(l) + "\n").collect()
}

fn hrp_lines(t: &str, hrp: &str) -> Vec<String> {
    t.lines()
        .map(str::trim)
        .filter(|l| l.starts_with(hrp))
        .map(str::to_string)
        .collect()
}

fn emit_b1(extra: &[&str]) -> std::process::Output {
    let mut c = md();
    c.arg("descriptor");
    for p in hrp_lines(V_B1_WALLET, "md1") {
        c.arg(p);
    }
    for s in hrp_lines(V_B1_WALLET, "mk1") {
        c.args(["--from-mk1", &s]);
    }
    c.args(["--emit", "md1"]);
    c.args(extra);
    c.output().unwrap()
}

#[test]
fn encode_prints_each_chunk_once() {
    let o = encode(&[]);
    assert_eq!(o.status.code(), Some(0));
    let out = text(&o.stdout);
    assert_eq!(out.lines().count(), 3, "three chunks; got {out:?}");
    for l in out.lines() {
        assert!(
            l.starts_with("md1") && !l.contains(' '),
            "ungrouped; got {l:?}"
        );
    }
    assert_eq!(text(&o.stderr), format!("{CSID}{WATCH_NOTE}"));
}

#[test]
fn encode_group_size_groups_stdout() {
    let plain = text(&encode(&[]).stdout);
    let o = encode(&["--group-size", "5"]);
    assert_eq!(o.status.code(), Some(0));
    assert_eq!(text(&o.stdout), regroup5(&plain));
    assert_eq!(text(&o.stderr), format!("{CSID}{WATCH_NOTE}"));
}

#[test]
fn encode_out_writes_the_stdout_bytes() {
    let grouped = text(&encode(&["--group-size", "5"]).stdout);
    let d = tempfile::tempdir().unwrap();
    let f = d.path().join("policy.md1");
    let o = encode(&["--group-size", "5", "--out", f.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(0));
    assert_eq!(text(&o.stdout), "");
    assert_eq!(std::fs::read_to_string(&f).unwrap(), grouped);
    assert_eq!(text(&o.stderr), format!("{CSID}{WATCH_NOTE}"));
}

#[test]
fn encode_single_chunk_template_prints_once() {
    let o = md()
        .args(["encode", "wpkh(@0/<0;1>/*)", "--network", "signet"])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(0));
    assert_eq!(text(&o.stdout), "md1yqpqqxqq8xtwhw4xwn4qh\n");
    assert_eq!(text(&o.stderr), TEMPLATE_NOTE);
}

#[test]
fn descriptor_emit_md1_prints_each_chunk_once() {
    let o = emit_b1(&[]);
    assert_eq!(o.status.code(), Some(0), "{}", text(&o.stderr));
    let out = text(&o.stdout);
    assert!(!out.is_empty());
    for l in out.lines() {
        assert!(
            l.starts_with("md1") && !l.contains(' '),
            "ungrouped; got {l:?}"
        );
    }
    let err = text(&o.stderr);
    for l in err.lines() {
        assert!(
            !l.starts_with("md1") && !l.starts_with("group size:") && !l.starts_with("separator:"),
            "no engraving card on stderr; got {l:?} in {err:?}"
        );
    }
}

#[test]
fn descriptor_emit_md1_group_size_groups_stdout() {
    let plain = text(&emit_b1(&[]).stdout);
    let o = emit_b1(&["--group-size", "5"]);
    assert_eq!(o.status.code(), Some(0), "{}", text(&o.stderr));
    assert_eq!(text(&o.stdout), regroup5(&plain));
}

// ---- the `--separator` refusal remedies run as printed (review r1 I-2) ----

const WPKH: &str = "wpkh(@0/<0;1>/*)";

/// With the default group size 0, `--separator space` alone prints unbroken,
/// so the refusal's remedy for "a grouped card" must carry `--group-size 5`.
/// Both remedies are run exactly as the refusal prints them.
#[test]
fn separator_refusal_remedies_run_as_printed() {
    let refused = md()
        .args([
            "encode",
            WPKH,
            "--network",
            "signet",
            "--separator",
            "hyphen",
        ])
        .output()
        .unwrap();
    assert_eq!(refused.status.code(), Some(2));
    let se = text(&refused.stderr);
    assert!(
        se.contains(
            "Use `--group-size 5 --separator space` (or the literal \" \") for a grouped card, \
             or drop `--separator` for an unbroken one (the default)."
        ),
        "{se}"
    );

    let grouped = md()
        .args([
            "encode",
            WPKH,
            "--network",
            "signet",
            "--group-size",
            "5",
            "--separator",
            "space",
        ])
        .output()
        .unwrap();
    assert_eq!(grouped.status.code(), Some(0));
    assert_eq!(text(&grouped.stdout), "md1yq pqqxq q8xtw hw4xw n4qh\n");

    let unbroken = md()
        .args(["encode", WPKH, "--network", "signet"])
        .output()
        .unwrap();
    assert_eq!(unbroken.status.code(), Some(0));
    assert_eq!(text(&unbroken.stdout), "md1yqpqqxqq8xtwhw4xwn4qh\n");

    let other = md()
        .args(["encode", WPKH, "--network", "signet", "--separator", "x"])
        .output()
        .unwrap();
    assert_eq!(other.status.code(), Some(2));
    let se = text(&other.stderr);
    assert!(
        se.contains(
            "Drop `--separator` for an unbroken card (the default), or use \
             `--group-size 5 --separator space` for a grouped one."
        ),
        "{se}"
    );
}
```

## 5. Tests that change

Measured: the full suite with the section-3 production change and the base tests failed exactly 8 rows (the first-round measurement; the refusal-sentence row below is the ninth, measured in the fold round)
(`/scratch/cache/sp-md-basetests.log`, `1652 tests run: 1644 passed, 8 failed`). Two more pass on the change only
vacuously (they compare two empty card slices) and are rewritten:

| test | change |
|---|---|
| `cmd_encode::encode_default_groups_space_5` | renamed `encode_group_size_5_groups_stdout_space_5`; passes `--group-size 5` and reads the grouped line from stdout |
| `cmd_encode::encode_still_accepts_whitespace_separators` | passes `--group-size 5`; asserts stdout `md1yq pqqxq q8xtw hw4xw n4qh\n` |
| `cmd_encode::encode_stdout_is_unbroken_and_the_card_is_on_stderr` | renamed `encode_stdout_is_unbroken_and_stderr_has_no_copy`; stderr exactly `NT\n` |
| `cmd_encode::grouping_flags_move_the_card_and_not_stdout` | renamed `grouping_flags_shape_stdout`; no flag and `--group-size 0` -> unbroken, `--group-size 4` -> `md1y qpqq xqq8 xtwh w4xw n4qh\n` |
| `cmd_encode::chunked_stdout_is_bare_md1_tokens_and_the_card_carries_every_chunk` | renamed `chunked_stdout_is_bare_md1_tokens_and_group_size_groups_every_chunk`; stderr repeats no chunk; `--group-size 5` stdout = every chunk grouped |
| `cli_channels::out_suppresses_nothing_on_stderr` | stderr under `--out` equals the plain run's and is exactly `NT\n` |
| `n2_emit_md1::n2_emit_md1_default_engraving_card_matches_md_encodes_default_shape` | renamed `n2_emit_md1_default_output_matches_md_encodes_default_shape`; stdout equal to `md encode`'s, neither stderr has a card line |
| `n2_emit_md1::n2_emit_md1_out_writes_the_file_and_stdout_matches_md_encodes_out_contract` | asserts NO card line on stderr (was: a card present) |
| `n2_emit_md1::n2_emit_md1_group_size_behaves_identically_to_md_encodes` (vacuous) | compares stdout of both surfaces at `--group-size 0, 8, 3`, and that 8 and 3 group |
| `n2_emit_md1::n2_emit_md1_separator_behaves_identically_to_md_encodes` (vacuous) | compares stdout of both surfaces at `--separator space --group-size 6` |
| `cmd_encode::encode_refuses_the_retired_separators` (review r1 I-2) | pins the new remedy: stderr contains `--separator space`, `` `--group-size 5 --separator space` `` and ``drop `--separator` for an unbroken one`` (was: `--group-size 0`). Measured: with the reworded refusal and the old pin the row FAILS (`/scratch/cache/fold-sp-md-oldpin.log`, 1 failed) |

Every other `--group-size 0` use in the suite (fixtures, `seating/generate.sh`, `printed_remedy_runs`, ...) passes
unchanged: 0 is now the default, so those flags are redundant but valid.

Exact diff (`crates/md-cli/tests/`, excluding the new `single_print.rs`):

```diff
diff --git i/crates/md-cli/tests/cli_channels.rs w/crates/md-cli/tests/cli_channels.rs
index 8b99871b..f0107add 100644
--- i/crates/md-cli/tests/cli_channels.rs
+++ w/crates/md-cli/tests/cli_channels.rs
@@ -123,8 +123,8 @@ fn out_overwrites_on_a_second_run() {
     assert_eq!(mode_of(&f), 0o600);
 }
 
-/// `--out` SUPPRESSES NOTHING ELSE. The stderr engraving card, the chunk-set-id
-/// and the output-class advisory all still fire; only the artifact moved.
+/// `--out` SUPPRESSES NOTHING ELSE. The chunk-set-id and the output-class
+/// advisory still fire; only the artifact moved, and stderr carries no copy.
 #[test]
 fn out_suppresses_nothing_on_stderr() {
     let dir = tempfile::tempdir().unwrap();
@@ -136,8 +136,10 @@ fn out_suppresses_nothing_on_stderr() {
         out_err, plain_err,
         "--out must not change what stderr carries"
     );
-    assert!(out_err.contains("md1yq pqqxq q8xtw hw4xw n4qh"));
-    assert!(out_err.contains("group size: 5"));
+    assert_eq!(
+        out_err,
+        "note: stdout is a keyless descriptor template (no keys)\n"
+    );
 }
 
 /// A CHUNK SET through `--out`: every chunk lands in the file, unbroken, one
diff --git i/crates/md-cli/tests/cmd_encode.rs w/crates/md-cli/tests/cmd_encode.rs
index 95794ef4..d40c5e0d 100644
--- i/crates/md-cli/tests/cmd_encode.rs
+++ w/crates/md-cli/tests/cmd_encode.rs
@@ -22,19 +22,16 @@ fn encode_template_only_emits_a_phrase() {
 const WPKH_UNBROKEN: &str = "md1yqpqqxqq8xtwhw4xwn4qh";
 
 #[test]
-fn encode_default_groups_space_5() {
-    // mstring display-grouping (SPEC §3): default = space/5, single line.
-    //
-    // P3 §6c / D4 moved the grouped form off stdout and onto the stderr
-    // engraving card. The DEFAULT is unchanged -- space/5 -- and this test
-    // still pins it; only the stream it is read from moved.
+fn encode_group_size_5_groups_stdout_space_5() {
+    // mstring display-grouping (SPEC §3): `--group-size 5` = space/5, single
+    // line, on STDOUT (single print, 2026-11-22). The default is unbroken.
     let out = Command::cargo_bin("md")
         .unwrap()
-        .args(["encode", "wpkh(@0/<0;1>/*)"])
+        .args(["encode", "wpkh(@0/<0;1>/*)", "--group-size", "5"])
         .output()
         .unwrap();
     assert!(out.status.success());
-    let line = String::from_utf8(out.stderr)
+    let line = String::from_utf8(out.stdout)
         .unwrap()
         .lines()
         .next()
@@ -109,20 +106,31 @@ fn encode_refuses_the_retired_separators() {
             "the refusal must name what replaced it; got {stderr:?}"
         );
         assert!(
-            stderr.contains("--group-size 0"),
+            stderr.contains("`--group-size 5 --separator space`"),
+            "the grouped remedy needs the group size; got {stderr:?}"
+        );
+        assert!(
+            stderr.contains("drop `--separator` for an unbroken one"),
             "and the other way to get an unbroken card; got {stderr:?}"
         );
     }
 }
 
-/// The control: whitespace still works, by keyword AND by literal, and still
-/// shapes the card.
+/// The control: whitespace still works, by keyword AND by literal, and
+/// shapes stdout.
 #[test]
 fn encode_still_accepts_whitespace_separators() {
     for arg in ["space", " "] {
         let out = Command::cargo_bin("md")
             .unwrap()
-            .args(["encode", "wpkh(@0/<0;1>/*)", "--separator", arg])
+            .args([
+                "encode",
+                "wpkh(@0/<0;1>/*)",
+                "--group-size",
+                "5",
+                "--separator",
+                arg,
+            ])
             .output()
             .unwrap();
         assert!(
@@ -130,11 +138,8 @@ fn encode_still_accepts_whitespace_separators() {
             "--separator {arg:?} must still work: {}",
             String::from_utf8_lossy(&out.stderr)
         );
-        let stderr = String::from_utf8(out.stderr).unwrap();
-        assert!(
-            stderr.contains("md1yq pqqxq q8xtw hw4xw n4qh") && stderr.contains("separator: space"),
-            "got {stderr:?}"
-        );
+        let stdout = String::from_utf8(out.stdout).unwrap();
+        assert_eq!(stdout, "md1yq pqqxq q8xtw hw4xw n4qh\n", "got {stdout:?}");
     }
 }
 
@@ -1126,14 +1131,14 @@ fn the_auto_chunked_header_is_on_stderr_too() {
 }
 
 // ──────────────────────────────────────────────────────────────────────────
-// P3 §6c / D4 — stdout is the UNBROKEN artifact; the grouped form moves to a
-// stderr engraving card
+// Single print (Brian 2026-11-22, reversing D4) — stdout is the artifact,
+// printed once; `--group-size` groups it on request; stderr carries no copy
 // ──────────────────────────────────────────────────────────────────────────
 
 /// The single-string case, asserted as byte-equality on stdout rather than as
 /// "no space at position 5". Equality is what a pipeline sees.
 #[test]
-fn encode_stdout_is_unbroken_and_the_card_is_on_stderr() {
+fn encode_stdout_is_unbroken_and_stderr_has_no_copy() {
     let out = Command::cargo_bin("md")
         .unwrap()
         .args(["encode", "wpkh(@0/<0;1>/*)"])
@@ -1147,26 +1152,15 @@ fn encode_stdout_is_unbroken_and_the_card_is_on_stderr() {
         format!("{WPKH_UNBROKEN}\n"),
         "stdout must be the canonical artifact, unbroken, and nothing else"
     );
-    // The card: the grouped string first (it is the thing a human transcribes),
-    // then the two flags that shaped it, then the existing advisory last.
-    assert!(
-        stderr.contains("md1yq pqqxq q8xtw hw4xw n4qh"),
-        "the grouped form must survive, on the card; got {stderr:?}"
-    );
-    assert!(stderr.contains("group size: 5"), "got {stderr:?}");
-    assert!(stderr.contains("separator: space"), "got {stderr:?}");
-    assert!(
-        stderr
-            .trim_end()
-            .ends_with("note: stdout is a keyless descriptor template (no keys)"),
-        "the card ends with the existing output-class advisory; got {stderr:?}"
+    assert_eq!(
+        stderr, "note: stdout is a keyless descriptor template (no keys)\n",
+        "stderr carries only the output-class advisory"
     );
 }
 
-/// `--group-size` and `--separator` shape the CARD and never stdout. A run with
-/// non-default grouping must leave stdout byte-identical to the default run.
+/// `--group-size` and `--separator` shape STDOUT, on request.
 #[test]
-fn grouping_flags_move_the_card_and_not_stdout() {
+fn grouping_flags_shape_stdout() {
     let run = |extra: &[&str]| {
         let mut args = vec!["encode", "wpkh(@0/<0;1>/*)"];
         args.extend_from_slice(extra);
@@ -1176,39 +1170,28 @@ fn grouping_flags_move_the_card_and_not_stdout() {
             .output()
             .unwrap();
         assert!(out.status.success());
-        (
-            String::from_utf8(out.stdout).unwrap(),
-            String::from_utf8(out.stderr).unwrap(),
-        )
+        String::from_utf8(out.stdout).unwrap()
     };
-    let (base_out, _) = run(&[]);
-    let (g4_out, g4_err) = run(&["--group-size", "4"]);
-    let (g0_out, g0_err) = run(&["--group-size", "0"]);
-    assert_eq!(base_out, g4_out, "--group-size must not reach stdout");
-    assert_eq!(base_out, g0_out, "--group-size 0 must not reach stdout");
-    assert!(
-        g4_err.contains("md1y qpqq xqq8 xtwh w4xw n4qh") && g4_err.contains("group size: 4"),
-        "the card must follow --group-size; got {g4_err:?}"
-    );
-    assert!(
-        g0_err.contains("group size: 0") && g0_err.contains(WPKH_UNBROKEN),
-        "group size 0 renders the card unbroken; got {g0_err:?}"
+    assert_eq!(run(&[]), format!("{WPKH_UNBROKEN}\n"));
+    assert_eq!(run(&["--group-size", "0"]), format!("{WPKH_UNBROKEN}\n"));
+    assert_eq!(
+        run(&["--group-size", "4"]),
+        "md1y qpqq xqq8 xtwh w4xw n4qh\n"
     );
 }
 
 /// The chunked case. Every stdout line is a bare md1 token carrying NO
-/// whitespace — which is exactly what `me sysw pack` classifies a record by,
-/// and the property whose absence made `md encode | me sysw pack` need a
-/// `grep` plus a `--group-size 0`.
+/// whitespace under the default -- which is exactly what `me sysw pack`
+/// classifies a record by -- and `--group-size 5` groups EVERY chunk.
 #[test]
-fn chunked_stdout_is_bare_md1_tokens_and_the_card_carries_every_chunk() {
+fn chunked_stdout_is_bare_md1_tokens_and_group_size_groups_every_chunk() {
     let mut args = keyed_policy_args();
     // Drop the fixture's own `--group-size 0`, so this runs the DEFAULT.
     let g = args.iter().position(|a| a == "--group-size").unwrap();
     args.drain(g..g + 2);
     let out = Command::cargo_bin("md")
         .unwrap()
-        .args(args)
+        .args(&args)
         .output()
         .unwrap();
     assert!(out.status.success());
@@ -1222,20 +1205,26 @@ fn chunked_stdout_is_bare_md1_tokens_and_the_card_carries_every_chunk() {
             !l.contains(char::is_whitespace),
             "stdout must be unbroken; got {l:?}"
         );
+        assert!(!stderr.contains(l), "stderr repeats a chunk: {stderr}");
     }
-    // The card carries the grouped form of EVERY chunk, not just the first.
-    for l in &lines {
-        let grouped: String = l
-            .chars()
-            .collect::<Vec<_>>()
-            .chunks(5)
-            .map(|c| c.iter().collect::<String>())
-            .collect::<Vec<_>>()
-            .join(" ");
-        assert!(
-            stderr.contains(&grouped),
-            "the card is missing a chunk: {grouped:?}\nstderr: {stderr}"
-        );
-    }
-    assert!(stderr.contains("group size: 5") && stderr.contains("separator: space"));
+    args.extend(["--group-size".to_string(), "5".to_string()]);
+    let grouped_out = Command::cargo_bin("md")
+        .unwrap()
+        .args(&args)
+        .output()
+        .unwrap();
+    assert!(grouped_out.status.success());
+    let want: String = lines
+        .iter()
+        .map(|l| {
+            l.chars()
+                .collect::<Vec<_>>()
+                .chunks(5)
+                .map(|c| c.iter().collect::<String>())
+                .collect::<Vec<_>>()
+                .join(" ")
+                + "\n"
+        })
+        .collect();
+    assert_eq!(String::from_utf8(grouped_out.stdout).unwrap(), want);
 }
diff --git i/crates/md-cli/tests/n2_emit_md1.rs w/crates/md-cli/tests/n2_emit_md1.rs
index 21e0e50a..43ad3ad0 100644
--- i/crates/md-cli/tests/n2_emit_md1.rs
+++ w/crates/md-cli/tests/n2_emit_md1.rs
@@ -509,14 +509,12 @@ fn n2_emit_md1_composes_with_seat() {
 // `--group-size`, no `--separator`, and printed no engraving card at all —
 // `md encode` supplies all four, on the one card `md encode` cannot mint.
 // These rows measure that `--emit md1` now carries the SAME contract,
-// single-sourced through `cmd::encode::emit_engraving_card` /
-// `cmd::encode::mint_md1_cards` rather than a second renderer.
+// single-sourced through `cmd::encode::render_artifact` /
+// `cmd::encode::mint_md1_cards` rather than a second renderer. Since the
+// single print (2026-11-22) neither surface prints an engraving card.
 
-/// The engraving-card LINES on stderr: the rendered card string(s) plus the
-/// `group size:` / `separator:` lines `emit_engraving_card` writes. Filters
-/// out `chunk-set-id:`, the seating notes and the output-class advisory —
-/// `md encode`'s stderr has none of the seating notes, so this is the slice
-/// that is meaningful to compare between the two surfaces.
+/// Single print (2026-11-22): the engraving-card lines that used to sit on
+/// stderr -- a rendered md1 string, `group size:`, `separator:`. None remain.
 fn engraving_block(stderr: &str) -> Vec<&str> {
     stderr
         .lines()
@@ -552,31 +550,31 @@ fn emit_b1_with(extra: &[&str]) -> std::process::Output {
         .unwrap()
 }
 
-/// **The default shape.** With no group/separator flags, `--emit md1`'s
-/// engraving card is BYTE-IDENTICAL to `md encode`'s, for the same wallet —
-/// same grouped string(s), same `group size: 5`, same `separator: space`.
-/// Before the fold this row is RED: the baseline prints no engraving card at
-/// all, so `engraving_block` on the minted side is empty.
+/// **The default shape.** With no group/separator flags, `--emit md1`'s stdout
+/// is BYTE-IDENTICAL to `md encode`'s for the same wallet, and neither prints
+/// an engraving card on stderr.
 #[test]
-fn n2_emit_md1_default_engraving_card_matches_md_encodes_default_shape() {
+fn n2_emit_md1_default_output_matches_md_encodes_default_shape() {
     let oracle = oracle_encode(&[]);
     let minted = emit_b1_with(&["--emit", "md1"]);
     assert!(minted.status.success(), "{}", err_of(&minted));
     assert!(oracle.status.success(), "{}", err_of(&oracle));
-    assert_eq!(
-        engraving_block(&err_of(&minted)),
-        engraving_block(&err_of(&oracle)),
-        "minted stderr:\n{}\noracle stderr:\n{}",
-        err_of(&minted),
+    assert_eq!(out_of(&minted), out_of(&oracle));
+    assert!(!out_of(&oracle).is_empty());
+    assert!(
+        engraving_block(&err_of(&minted)).is_empty(),
+        "{}",
+        err_of(&minted)
+    );
+    assert!(
+        engraving_block(&err_of(&oracle)).is_empty(),
+        "{}",
         err_of(&oracle)
     );
-    // Not a vacuous comparison of two empty vectors.
-    assert!(!engraving_block(&err_of(&oracle)).is_empty());
 }
 
-/// **`--group-size` — a differential row.** Same strings in
-/// (`--group-size 0`, unbroken; `--group-size 8`), same rendered engraving
-/// card out, on both surfaces.
+/// **`--group-size` — a differential row.** Same flags in (`--group-size 0`,
+/// unbroken; `8`; `3`), same grouped stdout out, on both surfaces.
 #[test]
 fn n2_emit_md1_group_size_behaves_identically_to_md_encodes() {
     for gs in ["0", "8", "3"] {
@@ -584,13 +582,13 @@ fn n2_emit_md1_group_size_behaves_identically_to_md_encodes() {
         let minted = emit_b1_with(&["--emit", "md1", "--group-size", gs]);
         assert!(oracle.status.success(), "{}", err_of(&oracle));
         assert!(minted.status.success(), "{}", err_of(&minted));
-        assert_eq!(
-            engraving_block(&err_of(&minted)),
-            engraving_block(&err_of(&oracle)),
-            "--group-size {gs}: minted stderr:\n{}\noracle stderr:\n{}",
-            err_of(&minted),
-            err_of(&oracle)
-        );
+        assert_eq!(out_of(&minted), out_of(&oracle), "--group-size {gs}");
+        if gs != "0" {
+            assert!(
+                out_of(&minted).contains(' '),
+                "--group-size {gs} groups stdout"
+            );
+        }
     }
 }
 
@@ -603,16 +601,14 @@ fn n2_emit_md1_separator_behaves_identically_to_md_encodes() {
     let minted = emit_b1_with(&["--emit", "md1", "--separator", "space", "--group-size", "6"]);
     assert!(oracle.status.success(), "{}", err_of(&oracle));
     assert!(minted.status.success(), "{}", err_of(&minted));
-    assert_eq!(
-        engraving_block(&err_of(&minted)),
-        engraving_block(&err_of(&oracle))
-    );
+    assert_eq!(out_of(&minted), out_of(&oracle));
+    assert!(out_of(&minted).contains(' '));
 }
 
 /// **`--out` — a differential row, both sides of the contract.** stdout is
 /// empty and the artifact lands in the FILE, created 0600 — `md encode
 /// --out`'s exact behaviour (`cli_channels.rs::out_creates_the_file_owner_only`),
-/// and the stderr engraving card, chunk-set-id and advisories are UNAFFECTED
+/// and the stderr chunk-set-id and advisories are UNAFFECTED
 /// (mirrors `cli_channels.rs::out_suppresses_nothing_on_stderr`).
 #[test]
 fn n2_emit_md1_out_writes_the_file_and_stdout_matches_md_encodes_out_contract() {
@@ -642,9 +638,10 @@ fn n2_emit_md1_out_writes_the_file_and_stdout_matches_md_encodes_out_contract()
         assert_eq!(mode_of(&minted_f), 0o600, "--out must CREATE 0600");
     }
 
-    // Nothing on stderr is suppressed by routing the artifact to a file.
+    // Nothing on stderr is suppressed by routing the artifact to a file, and
+    // stderr carries no copy of it.
     assert!(err_of(&minted).contains("chunk-set-id: 0x"));
-    assert!(!engraving_block(&err_of(&minted)).is_empty());
+    assert!(engraving_block(&err_of(&minted)).is_empty());
     assert!(err_of(&minted).contains("stdout is watch-only"));
 }
 
```

## 6. Gate (md CI jobs, run locally on the GREEN build: section 3 + section 5 + `single_print.rs`)

| CI step | command | result |
|---|---|---|
| test | `cargo nextest run --locked --workspace --no-fail-fast` (`--build-jobs 6 -j 6`, own target dir) | 1659 passed, 6 skipped (`/scratch/cache/fold-sp-md-green2.log`, on `origin/main` 5df4cb92 + sections 3 and 5 + `single_print.rs`) |
| doc tests | `cargo test --locked --workspace --doc` | ok |
| clippy | `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings` | clean |
| docs | `RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps --document-private-items --all-features` | clean |
| no-default-features build | `cargo build --locked -p md-cli --no-default-features` | ok |
| fmt | `cargo fmt --all --check` | clean |
| display-grouping vectors | `cd design && sha256sum -c display-grouping-vectors.tsv.sha256` | file untouched |
| bitcoind-differential, man-pages, musl/cross `--release`, freebsd, fuzz, vendor-freshness | not run | no codec, dependency or descriptor-semantics change |

The build re-runs the whole gate on its own commits; this table is the author's pre-check.

## 7. Build order

1. RED commit: add `crates/md-cli/tests/single_print.rs`; run the row command; see 7 FAIL.
2. GREEN commit: section 3, section 5, the CHANGELOG entry (section 1); full gate.
3. Release (PROPOSED next: md-cli 0.21.0, a CLI-output change). md-codec is untouched (no codec release).

## 8. Lockstep

### 8.0 Landing order (review r1 I-4; replaces steps 3-5 of the author report's "Landing order"; steps 1-2 unchanged)

3. mnemonic-gui: `pinned-upstream.toml` md/ms/mk tags + the schema edits of md 8c, ms 8c, mk 8b; schema-mirror; gui
   release.
4. mnemonic-toolkit, ONE commit: bump the md/ms/mk install pins AND the mnemonic-gui install pin to the step-3
   release, with the toolkit lockstep of all three FIX docs (incl. the I-3 sites below and
   `docs/manual-gui/tests/expected_gui_schema_inventory.json`), so `{#installer-ahead}` never names these three
   releases; the `manual` job incl. doc-flag-lint mutation self-test. The replacement text for the
   `{#installer-ahead}` section (`82-binary-and-launch.md`) is in the mk FIX, section 8a-bis (one place for all
   three).

Why: with the toolkit installer ahead of the GUI, the installed GUI's schema still says `default_value: Some("5")`
for `--group-size` (md encode and md descriptor forms); its `is_at_default` (mnemonic-gui
`src/form/invocation.rs:96-121`) suppresses a Number flag equal to the schema default, so a user who sets 5 sends no
`--group-size`, and the new CLI applies 0: unbroken output with no message. If the toolkit must land first instead,
each FIX doc's toolkit section must carry this `{#installer-ahead}` sentence (PROPOSED text, from the review): "On
the `md encode`, `md descriptor --emit md1`, `ms encode` and `mk encode` forms, this GUI does not send `--group-size
5` (its shown default), and these CLI releases print the unbroken string unless `--group-size` is given: to print
groups of five, run the CLI with `--group-size 5`."

### 8a. Toolkit manual (`mnemonic-toolkit` `docs/manual/src/40-cli-reference/42-md.md`), lands in step 4 (the toolkit's one commit)

Line 30 (`md encode --out` row): replace "the stderr engraving card, chunk-set id and advisories are unaffected" with
"the file holds exactly what stdout would (grouped under `--group-size`); the chunk-set id and advisories stay on
stderr".

Line 40 (`--policy-id-fingerprint` row): replace "on stderr after the card" with "on stderr".

Line 41 (`md encode --group-size` row) becomes:

    | `--group-size <N>` | group each md1 string on stdout every N characters, for hand engraving; `0` = unbroken (default 0). `--out` receives the same bytes; `--json` stays unbroken. Separator-stripping on intake means a grouped or unbroken card both re-ingest on `decode`/`repair`/etc. |

Line 42 (`md encode --separator` row) becomes:

    | `--separator <SEPARATOR>` | the separator for `--group-size`: `space` (keyword) or the literal `" "` (default `space`). Whitespace only — `hyphen` and `comma` are no longer accepted |

Line 99 (`--in` row): replace "so a card copied off the engraving card re-ingests" with "so a card copied in
grouped form re-ingests".

Line 220 (`md descriptor --group-size` row) becomes:

    | `--group-size <N>` | with `--emit md1`: group each md1 string on stdout every N characters (default 0 = unbroken), exactly as `md encode --group-size` |

Line 221 (`md descriptor --separator` row): replace "the card's separator" with "the separator for `--group-size`".

### 8b. Toolkit technical manual and GUI manual, land in step 4 (the toolkit's one commit)

- `docs/technical-manual/transcripts/md1-encode-wpkh-basic.out` (cmd `$MD_BIN encode 'wpkh(@0/<0;1>/*)'`) becomes
  exactly `md1yqpqqxqq8xtwhw4xwn4qh\nnote: stdout is a keyless descriptor template (no keys)\n` (measured, GREEN
  build, stdout and stderr merged as the transcript records them).
- `docs/manual-gui/src/50-md/53-encode.md:233-235`: "The stderr engraving card, the chunk-set-id and the advisories
  are unaffected." becomes "The file holds exactly what standard output would (grouped under `--group-size`); the
  chunk-set-id and the advisories are unaffected."
- `docs/manual-gui/src/50-md/5E-descriptor.md:166`: "with its engraving card on stderr" is deleted (the sentence
  ends "minted from the seating result."); line 178-179: "Groups the engraving card on stderr." becomes "Groups
  each md1 string on standard output.", and "default `5`" becomes "default `0`".
- Review r1 I-3, more `docs/manual-gui/src/50-md/` sites (line anchors @cdedd8a2; the gui-manual lints check flag
  names, outlines and CLI pins, not defaults, so nothing else catches these):
  - `53-encode.md:23` becomes: "- [`--group-size`](#md-encode-group-size) — display grouping: group each md1 string
    on stdout every N characters (default 0 = unbroken)".
  - `53-encode.md:41-46` becomes: "Display-grouping control: print each `md1` string on stdout in groups of N
    characters. Number widget; range `0`–`65535`, default `0` (unbroken). The GUI renders the field unset with a
    `Set` button — leave it unset and the CLI applies its own default (`0`, unbroken); clicking `Set` activates a
    spin-box (which starts at the range minimum, `0`); set `5` for the five-character groups used for hand
    engraving."
  - `54-decode.md:21` and `5A-repair.md:63`: "so a card copied off the engraving card re-ingests" / "copied off an
    engraving card" becomes "so a card copied in grouped form re-ingests" (this FIX's own `--in` wording).
  - `5E-descriptor.md:51` becomes: "— group each md1 string on stdout (only with `--emit md1`)"; `:52` becomes: "—
    separator for `--group-size` (only with `--emit md1`)".

### 8c. mnemonic-gui (`src/schema/md.rs`), lands in step 3 (the gui release, AHEAD of the toolkit)

- `ENCODE_FLAGS` `--group-size`: `default_value: Some("0")`; `help: "Group each md1 string on stdout every N
  characters, for hand engraving (default 0 = unbroken). Cosmetic — intake strips separators."`
- `ENCODE_FLAGS` `--separator`: `help: "Separator for --group-size: `space` only (hyphen and comma were retired
  constellation-wide). Cosmetic — non-load-bearing."`
- `md encode --out` (line ~243): `help: "Write the md1 artifact to FILE (created 0600; OVERWRITES) instead of
  stdout; the file holds exactly what stdout would."`
- descriptor `--group-size` (line ~1017): `default_value: Some("0")`; `help: "Group each md1 string on stdout every
  N characters (0 = unbroken, the default). Only with --emit md1."`
- descriptor `--separator` (line ~1028): `help: "Separator for --group-size. Default space. Only with --emit md1."`
- `pinned-upstream.toml` `[md] tag` moves from `descriptor-mnemonic-md-cli-v0.20.3` to the release carrying this
  change; the gui builder runs `schema-mirror` and folds any other drift. `tests/part_b_forms.rs` and
  `src/form/conditional.rs` key on flag names, not defaults (unchanged).

### 8d. Other consumers (no edit required)

mnemonic-engrave journeys and `scripts/release-workflows/emit.py`, seedhammer `oracle/`, toolkit
`cli_cross_tool_differential.rs` and the md `seating/generate.sh` pass `--group-size 0` and read stdout only;
`--group-size 0` stays valid and now equals the default. `transcript_composer.sh:252`'s comment ("--group-size 0
also keeps the stderr echo unbroken") becomes moot, not false in effect.

## 9. Proposed FOLLOWUPS lines

None new for md beyond `mk-out-watch-only-note-says-stdout` (in `design/FIX_mk_single_print.md` section 9; it
names md too).

## 10. Not covered, STOP

- Not covered: bitcoind-differential, man-pages, release, musl, FreeBSD, fuzz jobs (not run); reviewer judgement of
  P1-P5.
- STOP: none. No consumer parses the stderr card.
