# Phase 8 — BIP Test Vectors section update (Task 8.8)

**Status:** DONE
**Commit:** `a4709fa`
**File(s):** `bip/bip-wallet-descriptor-mnemonic.mediawiki` (lines 637–658, modified)
**Role:** controller (controller-direct edit; no subagent)

## Summary

Replaced the placeholder "Test Vectors" + "Reference Implementation" sections of the BIP draft with concrete content now that the reference impl exists and the v0.1 vectors JSON is committed.

## Specifics added

- **Permalink** to the JSON file at the exact commit (`e2e8368e51618ad82073c46faa7799fddb86e082`) so the link is stable across future history rewrites:
  `https://github.com/bg002h/descriptor-mnemonic/blob/e2e8368e51618ad82073c46faa7799fddb86e082/crates/wdm-codec/tests/vectors/v0.1.json`
- **SHA-256 content hash** for content-addressed verification: `1957b542ed0388b51f01a7b467c8e802942dc6d6507abffaefaf777c90f3cd2c`
- **Schema version** annotation: 1 (with `#[non_exhaustive]` documented for forward compat)
- **Vector count breakdown**: 10 positive (corpus C1–C5, E10, E12, E13, E14, Coldcard) + 30 negative (one per Error variant)
- **Two-placeholder caveat**: explicitly documents that EmptyChunkList and PolicyTooLarge negative vectors have empty `input_strings` because their triggers don't fit the WDM-string CLI input shape (they're library-API reachable only)
- **Generator + verifier commands**: documents how to regenerate (`gen_vectors --output ...`) and verify (`gen_vectors --verify ...`) the file
- **Reference Implementation** section updated to point at the actual repo, lists the CLI surface (`wdm encode/decode/verify/inspect/bytecode/vectors`), and notes the in-flight upstream miniscript PR for hash-terminal support (`apoelstra/rust-miniscript#1`)

## Concerns

- The permalink resolves to GitHub only AFTER the user pushes the local branch to the public repo. Until then, the URL 404s. This is the standard expectation for a draft BIP linking its own ref impl; user awareness on next push handles it.
- A v0.2 vector regeneration will need to update the permalink + content hash here. P10 includes a "verify the BIP draft references current artifacts" check.

## Follow-up items

None from this task; the schema-versioning + content-hash convention is now documented in the BIP.
