import assert from "node:assert/strict";
import test from "node:test";

import { Engine, read, type ProjectionGenerationStatusV1 } from "../src/index.js";
import { freshDbPath } from "./helpers.js";

test("projection generation status is typed and stable", async () => {
  const path = freshDbPath();
  const engine = await Engine.open(path, { useDefaultEmbedder: false });
  const first: ProjectionGenerationStatusV1 = await read.projectionGenerationStatus(engine);
  assert.equal(first.schemaVersion, 1);
  assert.match(first.generationId, /^pgen1:[0-9a-f]{32}$/);
  assert.equal(first.origin, "fresh");
  assert.equal(first.readiness, "ready");
  assert.equal(first.runtimeState, "absent");
  await engine.close();

  const reopened = await Engine.open(path, { useDefaultEmbedder: false });
  assert.equal((await read.projectionGenerationStatus(reopened)).generationId, first.generationId);
  await reopened.close();
});

test("actuation receipt carries the additive nullable generation", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  const receipt = await engine.actuate({
    schemaVersion: 1,
    operationId: "slice40-typescript",
    operations: [
      {
        type: "put_canonical_node",
        record: {
          kind: "doc",
          body: "body",
          logicalId: "node",
          sourceId: "source:slice40-typescript",
          provenance: {
            schemaVersion: 1,
            artifactRevisionId: "slice40-typescript-r1",
            sourceVersionId: "slice40-typescript-v1",
            role: "canonical",
          },
        },
      },
    ],
  });
  assert.equal(receipt.projectionGenerationId, null);
  await engine.close();
});

test("receipt-keyed mutation status round-trips through TypeScript", async () => {
  const path = freshDbPath();
  const configuring = await Engine.open(path, { useDefaultEmbedder: true });
  await configuring.actuate({
    schemaVersion: 1,
    operationId: "slice40-typescript-status-seed",
    operations: [
      {
        type: "put_canonical_node",
        record: {
          kind: "doc",
          body: "seed body",
          logicalId: "seed-node",
          sourceId: "source:slice40-typescript-status-seed",
          provenance: {
            schemaVersion: 1,
            artifactRevisionId: "slice40-typescript-status-seed-r1",
            sourceVersionId: "slice40-typescript-status-seed-v1",
            role: "canonical",
          },
        },
      },
    ],
  });
  await configuring.configureProjections([
    { name: "memory", roles: ["searchable"], fts: false, vector: true },
  ]);
  await configuring.drain(10_000);
  await configuring.close();

  const engine = await Engine.open(path, { useDefaultEmbedder: false });
  const receipt = await engine.actuate({
    schemaVersion: 1,
    operationId: "slice40-typescript-mutation-status",
    operations: [
      {
        type: "put_canonical_node",
        record: {
          kind: "doc",
          body: "body",
          logicalId: "node",
          sourceId: "source:slice40-typescript-status",
          provenance: {
            schemaVersion: 1,
            artifactRevisionId: "slice40-typescript-status-r1",
            sourceVersionId: "slice40-typescript-status-v1",
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
  assert.equal(status.operationId, receipt.operationId);
  assert.equal(status.writeCursor, receipt.pendingProjectionWriteCursors[0]);
  assert.equal(status.generationId, receipt.projectionGenerationId);
  assert.equal(status.readiness, "blocked");
  assert.equal(status.runtimeState, "absent");
  assert.equal(status.pendingCount, "1");
  await engine.close();
});
