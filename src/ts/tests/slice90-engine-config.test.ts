import test from "node:test";
import assert from "node:assert/strict";
import { existsSync } from "node:fs";

import { Engine, type EngineConfig } from "../src/index.js";
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
