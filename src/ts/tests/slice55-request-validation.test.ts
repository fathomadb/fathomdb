import assert from "node:assert/strict";
import test from "node:test";

import {
  DependencyTraceError,
  Engine,
  type DependencyTraceRequestV1,
} from "../src/index.js";
import { freshDbPath } from "./helpers.js";

function malformedRequest(overrides: Record<string, unknown>): DependencyTraceRequestV1 {
  return {
    schemaVersion: 1,
    rootRevisionId: "source-r1",
    direction: "to_dependents",
    context: {
      schemaVersion: 1,
      effectiveValidAt: 1,
      context: { schemaVersion: 1, view: {}, eligibility: {} },
      token: "opaque",
    },
    ...overrides,
  } as DependencyTraceRequestV1;
}

test("slice55 request validation rejects schema before semantic fields", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    await assert.rejects(
      engine.traceDependency(
        malformedRequest({
          schemaVersion: 2,
          rootRevisionId: "!",
          direction: "invalid",
        }),
      ),
      (error: unknown) =>
        error instanceof DependencyTraceError &&
        error.reason === "unsupported_schema_version" &&
        error.fieldPath === "/schemaVersion",
    );
  } finally {
    await engine.close();
  }
});

test("slice55 request validation rejects canonical integer bounds before native", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    await assert.rejects(
      engine.traceDependency(malformedRequest({ maxRelations: 1.5 })),
      (error: unknown) =>
        error instanceof DependencyTraceError &&
        error.reason === "trace_limit_invalid" &&
        error.fieldPath === "/maxRelations",
    );
  } finally {
    await engine.close();
  }
});
