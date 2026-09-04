import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { DatabaseSync } from "node:sqlite";

import { Engine, StorageError } from "../src/index.js";
import { freshDbPath } from "./helpers.js";

async function seedAndRegister(engine: Engine): Promise<void> {
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
  await engine.registerSourceDependency({
    schemaVersion: 1,
    dependencyId: "dep-1",
    sourceRevisionId: "source-r1",
    derivedRevisionId: "derived-r1",
  });
}

test("consistent invalid canonical source revision maps to StorageError", async () => {
  const path = freshDbPath();
  const engine = await Engine.open(path);
  await seedAndRegister(engine);
  await engine.close();

  const db = new DatabaseSync(path);
  try {
    db.exec(
      "UPDATE _fathomdb_artifact_revisions SET revision_id='_bad-rev' " +
        "WHERE revision_id='source-r1'",
    );
    db.exec(
      "UPDATE _fathomdb_source_versions SET source_revision_id='_bad-rev' " +
        "WHERE source_revision_id='source-r1'",
    );
    db.exec(
      "UPDATE _fathomdb_source_links " +
        "SET artifact_revision_id='_bad-rev', source_revision_id='_bad-rev' " +
        "WHERE artifact_revision_id='source-r1'",
    );
    db.exec(
      "UPDATE _fathomdb_source_links SET source_revision_id='_bad-rev' " +
        "WHERE artifact_revision_id='derived-r1'",
    );
  } finally {
    db.close();
  }

  const reopened = await Engine.open(path);
  try {
    await assert.rejects(
      () =>
        reopened.dependencyForDerived({
          schemaVersion: 1,
          derivedRevisionId: "derived-r1",
        }),
      StorageError,
    );
    await assert.rejects(
      () =>
        reopened.registerSourceDependency({
          schemaVersion: 1,
          dependencyId: "dep-1",
          sourceRevisionId: "source-r1",
          derivedRevisionId: "derived-r1",
        }),
      StorageError,
    );
  } finally {
    await reopened.close();
  }
});
