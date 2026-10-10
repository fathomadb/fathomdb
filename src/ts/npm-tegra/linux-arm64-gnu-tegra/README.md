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
the loader keeps the generic `fathomdb-linux-arm64-gnu` binary. If this package
and `fathomdb` have different versions, the loader fails with an install error
and loads neither binary.

## License

MIT. See the `LICENSE` file shipped in this package.

Source, issues and full documentation: <https://github.com/coreyt/fathomdb>
