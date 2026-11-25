# Install — Python

The `fathomdb` Python SDK is a [PyO3](https://pyo3.rs/) binding over the
native Rust runtime. Wheels are platform-tagged (no source build
required on supported platforms).

> **Documentation target: 0.8.32 (pre-publish, pre-1.0 beta); newest published release: 0.8.31.**
> Wheels cover Linux x86_64/glibc, Linux AArch64/glibc, macOS x64/arm64, and
> Windows x64. Other hosts must use the from-source path below. FathomDB is
> pre-1.0 and the surface is **beta**.

## Requirements

- Python **3.10**, **3.11**, or **3.12**
- One of the supported platforms (per `release.yml` matrix):
  - Linux `x86_64-unknown-linux-gnu` (manylinux 2_28)
  - Linux `aarch64-unknown-linux-gnu` (manylinux 2_28)
  - macOS `x86_64-apple-darwin`
  - macOS `aarch64-apple-darwin`
  - Windows `x86_64-pc-windows-msvc`
- Official wheels bundle SQLite and statically link `sqlite-vec`; no system
  SQLite headers or separate extension installation are needed.

### Prebuilt wheel matrix (embedder, cross-encoder, CUDA, and glibc)

| Target | Wheel tag / channel | Linux glibc floor | Default embedder (`bge-small`) | Cross-encoder (`rerank`) | CUDA (`embed-cuda` + `rerank-cuda`) |
| --- | --- | --- | --- | --- | --- |
| Linux `x86_64-unknown-linux-gnu` | `manylinux_2_28_x86_64` (PyPI) | `2.28` | CPU + CUDA | CPU + CUDA | **yes** (driverless CPU fallback) |
| Linux `aarch64-unknown-linux-gnu` (generic) | `manylinux_2_28_aarch64` (PyPI) | `2.28` | CPU only | CPU only | no |
| Linux `aarch64-unknown-linux-gnu` (classic Jetson Orin) | `0.8.31+tegra` (Tegra Pages) | `2.35` | CPU + CUDA | CPU + CUDA | **yes** (`tegra-pool`, JetPack 6 / CUDA 12.6) |
| macOS `x86_64-apple-darwin` | `macosx_10_12_x86_64` (PyPI) | N/A | CPU only | CPU only | no |
| macOS `aarch64-apple-darwin` | `macosx_11_0_arm64` (PyPI) | N/A | CPU only | CPU only | no |
| Windows `x86_64-pc-windows-msvc` | `win_amd64` (PyPI) | N/A | CPU only | CPU only | no |

## Install the wheel

Version 0.8.31 is published on PyPI. Use the exact-version install below in
a virtual environment for your application.

```bash
pip install fathomdb==0.8.31
```

### Operator CLI extra (`fathomdb[cli]`)

The base `fathomdb` Python SDK package ships zero console scripts. Starting with
0.8.31, the operator CLI (`fathomdb doctor`, `fathomdb recover`,
and offline `fathomdb migrate`) is packaged as a lockstep `fathomdb-cli` binary
wheel across the same five native targets and linked via the optional `cli` extra:

```bash
pip install 'fathomdb[cli]==0.8.31'
```

You can also install the CLI from crates.io via
`cargo install fathomdb-cli --version '=0.8.31' --locked`. See the
[CLI reference](../reference/cli.md) for offline Schema `38 -> 39` (and chained
`37 -> 38 -> 39`) upgrades targeted in `0.8.32`.

## Build from source

Use a trusted `release/0.8.31` checkout and the declared Rust 1.95.0 toolchain.
For repository development, provision that checkout's own Python environment
and install its noneditable, source-bound native wheel:

```bash
git clone --branch release/0.8.31 https://github.com/fathomadb/fathomdb
cd fathomdb
bash scripts/setup-python-owned.sh --provision
. .venv/bin/activate
```

Developer gates require Python 3.11 or later even though installed packages
support Python 3.10. The setup script selects a suitable interpreter, installs
pinned developer tools, and builds through Maturin. Never borrow another
checkout's `.venv` or run an editable install from a worktree. After source
changes, `bash scripts/setup-python-owned.sh --build` refreshes this owned wheel.
See the [contributor setup](../contributing/index.md#verify-the-change).

## Jetson / Tegra CUDA

The published `0.8.31+tegra` wheel supplies CUDA on classic Jetson Orin. Its
separate Pages delivery passed public readback and installed Orin smokes;
verify its retained digest before installation.
It is distributed through first-party GitHub Pages, separately from PyPI.
The generic 0.8.31 AArch64 wheel remains CPU-only, including on Jetson.
CUDA 13 artifact delivery is deferred beyond 0.8.31 (`0.8.31` retains the CUDA 12.6 + classic Jetson Orin stack).

On a confirmed classic Jetson Orin (L4T R36 / JetPack 6, CUDA 12.6), install
the exact 0.8.31 Tegra build from the first-party index. Confirm its
`0.8.31+tegra` wheel entry and SHA-256 in the
[retained artifact manifest](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/scripts/release/tegra-pages-retained.json)
before installation:

```bash
python3 -m venv .venv
. .venv/bin/activate
python -m pip install --isolated --no-cache-dir --only-binary=:all: \
  --index-url https://fathomadb.github.io/fathomdb/tegra/simple/ \
  'fathomdb==0.8.31+tegra'
```

This is a detection-gated, exact-version route: do not use a floating version
or `--extra-index-url`. The GitHub Pages transport is interim 0.8.31 hosting
and is published only through the documented exact-SHA workflow. Unsupported JetPack,
generic AArch64/SBSA, and Thor hosts have no supported Tegra CUDA route. Do not
use a generic AArch64 CUDA build on classic Tegra; the SDK emits a visible
warning if it can confirm that mismatch.

## Default embedder

The wheel includes the default embedder; there is no
`default-embedder` Python extra. Opt in at `open`:

```python
engine = Engine.open("mydb.sqlite", use_default_embedder=True)
```

This pulls in the in-process `bge-small-en-v1.5` model and, on first use,
downloads + sha256-verifies ~133 MB of weights into your platform cache
(visible in `engine.open_report().embedder_events`). The flag defaults to
`False`. See the [Default Embedder guide](../embedder.md) for the opt-in
contract, offline/`HF_TOKEN` notes, caveats, and migration.

## Verify

```python
from fathomdb import Engine

engine = Engine.open("./hello.fdb")
engine.write([{"kind": "note", "body": "hello fathomdb", "source_id": "verify"}])
engine.search("hello")
engine.close()
print("ok")
```

Expected output: `ok`. See [Quickstart](../getting-started/quickstart.md)
for a richer walkthrough.

## Troubleshooting

- **SQLite or extension load errors** — official wheels bundle SQLite and
  `sqlite-vec`. Verify the exact wheel/platform and environment rather than
  installing unrelated system SQLite libraries. Source builds use the pinned
  bundled Rust dependencies and need supported C/C++ build tools.
- **Native build failure** — verify the checkout and the declared Rust
  1.95.0 toolchain, then rerun the source-build entry point for that checkout.
  Install the native extension through the checkout's documented build entry point.

- **`fathomdb.errors.DatabaseLockedError`** — another process holds an
  exclusive lock on the DB file. See
  [errors reference](../reference/errors.md).

## See also

- [Reference — Python API](../reference/python-api.md)
- [Reference — config](../reference/config.md)
- [Compatibility](../compatibility/index.md)
- [CHANGELOG](https://github.com/fathomadb/fathomdb/blob/main/CHANGELOG.md)
