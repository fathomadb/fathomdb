import test from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

const require = createRequire(import.meta.url);
const native = require(process.env.FATHOMDB_NATIVE_BINARY ?? "../fathomdb.linux-x64-gnu.node");
const dbPath = () => join(mkdtempSync(join(tmpdir(), "fathomdb-slice110-")), "db.sqlite");
const write = engine => engine.write([{ kind: "memo", body: "slice110", sourceId: "test:slice110" }]);
const tick = () => new Promise(resolve => setTimeout(resolve, 50));

test("native subscriber delivers owned engine events on the JavaScript thread", async () => {
  const engine = await native.Engine.open(dbPath());
  try {
    const received = [];
    engine.attachSubscriber(event => received.push(event));
    await write(engine);
    await tick();
    assert.ok(received.some(event => event.kind === "event"),
      "an engine operation must reach the listener");
    for (const event of received) {
      assert.equal(typeof event.droppedRecordsTotal, "string");
    }
    assert.ok(received.some(event => event.phase === "started" && event.category === "writer"));
    assert.ok(received.some(event => event.phase === "finished" && event.category === "writer"));
  } finally {
    await engine.close();
  }
});

test("native profile and slow records use tagged canonical decimal fields", async () => {
  const engine = await native.Engine.open(dbPath());
  try {
    const received = [];
    engine.attachSubscriber(event => received.push(event));
    engine.setProfiling(true);
    engine.setSlowThresholdMs(0);
    await engine.write([{ kind: "memo", body: "x".repeat(2_000_000), sourceId: "test:slice110" }]);
    await tick();
    const profile = received.find(event => event.kind === "profile");
    const slow = received.find(event => event.kind === "slowStatement");
    assert.ok(profile, "SQLite profile must arrive");
    assert.match(profile.wallClockMs, /^\d+$/);
    assert.match(profile.stepCount, /^\d+$/);
    assert.match(profile.cacheDelta, /^-?\d+$/);
    assert.ok(slow, "slow statement must arrive");
    assert.match(slow.wallClockMs, /^\d+$/);
    assert.equal(typeof slow.statement, "string");
  } finally {
    await engine.close();
  }
});

test("listener exception is contained, replacement silences old listener, and close detaches at entry", async () => {
  const engine = await native.Engine.open(dbPath());
  let first = 0;
  let second = 0;
  try {
    engine.attachSubscriber(() => { first++; throw new Error("listener fault"); });
    await write(engine);
    await tick();
    assert.ok(first > 0, "listener must have run despite its exception");
    const beforeReplace = first;
    engine.attachSubscriber(() => { second++; });
    await write(engine);
    await tick();
    assert.equal(first, beforeReplace, "prior generation must be silent");
    assert.ok(second > 0, "replacement listener must receive diagnostics");
    const close = engine.close();
    assert.equal(typeof close.then, "function", "native close retains Promise<void> shape");
    assert.throws(() => engine.attachSubscriber(() => {}), /FDB_CLOSING/);
    await close;
  } finally {
    await engine.close();
  }
});

test("a same-callback reentry can schedule engine work", async () => {
  const engine = await native.Engine.open(dbPath());
  let reentered;
  try {
    engine.attachSubscriber(event => {
      if (event.kind === "event" && event.phase === "finished" && !reentered) {
        reentered = native.readGet(engine, "missing");
      }
    });
    await write(engine);
    await tick();
    assert.ok(reentered);
    assert.equal(await reentered, null);
  } finally {
    await engine.close();
  }
});

test("callback-triggered replacement silences the current generation", async () => {
  const engine = await native.Engine.open(dbPath());
  let oldCalls = 0;
  let newCalls = 0;
  try {
    engine.attachSubscriber(event => {
      oldCalls++;
      if (event.kind === "event" && event.phase === "started") {
        engine.attachSubscriber(() => { newCalls++; });
      }
    });
    await write(engine);
    await tick();
    assert.equal(oldCalls, 1, "queued old-generation records must be silenced");
    const afterFirst = newCalls;
    await write(engine);
    await tick();
    assert.equal(oldCalls, 1);
    assert.ok(newCalls > afterFirst);
  } finally {
    await engine.close();
  }
});

test("callback-triggered close detaches before scheduling engine shutdown", async () => {
  const engine = await native.Engine.open(dbPath());
  let closePromise;
  let calls = 0;
  engine.attachSubscriber(event => {
    calls++;
    if (event.kind === "event" && event.phase === "finished" && !closePromise) {
      closePromise = engine.close();
      assert.equal(typeof closePromise.then, "function");
      assert.throws(() => engine.attachSubscriber(() => {}), /FDB_CLOSING/);
    }
  });
  try {
    await write(engine);
    await tick();
    assert.ok(closePromise, "callback must have entered close");
    await closePromise;
    const afterClose = calls;
    await tick();
    assert.equal(calls, afterClose, "no callback may start after close entry");
  } finally {
    await engine.close();
  }
});

test("live subscriber and pending wakeup do not keep Node alive", () => {
  const binary = process.env.FATHOMDB_NATIVE_BINARY;
  assert.ok(binary);
  for (const pendingWrite of [false, true]) {
    const code = `const n=require(${JSON.stringify(binary)}); n.Engine.open(${JSON.stringify(dbPath())})` +
      `.then(e=>{e.attachSubscriber(()=>{}); ${pendingWrite ?
        `return e.write([{kind:"memo",body:"exit",sourceId:"test:slice110"}]);` : ""}})`;
    const child = spawnSync(process.execPath, ["-e", code], { timeout: 5000, encoding: "utf8" });
    assert.equal(child.status, 0, child.stderr || String(child.error));
  }
});
