import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";

import {
  ActuationError,
  ClosingError,
  DatabaseNotFoundError,
  Engine,
  FathomDbError,
  read,
} from "../src/index.js";
import { freshDbPath } from "./helpers.js";

async function registerCollection(
  engine: Engine,
  name: string,
  kind: "latest_state" | "append_only_log",
  schemaJson: string,
): Promise<void> {
  await engine.write([
    {
      adminSchema: {
        name,
        kind,
        schemaJson,
        retentionJson: "{}",
      },
    },
  ]);
}

test("WP-40A2: put_latest_state in ActuationBatchV1 commits, replays, conflicts, and refuses with exact paths", async () => {
  const dbPath = freshDbPath();
  const engine = await Engine.open(dbPath);
  const schema = JSON.stringify({
    type: "object",
    required: ["step"],
    properties: { step: { type: "integer" } },
  });
  try {
    await registerCollection(engine, "checkpoints", "latest_state", schema);
    await registerCollection(engine, "events", "append_only_log", "{}");

    const batch = {
      schemaVersion: 1 as const,
      operationId: "ts-pin-c-op-1",
      operations: [
        {
          type: "put_canonical_node" as const,
          record: {
            kind: "note",
            body: "ts checkpointed node",
            sourceId: "src-ts-pin-c",
            logicalId: "ts-node-1",
            provenance: {
              schemaVersion: 1,
              role: "canonical",
              artifactRevisionId: "ts-rev-1",
              sourceVersionId: "ts-ver-1",
            },
          },
        },
        {
          type: "put_latest_state" as const,
          collection: "checkpoints",
          recordKey: "run-1",
          payloadJson: '{"step":11}',
        },
      ],
    };

    const receipt = await engine.actuate(batch);
    assert.equal(receipt.outcome, "committed");
    assert.deepEqual(receipt.affectedRevisionIds, ["ts-rev-1"]);
    assert.ok(receipt.resultingWriteBoundary !== null);

    const row = await read.operationalState(engine, "checkpoints", "run-1");
    assert.ok(row !== null);
    assert.equal(row.payload, '{"step":11}');
    assert.equal(row.schemaId, "checkpoints");

    // Exact replay returns identical receipt
    const replay = await engine.actuate(batch);
    assert.deepEqual(replay, receipt);

    // Differing payload under same operationId throws ActuationError(operation_id_conflict)
    await assert.rejects(
      () =>
        engine.actuate({
          ...batch,
          operations: [
            batch.operations[0],
            {
              type: "put_latest_state",
              collection: "checkpoints",
              recordKey: "run-1",
              payloadJson: '{"step":99}',
            },
          ],
        }),
      (err: unknown) => err instanceof ActuationError && err.reason === "operation_id_conflict",
    );

    // Unregistered collection -> /operations/0/collection
    const rUnreg = await engine.actuate({
      schemaVersion: 1,
      operationId: "ts-pin-c-unreg",
      operations: [
        {
          type: "put_latest_state",
          collection: "missing_col",
          recordKey: "k1",
          payloadJson: "{}",
        },
      ],
    });
    assert.equal(rUnreg.outcome, "refused");
    assert.deepEqual(rUnreg.reasonCodes, ["write_refused"]);
    assert.equal(rUnreg.refusedFieldPath, "/operations/0/collection");

    // Wrong kind -> /operations/0/collection
    const rKind = await engine.actuate({
      schemaVersion: 1,
      operationId: "ts-pin-c-kind",
      operations: [
        {
          type: "put_latest_state",
          collection: "events",
          recordKey: "k1",
          payloadJson: "{}",
        },
      ],
    });
    assert.equal(rKind.outcome, "refused");
    assert.equal(rKind.refusedFieldPath, "/operations/0/collection");

    // Schema violation -> /operations/0/payloadJson
    const rSchema = await engine.actuate({
      schemaVersion: 1,
      operationId: "ts-pin-c-schema",
      operations: [
        {
          type: "put_latest_state",
          collection: "checkpoints",
          recordKey: "k1",
          payloadJson: '{"step":"bad"}',
        },
      ],
    });
    assert.equal(rSchema.outcome, "refused");
    assert.equal(rSchema.refusedFieldPath, "/operations/0/payloadJson");
  } finally {
    await engine.close();
  }
});

test("WP-40A2: EP-07 createIfMissing=false refuses missing & zero-length paths cleanly", async () => {
  const missing = freshDbPath();
  const lockFile = `${missing}.lock`;

  await assert.rejects(
    () => Engine.open(missing, { createIfMissing: false }),
    (err: unknown) =>
      err instanceof DatabaseNotFoundError &&
      err instanceof FathomDbError &&
      typeof err.path === "string",
  );
  assert.equal(fs.existsSync(missing), false);
  assert.equal(fs.existsSync(lockFile), false);

  const zeroFile = path.join(path.dirname(missing), "zero.sqlite");
  fs.writeFileSync(zeroFile, "");
  await assert.rejects(
    () => Engine.open(zeroFile, { createIfMissing: false }),
    (err: unknown) => err instanceof DatabaseNotFoundError,
  );
  assert.equal(fs.statSync(zeroFile).size, 0);
  assert.equal(fs.existsSync(`${zeroFile}.lock`), false);

  const existing = freshDbPath();
  const e1 = await Engine.open(existing);
  await e1.close();
  const e2 = await Engine.open(existing, { createIfMissing: false });
  await e2.close();
});

test("WP-40A2: EP-01 read.visibilityState tracks writes, reopen stability, and close", async () => {
  const dbPath = freshDbPath();
  const e1 = await Engine.open(dbPath);
  let s0;
  try {
    s0 = await read.visibilityState(e1);
    assert.equal(s0.schemaVersion, 1);
    assert.equal(s0.databaseId.length, 32);
    assert.equal(s0.nonce.length, 64);
    assert.equal(s0.canonicalWriteBoundary, 0);
    assert.equal(s0.projectionCursor, 0);
    assert.deepEqual(await read.visibilityState(e1), s0);
  } finally {
    await e1.close();
  }

  await assert.rejects(
    () => read.visibilityState(e1),
    (err: unknown) => err instanceof ClosingError,
  );

  const e2 = await Engine.open(dbPath, { createIfMissing: false });
  try {
    assert.deepEqual(await read.visibilityState(e2), s0);
    await e2.write([
      {
        type: "node",
        kind: "note",
        body: "ts visibility test",
        sourceId: "src-ts-vis",
        logicalId: "ts-vis-1",
      },
    ]);
    const s1 = await read.visibilityState(e2);
    assert.equal(s1.databaseId, s0.databaseId);
    assert.ok(s1.generation > s0.generation);
    assert.notEqual(s1.nonce, s0.nonce);
    assert.equal(s1.canonicalWriteBoundary, 1);
  } finally {
    await e2.close();
  }
});
