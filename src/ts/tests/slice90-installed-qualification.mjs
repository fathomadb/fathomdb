// Copied into a fresh consumer directory by slice90-node-installed-qualification.sh.
// Package imports below must resolve only from that consumer's node_modules.
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { existsSync, mkdtempSync, realpathSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { DatabaseSync } from "node:sqlite";

import { Engine } from "fathomdb";

const require = createRequire(import.meta.url);
const consumer = dirname(fileURLToPath(import.meta.url));
const installedRoot = `${realpathSync(join(consumer, "node_modules"))}${sep}`;
for (const name of ["fathomdb", "fathomdb-linux-x64-gnu"]) {
  const resolved = realpathSync(require.resolve(name));
  assert.ok(resolved.startsWith(installedRoot), `${name} resolved outside the external consumer`);
}
const native = require("fathomdb-linux-x64-gnu");
const directory = mkdtempSync(join(tmpdir(), "fdb-node-installed-"));
let nextPath = 0;
const dbPath = () => join(directory, `db-${nextPath++}.fathom`);

const limits = [
  ["schedulerRuntimeThreads", 64, true],
  ["embedderPoolSize", 64, true],
  ["embedderCallTimeoutMs", 0xffff_ffff, true],
  ["provenanceRowCap", Number.MAX_SAFE_INTEGER, false],
  ["slowThresholdMs", Number.MAX_SAFE_INTEGER, false],
];

async function boundaryAndLifecycleCases() {
  const omittedPath = dbPath();
  const omitted = await Engine.open(omittedPath);
  assert.deepEqual(omitted.config, {});
  await omitted.close();
  const reopened = await Engine.open(omittedPath, {
    engineConfig: { schedulerRuntimeThreads: 1 },
  });
  assert.equal(reopened.config.schedulerRuntimeThreads, 1);
  await reopened.close();

  const minimum = await Engine.open(dbPath(), {
    engineConfig: {
      schedulerRuntimeThreads: 1,
      embedderPoolSize: 1,
      embedderCallTimeoutMs: 1,
      provenanceRowCap: 0,
      slowThresholdMs: 0,
    },
  });
  assert.equal(minimum.config.provenanceRowCap, 0);
  minimum.setSlowThresholdMs(100);
  assert.equal(minimum.config.slowThresholdMs, 0);
  await minimum.close();

  const maximum = await Engine.open(dbPath(), {
    engineConfig: {
      schedulerRuntimeThreads: 64,
      embedderPoolSize: 64,
      embedderCallTimeoutMs: 0xffff_ffff,
      provenanceRowCap: Number.MAX_SAFE_INTEGER,
      slowThresholdMs: Number.MAX_SAFE_INTEGER,
    },
  });
  assert.equal(maximum.config.slowThresholdMs, Number.MAX_SAFE_INTEGER);
  await maximum.close();

  for (const [field, max, positive] of limits) {
    const invalid = ["1", true, null, NaN, Infinity, 1.5, -1,
      Number.MAX_SAFE_INTEGER + 1, max + 1];
    if (positive) invalid.push(0);
    for (const value of invalid) {
      const path = dbPath();
      await assert.rejects(Engine.open(path, { engineConfig: { [field]: value } }),
        (error) => error instanceof TypeError || error instanceof RangeError);
      assert.equal(existsSync(path), false, `${field}=${String(value)} created a database`);
    }
  }

  const failedPath = dbPath();
  await assert.rejects(Engine.open(failedPath, {
    engineConfig: { schedulerRuntimeThreads: 65 },
  }), RangeError);
  assert.equal(existsSync(failedPath), false);
  const afterFailure = await Engine.open(failedPath);
  await afterFailure.close();
}

async function snapshotAndIndependenceCases() {
  const caller = { schedulerRuntimeThreads: 1, slowThresholdMs: 0 };
  let reads = 0;
  const options = new Proxy({ engineConfig: caller, useDefaultEmbedder: false }, {
    get(target, key, receiver) {
      if (key === "engineConfig") {
        return ++reads === 1 ? caller : { schedulerRuntimeThreads: 65 };
      }
      return Reflect.get(target, key, receiver);
    },
  });
  const first = await Engine.open(dbPath(), options);
  const second = await Engine.open(dbPath(), {
    engineConfig: { schedulerRuntimeThreads: 4, embedderPoolSize: 3 },
  });
  try {
    assert.equal(reads, 1);
    caller.schedulerRuntimeThreads = 8;
    assert.equal(first.config.schedulerRuntimeThreads, 1);
    assert.equal(second.config.schedulerRuntimeThreads, 4);
    assert.throws(() => Object.assign(first.config, { slowThresholdMs: 12 }), TypeError);
    first.setSlowThresholdMs(100);
    assert.equal(first.config.slowThresholdMs, 0);
  } finally {
    await Promise.all([first.close(), second.close()]);
  }
}

async function provenanceCases() {
  const counts = [];
  for (const cap of [1, 0]) {
    const path = dbPath();
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
      counts.push(db.prepare("SELECT COUNT(*) AS count FROM operational_mutations").get().count);
    } finally {
      db.close();
    }
  }
  assert.ok(counts[0] <= 2, `positive provenance cap did not prune: ${counts[0]}`);
  assert.ok(counts[1] >= 30, `zero provenance cap did not retain: ${counts[1]}`);
  return counts;
}

async function witnessCases() {
  const requested = {
    schedulerRuntimeThreads: 4,
    embedderPoolSize: 3,
    embedderCallTimeoutMs: 12_345,
    provenanceRowCap: Number.MAX_SAFE_INTEGER,
    slowThresholdMs: 456,
  };
  const raw = await native.Engine.open(dbPath(), { engineConfig: requested });
  try {
    assert.deepEqual(raw.requestedEngineConfigForTest(), requested);
    assert.match(await raw.bindingConnectionInventoryForTest(), /workers=4-autocommit/);
  } finally {
    await raw.close();
  }
  for (const count of [1, 64]) {
    const engine = await native.Engine.open(dbPath(), {
      engineConfig: { schedulerRuntimeThreads: count, embedderPoolSize: count },
    });
    try {
      const inventory = await engine.bindingConnectionInventoryForTest();
      assert.match(inventory, new RegExp(`workers=${count}-autocommit`));
      assert.match(inventory, new RegExp(`workers:${count},probes:0`));
      assert.equal(engine.requestedEngineConfigForTest().embedderPoolSize, count);
    } finally {
      await engine.close();
    }
  }
}

await boundaryAndLifecycleCases();
await snapshotAndIndependenceCases();
const provenanceCounts = await provenanceCases();
const mode = process.env.FATHOMDB_QUALIFICATION_MODE;
if (mode === "witness") {
  await witnessCases();
} else {
  assert.equal(mode, "production");
  assert.equal(native.Engine.prototype.requestedEngineConfigForTest, undefined);
}
console.log(JSON.stringify({
  result: "PASS", mode, node: process.version, installedRoot,
  provenanceCounts, nativeForwarding: mode === "witness" ? "all-five" : "production-surface",
}));
