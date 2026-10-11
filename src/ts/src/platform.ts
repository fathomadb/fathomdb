// Platform-resolution + native-binding loader contract for the napi-rs
// per-platform package split (R-REL-4f, dev/design/
// 0.8.18-slice-20-publish-pipeline.md).
//
// The published `fathomdb` package is a THIN main package: it ships no `.node`
// binary itself. Each platform's compiled binding is published as a separate
// platform package carrying `os`/`cpu` (+ `libc`) so npm
// installs only the one matching the host and SKIPS the rest (they are
// `optionalDependencies`). The loader below picks the right platform package
// for the running host and — critically — throws a CLEAR "unsupported
// platform" error when no matching binary is present, instead of letting a
// later `require` of a missing/mismatched `.node` segfault at runtime.
//
// This module has NO top-level side effects (it never loads a native binary on
// import) so it is unit-testable without a built binding.

/** Thrown when the host (platform, arch[, libc]) has no prebuilt binary. */
export class UnsupportedPlatformError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "UnsupportedPlatformError";
  }
}

/**
 * Thrown when an installed platform package's version differs from this
 * `fathomdb` package's version. Raised before any native addon is loaded, so
 * mismatched bytes are never silently selected.
 */
export class PlatformPackageVersionMismatchError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "PlatformPackageVersionMismatchError";
  }
}

/**
 * The separately named CUDA-capable binary package for classic NVIDIA Jetson
 * Orin. It shares the `linux-arm64-gnu` triple with the generic package, so it
 * is never an optionalDependency; users install it by exact tarball URL.
 */
export const TEGRA_PLATFORM_PACKAGE = "fathomdb-linux-arm64-gnu-tegra";

const TEGRA_TRIPLE = "linux-arm64-gnu";

/** napi-rs triple label for a host, or `null` if the host is not mapped. */
export function tripleFor(
  platform: NodeJS.Platform,
  arch: string,
  isMusl: boolean,
): string | null {
  switch (platform) {
    case "linux":
      switch (arch) {
        case "x64":
          return isMusl ? "linux-x64-musl" : "linux-x64-gnu";
        case "arm64":
          return isMusl ? "linux-arm64-musl" : "linux-arm64-gnu";
        case "arm":
          return "linux-arm-gnueabihf";
        default:
          return null;
      }
    case "darwin":
      switch (arch) {
        case "x64":
          return "darwin-x64";
        case "arm64":
          return "darwin-arm64";
        default:
          return null;
      }
    case "win32":
      switch (arch) {
        case "x64":
          return "win32-x64-msvc";
        case "ia32":
          return "win32-ia32-msvc";
        case "arm64":
          return "win32-arm64-msvc";
        default:
          return null;
      }
    default:
      return null;
  }
}

/**
 * Resolve the napi triple for a host, throwing `UnsupportedPlatformError`
 * with a clear message when the (platform, arch) pair is not mapped.
 */
export function resolveTriple(
  platform: NodeJS.Platform,
  arch: string,
  isMusl: boolean,
): string {
  const triple = tripleFor(platform, arch, isMusl);
  if (triple === null) {
    throw new UnsupportedPlatformError(
      `Unsupported platform: FathomDB has no prebuilt native binary for ${platform}/${arch}. ` +
        "See the compatibility matrix for published-artifact support.",
    );
  }
  return triple;
}

/** The published binary package name for a triple. */
export function platformPackageName(triple: string): string {
  if (triple === "win32-x64-msvc") {
    return "fathomdb-native-win32-x64-msvc";
  }
  return `fathomdb-${triple}`;
}

export interface LoaderSeams {
  platform: NodeJS.Platform;
  arch: string;
  isMusl: boolean;
  /** Returns the local dev binary module if `fathomdb.<triple>.node` exists next
   *  to the loader (napi `--platform` build output), else `null`. */
  loadLocal: (triple: string) => unknown | null;
  /** Require the published platform package for the triple; MUST throw if the (optional)
   *  platform package is not installed. */
  requirePackage: (pkg: string) => unknown;
  /** The installed version of `pkg`, or `null` when it is not installed. Must
   *  not load the package's native addon. Absent: no alternative package is
   *  considered. */
  installedPackageVersion?: (pkg: string) => string | null;
  /** This `fathomdb` package's own version. */
  mainPackageVersion?: () => string;
  /** Tier 1: the host reports the Tegra family (device-tree `nvidia,tegra`
   *  or `/etc/nv_tegra_release`). */
  tegraFamilySignal?: () => boolean;
  /** Tier 2: the GPU name reported by the host's approved `nvidia-smi`, or
   *  `null` when the probe is unavailable or fails. */
  tegraGpuName?: () => string | null;
}

// Both classic-Tegra tiers must confirm; anything else, including Thor and
// every indeterminate probe, keeps the generic package. Tier 1 runs only when
// the Tegra package is installed and Tier 2 only after Tier 1, so imports on
// hosts without the package pay for neither probe.
function selectTegraPackage(seams: LoaderSeams, triple: string): boolean {
  if (triple !== TEGRA_TRIPLE || seams.installedPackageVersion === undefined) {
    return false;
  }
  const tegraVersion = seams.installedPackageVersion(TEGRA_PLATFORM_PACKAGE);
  if (tegraVersion === null) {
    return false;
  }
  if (seams.tegraFamilySignal === undefined || !seams.tegraFamilySignal()) {
    return false;
  }
  const gpuName = seams.tegraGpuName?.() ?? null;
  if (gpuName === null || !gpuName.includes("nvgpu")) {
    return false;
  }
  const mainVersion = seams.mainPackageVersion?.() ?? null;
  if (mainVersion !== tegraVersion) {
    throw new PlatformPackageVersionMismatchError(
      `Installed "${TEGRA_PLATFORM_PACKAGE}" is version ${tegraVersion}, but "fathomdb" is ` +
        `version ${mainVersion ?? "unknown"}. The Jetson binary must match the fathomdb ` +
        `package exactly. Reinstall both at the same version: fathomdb@<version> together ` +
        `with the ${TEGRA_PLATFORM_PACKAGE}-<version>.tgz tarball URL from the Jetson ` +
        "install guide. No native binary was loaded.",
    );
  }
  return true;
}

/**
 * Load the native binding for the running host. Order:
 *   1. local dev binary (`fathomdb.<triple>.node`) if present;
 *   2. on `linux-arm64-gnu` only, `TEGRA_PLATFORM_PACKAGE` when it is
 *      installed and both classic-Tegra tiers confirm the host; a version
 *      mismatch with this package throws `PlatformPackageVersionMismatchError`
 *      before any addon loads, and a confirmed Tegra package that fails to
 *      load throws `UnsupportedPlatformError` rather than falling back;
 *   3. the published platform package selected by `platformPackageName`;
 *   4. otherwise throw `UnsupportedPlatformError` — a mac/win install that
 *      skipped the linux optionalDependency lands here at REQUIRE time (loud),
 *      never as a silent `.node` load / runtime segfault.
 */
export function loadPlatformBinding(seams: LoaderSeams): unknown {
  const triple = resolveTriple(seams.platform, seams.arch, seams.isMusl);

  const local = seams.loadLocal(triple);
  if (local != null) {
    return local;
  }

  const tegra = selectTegraPackage(seams, triple);
  const pkg = tegra ? TEGRA_PLATFORM_PACKAGE : platformPackageName(triple);
  try {
    return seams.requirePackage(pkg);
  } catch (cause) {
    if (tegra) {
      throw new UnsupportedPlatformError(
        `The installed Jetson package "${pkg}" was selected for this classic Jetson Orin, ` +
          "but its native binary failed to load. FathomDB does not fall back to the generic " +
          `binary. Reinstall "${pkg}" from the Jetson install guide, or uninstall it to use ` +
          `the CPU-only package. Cause: ${(cause as Error)?.message ?? String(cause)}`,
      );
    }
    throw new UnsupportedPlatformError(
      `Unsupported platform: no prebuilt native binary for ${seams.platform}/${seams.arch} ` +
        `(${triple}). The optional platform package "${pkg}" is not installed — npm skips ` +
        `platform packages that do not match the host os/cpu. If your platform should be ` +
        `supported, install "${pkg}" or build from source with \`npm run build\`. ` +
        `Cause: ${(cause as Error)?.message ?? String(cause)}`,
    );
  }
}
