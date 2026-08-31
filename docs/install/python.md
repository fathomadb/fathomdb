# Install — Python

The `fathomdb` Python SDK is a [PyO3](https://pyo3.rs/) binding over the
native Rust runtime. Published wheels are platform-tagged (no source build
required on supported platforms).

> **0.8.23 is published to PyPI** for Linux x86_64/glibc and Linux
> AArch64/glibc. Other hosts must use the from-source path below. FathomDB is
> pre-1.0 and the surface is **beta**.

## Requirements

- Python **3.10**, **3.11**, or **3.12**
- One of the supported platforms (per `release.yml` matrix):
  - Linux `x86_64-unknown-linux-gnu` (manylinux 2_28)
  - Linux `aarch64-unknown-linux-gnu` (manylinux 2_28)
- SQLite with the [`sqlite-vec`](https://github.com/asg017/sqlite-vec)
  extension available to the loader (statically linked into the wheel
  for supported platforms).

## Install the published wheel

```bash
pip install fathomdb==0.8.23
```

## Install (current path — from source)

Editable from `main` using [maturin](https://www.maturin.rs/):

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
the exact 0.8.24 Tegra build from the interim first-party index:

```bash
python3 -m venv .venv
. .venv/bin/activate
python -m pip install --isolated --no-cache-dir --only-binary=:all: \
  --index-url https://fathomadb.github.io/fathomdb/tegra/simple/ \
  'fathomdb==0.8.24+tegra'
```

This is a detection-gated, exact-version route: do not use a floating version
or `--extra-index-url`. The GitHub Pages transport is interim 0.8.24 hosting
and must be re-reviewed before a later Tegra release. Unsupported JetPack,
generic AArch64/SBSA, and Thor hosts have no supported Tegra CUDA route. Do not
use a generic AArch64 CUDA build on classic Tegra; the SDK emits a visible
warning if it can confirm that mismatch.

## Default embedder

The published wheel already includes the default embedder; there is no
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

- **`ImportError: libsqlite3 ...`** — your system SQLite is older than
  the version the wheel was built against. Install via your package
  manager (`apt install libsqlite3-dev`, `brew install sqlite`, etc.)
  or upgrade.
- **`OSError: sqlite-vec extension not found`** — `sqlite-vec` is
  statically linked into the official wheels. If you are building from
  source, ensure `sqlite-vec` is installed and discoverable by the
  build script.
- **`pip install -e src/python/` fails on `maturin`** — install maturin
  explicitly (`pip install maturin`) and retry. The build also requires
  a stable Rust toolchain (`rustup default stable`).
- **`fathomdb.errors.DatabaseLockedError`** — another process holds an
  exclusive lock on the DB file. See
  [errors reference](../reference/errors.md).

## See also

- [Reference — Python API](../reference/python-api.md)
- [Reference — config](../reference/config.md)
- [Compatibility](../compatibility/index.md)
- [CHANGELOG](https://github.com/fathomadb/fathomdb/blob/main/CHANGELOG.md)
