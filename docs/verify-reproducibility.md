# Verifying reproducibility of the `md` musl release binaries

> **Scope.** This document covers the **x86_64-unknown-linux-musl** and
> **aarch64-unknown-linux-musl** `md` CLI binaries published on each
> `descriptor-mnemonic-md-cli-v*` release. It is the per-binary verify recipe
> for descriptor-mnemonic's leg of the constellation **`reproducible-builds-musl`**
> cycle (toolkit-led — `mnemonic-toolkit`). The hermetic build environment, the
> gate scripts, and the digest-pinned container are **homed in the toolkit**
> (`bg002h/mnemonic-toolkit`); this repo commits only its own `vendor/` tree, a
> ~15-line caller stub (`.github/workflows/man-pages.yml` `repro` job), the
> re-homed `musl-binaries` build, and `Cross.toml`.
>
> **`md` is fork-free.** Unlike the toolkit, `md` does NOT depend on the
> miniscript `[patch.crates-io]` git fork (it uses the crates.io
> `miniscript = "13.0.0"`). Its `cargo vendor` graph is a clean crates.io graph,
> so the `[source]` activation is the **TWO-block** form (crates-io +
> vendored-sources) — there is **no git-fork `[source]` stanza** here.

## 1. What reproducibility buys you — provenance, not just integrity

The published `SHA256SUMS.<arch>` next to each release tarball lets you confirm a
download matches **what the maintainer uploaded** (integrity). It does NOT, on
its own, tell you the maintainer built it **from the claimed source**. A
*reproducible* build closes that gap: if you rebuild from the exact source at the
exact tag in the exact pinned environment and get the **bit-for-bit identical**
tarball, the published hash becomes a **provenance** statement — "provably built
from this source@commit" — verifiable by anyone, with no trust in the
maintainer's machine.

## 2. Source — keyed off the COMMIT SHA (not the tag name)

```sh
git clone https://github.com/bg002h/descriptor-mnemonic
cd descriptor-mnemonic
git checkout <descriptor-mnemonic-md-cli-vX.Y.Z>   # the release tag
git rev-parse HEAD                                  # MUST equal published source_commit
```

**Invariant.** The published hash is valid **only** for the tuple
`{ source COMMIT SHA + SOURCE_DATE_EPOCH + container digest }`. A tag is mutable;
if it is moved / re-cut / force-pushed, the tagged-commit timestamp changes, the
epoch changes, the binary changes, and the previously-published hash is
**invalidated and re-published**. Always verify against the **commit SHA**
published in `PROVENANCE.<arch>.txt`, not the tag name.

`SOURCE_DATE_EPOCH` is derived identically by the maintainer's CI and by you:

```sh
SOURCE_DATE_EPOCH=$(git show -s --format=%ct <tagged-commit-SHA>)
```

`%ct` is the **committer date of the exact tagged commit** — NOT the tag name,
NOT an annotated tag's own `%(taggerdate)`.

## 3. The pinned environment — `docker pull` the container BY DIGEST

The x86_64 build runs inside a **digest-pinned** container homed in the toolkit
(`Dockerfile.repro` in `bg002h/mnemonic-toolkit`). The **built, layered image is
the source of truth** and is published by digest to GHCR — you `docker pull` it,
you do **not** rebuild `Dockerfile.repro` from apt (the transitive `musl-tools`
apt deps are NOT pinned by the base image digest and would resolve to whatever
the Debian mirror serves that day).

```sh
# From PROVENANCE.x86_64.txt:
CONTAINER_IMAGE=ghcr.io/bg002h/repro-musl@sha256:<BUILT-DIGEST>
docker pull "$CONTAINER_IMAGE"      # no auth needed — the package is PUBLIC
```

> **Maintainer one-time setup — the `repro-musl` GHCR package MUST be Public.**
> GHCR container packages are **private by default**, and an external rebuilder
> pulling by digest does so **without a token** — so the package has to be
> public for this provenance model to work. The toolkit's
> `reproducible-musl-build.yml` `build-container` job attempts to self-promote it
> to public (it is `|| true`, never hard-failing). If the self-promotion does not
> take, an admin sets it **once** by hand on the toolkit repo: GitHub →
> Packages → `repro-musl` → Package settings → Danger Zone → Change visibility →
> Public.

- **Base image** (recorded in the toolkit's `Dockerfile.repro`): the official
  `rust:1.85.0` Debian image pinned by index digest
  `sha256:0ff31c9ffa641a62e48d543fb00b4960955ea375f40776f40f585b89e654cc5e`.
- **Built image** (`<BUILT-DIGEST>`): resolved + pushed by CI
  (`reproducible-musl-build.yml` → `build-container`) and recorded in each
  release's `PROVENANCE.x86_64.txt`.
- **Fallback only — building the container from `Dockerfile.repro`:** pin apt via
  `snapshot.debian.org` at a fixed timestamp + `apt-get install
  musl-tools=<exact-version>`. The canonical channel is
  `docker pull <built-digest>`, not a from-source rebuild.

### The fixed in-container layout

The container fixes `WORKDIR /build/src` and `CARGO_HOME=/cargo`. These fixed
literals are what make the `--remap-path-prefix` from-side a known constant
(`/build/src=/build`, `/cargo=/cargo`) — see §4.

## 4. The exact x86_64 build command + full env

Run **inside the pinned container**, at the fixed `/build/src`, with
`--network=none` (the build is fully offline against the committed `vendor/`
tree):

```sh
docker run --rm --network=none \
  -v "$PWD":/build/src -w /build/src \
  -e CARGO_HOME=/cargo \
  -e SOURCE_DATE_EPOCH="$SOURCE_DATE_EPOCH" \
  -e LC_ALL=C -e TZ=UTC \
  -e CARGO_BUILD_RUSTFLAGS="--remap-path-prefix=/build/src=/build --remap-path-prefix=/cargo=/cargo" \
  -e CFLAGS="-ffile-prefix-map=/build/src=/build -ffile-prefix-map=/cargo=/cargo" \
  -e CFLAGS_x86_64_unknown_linux_musl="-ffile-prefix-map=/build/src=/build -ffile-prefix-map=/cargo=/cargo" \
  "$CONTAINER_IMAGE" \
  bash -euxo pipefail -c '
    umask 022
    cargo build --locked --offline --release \
      --target x86_64-unknown-linux-musl -p md-cli --bin md \
      --config '"'"'source.crates-io.replace-with="vendored-sources"'"'"' \
      --config '"'"'source.vendored-sources.directory="vendor"'"'"'
    tar --sort=name --owner=0 --group=0 --numeric-owner --mtime="@$SOURCE_DATE_EPOCH" \
      -cf - -C target/x86_64-unknown-linux-musl/release md \
      | gzip -n -9 > md-<VER>-x86_64-linux-musl.tar.gz
  '
```

Notes on each load-bearing flag:

- **`--remap-path-prefix` is the *top-level* flag** (NOT `-Cremap-path-prefix`,
  which errors `unknown codegen option` on 1.85.0). It is delivered via the
  `CARGO_BUILD_RUSTFLAGS` **env** at the fixed `/build/src` — NOT a committed
  `.cargo/config.toml` value (a committed config value is passed to rustc
  verbatim with no `$PWD` expansion → it would no-op and give false assurance).
  This remap is the single biggest lever; it removes the absolute build-path
  leak in `.rodata` (a `file!()`/panic-`Location` literal) that makes default
  builds non-reproducible — and closes the `$HOME` privacy leak. (This is the
  exact remap the `reproducible-builds` phase-1 plan listed as the key
  path-reproducibility win and that the pre-cycle config had OMITTED.)
- **`CFLAGS` / `CFLAGS_<triple>` `-ffile-prefix-map`** strips absolute paths from
  the libsecp256k1 objects compiled by `cc-rs` under `musl-gcc`.
- **`SOURCE_DATE_EPOCH`** neutralizes `cc`'s `__DATE__` / `__TIME__`.
- **The TWO-block job-scoped `[source]` activation is mandatory.** The `vendor/`
  directory is committed but **inert** — there is **no committed `.cargo/config.toml
  [source]` block** (a repo-global `[source]` block would bleed into every other
  cargo job via cargo's directory-ancestry config discovery). The redirect is
  activated **job-scoped** on the build command via `cargo --config` (stable
  since 1.63). Because `md` is **fork-free**, only the two `cargo vendor`-emitted
  blocks are needed (crates-io + vendored-sources) — there is **no git-fork
  `source."git+…"` stanza** (that block exists only in the toolkit, which depends
  on the miniscript fork). An external rebuilder MUST pass the same two
  `--config` overrides (or use an isolated `$CARGO_HOME/config.toml` carrying the
  verbatim `cargo vendor` output).
- **`--locked --offline`** + the committed `vendor/` tree mean the compile
  touches **no live external registry or git host** at build OR vendor time.

This is the EXACT command the maintainer's CI runs (re-homed into the same
container — see `.github/workflows/man-pages.yml` `musl-binaries` x86_64 leg).

## 5. Expected per-artifact SHA-256 + the provenance tuple

Each release attaches, per arch:

- `md-<VER>-<arch>-linux-musl.tar.gz` — the static musl binary tarball.
- `SHA256SUMS.<arch>` — its SHA-256.
- `PROVENANCE.<arch>.txt` — the tuple. For x86_64:

  ```
  artifact:          md-<VER>-x86_64-linux-musl.tar.gz
  sha256:            <hash>
  source_commit:     <full 40-char SHA>
  source_date_epoch: <epoch>
  container_image:   ghcr.io/bg002h/repro-musl@sha256:<BUILT-DIGEST>
  ```

  For aarch64 the tuple cites the **`cross` image digest** instead (see §9):

  ```
  artifact:          md-<VER>-aarch64-linux-musl.tar.gz
  sha256:            <hash>
  source_commit:     <full 40-char SHA>
  source_date_epoch: <epoch>
  cross_image:       ghcr.io/cross-rs/aarch64-unknown-linux-musl@sha256:<CROSS-DIGEST>
  ```

## 6. Compare

```sh
sha256sum -c SHA256SUMS.x86_64        # OK ⇒ your rebuild matches the published artifact
```

On a mismatch, `diffoscope` the two tarballs. **Ignore** `target/.fingerprint`
and `.rustc_info.json` — they are non-reproducible cache artifacts, not part of
the shipped binary or tarball. Also check the gzip header: the mtime field
(offset 4–7) must be **zero** and the OS byte (offset 9) must equal the pinned
`03` (Unix) — a non-`-n` or divergent-gzip build would ship a different tarball
hash even with a byte-identical inner binary.

## 7. Scope honesty — local installs are NOT reproducible by default

Reproducibility is guaranteed **only** when building **at the fixed `/build/src`
inside the pinned container** with the env above. **`cargo install` /
`cargo install --git` at an arbitrary `$PWD` are NOT reproducible-by-default** —
they build at a path no static config canonicalizes, so the `.rodata` build-path
leak returns. (A future toolchain bump to Cargo-native `trim-paths`, which
self-canonicalizes, would restore local-install reproducibility; that is
nightly-gated on the deliberate 1.85.0 pin.)

## 8. CI proof — the two-distinct-path self-test (homed in the toolkit)

The toolkit's reusable workflow (`reproducible-musl-build.yml`, called by this
repo's `man-pages.yml` `repro` job) does not just rebuild once — it builds `md`
at **two distinct** real paths (`/build-a/src` and `/build-b/src`, both remapped
to `/build`) and asserts the two binaries + tarballs + libsecp `.o` are
byte-identical. **The two-distinct-path shape is load-bearing:** the
`--remap-path-prefix` from-side only *varies* — and the remap is only *proven
effective* — when the two real paths differ.

**The repro gate is runnable WITHOUT a release tag.** This repo's
`man-pages.yml` has a `workflow_dispatch` trigger; a bare "Run workflow" click
runs the `repro` caller job (the toolkit gate) against the current branch — the
man-pages build + the release-upload steps are `if:`-guarded off for a manual
dispatch, so only the gate runs. That proves `md`'s offline two-block
reproducibility BEFORE any release tag relies on it.

To reproduce the two-distinct-path proof yourself, inside the container:

```sh
# materialize the source at two distinct paths, then run the toolkit gate scripts:
ci/repro/double-build.sh /build-a/src /build-b/src   # binary + tar + libsecp .o identical
ci/repro/cc-validate.sh  /build-a/src                # epoch load-bearing + zero __DATE__/path residue
ci/repro/gzip-residue.sh md-<VER>-x86_64-linux-musl.tar.gz 03
```

(The `MINISCRIPT_REV` env the gate scripts read is **empty** for `md` — that
selects the two-block `--config` form. The toolkit passes its own fork rev to
select three-block.)

## 9. aarch64-unknown-linux-musl — built via `cross` under QEMU

The aarch64 binary has no native runner, so it is built with
[`cross`](https://github.com/cross-rs/cross) under QEMU user-mode emulation.
`cross` ships its **own bundled aarch64-musl C toolchain** inside a runtime
container image — a **different** `musl-gcc` than the x86_64 leg's container
`musl-tools`.

### 9.1 The pinned cross toolchain — `Cross.toml`, by digest

The cross runtime image is pinned **by sha256** in this repo's committed
`Cross.toml` (not a floating `cross`-version tag) — this is the real aarch64
toolchain pin:

```toml
[target.aarch64-unknown-linux-musl]
image = "ghcr.io/cross-rs/aarch64-unknown-linux-musl@sha256:702154f52b2d8091671aa2c84d5582d849f949977228c735ff8462f93cc0e1e4"
```

(Resolved at adoption time, 2026-06-24, via the GHCR registry manifest API:
`ghcr.io/cross-rs/aarch64-unknown-linux-musl` tag `0.2.5` == `latest` — the
version `cargo install --locked cross` resolves to.)

`cross` does **not** forward host env into its container automatically, so the
determinism-bearing vars are listed under `Cross.toml [build.env] passthrough`
(`SOURCE_DATE_EPOCH`, `CFLAGS`, `CFLAGS_aarch64_unknown_linux_musl`, `LC_ALL`,
`TZ`, `CARGO_BUILD_RUSTFLAGS`). Without this list the cc/rustflags mitigations
would silently never reach the aarch64 compiler.

### 9.2 The remap from-side is `/project`, not the host path

`cross` bind-mounts the project to its **fixed internal path `/project`** inside
the container (cross v0.2.5 `src/docker/local.rs`: `-v <host_root>:/project`),
sets `CARGO_HOME=/cargo`. So the in-container compiler sees the source at
`/project`, and the remap from-side is **`/project=/build`** (plus
`/cargo=/cargo`) — **not** the host checkout path.

### 9.3 The exact aarch64 build command (release leg)

Run from the repo root (where `Cross.toml` + `vendor/` are committed):

```sh
SOURCE_DATE_EPOCH=$(git show -s --format=%ct <tagged-commit-SHA>) \
LC_ALL=C TZ=UTC \
CARGO_BUILD_RUSTFLAGS="--remap-path-prefix=/project=/build --remap-path-prefix=/cargo=/cargo" \
CFLAGS="-ffile-prefix-map=/project=/build -ffile-prefix-map=/cargo=/cargo" \
CFLAGS_aarch64_unknown_linux_musl="-ffile-prefix-map=/project=/build -ffile-prefix-map=/cargo=/cargo" \
cross build --locked --offline --release \
  --target aarch64-unknown-linux-musl -p md-cli --bin md \
  --config 'source.crates-io.replace-with="vendored-sources"' \
  --config 'source.vendored-sources.directory="vendor"'

tar --sort=name --owner=0 --group=0 --numeric-owner --mtime="@$SOURCE_DATE_EPOCH" \
  -cf - -C target/aarch64-unknown-linux-musl/release md \
  | gzip -n -9 > md-<VER>-aarch64-linux-musl.tar.gz
```

The same TWO-block `[source]` activation (md is fork-free), the same `--locked
--offline`, the same gzip-pinned tar as the x86_64 leg. `cross` forwards the
`--config` flags to the inner cargo; the committed `vendor/` (at
`/project/vendor`) makes the build fully offline.

> **Honest caveat — local rebuilds may not byte-match without the pinned env.**
> A casual `cargo build`/`cross build` at an arbitrary `$PWD` without the remap +
> `SOURCE_DATE_EPOCH` + the pinned image will NOT reproduce the published hash
> (the build-path leak and `cc` timestamps return). Reproducibility holds only
> for the documented commands run in the digest-pinned environment.
