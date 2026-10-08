import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { existsSync, rmSync } from "node:fs";
import { dirname } from "node:path";
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

test("slice55 installed trace returns a committed dependency edge", async () => {
  const databasePath = freshDbPath();
  const engine = await Engine.open(databasePath, { useDefaultEmbedder: false });
  const body = "trace source body";
  try {
    await engine.write([
      {
        kind: "doc", body, sourceId: "trace-source", logicalId: "source",
        provenance: {
          schemaVersion: 1, role: "canonical", artifactRevisionId: "source-r1",
          sourceVersionId: "source-v1",
        },
      },
      {
        kind: "fact", body: "derived body", sourceId: "trace-source", logicalId: "derived",
        provenance: {
          schemaVersion: 1, role: "derived", artifactRevisionId: "derived-r1",
          sourceVersionId: "source-v1", sourceRevisionId: "source-r1",
          sourceLocator: { kind: "whole_body" },
          canonicalSourceHash: {
            algorithm: "sha256", digestHex: createHash("sha256").update(body).digest("hex"),
          },
        },
      },
    ]);
    const registered = await engine.registerSourceDependency({
      schemaVersion: 1, dependencyId: "trace-dependency",
      sourceRevisionId: "source-r1", derivedRevisionId: "derived-r1",
    });
    const context = await engine.freezeReadContext({ schemaVersion: 1, view: {}, eligibility: {} });
    const result = await engine.traceDependency({
      schemaVersion: 1, rootRevisionId: "source-r1", direction: "to_dependents", context,
    });
    assert.equal(result.complete, true);
    assert.deepEqual(result.dependencyEdges.map((edge) => [
      edge.dependencyId, edge.sourceRevisionId, edge.derivedRevisionId,
    ]), [[registered.dependencyId, "source-r1", "derived-r1"]]);
  } finally {
    await engine.close();
  }
  const reopened = await Engine.open(databasePath, { useDefaultEmbedder: false });
  try {
    const context = await reopened.freezeReadContext({ schemaVersion: 1, view: {}, eligibility: {} });
    const result = await reopened.traceDependency({
      schemaVersion: 1, rootRevisionId: "source-r1", direction: "to_dependents", context,
    });
    assert.equal(result.dependencyEdges[0]?.dependencyId, "trace-dependency");
  } finally {
    await reopened.close();
  }
});

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
  const databasePath = freshDbPath();
  const databaseDirectory = dirname(databasePath);
  const engine = await Engine.open(databasePath, { useDefaultEmbedder: false });
  try {
    const context = await engine.freezeReadContext({
      schemaVersion: 1,
      view: {},
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
    rmSync(databaseDirectory, { recursive: true, force: true });
  }
  assert.equal(existsSync(databaseDirectory), false, "the test must remove its temporary database");
});

const malformedFrozenCases: Array<[
  string,
  (context: Record<string, any>) => void,
  string,
  string,
]> = [
  ["token", (context) => { context.token = 7; }, "token_malformed", "/context/token"],
  [
    "effective instant",
    (context) => { context.effectiveValidAt = true; },
    "context_invalid",
    "/context/effectiveValidAt",
  ],
  ["context container", (context) => { context.context = []; }, "context_invalid", "/context/context"],
  [
    "view container",
    (context) => { context.context.view = []; },
    "context_invalid",
    "/context/context/view",
  ],
  [
    "eligibility container",
    (context) => { context.context.eligibility = []; },
    "context_invalid",
    "/context/context/eligibility",
  ],
  [
    "view boolean",
    (context) => { context.context.view.includeSuperseded = 1; },
    "context_invalid",
    "/context/context/view/includeSuperseded",
  ],
  [
    "attribute pair",
    (context) => { context.context.eligibility.attributes = [["missing-value"]]; },
    "context_invalid",
    "/context/context/eligibility/attributes/0",
  ],
];

for (const [name, mutate, reason, fieldPath] of malformedFrozenCases) {
  test(`slice55 validates complete frozen shape: ${name}`, async () => {
    const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
    try {
      const context = await engine.freezeReadContext({ schemaVersion: 1, view: {}, eligibility: {} });
      mutate(context as unknown as Record<string, any>);
      await assert.rejects(
        engine.traceDependency(malformedRequest({ context })),
        (error: unknown) =>
          error instanceof FrozenReadError &&
          error.reason === reason &&
          error.fieldPath === fieldPath,
      );
    } finally {
      await engine.close();
    }
  });
}

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

test("slice55 TypeScript rejects edge generation outside the read boundary", () => {
  assert.doesNotThrow(() => validateDependencyTraceResponse(response()));
  for (const generation of ["0", "2"]) {
    const value = response();
    const edges = value.dependencyEdges as Array<Record<string, unknown>>;
    edges[0]!.registeredDependencyGeneration = generation;
    assert.throws(
      () => validateDependencyTraceResponse(value),
      (error: unknown) =>
        error instanceof DependencyTraceError &&
        error.reason === "trace_corrupt" &&
        error.fieldPath === "/dependencyEdges/0/registeredDependencyGeneration",
    );
  }
});
