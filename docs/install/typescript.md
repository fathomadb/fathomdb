# Install — TypeScript / Node.js

The `fathomdb` npm package is a [napi-rs](https://napi.rs/) binding
over the native Rust runtime. The package selects a
platform-tagged `.node` binary at load time.

> **Release 0.8.27, pre-1.0 beta.** The release supplies
> Linux x86_64/glibc, Linux AArch64/glibc, macOS x64/arm64, and Windows x64
> native packages; other hosts must build from source.
> FathomDB is pre-1.0 and the surface is **beta**.
>
> See [SDK parity](../positions/sdk-parity.md) for language conventions.

## Requirements

- Node **25.x** (release CI uses Node 25.9.0).
- The `0.8.27` npm package supports Linux
  `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`, macOS
  `x86_64-apple-darwin` and `aarch64-apple-darwin`, and Windows
  `x86_64-pc-windows-msvc`. Other hosts must build from source.
- Linux glibc floor: **2.28** for `linux-arm64-gnu` and **2.39** for the
  CUDA-built `linux-x64-gnu` package (see
  [compatibility](../compatibility/index.md)).
- SQLite + `sqlite-vec` (statically linked into the platform binary).

## Install the package

```bash
npm install --save-exact fathomdb@0.8.27
```

On Linux AArch64 this installs the CPU-only `fathomdb-linux-arm64-gnu`
package, including on a Jetson. For CUDA on a classic Jetson Orin, see the
next section.

## Classic Jetson Orin with CUDA

A CUDA-capable Node addon for **classic Jetson Orin** (JetPack 6 / L4T R36,
CUDA 12.6) ships as the separately named package
`fathomdb-linux-arm64-gnu-tegra`. It is not on the npm registry. It is
published only as an exact-version tarball on the first-party Tegra Pages
site, beside the `+tegra` Python wheel:

```text
https://fathomadb.github.io/fathomdb/tegra/packages/fathomdb-linux-arm64-gnu-tegra-<version>.tgz
```

The [retained artifact manifest](https://github.com/fathomadb/fathomdb/blob/release/0.8.27/scripts/release/tegra-pages-retained.json)
records exact URLs and SHA-256 digests. The install below requires its
`node-tarball` entry for 0.8.27; it stops if the entry is absent or the bytes
do not match.

Use it only on a confirmed classic Jetson Orin: the loader requires a Tegra
family signal (device-tree compatibility containing `nvidia,tegra`, or
`/etc/nv_tegra_release`) and an `nvgpu` device reported by
`nvidia-smi --query-gpu=name --format=csv,noheader`. Thor, ARM64 SBSA servers and other AArch64 hosts are not
supported by this package; use the generic package there.

Fetch and verify the exact tarball against that manifest, then install:

```bash
(
set -e
node --input-type=commonjs - <<'NODE'
const fs = require('node:fs');
const crypto = require('node:crypto');
(async () => {
  const manifestUrl = 'https://raw.githubusercontent.com/fathomadb/fathomdb/release/0.8.27/scripts/release/tegra-pages-retained.json';
  const response = await fetch(manifestUrl);
  if (!response.ok) throw new Error(`Manifest fetch failed: ${response.status}`);
  const manifest = await response.json();
  const entry = manifest.artifacts.find(item => item.kind === 'node-tarball' && item.version === '0.8.27');
  const name = 'fathomdb-linux-arm64-gnu-tegra-0.8.27.tgz';
  const url = `https://fathomadb.github.io/fathomdb/tegra/packages/${name}`;
  if (!entry || entry.filename !== name || entry.url !== url || !/^[0-9a-f]{64}$/.test(entry.sha256)) {
    throw new Error('Matching 0.8.27 Tegra artifact is missing from the retained manifest');
  }
  const tarball = await fetch(url);
  if (!tarball.ok) throw new Error(`Tarball fetch failed: ${tarball.status}`);
  const bytes = Buffer.from(await tarball.arrayBuffer());
  if (crypto.createHash('sha256').update(bytes).digest('hex') !== entry.sha256) {
    throw new Error('Tegra tarball SHA-256 differs from the retained manifest');
  }
  fs.writeFileSync(name, bytes);
  console.log('Tegra tarball SHA-256 verified');
})().catch(error => { console.error(error.message); process.exitCode = 1; });
NODE
npm install --save-exact --package-lock-only --ignore-scripts fathomdb@0.8.27
npm install --save-optional --package-lock-only --ignore-scripts https://fathomadb.github.io/fathomdb/tegra/packages/fathomdb-linux-arm64-gnu-tegra-0.8.27.tgz
node --input-type=commonjs - <<'NODE'
const fs = require('node:fs');
const crypto = require('node:crypto');
const name = 'fathomdb-linux-arm64-gnu-tegra-0.8.27.tgz';
const lock = JSON.parse(fs.readFileSync('package-lock.json', 'utf8'));
const installed = lock.packages?.['node_modules/fathomdb-linux-arm64-gnu-tegra']?.integrity;
const checked = 'sha512-' + crypto.createHash('sha512').update(fs.readFileSync(name)).digest('base64');
if (installed !== checked) throw new Error('Tegra lockfile integrity differs from the checked tarball');
console.log('Tegra lockfile integrity matches the checked tarball');
NODE
npm ci
)
```

The lock-only steps record dependencies without extracting packages or
running lifecycle scripts. The SHA-512 comparison binds the remote URL's
lockfile entry to the SHA-256-verified archive before `npm ci` installs
anything. `npm ci` checks the pinned integrity before extracting the archive
and running lifecycle scripts; a mismatch rejects the Tegra archive.

The result is this `package.json` form. The Tegra tarball is an
`optionalDependency`, so the same `package.json` and lockfile also install
on non-Jetson hosts: npm skips the tarball wherever its `linux`/`arm64`/`glibc`
metadata does not match, and the lockfile pins its integrity. Other Linux
AArch64 glibc hosts install it, but the loader ignores it there.

The Tegra package path supports npm with a flat (hoisted) `node_modules` only.
The loader resolves the Tegra package from `fathomdb`'s own install location,
so a pnpm isolated layout or Yarn Plug'n'Play does not find it and silently
loads the generic CPU binary instead.

```json
{
  "dependencies": {
    "fathomdb": "0.8.27"
  },
  "optionalDependencies": {
    "fathomdb-linux-arm64-gnu-tegra": "https://fathomadb.github.io/fathomdb/tegra/packages/fathomdb-linux-arm64-gnu-tegra-0.8.27.tgz"
  }
}
```

How it behaves:

- **Selection.** The loader uses the Tegra package only when it is installed
  and the host is confirmed as a classic Jetson Orin (the Tegra family signal,
  then an `nvgpu` GPU name). On any other host, including a Thor-like or
  undetermined one, it loads the generic package, which must also be
  installed. Without the Tegra package nothing changes.
- **Exact version.** The tarball's version must equal the installed
  `fathomdb` version. On a confirmed classic Jetson Orin (both the Tegra
  family signal and an `nvgpu` GPU name), a mismatch fails the import with
  `PlatformPackageVersionMismatchError` before any native code loads. On
  any other host the loader ignores the Tegra package, so no mismatch is
  raised. Upgrade both together.
- **CUDA starts at import (AArch64 only).** Unlike the Linux x86_64 package,
  which never probes the CUDA driver at import, this addon calls `cuInit`
  when it is loaded; see
  [Node.js on Jetson](../embedder.md#nodejs-on-jetson-load-fathomdb-first).
  It does not when every CUDA-built component is set to `cpu`
  (`FATHOMDB_EMBED_DEVICE`, `FATHOMDB_RERANK_DEVICE`) at load time, or when
  `FATHOMDB_CUDA_EARLY_INIT=off`. A `cpu` option passed only later at open
  cannot prevent the earlier probe. A missing driver never fails the import.
- **No postinstall step.** Nothing is downloaded or built at install time.

## Build from source

```bash
git clone https://github.com/fathomadb/fathomdb
cd fathomdb/src/ts
npm install
npm run build
```

`npm run build` invokes `napi build` against the workspace Rust crate
`fathomdb-napi` and emits `fathomdb.<platform>-<arch>.node` plus the
TypeScript output in `dist/`.

## Default embedder (optional)

To let FathomDB embed documents for you, use the embedder-enabled native binary
and opt in at open:

```ts
import { Engine } from "fathomdb";

const engine = await Engine.open("mydb.sqlite", { useDefaultEmbedder: true });
```

This enables the in-process `bge-small-en-v1.5` model and, on first use,
downloads + sha256-verifies ~133 MB of weights into your platform cache
(visible in `engine.openReport().embedderEvents`). The flag defaults to
`false`, and the embedder-enabled binary is larger. See the
[Default Embedder guide](../embedder.md) for the opt-in contract,
offline/`HF_TOKEN` notes, caveats, and migration.

## Verify

```ts
import { Engine } from "fathomdb";

const engine = await Engine.open("./hello.fdb");
await engine.write([]);
await engine.search("hello");
await engine.close();
console.log("ok");
```

Expected output: `ok`. See [Quickstart](../getting-started/quickstart.md).

## Troubleshooting

- **`Error: Cannot find module 'fathomdb.<platform>-<arch>.node'`** —
  no platform binary matched your runtime. Confirm your platform is on
  the supported matrix above. For source builds, ensure
  `npm run build` completed before `node` resolves the package.
- **`FathomDbError`** — every native error is rethrown as a typed
  subclass of `FathomDbError`. See [errors reference](../reference/errors.md).
- **Jetson, forced CUDA refused with `cuInit returned
  CUDA_ERROR_OUT_OF_MEMORY`** — the JavaScript heap grew before fathomdb was
  loaded. Import fathomdb first, or start Node with `--import fathomdb`. See
  [Node.js on Jetson](../embedder.md#nodejs-on-jetson-load-fathomdb-first).

## See also

- [Reference — TypeScript API](../reference/typescript-api.md)
- [Reference — config](../reference/config.md)
- [Compatibility](../compatibility/index.md)
- [CHANGELOG](https://github.com/fathomadb/fathomdb/blob/main/CHANGELOG.md)
