// 0.8.28 Slice 31 § 15: the CUDA pool settings are open settings. They are
// validated before the native open, read back through `engine.config`, and
// fixed once per process, so the conflict cases run in a child process.
import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { Engine, type EngineConfig } from "../src/index.js";
import { freshDbPath } from "./helpers.js";

const MIB = 1 << 20;
const moduleUrl = new URL("../src/index.js", import.meta.url).href;

function runChild(body: string): string {
  const directory = mkdtempSync(join(tmpdir(), "fathomdb-slice31-"));
  const env = Object.fromEntries(
    Object.entries(process.env).filter(([name]) => !name.startsWith("FATHOMDB_POOL_")),
  );
  return execFileSync(
    process.execPath,
    [
      "--input-type=module",
      "-e",
      `const { Engine, InvalidArgumentError } = await import(${JSON.stringify(moduleUrl)}); const dir = ${JSON.stringify(directory)}; ${body}`,
    ],
    { stdio: "pipe", timeout: 60_000, env, encoding: "utf8" },
  );
}

test("an unknown pool setting is rejected before creating a database", async () => {
  const cases: Array<[string, unknown, typeof TypeError | typeof RangeError]> = [
    ["cudaPool", "AUTO", TypeError],
    ["cudaPool", "enabled", TypeError],
    ["cudaPool", 1, TypeError],
    ["cudaPoolMaxSize", "64M", TypeError],
    ["cudaPoolMaxSize", 32 * MIB - 1, RangeError],
    ["cudaPoolMaxSize", 64 * MIB + 0.5, RangeError],
    ["cudaPoolMaxSize", Number.MAX_SAFE_INTEGER + 1, RangeError],
    ["cudaPoolReleaseThreshold", 0, TypeError],
    ["cudaPoolReleaseThreshold", "MAX", TypeError],
  ];
  for (const [field, value, errorType] of cases) {
    const path = freshDbPath();
    await assert.rejects(
      Engine.open(path, { engineConfig: { [field]: value } as EngineConfig }),
      (error: unknown) => error instanceof errorType,
      `${field}=${String(value)}`,
    );
    assert.equal(existsSync(path), false, `${field}=${String(value)} must not create a database`);
  }
});

test("a conflicting second open raises InvalidArgumentError and the settings read back", () => {
  const out = runChild(`
    const first = await Engine.open(dir + "/first.sqlite", { engineConfig: {
      cudaPool: "on", cudaPoolMaxSize: ${64 * MIB}, cudaPoolReleaseThreshold: "max" } });
    if (first.config.cudaPool !== "on" || first.config.cudaPoolMaxSize !== ${64 * MIB}
        || first.config.cudaPoolReleaseThreshold !== "max") throw new Error("read-back");
    try {
      await Engine.open(dir + "/second.sqlite", { engineConfig: { cudaPool: "off" } });
      throw new Error("missing conflict");
    } catch (error) {
      if (!(error instanceof InvalidArgumentError)) throw error;
      console.log("conflict: " + error.message);
    }
    const same = await Engine.open(dir + "/same.sqlite", { engineConfig: { cudaPool: "on" } });
    const omitted = await Engine.open(dir + "/omitted.sqlite");
    await Promise.all([first.close(), same.close(), omitted.close()]);
    console.log("done");
  `);
  assert.match(
    out,
    /cuda_pool=off conflicts with cuda_pool=on already in force in this process \(from sdk\); open with the same value or omit it/,
  );
  assert.match(out, /done/);
});

test("a different pool size or release threshold conflicts naming the setting", () => {
  const out = runChild(`
    const first = await Engine.open(dir + "/first.sqlite", { engineConfig: { cudaPoolMaxSize: ${64 * MIB} } });
    for (const engineConfig of [{ cudaPoolMaxSize: ${128 * MIB} }, { cudaPoolReleaseThreshold: "max" }]) {
      try {
        await Engine.open(dir + "/other.sqlite", { engineConfig });
        throw new Error("missing conflict for " + JSON.stringify(engineConfig));
      } catch (error) {
        if (!(error instanceof InvalidArgumentError)) throw error;
        console.log("conflict: " + error.message);
      }
    }
    const same = await Engine.open(dir + "/same.sqlite", { engineConfig: { cudaPoolMaxSize: ${64 * MIB} } });
    await Promise.all([first.close(), same.close()]);
    console.log("done");
  `);
  assert.match(
    out,
    /cuda_pool_max_size=134217728 conflicts with cuda_pool_max_size=67108864 already in force in this process \(from sdk\); open with the same value or omit it/,
  );
  assert.match(
    out,
    /cuda_pool_release_threshold=max conflicts with cuda_pool_release_threshold=0 already in force in this process \(from default\); open with the same value or omit it/,
  );
  assert.match(out, /done/);
});
