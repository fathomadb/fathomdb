import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import {
  Engine,
  FrozenReadError,
  GraphExpansionError,
  graph,
  validateGraphExpandResult,
  type GraphExpandRequestV1,
  type GraphExpandResultV1,
  type TraversalDirection,
} from "../src/index.js";
import { freshDbPath } from "./helpers.js";

const SOURCE_ID = "test:slice60-typescript";

function node(logicalId: string, kind = "fact", body = logicalId): object {
  return { logicalId, kind, body, sourceId: SOURCE_ID };
}

function edge(logicalId: string, from: string, to: string, kind = "link"): object {
  return { edge: { logicalId, kind, from, to, sourceId: SOURCE_ID } };
}

function request(
  ids: string[],
  overrides: Partial<GraphExpandRequestV1> = {},
): GraphExpandRequestV1 {
  return {
    schemaVersion: 1,
    seed: {
      schemaVersion: 1,
      type: "explicit",
      logicalIds: ids.map((value) => ({ space: "logical", value })),
    },
    direction: "outgoing",
    edgeKinds: [],
    targetKinds: [],
    context: {
      schemaVersion: 1,
      type: "current",
      context: {
        schemaVersion: 1,
        view: { validAsOf: 1_700_000_000 },
        eligibility: {},
      },
    },
    maxDepth: 3,
    resultLimit: 50,
    maxWorkUnits: "10000",
    includeExplanation: false,
    ...overrides,
  };
}

function response(): Record<string, any> {
  const fixture = JSON.parse(
    readFileSync(
      new URL(
        "../../../../dev/fixtures/slice60-graph-expand-conformance-v1.json",
        import.meta.url,
      ),
      "utf8",
    ),
  ) as { response: Record<string, any> };
  return structuredClone(fixture.response);
}

test("graph.expand is a governed consumer surface", () => {
  const allowlist = JSON.parse(
    readFileSync(new URL("../../../conformance/governed-surface-allowlist.json", import.meta.url), "utf8"),
  ) as { allowlist: string[] };
  assert.equal(typeof graph.expand, "function");
  assert.ok(allowlist.allowlist.includes("graph.expand"));
});

test("graph.expand depth zero preserves seed order and canonical u64 strings", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    await engine.write([node("b", "seed"), node("a", "seed"), edge("ab", "a", "b")]);
    const result = await graph.expand(
      engine,
      request(["b", "a"], { maxDepth: 0, maxWorkUnits: "1" }),
    );
    assert.deepEqual(
      result.seeds.map((seed) => [seed.logicalId, seed.seedOrdinal]),
      [
        ["b", 0],
        ["a", 1],
      ],
    );
    assert.deepEqual(result.targets, []);
    assert.equal(result.workUnits, "0");
    assert.equal(result.complete, true);
    assert.equal(result.explanation, null);
  } finally {
    await engine.close();
  }
});

test("graph.expand applies direction, edge kind, and return-only target kind", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    await engine.write([
      node("a", "x"),
      node("b", "y"),
      node("c", "x"),
      node("d", "x"),
      edge("ab", "a", "b", "supports"),
      edge("bc", "b", "c", "supports"),
      edge("da", "d", "a", "refutes"),
    ]);
    const outgoing = await graph.expand(
      engine,
      request(["a"], {
        maxDepth: 2,
        edgeKinds: ["supports"],
        targetKinds: ["x"],
      }),
    );
    assert.deepEqual(outgoing.targets.map((target) => target.logicalId), ["c"]);

    const incoming = await graph.expand(
      engine,
      request(["a"], { maxDepth: 1, direction: "incoming" }),
    );
    assert.deepEqual(incoming.targets.map((target) => target.logicalId), ["d"]);
    assert.equal(incoming.targets[0]?.origin.terminalDirection, "incoming");
  } finally {
    await engine.close();
  }
});

for (const direction of ["incoming", "outgoing", "both"] as const) {
  test(`graph.expand ${direction}: W succeeds and W+1 has no partial result`, async () => {
    async function run(count: number): Promise<GraphExpandResultV1> {
      const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
      const writes: object[] = [node("root", "seed")];
      for (let index = 0; index < count; index += 1) {
        const child = `n-${index}`;
        writes.push(node(child));
        writes.push(
          direction === "incoming"
            ? edge(`e-${index}`, child, "root")
            : edge(`e-${index}`, "root", child),
        );
      }
      await engine.write(writes);
      try {
        return await graph.expand(
          engine,
          request(["root"], { direction, maxDepth: 1, maxWorkUnits: "3" }),
        );
      } finally {
        await engine.close();
      }
    }

    assert.equal((await run(3)).workUnits, "3");
    await assert.rejects(
      run(4),
      (error: unknown) =>
        error instanceof GraphExpansionError &&
        error.code === "FDB_GRAPH_EXPANSION" &&
        error.reason === "graph_expansion_bound_exceeded" &&
        error.fieldPath === "/maxWorkUnits",
    );
  });
}

test("graph.expand recursively rejects unknown request fields before native execution", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    await assert.rejects(
      graph.expand(
        engine,
        request(["root"], { "a/b~c": true } as Partial<GraphExpandRequestV1>),
      ),
      (error: unknown) =>
        error instanceof GraphExpansionError &&
        error.reason === "unknown_field" &&
        error.fieldPath === "/a~1b~0c",
    );
    const nested = request(["root"]);
    Object.assign(nested.seed, { schemaVersion: 2, zzz: true });
    await assert.rejects(
      graph.expand(engine, nested),
      (error: unknown) =>
        error instanceof GraphExpansionError &&
        error.reason === "unsupported_schema_version" &&
        error.fieldPath === "/seed/schemaVersion",
    );
  } finally {
    await engine.close();
  }
});

test("graph.expand validates canonical u64, real booleans, and closed unions", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    for (const [overrides, reason, fieldPath] of [
      [{ maxWorkUnits: "01" }, "graph_work_limit_invalid", "/maxWorkUnits"],
      [{ maxWorkUnits: 1 }, "graph_work_limit_invalid", "/maxWorkUnits"],
      [{ maxDepth: true }, "graph_depth_invalid", "/maxDepth"],
      [{ includeExplanation: 1 }, "graph_context_invalid", "/includeExplanation"],
      [{ direction: "sideways" }, "graph_direction_invalid", "/direction"],
    ] as const) {
      await assert.rejects(
        graph.expand(engine, request(["root"], overrides as Partial<GraphExpandRequestV1>)),
        (error: unknown) =>
          error instanceof GraphExpansionError &&
          error.reason === reason &&
          error.fieldPath === fieldPath,
      );
    }
  } finally {
    await engine.close();
  }
});

test("malformed native graph responses map to one exact typed family", () => {
  const mutations: Array<[(value: Record<string, any>) => void, string]> = [
    [(value) => { value.targets[0].origin.seedOrdinal = 1; }, "/targets/0/origin/seedOrdinal"],
    [(value) => { value.targets[0].origin.seedLogicalId = "other"; }, "/targets/0/origin/seedLogicalId"],
    [(value) => { value.targets[0].origin.targetLogicalId = "other"; }, "/targets/0/origin/targetLogicalId"],
    [(value) => { value.explanation.perTarget = []; }, "/explanation/perTarget"],
    [(value) => { value.explanation.perTarget[0].targetIndex = 1; }, "/explanation/perTarget/0/targetIndex"],
    [(value) => { value.explanation.perTarget[0].origin.predecessorLogicalId = "other"; }, "/explanation/perTarget/0/origin"],
    [(value) => { value.explanation.degradationCodes = ["projection_degraded"]; }, "/explanation/degradationCodes"],
  ];
  for (const [mutate, fieldPath] of mutations) {
    const value = response();
    mutate(value);
    assert.throws(
      () => validateGraphExpandResult(value),
      (error: unknown) =>
        error instanceof GraphExpansionError &&
        error.code === "FDB_GRAPH_EXPANSION" &&
        error.reason === "graph_corrupt" &&
        error.fieldPath === fieldPath &&
        error.message === `graph_corrupt at ${fieldPath}`,
    );
  }
});

test("native graph response is open but closed variants and noncanonical values fail", () => {
  const additive = response();
  additive.futureField = { ignored: true };
  additive.targets[0].futureTarget = true;
  assert.equal(validateGraphExpandResult(additive).targets[0]?.logicalId, "target-c");

  for (const [mutate, fieldPath] of [
    [(value: Record<string, any>) => { value.workUnits = "02"; }, "/workUnits"],
    [(value: Record<string, any>) => { value.workUnits = 2; }, "/workUnits"],
    [(value: Record<string, any>) => { value.complete = false; }, "/complete"],
    [(value: Record<string, any>) => { value.explanation.projectionOrigin = "unknown"; }, "/explanation/projectionOrigin"],
  ] as const) {
    const value = response();
    mutate(value);
    assert.throws(
      () => validateGraphExpandResult(value),
      (error: unknown) =>
        error instanceof GraphExpansionError && error.fieldPath === fieldPath,
    );
  }
});

test("frozen authentication outranks invalid seed and graph bounds", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    const frozen = await engine.freezeReadContext({ schemaVersion: 1, view: {}, eligibility: {} });
    frozen.token += "0";
    await assert.rejects(
      graph.expand(
        engine,
        request(["duplicate", "duplicate"], {
          maxDepth: 4,
          maxWorkUnits: "0",
          context: { schemaVersion: 1, type: "frozen", context: frozen },
        }),
      ),
      FrozenReadError,
    );
  } finally {
    await engine.close();
  }
});

test("legacy graph verbs remain source compatible", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    await engine.write([node("root", "seed"), node("target"), edge("e", "root", "target")]);
    assert.equal((await graph.neighbors(engine, "root", 1, "outgoing"))[0]?.logicalId, "target");
    assert.ok(Array.isArray((await graph.searchExpand(engine, "root", 0)).searchHits));
    const direction: TraversalDirection = "both";
    assert.equal(direction, "both");
  } finally {
    await engine.close();
  }
});
