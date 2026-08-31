# Compatibility

Supported platforms, toolchains, and version-alignment policy for the
published **0.8.23** release.

> **Pre-1.0 = beta.** FathomDB is on a pre-1.0 line. The surface may
> change between micro releases; consult the
> [CHANGELOG](https://github.com/fathomadb/fathomdb/blob/main/CHANGELOG.md)
> for published-version detail.
> "GA" in earlier release notes referred to release *engineering* — a
> tagged, published artifact — not to an API-stability promise.

## Supported Python versions

`3.10`, `3.11`, `3.12`. `requires-python = ">=3.10"`; the wheel build
targets those three interpreters explicitly.

## Supported Node versions

Node **18** or later. CI and the release build use the exact Node **25.9.0**
toolchain (which bundles npm **11.12.1**).

## Supported Rust toolchain

Rust **1.95.0**. This is the declared MSRV and the exact compiler used by CI
and release jobs.

## Prebuilt artifacts — Linux x86_64 and AArch64 published

⚠ **The published 0.8.23 wheel and npm platform binaries support Linux
`x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu` glibc.** Linux
aarch64 is published for Python and npm. Do not expect the published npm
package to install on macOS, Windows, or Linux musl.

**Measured glibc floor (manylinux): 2.28**, for the Python wheel on both
architectures and the `linux-arm64-gnu` npm platform binary. As of Slice 80.1
the `linux-arm64-gnu` npm binding is built inside the same digest-pinned
`manylinux_2_28` container as the Python wheel (`scripts/release/
Dockerfile.napi-manylinux`), rather than on the bare CI runner, so this
number is enforced by `scripts/check-glibc-floor.sh` against the artifact
actually published — not merely asserted here.

**Measured glibc floor (cuda-napi-host): 2.39**, for the
`linux-x64-gnu` CUDA npm platform binary. It is built host-natively with the
CUDA toolchain on the release runner, rather than in the manylinux container,
and its higher ABI floor is therefore a distinct, checked publication
contract.

**Measured glibc floor (tegra): 2.35**, for the Jetson/Tegra CUDA Python
wheel only. That artifact is **not published to PyPI** and is not covered by
the table below; it is built host-natively on a Jetson Orin against JetPack's
CUDA 12.6 (`scripts/release/build-python-cuda-tegra.sh`), because the Tegra
CUDA runtime is host-bound and no manylinux image carries it. It therefore
inherits JetPack's own glibc — Ubuntu 22.04's 2.35 — rather than the
`manylinux_2_28` floor. This higher floor is *declared and asserted* by the
same gate, not exempted from it: both numbers come from
`scripts/release/glibc-floor-contract.sh`, and
`scripts/check-glibc-floor-doc-truth.sh` fails if either claim on this page
drifts from it.

For a confirmed classic Jetson Orin only, 0.8.24 serves the exact
`fathomdb==0.8.24+tegra` wheel from the interim first-party PEP 503 index:

```bash
python -m pip install --isolated --no-cache-dir --only-binary=:all: \
  --index-url https://fathomadb.github.io/fathomdb/tegra/simple/ \
  'fathomdb==0.8.24+tegra'
```

Do not use a floating requirement or `--extra-index-url`: pip merges candidate
sets and does not prioritize indexes. The Pages route is interim 0.8.24
hosting, not a durable multi-version distribution decision. Use
`fathomdb doctor platform --json` to distinguish classic Tegra from ARM64 SBSA
before diagnosing an incompatible CUDA provider.

| OS      | Architecture                | Published prebuilt? |
| ------- | --------------------------- | ------------------- |
| Linux   | `x86_64-unknown-linux-gnu`  | **yes** (CUDA host glibc 2.39) |
| Linux   | `aarch64-unknown-linux-gnu` | **yes** (manylinux 2_28) |
| macOS   | `x86_64-apple-darwin`       | no — unsupported published artifact |
| macOS   | `aarch64-apple-darwin`      | no — unsupported published artifact |
| Windows | `x86_64-pc-windows-msvc`    | no — unsupported published artifact |

Platforms outside the two published Linux glibc artifacts are unsupported for the
published Python and npm packages in this release.

## SQLite + sqlite-vec

- Engine uses SQLite **FTS5** + the
  [`sqlite-vec`](https://github.com/asg017/sqlite-vec) extension.
- Wheels and platform `.node` binaries statically link a compatible
  build of `sqlite-vec`.
- Rust + source-build users need a working SQLite + `sqlite-vec`
  available to the loader.

## On-disk schema

The published 0.8.23 line sets `SCHEMA_VERSION` to **26** (the released
0.8.21 line is **25**; 0.8.20 shipped **24**). Migration runs at
`Engine.open` and only there.

⚠ **Migration step 23 does not preserve edge data.** The step recreates
`canonical_edges` so `t_valid` / `t_invalid` are INTEGER epoch seconds
with type CHECKs. There is **no data migration**: existing edge rows do
not survive, and no stored ISO-8601 value is converted. Nodes are
unaffected. If your workspace carries edges you need, **re-ingest them
after upgrading** rather than relying on an in-place upgrade.

## Versioning — two axes

0.8.23 follows two-axis versioning:

- **Axis W (workspace lockstep)** — the runtime / binding / CLI crates
  plus the Python and TypeScript packages all carry the same workspace
  version. `scripts/set-version.sh --check-files` enforces this
  pre-publish; the pre-push hook runs the check.
- **Axis E (`fathomdb-embedder-api`)** — the embedder-trait crate is
  versioned independently per `ADR-0.6.0-embedder-protocol`. Bumping a
  binding does not force an embedder-api bump.

This decouples embedder-protocol stability from binding cadence.

## Breaking changes since 0.8.9

| Change | What to do |
| ------ | ---------- |
| `SearchHit.id` is a typed `IdSpace` (`{space, value}`), not an integer row cursor | read `hit.id.space` / `hit.id.value`; the prefixed form reproduces the old `stable_id` |
| `source_id` is **mandatory** on every canonical node/edge write | add a provenance id to every write item |
| `write_cursor` is no longer surfaced as a hit id | key on `SearchHit.id` or `logical_id` |
| Search results are ordered by **RRF fusion**, not union-dedup | re-baseline any pinned result ordering |
| `valid_from >= valid_until` is refused with a **message-less** `WriteValidationError` (was `InvalidArgumentError` carrying both bounds) | validate the pair before calling |
| A projection spec with `fts` / `vector` but no `searchable` role is refused | add the role, or name the projection in `drop` |
| Migration step 23 drops existing edge rows | re-ingest edges after upgrading |

The [CHANGELOG](https://github.com/fathomadb/fathomdb/blob/main/CHANGELOG.md)
is the authoritative list.

## 0.5.x / 0.6.x compatibility

**No 0.5.x compatibility shims or migrations.** Do not point 0.8.x
binaries at a 0.5.x database. Databases created by 0.6.x–0.8.x migrate
forward at open, subject to the step-23 edge caveat above.

## Performance posture — open gates

Four performance ACs remain open. Clients evaluating FathomDB for
perf-sensitive workloads should measure on their own corpus rather than
relying on a published number.

| AC      | Surface                                                 | Status |
| ------- | ------------------------------------------------------- | ------ |
| AC-012  | text query latency on FTS5 (p50 ≤ 20 ms / p99 ≤ 150 ms) | open   |
| AC-013  | vector retrieval latency (p50 ≤ 50 ms / p99 ≤ 200 ms)   | open   |
| AC-019  | mixed-retrieval stress tail                             | open (inherits AC-013) |
| AC-020  | N=8 concurrent reader scaling (architectural)           | open   |

Vector retrieval is a **full scan** over `sqlite-vec` (1-bit sign-quant
bit-KNN, then an f32 rerank of the shortlist) — **there is no ANN
index**, so latency grows with corpus size rather than staying flat.
Engine surfaces these as documented gates, not weakened ones. See
`dev/test-plan.md` § Current Perf Attribution.

## SDK maturity posture

Both bindings expose the same governed command surface and the same
27-class error taxonomy. **Python is the more heavily exercised
binding; prefer it for production pilots.** See
[SDK parity](../positions/sdk-parity.md).

## API surface gaps

- **Custom Python / TypeScript embedder implementations** are not
  exposed; the choice is the built-in default embedder or none.
- **`SearchFilter.status`** is wired end-to-end but has no population
  source, so a `status=`-filtered query prunes every row.
- **No restore verb.** `purge` is irreversible by design; there is no
  `restore_logical_id` on any surface.
