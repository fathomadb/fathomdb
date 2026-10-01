import test from "node:test";
import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { DatabaseSync } from "node:sqlite";

import { Engine, type EngineConfig } from "../src/index.js";
import { native } from "../src/binding.js";
import { freshDbPath } from "./helpers.js";

const limits = [
  ["schedulerRuntimeThreads", 64, true],
  ["embedderPoolSize", 64, true],
  ["embedderCallTimeoutMs", 0xffff_ffff, true],
  ["provenanceRowCap", Number.MAX_SAFE_INTEGER, false],
  ["slowThresholdMs", Number.MAX_SAFE_INTEGER, false],
] as const;

test("configuration rejects invalid JavaScript values before creating a database", async () => {
  for (const [field, max, positive] of limits) {
    const invalid: unknown[] = ["2", true, false, null, NaN, Infinity, -Infinity, 1.5, -1,
      Number.MAX_SAFE_INTEGER + 1, max + 1];
    if (positive) invalid.push(0);
    for (const value of invalid) {
      const path = freshDbPath();
      await assert.rejects(
        Engine.open(path, { engineConfig: { [field]: value } as EngineConfig }),
        (error: unknown) => error instanceof TypeError || error instanceof RangeError,
        `${field}=${String(value)}`,
      );
      assert.equal(existsSync(path), false, `${field}=${String(value)} must not create a database`);
    }
  }
});

test("configuration forwards safe upper bounds and zero policy", async () => {
  const engine = await Engine.open(freshDbPath(), { engineConfig: {
    schedulerRuntimeThreads: 1,
    embedderPoolSize: 1,
    embedderCallTimeoutMs: 0xffff_ffff,
    provenanceRowCap: Number.MAX_SAFE_INTEGER,
    slowThresholdMs: Number.MAX_SAFE_INTEGER,
  } });
  try {
    assert.equal(engine.config.provenanceRowCap, Number.MAX_SAFE_INTEGER);
    assert.equal(engine.config.slowThresholdMs, Number.MAX_SAFE_INTEGER);
  } finally {
    await engine.close();
  }

  const zero = await Engine.open(freshDbPath(), { engineConfig: {
    provenanceRowCap: 0,
    slowThresholdMs: 0,
  } });
  try {
    assert.equal(zero.config.provenanceRowCap, 0);
    assert.equal(zero.config.slowThresholdMs, 0);
  } finally {
    await zero.close();
  }
});

test("configuration is per engine and the requested snapshot stays immutable", async () => {
  const caller = { schedulerRuntimeThreads: 1, slowThresholdMs: 0 };
  const first = await Engine.open(freshDbPath(), { engineConfig: caller });
  const second = await Engine.open(freshDbPath(), { engineConfig: { schedulerRuntimeThreads: 4 } });
  const omitted = await Engine.open(freshDbPath());
  try {
    caller.schedulerRuntimeThreads = 9;
    assert.equal(first.config.schedulerRuntimeThreads, 1);
    assert.equal(second.config.schedulerRuntimeThreads, 4);
    assert.deepEqual(omitted.config, {});
    first.setSlowThresholdMs(100);
    assert.equal(first.config.slowThresholdMs, 0);
    assert.throws(() => Object.assign(first.config, { schedulerRuntimeThreads: 8 }), TypeError);
  } finally {
    await Promise.all([first.close(), second.close(), omitted.close()]);
  }
});

test("Engine.open reads a changing engineConfig accessor once and forwards that snapshot", async () => {
  for (const later of [undefined, { schedulerRuntimeThreads: 65 }]) {
    let reads = 0;
    const requested = { schedulerRuntimeThreads: 1 };
    const options = new Proxy({ engineConfig: requested, useDefaultEmbedder: false }, {
      get(target, key, receiver) {
        if (key === "engineConfig") {
          reads++;
          return reads === 1 ? requested : later;
        }
        return Reflect.get(target, key, receiver);
      },
    });
    const engine = await Engine.open(freshDbPath(), options);
    try {
      assert.equal(reads, 1, "a changing accessor must not replace the validated request");
      assert.equal(engine.config.schedulerRuntimeThreads, 1);
    } finally {
      await engine.close();
    }
  }
});

test("native configured open changes the owned projection worker inventory", async () => {
  for (const count of [1, 4, 64]) {
    const engine = await native.Engine.open(freshDbPath(), {
      engineConfig: { schedulerRuntimeThreads: count, embedderPoolSize: count },
    });
    try {
      const inventory = await engine.bindingConnectionInventoryForTest?.();
      assert.match(inventory ?? "", new RegExp(`workers=${count}-autocommit`));
      assert.match(inventory ?? "", new RegExp(`workers:${count},probes:0`));
    } finally {
      await engine.close();
    }
  }
});

test("native number boundary accepts safe integer caps and rejects invalid values", async () => {
  const accepted = await native.Engine.open(freshDbPath(), {
    engineConfig: {
      provenanceRowCap: Number.MAX_SAFE_INTEGER,
      slowThresholdMs: Number.MAX_SAFE_INTEGER,
    },
  });
  await accepted.close();
  for (const value of [Number.MAX_SAFE_INTEGER + 1, -1, 1.5, Infinity, true, "1"]) {
    const path = freshDbPath();
    await assert.rejects(Promise.resolve().then(() => native.Engine.open(path, {
      engineConfig: { provenanceRowCap: value as number },
    })));
    assert.equal(existsSync(path), false);
  }
});

test("provenance zero disables pruning while a positive cap consumes rows", async () => {
  const counts: number[] = [];
  for (const cap of [1, 0]) {
    const path = freshDbPath();
    const engine = await Engine.open(path, { engineConfig: { provenanceRowCap: cap } });
    try {
      await engine.write([{ adminSchema: { name: "config-retention", kind: "append_only_log",
        schemaJson: '{"type":"object"}', retentionJson: "{}" } }]);
      for (let index = 0; index < 30; index++) {
        await engine.write([{ opStore: { collection: "config-retention",
          recordKey: `record-${index}`, body: `{"value":${index}}` } }]);
      }
    } finally {
      await engine.close();
    }
    const db = new DatabaseSync(path, { readOnly: true });
    try {
      counts.push((db.prepare("SELECT COUNT(*) AS count FROM operational_mutations").get() as
        { count: number }).count);
    } finally {
      db.close();
    }
  }
  assert.ok(counts[0] <= 2, `positive cap should prune: ${counts[0]}`);
  assert.ok(counts[1] >= 30, `zero cap should retain: ${counts[1]}`);
});
