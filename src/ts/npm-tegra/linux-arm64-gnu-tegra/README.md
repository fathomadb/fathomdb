# fathomdb-linux-arm64-gnu-tegra

This package holds the CUDA-capable FathomDB native binary for a classic NVIDIA
Jetson Orin (JetPack 6 / L4T R36, CUDA 12.6). It has no JavaScript API. Install
it next to the [`fathomdb`](https://www.npmjs.com/package/fathomdb) package of
the same version.

It is not on the npm registry. It is published only as an exact-version
tarball on the FathomDB Tegra Pages site, with a SHA-256 digest. The `fathomdb`
package never lists it as an optional dependency, so npm never installs it
automatically.

The `fathomdb` loader uses this binary only on a Linux AArch64 glibc host that
it confirms is a classic Jetson Orin. On any other host, including Jetson Thor,
the loader keeps the generic `fathomdb-linux-arm64-gnu` binary. On a confirmed
classic Jetson Orin (both the Tegra family signal and an `nvgpu` GPU name), if
this package and `fathomdb` have different versions, the loader fails with
`PlatformPackageVersionMismatchError` and loads neither binary. On any other
host it ignores this package, so no mismatch is raised.

This package path supports npm with a flat (hoisted) `node_modules` only. The
loader resolves this package from `fathomdb`'s own install location, so a pnpm
isolated layout or Yarn Plug'n'Play does not find it and silently loads the
generic CPU binary instead.

## License

MIT. See the `LICENSE` file shipped in this package.

Source, issues and full documentation: <https://github.com/fathomadb/fathomdb>
