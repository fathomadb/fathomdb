import assert from "node:assert/strict";
import test from "node:test";

import {
  DependencyTraceError,
  Engine,
  FrozenReadError,
  type DependencyTraceRequestV1,
  validateDependencyTraceResponse,
  mapPerHitExplain,
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

test("slice55 unknown trace request fields use escaped RFC 6901 paths", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    await assert.rejects(
      engine.traceDependency(malformedRequest({ "a/b~c": true })),
      (error: unknown) =>
        error instanceof DependencyTraceError &&
        error.reason === "unknown_field" &&
        error.fieldPath === "/a~1b~0c",
    );
  } finally {
    await engine.close();
  }
});

test("slice55 null trace context is typed at the declared request path", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    await assert.rejects(
      engine.traceDependency(malformedRequest({ context: null })),
      (error: unknown) =>
        error instanceof DependencyTraceError &&
        error.reason === "trace_corrupt" &&
        error.fieldPath === "/context",
    );
  } finally {
    await engine.close();
  }
});

test("slice55 malformed nested frozen context preserves FrozenReadError", async () => {
  const engine = await Engine.open("slice55-malformed-frozen-context", { useDefaultEmbedder: false });
  try {
    const context = await engine.freezeReadContext({
      schemaVersion: 1,
      view: { schemaVersion: 1 } as never,
      eligibility: {},
    });
    (context.context as { schemaVersion: number }).schemaVersion = 2;
    await assert.rejects(
      engine.traceDependency({
        schemaVersion: 1,
        rootRevisionId: "source-r1",
        direction: "to_dependents",
        context,
      }),
      (error: unknown) =>
        error instanceof FrozenReadError &&
        error.reason === "unsupported_schema_version" &&
        error.fieldPath === "/context/context/schemaVersion",
    );
  } finally {
    await engine.close();
  }
});

test("slice55 TypeScript rejects unknown arms and nonfinite explanation scores", () => {
  const native = {
    id: 1,
    arm: "unknown",
    vectorRank: null,
    textRank: 0,
    graphRank: null,
    fusedScore: 1,
    ceScore: null,
    blended: 1,
    importance: null,
    confidence: null,
  } as Parameters<typeof mapPerHitExplain>[0];
  assert.throws(
    () => mapPerHitExplain(native),
    (error: unknown) => error instanceof Error && error.message.endsWith("/arm"),
  );
  native.arm = "text";
  native.fusedScore = Number.NaN;
  assert.throws(
    () => mapPerHitExplain(native),
    (error: unknown) => error instanceof Error && error.message.endsWith("/fusedScore"),
  );
});

function response(): Record<string, unknown> {
  return {
    schemaVersion: 1,
    rootRevisionId: "source-r1",
    direction: "to_dependents",
    nodes: [
      {
        schemaVersion: 1,
        artifactRevisionId: "source-r1",
        artifactClass: "node",
        role: "canonical_source",
        depth: 0,
        lifecycle: {
          schemaVersion: 1,
          artifactClass: "node",
          state: "active",
          superseded: false,
          validAtEffective: true,
        },
      },
      {
        schemaVersion: 1,
        artifactRevisionId: "derived-r1",
        artifactClass: "node",
        role: "derived",
        depth: 1,
        lifecycle: {
          schemaVersion: 1,
          artifactClass: "node",
          state: "active",
          superseded: false,
          validAtEffective: true,
        },
      },
    ],
    dependencyEdges: [
      {
        schemaVersion: 1,
        dependencyId: "dep-1",
        sourceRevisionId: "source-r1",
        derivedRevisionId: "derived-r1",
        registeredDependencyGeneration: "1",
      },
    ],
    checkedWorkUnits: 2,
    complete: true,
    readBoundary: {
      schemaVersion: 1,
      effectiveAtEpochS: 1,
      observedWriteBoundary: "1",
      dependencyGeneration: "1",
      projectionGenerationId: "pgen1:00000000000000000000000000000000",
    },
  };
}

test("slice55 TypeScript recursively validates dependency trace responses", () => {
  const value = response();
  const nodes = value.nodes as Array<Record<string, unknown>>;
  (nodes[1]!.lifecycle as Record<string, unknown>).schemaVersion = 2;
  assert.throws(
    () => validateDependencyTraceResponse(value),
    (error: unknown) =>
      error instanceof DependencyTraceError &&
      error.reason === "unsupported_schema_version" &&
      error.fieldPath === "/nodes/1/lifecycle/schemaVersion",
  );
});

test("slice55 TypeScript rejects noncanonical trace integers at exact paths", () => {
  const value = response();
  const edges = value.dependencyEdges as Array<Record<string, unknown>>;
  edges[0]!.registeredDependencyGeneration = "01";
  assert.throws(
    () => validateDependencyTraceResponse(value),
    (error: unknown) =>
      error instanceof DependencyTraceError &&
      error.reason === "trace_corrupt" &&
      error.fieldPath === "/dependencyEdges/0/registeredDependencyGeneration",
  );
});
