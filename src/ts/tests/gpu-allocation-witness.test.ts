// Pure SDK mapping: importing the private mapper never loads a native addon.
import test from "node:test";
import assert from "node:assert/strict";
import { mapOpenReport } from "../src/open.js";

const globalMeasurement = {
  kind: "global_free",
  controlAllocationRequestBytes: 1_073_741_824,
  controlBlockCount: 8,
  controlFreeBeforeBytes: 42_000_000_000,
  controlFreeAfterBytes: 40_800_000_000,
  controlDeltaBytes: 1_200_000_000,
  soleGpuConsumerPrecondition: "the witness process must be the sole GPU consumer of the selected discrete device",
} as const;

const privateMeasurement = {
  kind: "private_pool_used",
  contextId: "18446744073709551615",
  poolIdentity: "0x1234",
  usedBeforeBytes: 0,
  usedAfterBytes: 132_848_640,
  deltaBytes: 132_848_640,
  controlRequestBytes: 67_108_864,
  controlBeforeBytes: 132_848_640,
  controlLiveBytes: 199_957_504,
  controlReleasedBytes: 132_848_640,
  poolExclusivityPrecondition: "no competing allocations or frees may occur in the witnessed private pool during model and control observation",
} as const;

for (const measurement of [globalMeasurement, privateMeasurement, { ...privateMeasurement, contextId: "0" }] as const) {
  test(`openReport maps current GPU witness ${measurement.kind} ${"contextId" in measurement ? measurement.contextId : ""} intact`, () => {
    const integrated = measurement.kind === "private_pool_used";
    const witness = {
      schema: "fathomdb.tegra-gpu-allocation-witness/v2",
      processId: 4242,
      deviceIntegrated: integrated,
      deviceOrdinalRequested: 0,
      deviceOrdinalActual: 0,
      deviceUuid: "GPU-11111111-2222-3333-4444-555555555555",
      deviceName: "Orin",
      computeCapability: "8.7",
      freeBeforeBytes: 40_000_000_000,
      freeAfterBytes: integrated ? 40_374_050_816 : 39_856_635_904,
      totalBytes: 65_000_000_000,
      deltaBytes: integrated ? -374_050_816 : 143_364_096,
      deltaFloorBytes: 67_108_864,
      embeddedVectorDim: 384,
      measurement,
    };
    const report = mapOpenReport({
      schemaVersionBefore: 1, schemaVersionAfter: 1, migrationSteps: [],
      embedderWarmupMs: 0, queryBackend: "sqlite",
      defaultEmbedder: { name: "test", revision: "test", dimension: 384 },
      embedderDownloadMs: null, embedderEvents: [],
      embedderMeanCenteringRequired: false, embedderMeanVecPinned: false,
      embedderDeviceResolution: null, rerankerDeviceResolution: null,
      embedderGpuAllocationWitness: witness,
    });
    assert.deepEqual(report.embedderGpuAllocationWitness, witness);
    const mapped = report.embedderGpuAllocationWitness;
    assert.ok(mapped !== null);
    assert.equal(mapped.freeBeforeBytes - mapped.freeAfterBytes, mapped.deltaBytes);
    assert.equal(mapped.processId, 4242);
    assert.equal(mapped.deviceIntegrated, integrated);
    if (mapped.measurement.kind === "global_free") {
      assert.ok(mapped.deltaBytes >= mapped.deltaFloorBytes);
      assert.equal(mapped.measurement.controlFreeBeforeBytes - mapped.measurement.controlFreeAfterBytes,
        mapped.measurement.controlDeltaBytes);
      assert.ok(mapped.measurement.controlDeltaBytes >= mapped.measurement.controlAllocationRequestBytes);
    } else {
      assert.ok(mapped.deltaBytes < 0, "global diagnostic remains negative");
      assert.equal(mapped.measurement.usedAfterBytes - mapped.measurement.usedBeforeBytes, mapped.measurement.deltaBytes);
      assert.ok(mapped.measurement.deltaBytes >= mapped.deltaFloorBytes);
      assert.equal(mapped.measurement.controlLiveBytes - mapped.measurement.controlBeforeBytes, 67_108_864);
      assert.equal(mapped.measurement.controlReleasedBytes, mapped.measurement.controlBeforeBytes);
      assert.equal(typeof mapped.measurement.contextId, "string");
      assert.equal(JSON.parse(JSON.stringify(mapped)).measurement.contextId,
        "contextId" in measurement ? measurement.contextId : "unexpected");
    }
  });
}
