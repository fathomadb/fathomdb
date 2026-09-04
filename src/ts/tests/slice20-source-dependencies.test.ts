import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { Engine } from "../src/index.js";
import { DependencyError } from "../src/errors.js";
import { freshDbPath } from "./helpers.js";

const fixture = JSON.parse(
  readFileSync(
    resolve(process.cwd(), "../../dev/fixtures/slice20-dependency-conformance-v1.json"),
    "utf8",
  ),
) as {
  validRegistration: {
    dependencyId: string;
    sourceRevisionId: string;
    derivedRevisionId: string;
  };
  failures: Array<{
    request: Record<string, unknown>;
    reason: string;
    fieldPath: string;
  }>;
};

async function seed(engine: Engine): Promise<void> {
  const body = "source bytes";
  await engine.write([
    {
      kind: "doc",
      body,
      sourceId: "bucket",
      logicalId: "source",
      provenance: {
        schemaVersion: 1,
        role: "canonical",
        artifactRevisionId: "source-r1",
        sourceVersionId: "v1",
      },
    },
    {
      kind: "fact",
      body: "derived",
      sourceId: "bucket",
      logicalId: "derived",
      provenance: {
        schemaVersion: 1,
        role: "derived",
        artifactRevisionId: "derived-r1",
        sourceVersionId: "v1",
        sourceRevisionId: "source-r1",
        sourceLocator: { kind: "whole_body" },
        canonicalSourceHash: {
          algorithm: "sha256",
          digestHex: createHash("sha256").update(body).digest("hex"),
        },
      },
    },
  ]);
}

test("dependency round trip uses canonical decimal generation", async () => {
  const engine = await Engine.open(freshDbPath());
  try {
    await seed(engine);
    const request = { schemaVersion: 1 as const, ...fixture.validRegistration };
    const registered = await engine.registerSourceDependency(request);
    assert.equal(registered.registeredDependencyGeneration, "1");
    assert.deepEqual(await engine.registerSourceDependency(request), registered);
    assert.deepEqual(
      (await engine.dependenciesForSource({ schemaVersion: 1, sourceRevisionId: "source-r1" }))
        .items,
      [registered],
    );
    assert.deepEqual(
      await engine.dependencyForDerived({ schemaVersion: 1, derivedRevisionId: "derived-r1" }),
      registered,
    );
  } finally {
    await engine.close();
  }
});

for (const failure of fixture.failures) {
  test(`shared fixture dependency error ${failure.reason} ${failure.fieldPath}`, async () => {
    const engine = await Engine.open(freshDbPath());
    try {
      await assert.rejects(
        () => engine.registerSourceDependency(failure.request as never),
        (error: unknown) => {
          assert.ok(error instanceof DependencyError);
          assert.equal(error.code, "FDB_DEPENDENCY");
          assert.equal(error.reason, failure.reason);
          assert.equal(error.fieldPath, failure.fieldPath);
          return true;
        },
      );
    } finally {
      await engine.close();
    }
  });
}
