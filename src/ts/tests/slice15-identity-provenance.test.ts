// 0.8.25 Slice 15 TypeScript provenance-wire parity.

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { Engine } from "../src/index.js";
import { ProvenanceError, WriteValidationError } from "../src/errors.js";
import { freshDbPath } from "./helpers.js";

interface ErrorCase {
  name: string;
  provenance: Record<string, unknown>;
  reason: string;
  fieldPath: string;
  seedCanonical?: boolean;
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
      if (errorCase.seedCanonical) {
        await engine.write([
          {
            kind: "doc",
            body: fixture.canonicalBody,
            sourceId: "source-1",
            logicalId: "fixture-source",
            provenance: fixture.camel.canonical,
          },
        ]);
      }
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

for (const wrapped of [false, true]) {
  for (const entity of ["node", "edge"] as const) {
    for (const badId of ["bad\0revision", "bad\ud800revision"]) {
      test(`${wrapped ? "wrapped" : "direct"} ${entity} provenance encoding maps to provenance`, async () => {
        const engine = await Engine.open(freshDbPath());
        try {
          const provenance: Record<string, unknown> = {
            ...fixture.camel.canonical,
            artifactRevisionId: badId,
          };
          let inner: Record<string, unknown>;
          if (entity === "node") {
            inner = {
              kind: "doc",
              body: "body",
              sourceId: "source-1",
              provenance,
            };
          } else {
            Object.assign(provenance, {
              role: "derived",
              sourceRevisionId: "missing-source",
              sourceLocator: { kind: "whole_body" },
              canonicalSourceHash: fixture.camel.derivedWholeBody.canonicalSourceHash,
            });
            inner = {
              kind: "mentions",
              from: "a",
              to: "b",
              body: "body",
              sourceId: "source-1",
              provenance,
            };
          }
          const item = wrapped ? { [entity]: inner } : inner;
          await assert.rejects(
            () => engine.write([item]),
            (error: unknown) => {
              assert.ok(error instanceof ProvenanceError);
              assert.equal(error.code, "FDB_PROVENANCE");
              assert.equal(error.reason, "revision_id_invalid");
              assert.equal(error.fieldPath, "/provenance/artifactRevisionId");
              return true;
            },
          );
        } finally {
          await engine.close();
        }
      });
    }
  }
}

for (const entity of ["node", "edge"] as const) {
  for (const wrapped of [false, true]) {
    test(`${wrapped ? "wrapped" : "direct"} versioned ${entity} body must be a string`, async () => {
      const engine = await Engine.open(freshDbPath());
      try {
        const inner: Record<string, unknown> = {
          kind: entity === "node" ? "doc" : "mentions",
          body: { not: "a string" },
          sourceId: "source-1",
          provenance:
            entity === "node" ? fixture.camel.canonical : fixture.camel.derivedWholeBody,
        };
        if (entity === "edge") {
          Object.assign(inner, { from: "a", to: "b" });
        }
        const item = wrapped ? { [entity]: inner } : inner;
        await assert.rejects(() => engine.write([item]), WriteValidationError);
      } finally {
        await engine.close();
      }
    });
  }
}

test("canonical role is illegal for edges", async () => {
  const engine = await Engine.open(freshDbPath());
  try {
    await assert.rejects(
      () =>
        engine.write([
          {
            edge: {
              kind: "mentions",
              from: "a",
              to: "b",
              body: "body",
              sourceId: "source-1",
              provenance: fixture.camel.canonical,
            },
          },
        ]),
      (error: unknown) => {
        assert.ok(error instanceof ProvenanceError);
        assert.equal(error.reason, "role_invalid");
        assert.equal(error.fieldPath, "/provenance/role");
        return true;
      },
    );
  } finally {
    await engine.close();
  }
});

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
