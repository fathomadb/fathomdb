// 0.8.25 Slice 15 TypeScript provenance-wire parity.

import test from "node:test";
import assert from "node:assert/strict";

import { Engine } from "../src/index.js";
import { ProvenanceError } from "../src/errors.js";
import { freshDbPath } from "./helpers.js";

test("canonical provenance write preserves the compact receipt", async () => {
  const engine = await Engine.open(freshDbPath());
  try {
    const receipt = await engine.write([
      {
        kind: "doc",
        body: "AéB",
        sourceId: "source-1",
        logicalId: "source-logical",
        provenance: {
          schemaVersion: 1,
          role: "canonical",
          artifactRevisionId: "source-revision-1",
          sourceVersionId: "source-v1",
        },
      },
    ]);
    assert.deepEqual(Object.keys(receipt).sort(), [
      "cursor",
      "danglingEdgeEndpoints",
      "rowCursors",
    ]);
  } finally {
    await engine.close();
  }
});

test("versioned provenance object rejects unknown fields with reason and pointer", async () => {
  const engine = await Engine.open(freshDbPath());
  try {
    await assert.rejects(
      () =>
        engine.write([
          {
            kind: "doc",
            body: "body",
            sourceId: "source-1",
            provenance: {
              schemaVersion: 1,
              role: "canonical",
              artifactRevisionId: "revision-1",
              sourceVersionId: "source-v1",
              futureField: true,
            },
          },
        ]),
      (error: unknown) => {
        assert.ok(error instanceof ProvenanceError);
        assert.equal(error.reason, "unknown_field");
        assert.equal(error.fieldPath, "/provenance/futureField");
        return true;
      },
    );
  } finally {
    await engine.close();
  }
});
