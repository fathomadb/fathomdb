// 0.8.25 Slice 15 TypeScript provenance-wire parity.

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { Engine } from "../src/index.js";
import { ProvenanceError } from "../src/errors.js";
import { freshDbPath } from "./helpers.js";

interface ErrorCase {
  name: string;
  provenance: Record<string, unknown>;
  reason: string;
  fieldPath: string;
}

interface ProvenanceFixture {
  canonicalBody: string;
  camel: {
    canonical: Record<string, unknown>;
    derivedWholeBody: Record<string, unknown>;
    derivedUtf8Bytes: Record<string, unknown>;
    errors: ErrorCase[];
  };
}

const fixture = JSON.parse(
  readFileSync(resolve(process.cwd(), "../conformance/provenance-v1.json"), "utf8"),
) as ProvenanceFixture;

test("shared fixture canonical and both locator variants preserve compact receipt", async () => {
  const engine = await Engine.open(freshDbPath());
  try {
    const receipt = await engine.write([
      {
        kind: "doc",
        body: fixture.canonicalBody,
        sourceId: "source-1",
        logicalId: "source-logical",
        provenance: fixture.camel.canonical,
      },
      {
        kind: "entity",
        body: "derived whole body",
        sourceId: "source-1",
        logicalId: "derived-whole",
        provenance: fixture.camel.derivedWholeBody,
      },
      {
        kind: "mentions",
        from: "source-logical",
        to: "derived-whole",
        body: "é",
        sourceId: "source-1",
        logicalId: "derived-bytes",
        provenance: fixture.camel.derivedUtf8Bytes,
      },
    ]);
    assert.deepEqual(Object.keys(receipt).sort(), [
      "cursor",
      "danglingEdgeEndpoints",
      "rowCursors",
    ]);
    assert.deepEqual(receipt.rowCursors, [1, 2, 3]);
  } finally {
    await engine.close();
  }
});

for (const errorCase of fixture.camel.errors) {
  test(`shared fixture rejects ${errorCase.name} with reason/path parity`, async () => {
    const engine = await Engine.open(freshDbPath());
    try {
      await assert.rejects(
        () =>
          engine.write([
            {
              kind: "doc",
              body: "body",
              sourceId: "source-1",
              provenance: errorCase.provenance,
            },
          ]),
        (error: unknown) => {
          assert.ok(error instanceof ProvenanceError);
          assert.equal(error.code, "FDB_PROVENANCE");
          assert.equal(error.reason, errorCase.reason);
          assert.equal(error.fieldPath, errorCase.fieldPath);
          return true;
        },
      );
    } finally {
      await engine.close();
    }
  });
}

for (const provenance of [
  { schemaVersion: 1, role: "future" },
  { schemaVersion: 1, role: "future", artifactRevisionId: null, sourceVersionId: 7 },
]) {
  test("unknown role precedes missing or malformed shared IDs", async () => {
    const engine = await Engine.open(freshDbPath());
    try {
      await assert.rejects(
        () => engine.write([{ kind: "doc", body: "body", sourceId: "s", provenance }]),
        (error: unknown) => {
          assert.ok(error instanceof ProvenanceError);
          assert.equal(error.code, "FDB_PROVENANCE");
          assert.equal(error.reason, "role_invalid");
          assert.equal(error.fieldPath, "/provenance/role");
          return true;
        },
      );
    } finally {
      await engine.close();
    }
  });
}
