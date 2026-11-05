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

test("complete receipt follows physical byte removal and preserves survivor", async () => {
  const { readFileSync, existsSync } = await import("node:fs");
  const dir = mkdtempSync(join(tmpdir(), "slice85-physical-"));
  const path = join(dir, "store.sqlite");
  const engine = await Engine.open(path, { engineConfig: { physicalErasurePolicy: "explicit_maintenance" } });
  const marker = "typescriptphysicalmarker85";
  await engine.write([
    { kind: "doc", body: marker, sourceId: "target-source" },
    { kind: "doc", body: "retainedsurvivor85", sourceId: "kept-source" },
  ]);
  assert.ok(Buffer.concat([readFileSync(path), readFileSync(`${path}-wal`)]).includes(Buffer.from(marker)));
  try {
    const handle = await engine.beginErasureMaintenance(1000);
    try {
      const accepted = await handle.queuePhysicalErasure({ kind: "source_all_at_cut", sourceId: "target-source" }, "f23e2130b50242e3b8d545d4fa3d8015");
      if (accepted.outcome !== "accepted") throw new Error("heavy admission refused");
      const result = await handle.completePhysicalErasure(accepted.ticket);
      assert.equal(result.outcome, "complete");
      if (result.outcome !== "complete") throw new Error("physical completion blocked");
      assert.equal(result.receipt.logicalState, "removed");
      assert.equal(result.receipt.phase, "complete");
    } finally { await handle.close(); }
    for (const artifact of [path, `${path}-wal`]) {
      if (existsSync(artifact)) assert.ok(!readFileSync(artifact).includes(Buffer.from(marker)));
    }
    assert.equal((await engine.searchTextOnly("retainedsurvivor85")).results.length, 1);
  } finally { await engine.close(); rmSync(dir, { recursive: true }); }
});

test("discarded caller and handle retain native ownership to terminal receipt", async (t) => {
  const { spawnSync } = await import("node:child_process");
  const { native } = await import("../src/binding.js");
  const probeDir = mkdtempSync(join(tmpdir(), "slice85-probe-"));
  const raw = await native.Engine.open(join(probeDir, "probe.sqlite"), { engineConfig: { physicalErasurePolicy: "explicit_maintenance" } });
  const rawHandle = await raw.beginErasureMaintenance(1000);
  const available = typeof (rawHandle as unknown as Record<string, unknown>)._armErasurePauseForTest === "function";
  await rawHandle.close(); await raw.close(); rmSync(probeDir, { recursive: true });
  if (!available) { t.skip("requires dev-only native erasure rendezvous"); return; }
  const dir = mkdtempSync(join(tmpdir(), "slice85-drop-"));
  const module = new URL("../src/index.js", import.meta.url).href;
  const script = `
    import assert from "node:assert/strict";
    import { setTimeout as delay } from "node:timers/promises";
    const { Engine, MaintenanceError } = await import(${JSON.stringify(module)});
    const engine = await Engine.open(${JSON.stringify(join(dir, "store.sqlite"))}, { engineConfig: { physicalErasurePolicy: "explicit_maintenance" } });
    await engine.write([{kind:"doc",body:"discardedcallermarker85",sourceId:"target-source"}]);
    let handle = await engine.beginErasureMaintenance(1000);
    const accepted = await handle.queuePhysicalErasure({kind:"source_all_at_cut",sourceId:"target-source"}, "f23e2130b50242e3b8d545d4fa3d8015");
    assert.equal(accepted.outcome, "accepted");
    const control = handle.native._armErasurePauseForTest();
    let failure;
    void handle.completePhysicalErasure(accepted.ticket).catch(error => { failure = error; });
    try {
      await control.waitEntered();
      handle = undefined;
      global.gc();
    } finally { control.release(); }
    const deadline = Date.now() + 15000;
    while (true) {
      global.gc();
      if (failure) throw failure;
      try {
        const status = await engine.physicalErasureStatus(accepted.ticket);
        if (status.phase === "complete") { assert.equal(status.logicalState, "removed"); break; }
      } catch (error) { if (!(error instanceof MaintenanceError && error.reason === "maintenance_busy")) throw error; }
      assert.ok(Date.now() < deadline, "native operation lost after caller discard");
      await delay(10);
    }
    await engine.close();
    console.log("discarded-caller-complete");
  `;
  try {
    const result = spawnSync(process.execPath, ["--expose-gc", "--input-type=module", "-e", script], { encoding: "utf8", timeout: 25000 });
    assert.equal(result.status, 0, result.stdout + result.stderr);
    assert.ok(result.stdout.includes("discarded-caller-complete"));
  } finally { rmSync(dir, { recursive: true }); }
});

test("original sink blocker and authenticated reattachment preserve exact receipt", async () => {
  const { writeFileSync } = await import("node:fs");
  const dir = mkdtempSync(join(tmpdir(), "slice85-sink-"));
  const path = join(dir, "store.sqlite"), sink = join(dir, "original.jsonl");
  let engine = await Engine.open(path, { engineConfig: { physicalErasurePolicy: "explicit_maintenance" } });
  await engine.enableTelemetry(sink);
  await engine.write([{ kind: "doc", body: "sinktargetmarker85", sourceId: "target-source" }]);
  await engine.searchTextOnly("sinktargetmarker85");
  let handle = await engine.beginErasureMaintenance(1000);
  const accepted = await handle.queuePhysicalErasure({ kind: "source_all_at_cut", sourceId: "target-source" }, "f23e2130b50242e3b8d545d4fa3d8015");
  if (accepted.outcome !== "accepted") throw new Error("heavy admission refused");
  await handle.close(); await engine.close();
  engine = await Engine.open(path);
  try {
    handle = await engine.beginErasureMaintenance(1000);
    try {
      const blocked = await handle.completePhysicalErasure(accepted.ticket);
      assert.equal(blocked.outcome, "incomplete");
      if (blocked.outcome !== "incomplete") throw new Error("unattached original reported complete");
      assert.equal(blocked.status.blocker, "original_sink_unavailable");
      assert.equal(blocked.status.logicalState, "unchanged_by_this_request");
      const wrong = join(dir, "replacement.jsonl"); writeFileSync(wrong, "");
      await assert.rejects(handle.reattachOriginalErasureSink(accepted.ticket, wrong), { reason: "original_sink_mismatch" });
      await handle.reattachOriginalErasureSink(accepted.ticket, sink);
      assert.equal((await handle.completePhysicalErasure(accepted.ticket)).outcome, "complete");
    } finally { await handle.close(); }
    assert.equal((await engine.physicalErasureStatus(accepted.ticket))?.phase, "complete");
  } finally { await engine.close(); rmSync(dir, { recursive: true }); }
});
