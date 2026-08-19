// AC-068d — `engine.openReport()` surfaces the native `OpenReport`.
//
// Shape D (locked HITL 2026-05-24): the report is exposed as an
// engine-attached accessor, not a return-shape change on `Engine.open`.
// `engine.openReport()` is sync — the report is a snapshot captured at
// open time and stashed on the napi engine struct; repeat calls return
// identical data.
//
// Spec refs:
// - `dev/design/engine.md` § "`Engine.open` success result" — locked
//   field subset (`schemaVersionBefore`, `schemaVersionAfter`,
//   `migrationSteps`, `embedderWarmupMs`).
// - Native struct: `src/rust/crates/fathomdb-engine/src/lib.rs:541-548`
//   carries two additional fields (`queryBackend`, `defaultEmbedder`).
// - `dev/interfaces/typescript.md` Engine-attached instrumentation list.

import test from "node:test";
import assert from "node:assert/strict";

import { Engine, mapOpenReport } from "../src/index.js";
import { freshDbPath } from "./helpers.js";

test("openReport returns the spec-locked native fields", async () => {
  const engine = await Engine.open(freshDbPath());
  try {
    const report = engine.openReport();

    assert.equal(typeof report.schemaVersionBefore, "number");
    assert.equal(typeof report.schemaVersionAfter, "number");
    assert.ok(report.schemaVersionAfter >= report.schemaVersionBefore);
    assert.ok(Array.isArray(report.migrationSteps));
    assert.equal(typeof report.embedderWarmupMs, "number");
    assert.ok(report.embedderWarmupMs >= 0);
    assert.equal(typeof report.queryBackend, "string");
    assert.ok(report.queryBackend.length > 0);

    const id = report.defaultEmbedder;
    assert.equal(typeof id.name, "string");
    assert.ok(id.name.length > 0);
    assert.equal(typeof id.revision, "string");
    assert.ok(id.revision.length > 0);
    assert.equal(typeof id.dimension, "number");
    assert.ok(id.dimension > 0);
  } finally {
    await engine.close();
  }
});

test("openReport is idempotent — repeat calls return identical data", async () => {
  const engine = await Engine.open(freshDbPath());
  try {
    const first = engine.openReport();
    const second = engine.openReport();

    assert.equal(first.schemaVersionBefore, second.schemaVersionBefore);
    assert.equal(first.schemaVersionAfter, second.schemaVersionAfter);
    assert.equal(first.embedderWarmupMs, second.embedderWarmupMs);
    assert.equal(first.queryBackend, second.queryBackend);
    assert.equal(first.defaultEmbedder.name, second.defaultEmbedder.name);
    assert.equal(first.defaultEmbedder.revision, second.defaultEmbedder.revision);
    assert.equal(first.defaultEmbedder.dimension, second.defaultEmbedder.dimension);

    assert.equal(first.migrationSteps.length, second.migrationSteps.length);
    for (let i = 0; i < first.migrationSteps.length; i += 1) {
      const a = first.migrationSteps[i];
      const b = second.migrationSteps[i];
      assert.equal(a.stepId, b.stepId);
      assert.equal(a.durationMs, b.durationMs);
      assert.equal(a.failed, b.failed);
    }
  } finally {
    await engine.close();
  }
});

test("openReport exposes absent device resolution without an embedder", async () => {
  const engine = await Engine.open(freshDbPath());
  try {
    assert.equal(engine.openReport().embedderDeviceResolution, null);
  } finally {
    await engine.close();
  }
});

test("openReport maps a present auto-to-CPU device resolution", () => {
  const report = mapOpenReport({
    schemaVersionBefore: 1,
    schemaVersionAfter: 1,
    migrationSteps: [],
    embedderWarmupMs: 0,
    queryBackend: "sqlite",
    defaultEmbedder: { name: "test", revision: "test", dimension: 384 },
    embedderDownloadMs: null,
    embedderEvents: [],
    embedderMeanCenteringRequired: false,
    embedderMeanVecPinned: false,
    denseDisabled: false,
    denseDisabledReason: null,
    embedderDeviceResolution: {
      requestedPolicy: "auto",
      cudaCompiled: true,
      effectiveDevice: { kind: "cpu", cudaDevice: null },
      visibleCudaDevices: [
        { visibleOrdinal: 0, uuid: "GPU-first", name: "RTX 3090", computeCapability: "8.6" },
      ],
      selectedCudaUuid: null,
      reason: "cuda_probe_failed",
    },
    rerankerDeviceResolution: {
      requestedPolicy: "cpu",
      cudaCompiled: true,
      effectiveDevice: { kind: "cpu", cudaDevice: null },
      visibleCudaDevices: [],
      selectedCudaUuid: null,
      reason: null,
    },
    embedderGpuAllocationWitness: null,
  });

  assert.deepEqual(report.embedderDeviceResolution, {
    requestedPolicy: "auto",
    cudaCompiled: true,
    effectiveDevice: { kind: "cpu", cudaDevice: null },
    visibleCudaDevices: [
      { visibleOrdinal: 0, uuid: "GPU-first", name: "RTX 3090", computeCapability: "8.6" },
    ],
    selectedCudaUuid: null,
    reason: "cuda_probe_failed",
  });
  assert.deepEqual(report.rerankerDeviceResolution, {
    requestedPolicy: "cpu",
    cudaCompiled: true,
    effectiveDevice: { kind: "cpu", cudaDevice: null },
    visibleCudaDevices: [],
    selectedCudaUuid: null,
    reason: null,
  });
  // 0.8.23 Slice 80.6 (D-80.6-6) — a CUDA policy outcome is not a measurement.
  assert.equal(report.embedderGpuAllocationWitness, null);
});

// 0.8.23 Slice 80.6 (D-80.6-6, AC80-6) — the in-process GPU allocation witness
// reaches the TypeScript surface, or the artifact's own process cannot carry
// the evidence AC80-6's "in-process" clause claims.
test("openReport reports no GPU allocation witness on an ordinary open", async () => {
  const engine = await Engine.open(freshDbPath());
  try {
    assert.equal(engine.openReport().embedderGpuAllocationWitness, null);
  } finally {
    await engine.close();
  }
});

// R80-13 — the record must stay re-derivable: raw samples, floor, and the
// control numbers all survive the mapping, so a reader checks the verdict
// rather than trusting it.
test("openReport maps a present GPU allocation witness without losing a number", () => {
  const witness = {
    schema: "fathomdb.tegra-gpu-allocation-witness/v1",
    soleGpuConsumerPrecondition: "the witness run must be the sole GPU consumer",
    deviceOrdinalRequested: 0,
    deviceOrdinalActual: 0,
    deviceUuid: "GPU-11111111-2222-3333-4444-555555555555",
    deviceName: "Orin",
    computeCapability: "8.7",
    freeBeforeBytes: 40_000_000_000,
    freeAfterBytes: 39_856_635_904,
    totalBytes: 65_000_000_000,
    deltaBytes: 143_364_096,
    deltaFloorBytes: 67_108_864,
    controlAllocationRequestBytes: 1_073_741_824,
    controlBlockCount: 8,
    controlFreeBeforeBytes: 42_000_000_000,
    controlFreeAfterBytes: 40_800_000_000,
    controlDeltaBytes: 1_200_000_000,
    embeddedVectorDim: 384,
  };

  const report = mapOpenReport({
    schemaVersionBefore: 1,
    schemaVersionAfter: 1,
    migrationSteps: [],
    embedderWarmupMs: 0,
    queryBackend: "sqlite",
    defaultEmbedder: { name: "test", revision: "test", dimension: 384 },
    embedderDownloadMs: null,
    embedderEvents: [],
    embedderMeanCenteringRequired: false,
    embedderMeanVecPinned: false,
    denseDisabled: false,
    denseDisabledReason: null,
    embedderDeviceResolution: null,
    rerankerDeviceResolution: null,
    embedderGpuAllocationWitness: witness,
  });

  assert.deepEqual(report.embedderGpuAllocationWitness, witness);

  const mapped = report.embedderGpuAllocationWitness;
  assert.ok(mapped !== null);
  assert.equal(mapped.freeBeforeBytes - mapped.freeAfterBytes, mapped.deltaBytes);
  assert.ok(mapped.deltaBytes >= mapped.deltaFloorBytes);
  assert.ok(
    mapped.controlFreeBeforeBytes - mapped.controlFreeAfterBytes >=
      mapped.controlAllocationRequestBytes,
  );
});

test("Engine.open Promise signature unchanged — resolves to just Engine", async () => {
  const engine = await Engine.open(freshDbPath());
  try {
    assert.ok(engine instanceof Engine);
  } finally {
    await engine.close();
  }
});
