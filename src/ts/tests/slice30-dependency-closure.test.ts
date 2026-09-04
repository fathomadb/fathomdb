import test from "node:test";
import assert from "node:assert/strict";

import { Engine } from "../src/index.js";
import { DependencyClosureError } from "../src/errors.js";
import { freshDbPath } from "./helpers.js";

test("closure lookup is closed and absent ids disclose nothing", async () => {
  const engine = await Engine.open(freshDbPath());
  try {
    assert.equal(
      await engine.readDependencyClosure({
        schemaVersion: 1,
        closureOperationId: `_fdb:c:${"a".repeat(64)}`,
      }),
      null,
    );
    await assert.rejects(
      () =>
        engine.readDependencyClosure({
          schemaVersion: 2 as 1,
          closureOperationId: `_fdb:c:${"a".repeat(64)}`,
        }),
      (error: unknown) => {
        assert.ok(error instanceof DependencyClosureError);
        assert.equal(error.reason, "unsupported_schema_version");
        assert.equal(error.fieldPath, "/schemaVersion");
        return true;
      },
    );
  } finally {
    await engine.close();
  }
});
