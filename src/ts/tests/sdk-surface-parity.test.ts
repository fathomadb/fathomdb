import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import * as sdk from "../src/index.js";
import { Engine, admin, graph, read } from "../src/index.js";
import { freshDbPath } from "./helpers.js";

const ENGINE_NON_COMMAND = new Set([
  "attachSubscriber",
  "config",
  "counters",
  "denseDisabled",
  "denseDisabledReason",
  "drain",
  "enableTelemetry",
  "lastTelemetryQueryId",
  "openReport",
  "recordFeedback",
  "setProfiling",
  "setSlowThresholdMs",
  "vectorEquivalenceRefusalCount",
]);
const PACKAGE_NON_COMMAND = new Set([
  "embedBatchCls",
  "filterToSearchFilter",
  "isKnownEmbedderEvent",
  "mapEmbedderEvent",
  "mapNativeSearchResult",
  "mapOpenReport",
  "mapPerHitExplain",
  "rethrowTyped",
  "searchFilterToFilter",
  "validateDependencyTraceResponse",
  "validateGraphExpandResult",
]);
const ADMIN_NON_COMMAND = new Set(["configureRuntime"]);

function observedTypescriptSurface(engine: Engine): Set<string> {
  const observed = new Set<string>();
  for (const [name, value] of Object.entries(sdk)) {
    if (/^[A-Z]/.test(name) || PACKAGE_NON_COMMAND.has(name)) continue;
    if (typeof value === "function") observed.add(`package:${name}`);
  }
  for (const name of Object.getOwnPropertyNames(Engine)) {
    if (name.startsWith("_") || ["length", "name", "prototype"].includes(name)) continue;
    if (typeof (Engine as unknown as Record<string, unknown>)[name] === "function") {
      observed.add(`engine_static:${name}`);
    }
  }
  const instanceNames = new Set([
    ...Object.getOwnPropertyNames(Object.getPrototypeOf(engine)),
    ...Object.keys(engine as unknown as Record<string, unknown>),
  ]);
  for (const name of instanceNames) {
    if (name.startsWith("_") || name === "constructor" || ENGINE_NON_COMMAND.has(name)) continue;
    if (typeof (engine as unknown as Record<string, unknown>)[name] === "function") {
      observed.add(`engine_instance:${name}`);
    }
  }
  for (const [namespace, locator, exclusions] of [
    [admin, "admin", ADMIN_NON_COMMAND],
    [read, "read", new Set<string>()],
    [graph, "graph", new Set<string>()],
  ] as const) {
    for (const [name, value] of Object.entries(namespace)) {
      if (!exclusions.has(name) && typeof value === "function") observed.add(`${locator}:${name}`);
    }
  }
  return observed;
}

test("TypeScript surface equals the live canonical operation set", async () => {
  const here = dirname(fileURLToPath(import.meta.url));
  const root = join(here, "..", "..", "..", "..");
  const engine = await Engine.open(freshDbPath());
  try {
    const python = process.env.PYTHON ?? (process.platform === "win32" ? "python" : "python3");
    const output = execFileSync(
      python,
      [
        join(root, "scripts", "check-sdk-surface-parity.py"),
        "--signed",
        join(root, "src", "conformance", "governed-surface-allowlist.json"),
        "--companion",
        join(root, "src", "conformance", "governed-operation-parity.json"),
        "--binding",
        "typescript",
        "--observed-json",
        JSON.stringify([...observedTypescriptSurface(engine)].sort()),
      ],
      { encoding: "utf8" },
    );
    assert.match(output, /ok +sdk-surface-parity: typescript 44\/44 live canonical operations/);
  } finally {
    await engine.close();
  }
});
