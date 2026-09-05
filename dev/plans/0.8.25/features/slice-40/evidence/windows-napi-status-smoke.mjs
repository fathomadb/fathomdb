import assert from "node:assert/strict";
import { pathToFileURL } from "node:url";

const moduleUrl = pathToFileURL(process.argv[2]).href;
const { Engine, read } = await import(moduleUrl);
const database = process.argv[3];
let engine = await Engine.open(database, { useDefaultEmbedder: true });
const first = await read.projectionGenerationStatus(engine);
assert.equal(first.origin, "fresh");
assert.equal(first.readiness, "ready");
await engine.actuate({
  schemaVersion: 1,
  operationId: "slice40-windows-napi-package-seed",
  operations: [
    {
      type: "put_canonical_node",
      record: {
        kind: "doc",
        body: "seed",
        logicalId: "seed",
        sourceId: "source:slice40-windows-napi-package-seed",
        provenance: {
          schemaVersion: 1,
          artifactRevisionId: "slice40-windows-napi-package-seed-r1",
          sourceVersionId: "slice40-windows-napi-package-seed-v1",
          role: "canonical",
        },
      },
    },
  ],
});
await engine.configureProjections([
  { name: "memory", roles: ["searchable"], fts: false, vector: true },
]);
await engine.drain(10_000);
await engine.close();
engine = await Engine.open(database, { useDefaultEmbedder: false });
const receipt = await engine.actuate({
  schemaVersion: 1,
  operationId: "slice40-windows-napi-package-smoke",
  operations: [
    {
      type: "put_canonical_node",
      record: {
        kind: "doc",
        body: "windows napi generation status",
        logicalId: "node",
        sourceId: "source:slice40-windows-napi-package-smoke",
        provenance: {
          schemaVersion: 1,
          artifactRevisionId: "slice40-windows-napi-package-smoke-r1",
          sourceVersionId: "slice40-windows-napi-package-smoke-v1",
          role: "canonical",
        },
      },
    },
  ],
});
assert.ok(receipt.projectionGenerationId);
assert.equal(receipt.pendingProjectionWriteCursors.length, 1);
const status = await read.mutationProjectionStatus(engine, {
  schemaVersion: 1,
  operationId: receipt.operationId,
  writeCursor: receipt.pendingProjectionWriteCursors[0],
  expectedGenerationId: receipt.projectionGenerationId,
});
assert.equal(status.readiness, "blocked");
assert.equal(status.runtimeState, "absent");
const currentId = (await read.projectionGenerationStatus(engine)).generationId;
await engine.close();
const reopened = await Engine.open(database, { useDefaultEmbedder: false });
assert.equal((await read.projectionGenerationStatus(reopened)).generationId, currentId);
await reopened.close();
console.log(JSON.stringify({
  outcome: "pass",
  consumer: "napi-native",
  module: process.argv[2],
  generationId: currentId,
  mutationReadiness: status.readiness,
  runtimeState: status.runtimeState,
}));
