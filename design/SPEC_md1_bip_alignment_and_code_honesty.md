# SPEC — md1 BIP↔code alignment + md-codec/md-cli honesty fixes

**Repo:** `descriptor-mnemonic` (md-codec + md-cli). Source SHA at authoring: `origin/main ef1f3e71`.
**Origin:** Fable adversarial BIP-vs-impl review (`mnemonic-toolkit/design/agent-reports/bip-review-md1-fable-r0.md`); consolidated bug list `mnemonic-toolkit/design/BUGLIST_bip_alignment_cycle_2026-07-10.md`.
**R0 history:** round 1 `…/md1-bip-alignment-spec-r0-round-1.md` (1C/4I — folded); round 2 `…/md1-bip-alignment-spec-r0-round-2.md` (GREEN on the F-A1b-incl. spec, superseded by the user redirect); round 3 `…/md1-bip-alignment-spec-r0-round-3.md` (1I/3M post-F-A1b-drop — folded here). All under `mnemonic-toolkit/design/agent-reports/`.
**User directives (2026-07-10):** "BIP-alignment cross-repo cycle now"; execution order **BIPs → bugs → test-hardening → minors → docs**; A4 scope = **HYBRID** (permissive decode + calibrated encode advisory + re-scoped BIP); unimplemented MUSTs = **make BIP honest + downgrade ledger** (DG-1…DG-5).
**Companion cycle:** mk1 (`mnemonic-key`) — separate SPEC, same program.

## Goal

Make an independent implementer following only `bip/bip-mnemonic-descriptor.mediawiki` reconstruct cards byte-identically to the shipped `md-codec`, and fix the live code bugs the review reproduced. **The BIP is the normative spec: it states the *correct target* behavior; the code bugs are the code failing to meet it.** Per the user's BIPs-first order, the BIP prose lands first (stating the target), then the code conforms, then the BIP §Test Vectors table is finalized against the regenerated corpus. **No wire-format change; no version-byte bump.** Existing decodable cards remain decodable (recovery-safety invariant, R0-verified round 1).

## Non-goals (deferred to FOLLOWUPs — downgrade ledger DG-1…DG-5)

Erasure-aware BCH decoding, guided/constrained recovery search, confidence-tier reporting, and reinstating the long code are OUT. This cycle makes the BIP *honest* about their absence; implementation is future work. The BCH **substitution**-error correction that `md repair` genuinely performs (BM+Forney, t=4, `chunk.rs:536-601`) stays normative. (DG-5 non-zero-pad is NOT deferred — F-A8 implements it.)

---

## Part 1 — Code fixes (md-codec / md-cli), TDD — the "bugs" phase (executes AFTER Part 2 BIP prose)

### F-A1. `sh(wpkh)` elided-origin self-rejecting card (funds/usability) — VERIFIED
**Now:** `md encode 'sh(wpkh(@0/<0;1>/*))'` (no `--path`) → `md1yqpqqxpsq258xsks3kh0ye`; `md decode` rejects it (`non-canonical wrapper requires explicit origin for @0`). Root cause: `canonical_origin()` (`canonical_origin.rs:65-75`) returns `None` for `sh(wpkh)` (inner tag is `Wpkh`, not `Wsh`).
**Fix:** add an arm to `canonical_origin()`: `(Tag::Sh, Body::Children([inner])) if inner.tag == Tag::Wpkh` (single-key body) ⇒ `Some(m/49'/0'/0')` (standard BIP49 nested-segwit single-sig).
**Correct symmetry mechanism (R0 I-1 fix):** the wire encoder does NOT consult `canonical_origin` (`encode.rs:119` writes `path_decl` verbatim; md-cli leaves it empty absent `--path`). Round-trip symmetry arises because the **decode gate** `validate_explicit_origin_required` (`validate.rs:222`) becomes a no-op once `canonical_origin` returns `Some`. Three consumers flip together, all must be accounted for:
  1. `validate.rs:222` — decode-side `MissingExplicitOrigin` gate → no-op ⇒ elided sh(wpkh) now decodes.
  2. `canonicalize.rs:464-469` — `expand_per_at_n`'s `MissingExplicitOrigin` gate → no-op.
  3. `identity.rs:176-220` — `compute_wallet_policy_id`'s L14 canonical-fill → the WalletPolicyId (12-word anchor) now computes for elided sh(wpkh) and MUST equal the explicit-49′ form's.
**Recovery-safety (R0-verified):** only ADDS a decodable elided form; removes no existing decode path (explicit-origin cards always decoded and still do — the gate no-ops only when `Some`); no existing card's wire bytes change (no encoder collapse of explicit==canonical).
**Tests (RED-first):** unit `canonical_origin(sh(wpkh(@0))) == Some(49'/0'/0')`; CLI e2e (the VERIFIED repro now round-trips); **policy-id equality** — `WalletPolicyId`/12-word phrase of elided sh(wpkh) == explicit-`m/49'/0'/0'` form; `compute_wallet_policy_id` on elided sh(wpkh) flips error→success; existing explicit-49′ sh(wpkh) card still decodes byte-identically.

### F-A1b. Self-rejecting-card CLASS — DEFERRED to a separate brainstorm (user 2026-07-10)
**Status: OUT of this cycle.** R0 C-1 verified that `md encode` mints un-decodable "dead cards" for EVERY `canonical_origin=None` shape without `--path` (`tr(@0,pk(@1))` → `md1yppqqxqj2s4dk6hk0wrt5n3`; `wsh(or_d(...))` → `md1yppqqxpxpg2srvh08vnhjktj2`; `sh(sortedmulti(2,…))` → `md1yppqqxp3cg2x3r70ckk4kjaf` — all decode-reject). **User direction:** do NOT add a hard encode refusal. Instead redesign pathless/non-canonical backup as (1) a loud encode-time advisory (encoder still emits) + (2) a decoder that partial-decodes with PLACEHOLDERS for unsupplied keys/paths, instead of rejecting `MissingExplicitOrigin`. Both halves are a **separate brainstorm** (`pathless-wallet-backup-partial-decode`), coupled encoder+decoder, own spec→R0→impl.
**Consequence for THIS cycle:** the C-1 class stays OPEN (tracked in the brainstorm). This cycle does NOT change encode behavior for the non-canonical class and does NOT change the decoder's current `MissingExplicitOrigin` reject. `sh(wpkh)` still leaves the class via F-A1 (it has a real canonical path). B-C2 (below) documents the canonical-default table + marks the non-canonical/pathless case "under separate design," NOT MUST-reject/MUST-refuse.

### F-A2. `md decode` cannot read chunked cards (recovery/usability) — VERIFIED
**Now:** `md decode <chunk-string>` → `wire-format version mismatch: got 9, expected 4`; `md repair <same>` round-trips. `cmd/decode.rs` routes `len==1` → `decode_md1_string` (no chunked-bit dispatch); `decode_with_correction` (chunk.rs:604) already auto-dispatches.
**Fix:** in the `len==1` path, peek the first symbol's chunked flag (`[v3][v2][v1][v0][chunked]` MSB, `chunk.rs:4/147`; `header.rs:1-10,27`) and if set route the single string through `reassemble` (a 1-element set — accepts a 1-slice per `chunk.rs:311-357`), mirroring `decode_with_correction`'s single-string auto-dispatch. Preferred locus: make `decode_md1_string` itself dispatch (every library caller + the CLI benefit), `decode_payload` stays the non-dispatching primitive. No recursion cycle (`reassemble` → `decode_payload`).
**Recovery-safety (R0-verified strictly additive):** usable version set {4,8,12} is all-even ⇒ every currently-valid single-payload string has first-symbol LSB=0, so dispatch never diverts a currently-decoding input; a stray chunk-i-of-N correctly yields `ChunkSetIncomplete`.
**Tests:** VERIFIED repro round-trips via `md decode`; genuine ≥2-chunk via multi-arg `md decode` still works; non-chunked unchanged; `got 9` no longer reachable for a well-formed chunked string.

### F-A3. `--force-long-code` silent no-op — VERIFIED. **RULING (R0): hard error, keep the flag in clap.**
**Now:** `encode.rs:109-113` deliberately ignores the flag; long code dropped v0.12.0.
**Fix:** passing `--force-long-code` → exit≠0 with "the long BCH code was removed in v0.12.0; md1 is regular-code-only (payloads >400 bits are chunked)". Keep the flag in the clap surface (no flag-NAME removal → manual flag-coverage lint stays green; help-text update ripples to the manual chapter only — **md-cli flag, manual-only ripple, NOT GUI schema_mirror** per R0 M-1). Matches "refusals exit≠0"; the long code is excised from the BIP (DG-4) so a flag referencing a nonexistent mode must not exit 0.
**Tests:** `md encode … --force-long-code` exits non-zero with the message; without the flag unchanged.

### F-A4. Hybrid scope: permissive decode + calibrated encode advisory. **RULING (R0): pkh silent.**
**Decode:** unchanged — `pkh`, `sh(multi)`, `sh(sortedmulti)`, `sh(wpkh)`, `sh(wsh(...))` all keep decoding (recovery-safety; `pkh_basic` stays in the corpus).
**Encode advisory (new, `cmd/encode.rs`), warn-only, no new flag:**
- **Footgun warning** for top-level bare `sh(multi(...))` / `sh(sortedmulti(...))` (legacy P2SH multisig): third-party txid malleability, 520-byte redeemScript ≤~15-key ceiling, no witness discount, superseded by `wsh`/`sh(wsh)`.
- **pkh: SILENT, no note** (R0 ruling) — funds-safe, canonical BIP44 default, ships in corpus, toolkit emits bip44; a note would fire on every legitimate encode, churn manual transcripts, and dilute the sh(multi) signal.
- Advisory → **stderr** only (stdout stays the card). Must not fire on safe modern forms (`wsh(multi)`, `wpkh`, `tr`, `pkh`).
- **Must fire on BOTH code paths (R0 M-3):** `cmd/encode.rs` has an early-return `--json` branch (:51-82) plus the normal branch (precedent: `emit_output_class_advisory` fires in both).
**Tests:** stderr carries the footgun advisory for `sh(sortedmulti)` (test with `--path` for a clean round-tripping card) AND for the `--json` branch; stdout byte-identical to no-advisory; NO advisory for `wsh(multi)`/`wpkh`/`tr`/`pkh`; advisory never on stdout (`cli_output_class`-style assertion). (Advisory is orthogonal to the deferred F-A1b dead-card class — it keys on script TYPE, not the missing-path case.)

### F-A8. Non-zero trailing-pad rejection (BIP cites a fictional error) — implement, don't downgrade
**Now:** `tlv.rs:286-302` rolls back ≤7 remaining bits **without** checking they are zero (R0-verified value-blind); BIP line 250 cites `Error::InvalidBytecode{ kind: BytecodeErrorKind::MalformedPayloadPadding }` which does not exist.
**Fix:** implement the zero-check on the ≤7 trailing bits; on non-zero, return a real error variant appended to md-codec `Error` (insertion-ordered, NOT alphabetical — R0-confirmed `error.rs:20`ff). Name it to match the BIP cite (B-I4 uses the real name verbatim).
**Recovery-safety (R0-verified):** the reference encoder always zero-pads (BitWriter + `wrap_payload`) ⇒ rejects no well-formed card; only malformed non-zero-pad inputs (previously silently decoded) now error.
**Tests:** hand-crafted non-zero-pad card → the new error; every corpus card still decodes; encoder output always zero-pads.

### F-A5. `bch.rs` wrong init narrative (cosmetic) — R0 M-2: rewrite the WHOLE block
`bch.rs:19-31` — rewrite the entire narrative: the "deliberately NOT codex32/BIP-93's initial residue `1`" sentence AND the "Only `ms1` must use `1`" claim are both wrong (BIP-93's `ms32_polymod` init IS `0x23181b3`, reviewer-verified vs source). Keep byte-consistent with the mk-codec A6 companion fix. No behavior change.

### F-A9. `TooManyErrors` message conflates 2t=8 with t=4 (R0 M-4)
`error.rs:421-427` user-facing text "more than 8 errors" conflates detection radius 2t=8 with correction capacity t=4 — the same myth the cycle fixes on the mk side (A6/mk1-I2). Fix the message to correction capacity (t=4). Cosmetic; keeps the BIP's kept-normative correction claims from citing a wrong number.

---

## Part 2 — md1 BIP → code alignment (`bip/bip-mnemonic-descriptor.mediawiki`) — the "BIPs" phase (executes FIRST)

Doc-only edits stating the correct normative (target) behavior; the code conforms in Part 1. Itemized cites: review + BUGLIST bucket B. **The §Test Vectors *table/pins* (part of B-I6) finalize in Part 3, after the corpus is regenerated (R0 I-4).**

- **B-C1 (=D1) §Chunking rewrite:** fragments = whole bytes of the byte-padded assembled payload; header/fragment boundary at bit 37 (contiguous, no slack); reassembly concatenates fragment BYTES. Delete "encoder-chosen bit boundaries", "Decoders MUST accept any valid division", and the 3-bit-slack framing (lines 201/246/306/773/786). Fix the fragment-max arithmetic. Contrast mk1's fixed-53-byte framing so implementers don't cross-import.
- **B-C2 §Elided-origin / canonical-default:** specify the settled part normatively — depth-0 origin ⇒ resolve via the canonical-origin table (reproduce `canonical_origin.rs` incl. the NEW `sh(wpkh)`→49' arm): pkh→44', wpkh→84', sh(wpkh)→49', tr-keyonly→86', wsh(multi/sortedmulti)→48'/2', sh(wsh(multi/sortedmulti))→48'/1'. Reconcile the §Default-derivation-paths table (add pkh→44', sh(wpkh)→49'; correct the sh set). Delete "The path is encoded explicitly". **For shapes with NO canonical entry (tr+tree, sh(sortedmulti), bare wsh, miniscript): the BIP text says ONLY "pathless/non-canonical backup handling is under separate design; the current reference decoder rejects `MissingExplicitOrigin`."** Do NOT name the candidate mechanism (advisory + partial-decode-with-placeholders) in the public BIP — that design has not converged; keep it in the `pathless-wallet-backup-partial-decode` FOLLOWUP/brainstorm doc only (R0 M-B). Do NOT lock in MUST-reject/MUST-refuse (user 2026-07-10). No encoder-MUST.
- **B-C3 §Long code excision:** remove the long-code definition/constants/guarantees/envelope-rows/`ms32_long_polymod` refs/"94 or 95 invalid". Restate the cap: ≤80 data symbols / ≤93 codeword symbols (HRP excluded) / payload ≤400 bits, chunk above. (Ledger DG-4.)
- **B-C4 §Top-level scope (hybrid):** delete the "MUST reject pkh/sh(multi)" scope MUSTs + the sh-wrapper-matrix MUSTs + the FAQ "rejects sh(multi) at spec level" claim. Document permissive decode + the encode-time advisory model (F-A4); pkh fully supported (BIP44 default).
- **B-I1 auto-dispatch:** make in-band auto-dispatch normative (delete the "by reader role" contradiction, line 330); add the `got 9` mis-parse row to the §2.5 trace table; note `md decode` dispatches (F-A2).
- **B-I2 PolicyId/WalletInstanceId:** rewrite to the ACTUAL `WalletPolicyId` preimage (`identity.rs:141-186`) — the 12-word anchor is `WalletPolicyId::to_phrase()`; reconcile `Md1EncodingId ≠ PolicyId` line 217 vs 798; drop the un-computable 78-byte `bip32_serialize` concat (wire carries 65-byte entries); fix the fingerprint API cite.
- **B-I3 decoder-side validators:** enumerate every decode-rejection rule (PlaceholderNotReferenced, PlaceholderFirstOccurrenceOutOfOrder, MultipathAltCountMismatch, Baseline/RedundantUseSiteOverride, tap-leaf forbidden-tag set, MissingExplicitOrigin, InvalidXpubBytes).
- **B-I4 non-zero padding:** cite the REAL error variant from F-A8; state the ≤7-bit zero-check rule.
- **B-I5 erasure/guided-recovery/confidence — DOWNGRADE (DG-1/2/3):** MUST→SHOULD/informative. Keep substitution-correction (honest). Note: code distance supports N erasures; the reference decoder implements substitution-error correction only; erasure-aware decoding + guided recovery + confidence tiers are future work (cite FOLLOWUP slugs).
- **B-I6 §Test Vectors machinery:** correct the machinery description (`md vectors --out` / `src/test_vectors.rs::MANIFEST`, not `--test vectors`; four files not three; the extra `chunk-set-id:` line; the NEW `path` field per I-3). **Table/pins finalized in Part 3.**
- **B-I7 unknown-TLV:** specify preserve-verbatim (incl. re-emission ordering) as normative.
- **B-I8 varint canonicality — RULING (R0): encoder-MUST minimal + decoder-lenient, documented.** Mandate minimal-length encodings for encoders; state the decoder's leniency (NO decode-side minimality check — it would flip a previously-decodable non-minimal single-string wire to reject, violating recovery-safety). Add: non-minimal **chunked** wires already fail closed via the CSI mismatch.
- **B-I9 chunk-count sizing:** document the 320-bit budget + `--force-chunked` CLI requirement (SHOULD; no code change).
- **B-M1…M8:** the minors (init comment→F-A5; 12-word=132 bits; stale/fabricated refs; stale rationale; encoder-vs-decoder error framing; BIP-2 preamble; erasure-table citation; canonical-bytecode uniqueness caveat).

## Part 3 — Test vectors + BIP §Test Vectors finalization (code phase → then BIP sync)

**Corpus machinery change (R0 I-3):** add `path: Option<&'static str>` to the `#[non_exhaustive]` `Vector` struct (`test_vectors.rs:13-32`) + `md vectors --out` runner support (`cmd/vectors.rs:30-38`). This unblocks vectors for non-canonical shapes (NUMS-taproot, tr_with_leaf) and enables un-omitting them.
**Path surfacing in emitted files (R0 M-7):** the four emitted files are `.template`/`.bytes.hex`/`.phrase.txt`/`.descriptor.json`; a path-carrying vector is under-determined by `.template` alone (path enters via `path_decl`, carried in `.descriptor.json`). The BIP §Test Vectors table MUST pin **template+path pairs** for the NUMS/tr_with_leaf rows so acceptance #4's independent-reader reproduction is unambiguous.

- **F-V1.** A genuine **≥2-chunk** vector (the gap that hid D1) — post-fix bytes; pin per-chunk strings + `chunk-set-id`.
- **F-V2.** A **94–96-char single-string boundary** vector (the C3 band) proving the regular-only cap.
- **F-V3.** Un-omit **`sh_wpkh`** (now round-trips via F-A1, elided) + un-omit **`tr_with_leaf`** (now expressible via the `path` field, explicit origin) + add a **NUMS-taproot** vector (`tr(NUMS_H,{…})`, explicit origin — `is_nums=1` wire path currently vectorless).
- Regenerate the corpus via the real `md vectors --out`; every existing vector round-trips unchanged; **`sh_wpkh` is an ADDITION** (R0 M-5 — currently absent from MANIFEST, not a change).
- **THEN** re-sync the BIP §Test Vectors table/pins to the final regenerated corpus (R0 I-4); acceptance #4's independent-reader check runs against the FINAL corpus.

---

## Ripple / lockstep (per CLAUDE.md)

- **manual mirror** (`docs/manual/` — TOOLKIT repo): md-cli surface changes (A2 behavior, A3 error message, A4 advisory) → update `40-cli-reference`; `verify-examples` reruns live cmds. **md-cli flag help-text = manual-only ripple** (R0 M-1). **M-D:** CI's `MD_BIN` is tag-pinned at `descriptor-mnemonic-md-cli-v0.11.2` (`.github/workflows/manual.yml:86`); any transcript demonstrating NEW behavior (A3 hard error, A2 chunked decode) reds the manual gate until manual.yml's tag bumps to the new md-cli release (doc-verification-only; independent of the deferred toolkit lib re-pin). **Bump manual.yml's tag, NEVER `scripts/install.sh:35`'s FROZEN md-cli sibling pin** (v0.11.2 baseline, policed by sibling-pin-check — the v0.75.0 revert precedent). Help-text-table-only updates are un-gated by `lint.sh` and can land without the bump; only new-behavior transcripts are blocked. Coordinate toolkit-side.
- **GUI `schema_mirror`:** mirrors only the TOOLKIT's `mnemonic gui-schema` surface — an md-cli flag/help change does NOT touch it. A3-error / A4-warn-only add NO flags → no mirror change either way.
- **toolkit re-pin — DEFERRED to a tested follow-up (R0 I-1/I-2 + M-6/M-A):** toolkit consumes md-codec as a lib; A1/A2/A8 are behavior changes. **This cycle does NOT re-pin the toolkit** (see the DECISION below) — it stays on its current md-codec pin. When the re-pin follow-up runs it must: bump the pin, re-vendor, run the FULL toolkit suite, check `.examples-build` corpus + manual verify-examples, AND handle the two flip tiers: **F-A1 makes `canonical_origin(sh(wpkh))` flip `None`→`Some`. This flips two distinct toolkit surfaces at re-pin (R0 round-3 I-1):**
  **(A) Comment/routing tier — NO runtime flip, but stale comments that must NOT be "simplified" into reversing a pinned funds-path refusal.** KEEP the bip49 template refusal pinned; update:
  - `synthesize.rs:349-368` (`cli_template_from_tree`, "Mirrors `canonical_origin`") + `:1092-1127` (`template_admissible`, bip49 pinned REFUSED) — state the now-intentional divergence; **do NOT add sh(wpkh)→Bip49**.
  - `synthesize.rs:50-52` (Md1Form doc "REQUIRES `canonical_origin(&tree).is_some()`" — now false; real gate is `cli_template_from_tree`).
  - `restore.rs:303` (comment lists 3 clauses; code `:317-320` has 4 — the omitted `cli_template_from_tree` conjunct is the ONLY thing keeping an elided sh(wpkh) md1 out of the single-sig route).
  - `cli_bundle_md1_template_form.rs:239` (refusal rationale stale; test stays green).
  - `error.rs:343-349` (`TemplateFormUnsupportedShape` doc defines the class as "`canonical_origin` is `None` … e.g. bip49"; post-F-A1 bip49 is `Some` yet still refused → doc turns actively wrong — R0 M-A).
  - `cmd/gui_schema.rs:1317-1320` (`gui-schema --classify-descriptor 'sh(wpkh(...))'` verdict flips `non-canonical`→`canonical` — semantically correct; ADD an sh(wpkh)→`canonical` cell to `tests/cli_gui_schema_classify_descriptor.rs`).
  **(B) Descriptor-mode canonicity probes — a REAL runtime flip, one WALLET-CHANGING, ZERO test coverage** (`bundle.rs:1416-1418`, `verify_bundle.rs:1408-1414`; `is_non_canonical = canonical_origin(tree).is_none()`). At re-pin, F-A1 flips FOUR behaviors: (1) `mnemonic bundle --descriptor "sh(wpkh(...))"` silently switches its default origin from the (buggy) `m/48'/0'/0'/1'` to canonical `49'` → **same command, different wallet** (notice vanishes; `bind_descriptor_mode_paths` early-returns, `bundle.rs:2262-2266`); (2) `--account≠0` flips succeed→refuse (§4.12.g guard, `bundle.rs:1421-1427`); (3) `--slot @N.path=` override flips succeed→refuse (§6.6 row-4, `bundle.rs:1432ff`); (4) a pre-re-pin elided-sh(wpkh) bundle fails verify post-re-pin (fail-LOUD, not false-pass). None seed-unsafe (old bundles carry explicit origins, still recover), and 49' is the *correct* BIP49 target (the old 48'/0'/0'/1' single-sig default was itself a bug) — but it IS the same-command-different-wallet class.
  **DECISION (opus fold): DEFER the toolkit re-pin to a deliberate, tested follow-up cycle.** This md-codec/md-cli cycle ships F-A1/A2/A3/A5/A8/A9 + the BIPs; the toolkit stays on its current md-codec pin (no rushed wallet-changing flip). The re-pin follow-up (FOLLOWUP `toolkit-repin-sh-wpkh-canonical-flip`) will: land the tier-(A) comment updates, add descriptor-mode tests (elided-sh(wpkh)-descriptor→49'/no-notice, `--account≠0`→refuse, `[Phrase,Path]`→refuse), a CHANGELOG + manual note, and a migration note (old elided-mode bundles verify via their inline `[fp/48'/0'/0'/1']@0` origins since the `--slot @N.path=` route now refuses). File FOLLOWUPs `canonical-origin-sh-wpkh-toolkit-mirror-divergence` + `toolkit-repin-sh-wpkh-canonical-flip` in BOTH repos.
- **crates.io:** md-codec + md-cli publish in lockstep on tag.
- **Examples corpus (`.examples-build/`)** is a lockstep version site (toolkit).

## Acceptance criteria (per-phase R0 + post-impl whole-diff)

1. VERIFIED code bugs in scope fixed (F-A1, F-A2, F-A3, F-A8) with RED-first tests. (F-A1b / the broad non-canonical dead-card class is DEFERRED to the pathless-wallet brainstorm — NOT a gate item here; tracked OPEN.)
2. F-A1 policy-id-equality (elided==explicit-49′) + all-three-consumers pinned; full `cargo test -p md-codec` + `-p md-cli` green (R0 runs the FULL package suite).
3. Every pre-existing corpus card still decodes (recovery-safety); F-V1/V2/V3 (incl. NUMS + tr_with_leaf via the new `path` field) added + round-trip; corpus regenerated; BIP §Test Vectors synced to the FINAL corpus.
4. BIP text: no remaining bit-vs-byte / auto-dispatch / 93-cap / scope-vs-corpus contradiction; downgrades ledgered with FOLLOWUP cites; independent-reader spot-check (re-dispatch a Fable read of the rewritten §Chunking + §Elided-origin) reproduces the corpus.
5. FOLLOWUPs filed (both repos per cross-repo rule): DG-1 `impl-bch-erasure-decoding-md-mk`, DG-2 `impl-guided-recovery-md-mk`, DG-3 `impl-confidence-tier-reporting-md-mk`, DG-4 `reconsider-md1-long-code`, `canonical-origin-sh-wpkh-toolkit-mirror-divergence`, `toolkit-repin-sh-wpkh-canonical-flip` (the DEFERRED tested toolkit re-pin carrying the sh(wpkh) descriptor-mode wallet-flip — R0 round-3 I-1), + `pathless-wallet-backup-partial-decode` (the deferred F-A1b class — encode advisory + decoder partial-decode-with-placeholders; OPEN until that brainstorm ships).
6. Release ritual complete; CI green; toolkit re-pin done (incl. the synthesize.rs comment update) or explicitly deferred.

## Phasing (BIPs → bugs; R0 I-4 ordering fixed)

Global program order is **BIPs → bugs → …**; within this repo:

- **Phase 1 — BIP prose (first):** B-C1, B-C2 (canonical-default table + non-canonical/pathless marked "under separate design"; NO encoder-MUST), B-C3, B-C4, B-I1, B-I2, B-I3, B-I4 (real error name TBD-in-P2, use a placeholder + finalize), B-I5 (downgrades), B-I6 machinery, B-I7, B-I8, B-I9, B-M1-M8. States target behavior; no code yet.
- **Phase 2 — code/bugs:** P2a F-A1 (sh(wpkh) canonical arm — F-A1b is DEFERRED to the pathless-wallet brainstorm, NOT in this phase); P2b F-A2 (decode dispatch); P2c F-A3 + F-A8 (+ finalize B-I4's error name) + F-A5 + F-A9; P2d F-A4 (advisory); P2e the `Vector.path` machinery + generate F-V1/V2/V3 + regen corpus. Per-phase R0; RED-first.
- **Phase 3 — BIP §Test Vectors sync:** re-sync the BIP vector table/pins to the final corpus; run the independent-reader acceptance check; FOLLOWUPs; release ritual (md-codec + md-cli lockstep publish + tag) + bump `manual.yml`'s `MD_BIN` tag to the new md-cli release (M-D). **Toolkit re-pin is NOT in this cycle** (deferred — see the ripple DECISION; file the FOLLOWUPs).

Post-impl whole-diff review over the combined diff before release.
