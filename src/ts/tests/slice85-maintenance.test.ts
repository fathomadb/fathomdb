import test from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Engine } from "../src/index.js";

test("explicit policy and exclusive handle persist with typed newcomer refusal", async () => {
  const dir = mkdtempSync(join(tmpdir(), "slice85-policy-"));
  const path = join(dir, "store.sqlite");
  const engine = await Engine.open(path, { engineConfig: { physicalErasurePolicy: "explicit_maintenance" } });
  try {
    assert.equal(engine.physicalErasurePolicy(), "explicit_maintenance");
    const handle = await engine.beginErasureMaintenance(100);
    try {
      await assert.rejects(engine.eraseSource("test-source"), { reason: "maintenance_busy" });
    } finally {
      assert.equal(await handle.close(), "normal_with_queued_restrictions");
    }
    await assert.rejects(engine.eraseSource("test-source"), { reason: "maintenance_required" });
  } finally { await engine.close(); }
  const reopened = await Engine.open(path);
  try { assert.equal(reopened.physicalErasurePolicy(), "explicit_maintenance"); }
  finally { await reopened.close(); rmSync(dir, { recursive: true }); }
});
