# Install — Python

The `fathomdb` Python SDK is a [PyO3](https://pyo3.rs/) binding over the
native Rust runtime. Wheels are platform-tagged (no source build
required on supported platforms).

> **Release 0.8.28, pre-1.0 beta.** Wheels cover Linux x86_64/glibc, Linux
> AArch64/glibc, macOS x64/arm64, and Windows x64. Other hosts must use the
> from-source path below. FathomDB is pre-1.0 and the surface is **beta**.

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

## Install the wheel

```bash
pip install fathomdb==0.8.28
```

## Build from source

Editable from a trusted `release/0.8.28` checkout using [maturin](https://www.maturin.rs/):

```bash
git clone https://github.com/fathomadb/fathomdb
cd fathomdb
pip install -e src/python/
```

`pip install -e src/python/` invokes maturin against the workspace and
produces the native PyO3 extension `fathomdb._fathomdb`. **Do not run
`cargo build` and copy the `.so` manually.** Editable install is the
only supported native-build path for development.

## Jetson / Tegra CUDA

On a confirmed classic Jetson Orin (L4T R36 / JetPack 6, CUDA 12.6), install
the exact 0.8.28 Tegra build from the first-party index. Confirm its
`0.8.28+tegra` wheel entry and SHA-256 in the
[retained artifact manifest](https://github.com/fathomadb/fathomdb/blob/release/0.8.28/scripts/release/tegra-pages-retained.json)
before installation:

```bash
python3 -m venv .venv
. .venv/bin/activate
python -m pip install --isolated --no-cache-dir --only-binary=:all: \
  --index-url https://fathomadb.github.io/fathomdb/tegra/simple/ \
  'fathomdb==0.8.28+tegra'
```

This is a detection-gated, exact-version route: do not use a floating version
or `--extra-index-url`. The GitHub Pages transport is interim 0.8.28 hosting
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
engine.write([])
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
- **Editable native build failure** — follow the contributor bootstrap and
  `pip install -e src/python/` route with Rust 1.95.0 or newer. Do not copy a
  separately built native module into the package.

- **`fathomdb.errors.DatabaseLockedError`** — another process holds an
  exclusive lock on the DB file. See
  [errors reference](../reference/errors.md).

## See also

- [Reference — Python API](../reference/python-api.md)
- [Reference — config](../reference/config.md)
- [Compatibility](../compatibility/index.md)
- [CHANGELOG](https://github.com/fathomadb/fathomdb/blob/main/CHANGELOG.md)
