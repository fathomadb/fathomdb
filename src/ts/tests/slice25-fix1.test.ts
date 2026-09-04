import assert from "node:assert/strict";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";

import { ActuationError, Engine, type ActuationBatchV1 } from "../src/index.js";

test("nested provenance decode preserves its canonical pointer", async () => {
  const dir = mkdtempSync(join(tmpdir(), "fathomdb-actuation-provenance-"));
  const engine = await Engine.open(join(dir, "actuation.fathom"));
  const request = {
    schemaVersion: 1,
    operationId: "nested-provenance",
    operations: [
      {
        type: "put_canonical_node",
        record: {
          kind: "doc",
          body: "body",
          sourceId: "source-a",
          provenance: {
            schemaVersion: 1,
            role: "canonical",
            sourceVersionId: "version-r1",
          },
        },
      },
    ],
  };

  await assert.rejects(
    engine.actuate(request as ActuationBatchV1),
    (error: unknown) =>
      error instanceof ActuationError &&
      error.reason === "nested_request_invalid" &&
      error.fieldPath === "/operations/0/record/provenance/artifactRevisionId",
  );
  await engine.close();
});

test("nested dependency decode preserves its canonical pointer", async () => {
  const dir = mkdtempSync(join(tmpdir(), "fathomdb-actuation-dependency-"));
  const engine = await Engine.open(join(dir, "actuation.fathom"));
  const request = {
    schemaVersion: 1,
    operationId: "nested-dependency",
    operations: [
      {
        type: "register_source_dependency",
        dependency: {
          schemaVersion: 1,
          dependencyId: "dep-r1",
          sourceRevisionId: "source-r1",
        },
      },
    ],
  };

  await assert.rejects(
    engine.actuate(request as ActuationBatchV1),
    (error: unknown) =>
      error instanceof ActuationError &&
      error.reason === "nested_request_invalid" &&
      error.fieldPath === "/operations/0/dependency/derivedRevisionId",
  );
  await engine.close();
});
