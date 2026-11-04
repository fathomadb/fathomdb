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

test("queued intent is unchanged, exact retry and withdrawal are keyed", async () => {
  const dir = mkdtempSync(join(tmpdir(), "slice85-queue-"));
  const engine = await Engine.open(join(dir, "store.sqlite"), { engineConfig: { physicalErasurePolicy: "explicit_maintenance" } });
  const selector = { kind: "source_all_at_cut" as const, sourceId: "test-source" };
  const opaque = "0123456789abcdef0123456789abcdef";
  try {
    assert.deepEqual(await engine.queuePhysicalErasure(selector, opaque), { outcome: "needs_maintenance_admission" });
    const handle = await engine.beginErasureMaintenance(100);
    const accepted = await handle.queuePhysicalErasure(selector, opaque);
    assert.equal(accepted.outcome, "accepted");
    if (accepted.outcome !== "accepted") throw new Error("heavy admission refused");
    assert.equal(accepted.logicalState, "unchanged_by_this_request");
    await handle.close();
    assert.deepEqual(await engine.queuePhysicalErasure(selector, opaque), accepted);
    assert.equal((await engine.physicalErasureStatus(accepted.ticket))?.phase, "queued");
    assert.equal(await engine.withdrawQueuedErasure(accepted.ticket), "withdrawn_before_removal");
    assert.equal((await engine.physicalErasureStatus(accepted.ticket))?.phase, "withdrawn_before_removal");
  } finally { await engine.close(); rmSync(dir, { recursive: true }); }
});
