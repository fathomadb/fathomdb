# npm per-platform binary packages

Each subdirectory here is a standalone npm package that ships **one** prebuilt
napi-rs binding (`fathomdb.<triple>.node`) for a single host, tagged with
`os` / `cpu` / `libc`. They are published as `fathomdb-<triple>`, except
Windows x64 MSVC which is `fathomdb-native-win32-x64-msvc`, and
wired as `optionalDependencies` of the thin main `fathomdb` package, so npm
installs only the one matching the host and skips the rest.

The main package's loader (`src/ts/src/platform.ts`) resolves the host triple
and `require`s the matching platform package, throwing a clear
`UnsupportedPlatformError` when none is present — never a silent runtime
segfault (R-REL-4f, `dev/design/0.8.18-slice-20-publish-pipeline.md`).

## Packages for 0.8.27

- `linux-x64-gnu/` — `fathomdb-linux-x64-gnu`
- `linux-arm64-gnu/` — `fathomdb-linux-arm64-gnu`
- `darwin-x64/` — `fathomdb-darwin-x64`
- `darwin-arm64/` — `fathomdb-darwin-arm64`
- `win32-x64-msvc/` — `fathomdb-native-win32-x64-msvc`

The `.node` binary is NOT committed; the release workflow's `build-napi` job
stages it into this directory before publishing.

## Jetson Tegra package (not in this directory)

`fathomdb-linux-arm64-gnu-tegra` is the CUDA-capable `linux-arm64-gnu` binary
for a confirmed classic Jetson Orin (JetPack 6, CUDA 12.6). It lives in
`../npm-tegra/linux-arm64-gnu-tegra/`, outside this directory, because it
shares the generic package's `os`/`cpu`/`libc` tuple: listing it as an
`optionalDependency` would install a Tegra-linked binary on every AArch64
glibc host. `scripts/release/npm-inject-optional-deps.sh` refuses to inject
it. It is built on the Jetson by `scripts/release/build-napi-cuda-tegra.sh`
and, in 0.8.27, distributed only as an
exact-version tarball on the Tegra Pages site, never on the npm registry:

```text
https://fathomadb.github.io/fathomdb/tegra/packages/fathomdb-linux-arm64-gnu-tegra-<version>.tgz
```

The 0.8.27 Pages publication route provides this tarball alongside its
SHA-256. Check that published digest and install the same exact version as
`fathomdb`; follow the
[Jetson installation guide](https://github.com/fathomadb/fathomdb/blob/release/0.8.27/docs/install/typescript.md#classic-jetson-orin-with-cuda).

The loader selects it only when it is installed and the host is confirmed as a
classic Jetson Orin. A version that differs from `fathomdb` fails with
`PlatformPackageVersionMismatchError` before any addon loads. The addon calls
`cuInit` at import unless `FATHOMDB_CUDA_EARLY_INIT=off`. Full guide:
`docs/install/typescript.md`.

## Unsupported targets

Linux musl, Windows ARM/32-bit, and other targets deliberately have no
platform package. Their loader error is part of the public install contract;
they must not be described as supported until an artifact, registry smoke, and
compatibility documentation land together.
