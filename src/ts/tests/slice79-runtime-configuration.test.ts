import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { RuntimeConfigurationError, admin } from "../src/index.js";

void admin.configureRuntime;
void RuntimeConfigurationError;

const moduleUrl = new URL("../src/index.js", import.meta.url).href;

function run(body: string): void {
  const directory = mkdtempSync(join(tmpdir(), "fathomdb-slice79-"));
  execFileSync(
    process.execPath,
    [
      "--input-type=module",
      "-e",
      `const { Engine, admin, RuntimeConfigurationError } = await import(${JSON.stringify(moduleUrl)}); const db = ${JSON.stringify(join(directory, "runtime.fathom"))}; ${body}`,
    ],
    { stdio: "pipe", timeout: 60_000 },
  );
}

test("performance configuration precedes a real Engine open", () => {
  run(`
    assertMode(admin.configureRuntime({ sqliteMode: "performance" }), "performance");
    const engine = await Engine.open(db);
    await engine.close();
    function assertMode(value, expected) { if (value.sqliteMode !== expected) throw new Error("wrong mode"); }
  `);
});

test("diagnostics repeat and conflict carry typed data", () => {
  run(`
    admin.configureRuntime({ sqliteMode: "diagnostics" });
    admin.configureRuntime({ sqliteMode: "diagnostics" });
    try { admin.configureRuntime({ sqliteMode: "performance" }); throw new Error("missing conflict"); }
    catch (error) {
      if (!(error instanceof RuntimeConfigurationError)) throw error;
      if (error.reason !== "conflict" || error.requestedMode !== "performance" || error.effectiveMode !== "diagnostics") throw error;
    }
  `);
});

test("invalid mode is rejected before native mutation", () => {
  run(`
    try { admin.configureRuntime({ sqliteMode: "fast" }); throw new Error("missing range error"); }
    catch (error) { if (!(error instanceof RangeError)) throw error; }
    admin.configureRuntime({ sqliteMode: "performance" });
  `);
});
