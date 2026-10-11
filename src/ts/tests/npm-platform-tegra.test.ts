// Jetson Tegra platform-package selection through `LoaderSeams`
// (ADR-0.8.27-jetson-tegra-node-addon-distribution, decision 3). The
// separately named `fathomdb-linux-arm64-gnu-tegra` package is considered only
// on linux-arm64-gnu, only when installed, and only when both classic-Tegra
// tiers confirm. Every other outcome selects the generic package. A version
// mismatch is an install error raised before either addon is loaded.

import test from "node:test";
import assert from "node:assert/strict";

import {
  TEGRA_PLATFORM_PACKAGE,
  UnsupportedPlatformError,
  loadPlatformBinding,
  type LoaderSeams,
} from "../src/platform.js";

const GENERIC = "fathomdb-linux-arm64-gnu";

interface Probe {
  seams: LoaderSeams;
  required: string[];
  tier1Calls: number;
  tier2Calls: number;
  versionLookups: string[];
}

function jetsonSeams(options: {
  arch?: string;
  isMusl?: boolean;
  tegraInstalledVersion: string | null;
  mainVersion?: string;
  tier1: boolean;
  gpuName: string | null;
}): Probe {
  const probe: Probe = {
    required: [],
    tier1Calls: 0,
    tier2Calls: 0,
    versionLookups: [],
    seams: undefined as unknown as LoaderSeams,
  };
  probe.seams = {
    platform: "linux",
    arch: options.arch ?? "arm64",
    isMusl: options.isMusl ?? false,
    loadLocal: () => null,
    requirePackage: (pkg) => {
      probe.required.push(pkg);
      return { __fathomdb_native: pkg };
    },
    installedPackageVersion: (pkg) => {
      probe.versionLookups.push(pkg);
      return pkg === TEGRA_PLATFORM_PACKAGE ? options.tegraInstalledVersion : null;
    },
    mainPackageVersion: () => options.mainVersion ?? "0.8.27",
    tegraFamilySignal: () => {
      probe.tier1Calls += 1;
      return options.tier1;
    },
    tegraGpuName: () => {
      probe.tier2Calls += 1;
      return options.gpuName;
    },
  };
  return probe;
}

function withoutWarnings<T>(body: () => T): T {
  const warnings: unknown[] = [];
  const originalWarn = console.warn;
  const originalEmit = process.emitWarning;
  console.warn = (...args: unknown[]) => {
    warnings.push(args);
  };
  process.emitWarning = ((warning: unknown) => {
    warnings.push(warning);
  }) as typeof process.emitWarning;
  try {
    const result = body();
    assert.deepEqual(warnings, [], "the loader must not warn at import time");
    return result;
  } finally {
    console.warn = originalWarn;
    process.emitWarning = originalEmit;
  }
}

test("tegra_loader: the Tegra package name is the separately named unscoped package", () => {
  assert.equal(TEGRA_PLATFORM_PACKAGE, "fathomdb-linux-arm64-gnu-tegra");
});

test("tegra_loader: confirmed classic Tegra with the package installed selects the Tegra package", () => {
  const probe = jetsonSeams({ tegraInstalledVersion: "0.8.27", tier1: true, gpuName: "Orin (nvgpu)" });
  const mod = withoutWarnings(() => loadPlatformBinding(probe.seams));
  assert.deepEqual(mod, { __fathomdb_native: TEGRA_PLATFORM_PACKAGE });
  assert.deepEqual(probe.required, [TEGRA_PLATFORM_PACKAGE], "only the Tegra addon is loaded");
  assert.equal(probe.tier1Calls, 1);
  assert.equal(probe.tier2Calls, 1);
});

test("tegra_loader: a Tegra host without the package selects the generic package silently and skips both probes", () => {
  const probe = jetsonSeams({ tegraInstalledVersion: null, tier1: true, gpuName: "Orin (nvgpu)" });
  const mod = withoutWarnings(() => loadPlatformBinding(probe.seams));
  assert.deepEqual(mod, { __fathomdb_native: GENERIC });
  assert.deepEqual(probe.required, [GENERIC]);
  assert.equal(probe.tier1Calls, 0, "generic imports must not pay for the Tegra family probe");
  assert.equal(probe.tier2Calls, 0, "generic imports must not pay for the GPU-name probe");
});

test("tegra_loader: a version mismatch is an install error raised before either addon loads", () => {
  const probe = jetsonSeams({
    tegraInstalledVersion: "0.8.26",
    mainVersion: "0.8.27",
    tier1: true,
    gpuName: "Orin (nvgpu)",
  });
  assert.throws(
    () => loadPlatformBinding(probe.seams),
    (err: unknown) => {
      assert.ok(err instanceof Error);
      assert.equal((err as Error).name, "PlatformPackageVersionMismatchError");
      const message = (err as Error).message;
      assert.match(message, /fathomdb-linux-arm64-gnu-tegra/);
      assert.match(message, /0\.8\.26/);
      assert.match(message, /0\.8\.27/);
      assert.match(message, /reinstall/i);
      return true;
    },
  );
  assert.deepEqual(probe.required, [], "neither the Tegra nor the generic addon may load");
});

test("tegra_loader: a non-Tegra AArch64 host with the package installed selects the generic package", () => {
  const probe = jetsonSeams({ tegraInstalledVersion: "0.8.27", tier1: false, gpuName: "Orin (nvgpu)" });
  const mod = withoutWarnings(() => loadPlatformBinding(probe.seams));
  assert.deepEqual(mod, { __fathomdb_native: GENERIC });
  assert.deepEqual(probe.required, [GENERIC]);
  assert.equal(probe.tier1Calls, 1);
  assert.equal(probe.tier2Calls, 0, "the GPU-name probe runs only after the Tegra family signal");
});

test("tegra_loader: a Thor-like GPU name selects the generic package", () => {
  const probe = jetsonSeams({ tegraInstalledVersion: "0.8.27", tier1: true, gpuName: "NVIDIA Thor" });
  const mod = withoutWarnings(() => loadPlatformBinding(probe.seams));
  assert.deepEqual(mod, { __fathomdb_native: GENERIC });
  assert.deepEqual(probe.required, [GENERIC]);
  assert.equal(probe.tier2Calls, 1);
});

test("tegra_loader: an indeterminate GPU-name probe selects the generic package", () => {
  const probe = jetsonSeams({ tegraInstalledVersion: "0.8.27", tier1: true, gpuName: null });
  const mod = withoutWarnings(() => loadPlatformBinding(probe.seams));
  assert.deepEqual(mod, { __fathomdb_native: GENERIC });
  assert.deepEqual(probe.required, [GENERIC]);
});

test("tegra_loader: a Thor-like host ignores a mismatched Tegra package instead of failing", () => {
  const probe = jetsonSeams({
    tegraInstalledVersion: "0.8.26",
    mainVersion: "0.8.27",
    tier1: true,
    gpuName: "NVIDIA Thor",
  });
  const mod = withoutWarnings(() => loadPlatformBinding(probe.seams));
  assert.deepEqual(mod, { __fathomdb_native: GENERIC });
});

test("tegra_loader: the Tegra package is never considered off linux-arm64-gnu", () => {
  for (const host of [
    { arch: "x64", isMusl: false },
    { arch: "arm64", isMusl: true },
  ]) {
    const probe = jetsonSeams({
      ...host,
      tegraInstalledVersion: "0.8.27",
      tier1: true,
      gpuName: "Orin (nvgpu)",
    });
    loadPlatformBinding(probe.seams);
    assert.deepEqual(probe.versionLookups, [], `${host.arch}/${host.isMusl} must not look for the Tegra package`);
    assert.equal(probe.tier1Calls, 0);
    assert.equal(probe.tier2Calls, 0);
    assert.notEqual(probe.required[0], TEGRA_PLATFORM_PACKAGE);
  }
});

test("tegra_loader: a local dev binary still wins over the Tegra package", () => {
  const sentinel = { __fathomdb_native: "local" };
  const probe = jetsonSeams({ tegraInstalledVersion: "0.8.27", tier1: true, gpuName: "Orin (nvgpu)" });
  const mod = loadPlatformBinding({ ...probe.seams, loadLocal: () => sentinel });
  assert.equal(mod, sentinel);
  assert.equal(probe.tier1Calls, 0);
  assert.deepEqual(probe.required, []);
});

test("tegra_loader: a confirmed Tegra package that fails to load is a loud error, never a generic fallback", () => {
  const probe = jetsonSeams({ tegraInstalledVersion: "0.8.27", tier1: true, gpuName: "Orin (nvgpu)" });
  const seams: LoaderSeams = {
    ...probe.seams,
    requirePackage: (pkg) => {
      probe.required.push(pkg);
      throw new Error(`cannot load ${pkg}`);
    },
  };
  assert.throws(
    () => loadPlatformBinding(seams),
    (err: unknown) => {
      assert.ok(err instanceof UnsupportedPlatformError);
      assert.match((err as Error).message, /fathomdb-linux-arm64-gnu-tegra/);
      return true;
    },
  );
  assert.deepEqual(probe.required, [TEGRA_PLATFORM_PACKAGE]);
});
