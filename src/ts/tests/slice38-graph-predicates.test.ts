// 0.8.31 Slice 30 (`STRICT-GRAPH`) — Layer 4 TypeScript SDK parity tests.

import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";

import {
  Engine,
  graph,
  type GraphArmSeedSource,
  type GraphExpandRequestV1,
  type GraphSeedMode,
  type PropertyPredicate,
  type SearchOptions,
  type TextSearchOptions,
} from "../src/index.js";
import {
  GraphExpansionError,
  InvalidArgumentError,
  InvalidFilterError,
  WriteValidationError,
} from "../src/errors.js";
import { canonicalGraphRequest } from "../src/graph.js";
import { freshDbPath } from "./helpers.js";

const I64_MIN = -(1n << 63n);
const I64_MAX = (1n << 63n) - 1n;
const BEYOND_F64_LO = (1n << 53n) + 1n;
const BEYOND_F64_HI = (1n << 53n) + 2n;

// Compile-time type isolation check: graph-arm seeding options exist on
// SearchOptions and MUST NOT exist on TextSearchOptions.
type AssertTrue<T extends true> = T;
type _SearchHasGraphSeed = AssertTrue<"graphSeed" extends keyof SearchOptions ? true : false>;
type _SearchHasGraphSeedK = AssertTrue<"graphSeedK" extends keyof SearchOptions ? true : false>;
type _SearchHasGraphEdgeKinds = AssertTrue<
  "graphEdgeKinds" extends keyof SearchOptions ? true : false
>;
type _SearchHasGraphEdgePredicates = AssertTrue<
  "graphEdgePredicates" extends keyof SearchOptions ? true : false
>;
type _TextExcludesGraphSeed = AssertTrue<
  "graphSeed" extends keyof TextSearchOptions ? false : true
>;
type _TextExcludesGraphSeedK = AssertTrue<
  "graphSeedK" extends keyof TextSearchOptions ? false : true
>;
type _TextExcludesGraphEdgeKinds = AssertTrue<
  "graphEdgeKinds" extends keyof TextSearchOptions ? false : true
>;
type _TextExcludesGraphEdgePredicates = AssertTrue<
  "graphEdgePredicates" extends keyof TextSearchOptions ? false : true
>;

function canonicalProv(rev: string, ver: string) {
  return {
    schemaVersion: 1 as const,
    role: "canonical" as const,
    artifactRevisionId: rev,
    sourceVersionId: ver,
  };
}

function derivedProv(rev: string, srcRev: string, body: string) {
  return {
    schemaVersion: 1 as const,
    role: "derived" as const,
    artifactRevisionId: rev,
    sourceVersionId: "v1",
    sourceRevisionId: srcRev,
    sourceLocator: { kind: "whole_body" as const },
    canonicalSourceHash: {
      algorithm: "sha256" as const,
      digestHex: createHash("sha256").update(body, "utf8").digest("hex"),
    },
  };
}

function baseRequest(overrides: Partial<GraphExpandRequestV1> = {}): GraphExpandRequestV1 {
  return {
    schemaVersion: 1,
    seed: {
      schemaVersion: 1,
      type: "explicit",
      logicalIds: [{ space: "logical", value: "n:root" }],
    },
    direction: "outgoing",
    edgeKinds: ["rel"],
    targetKinds: [],
    context: {
      schemaVersion: 1,
      type: "current",
      context: {
        schemaVersion: 1,
        view: {},
        eligibility: {},
      },
    },
    maxDepth: 1,
    resultLimit: 10,
    maxWorkUnits: "100",
    includeExplanation: false,
    ...overrides,
  };
}

test("slice38 graph.expand predicates matrix, bigint > 2^53, wire omission, and hydration", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    await engine.write([
      {
        kind: "entity",
        body: "root node",
        sourceId: "src:g",
        logicalId: "n:root",
        properties: { role: "seed" },
      },
      {
        kind: "entity",
        body: "target alpha",
        sourceId: "src:g",
        logicalId: "n:a",
        properties: {
          tag: "alpha",
          score: 10,
          big: BEYOND_F64_LO,
          ratio: 1.5,
          active: true,
        },
      },
      {
        kind: "entity",
        body: "target beta",
        sourceId: "src:g",
        logicalId: "n:b",
        properties: {
          tag: "beta",
          score: 20,
          big: BEYOND_F64_HI,
          ratio: 2.5,
          active: false,
        },
      },
      {
        kind: "entity",
        // JSON body with matching keys, properties omitted -> must NOT satisfy predicates
        body: '{"tag": "alpha", "score": 10}',
        sourceId: "src:g",
        logicalId: "n:body_only",
      },
      {
        edge: {
          kind: "rel",
          from: "n:root",
          to: "n:a",
          sourceId: "src:g",
          logicalId: "e:a_low",
          properties: { weight: 1, verified: false, label: "low" },
        },
      },
      {
        edge: {
          kind: "rel",
          from: "n:root",
          to: "n:a",
          sourceId: "src:g",
          logicalId: "e:a_hi",
          properties: {
            weight: 10,
            verified: true,
            label: "hi",
            bigEdge: BEYOND_F64_HI,
            rate: 0.75,
          },
        },
      },
      {
        edge: {
          kind: "rel",
          from: "n:root",
          to: "n:b",
          sourceId: "src:g",
          logicalId: "e:b",
          properties: { weight: 5, verified: false, label: "mid" },
        },
      },
      {
        edge: {
          kind: "rel",
          from: "n:root",
          to: "n:body_only",
          sourceId: "src:g",
          logicalId: "e:body_only",
          properties: { weight: 10, verified: true },
        },
      },
    ]);

    // Wire canonicalization omits edgePredicates/targetPredicates when absent or empty
    const unpredWire = canonicalGraphRequest(baseRequest());
    assert.equal("edgePredicates" in unpredWire, false);
    assert.equal("targetPredicates" in unpredWire, false);
    const emptyPredWire = canonicalGraphRequest(
      baseRequest({ edgePredicates: [], targetPredicates: [] }),
    );
    assert.equal("edgePredicates" in emptyPredWire, false);
    assert.equal("targetPredicates" in emptyPredWire, false);

    // Parallel edge non-poisoning + hydration on GraphTargetV1 & GraphOriginV1
    const res = await graph.expand(
      engine,
      baseRequest({
        edgePredicates: [
          { key: "weight", op: "gte", value: 10 },
          { key: "verified", op: "eq", value: true },
        ],
        targetPredicates: [{ key: "tag", op: "eq", value: "alpha" }],
      }),
    );
    assert.deepEqual(
      res.targets.map((t) => t.logicalId),
      ["n:a"],
    );
    const target = res.targets[0]!;
    assert.equal(target.properties?.tag, "alpha");
    assert.equal(target.properties?.big, BEYOND_F64_LO);
    assert.equal(target.properties?.active, true);
    assert.equal(target.origin.terminalEdgeProperties?.label, "hi");
    assert.equal(target.origin.terminalEdgeProperties?.bigEdge, BEYOND_F64_HI);
    assert.equal(target.origin.terminalEdgeProperties?.verified, true);

    // Exact bigint > 2^53 precision
    const resBig = await graph.expand(
      engine,
      baseRequest({
        targetPredicates: [{ key: "big", op: "gt", value: BEYOND_F64_LO }],
      }),
    );
    assert.deepEqual(
      resBig.targets.map((t) => t.logicalId),
      ["n:b"],
    );

    const expandIds = async (
      edgePredicates?: readonly PropertyPredicate[],
      targetPredicates?: readonly PropertyPredicate[],
    ): Promise<string[]> => {
      const out = await graph.expand(
        engine,
        baseRequest({ edgePredicates, targetPredicates }),
      );
      return out.targets.map((t) => t.logicalId).sort();
    };

    assert.deepEqual(await expandIds(undefined, [{ key: "tag", op: "ne", value: "alpha" }]), [
      "n:b",
    ]);
    assert.deepEqual(await expandIds(undefined, [{ key: "score", op: "lt", value: 20 }]), ["n:a"]);
    assert.deepEqual(await expandIds(undefined, [{ key: "score", op: "lte", value: 20 }]), [
      "n:a",
      "n:b",
    ]);
    assert.deepEqual(await expandIds(undefined, [{ key: "ratio", op: "gt", value: 1.5 }]), ["n:b"]);
    assert.deepEqual(await expandIds(undefined, [{ key: "ratio", op: "gte", value: 1.5 }]), [
      "n:a",
      "n:b",
    ]);
    assert.deepEqual(await expandIds(undefined, [{ key: "active", op: "ne", value: true }]), [
      "n:b",
    ]);

    // Missing key & cross-type evaluate false for ALL 6 ops
    for (const op of ["eq", "ne", "lt", "lte", "gt", "gte"] as const) {
      assert.deepEqual(await expandIds(undefined, [{ key: "missing", op, value: "x" }]), []);
      assert.equal(
        (await expandIds(undefined, [{ key: "tag", op, value: "zzz" }])).includes("n:body_only"),
        false,
      );
      assert.deepEqual(await expandIds(undefined, [{ key: "tag", op, value: 10 }]), []);
    }
    assert.deepEqual(await expandIds(undefined, [{ key: "active", op: "ne", value: 1 }]), []);
    assert.deepEqual(await expandIds(undefined, [{ key: "score", op: "ne", value: true }]), []);
  } finally {
    await engine.close();
  }
});

test("slice38 graph.expand predicate validation refusals", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    const checkRefusal = async (
      overrides: Partial<GraphExpandRequestV1>,
      expectedReason: string,
    ) => {
      await assert.rejects(
        () => graph.expand(engine, baseRequest(overrides)),
        (err: unknown) => err instanceof GraphExpansionError && err.reason === expectedReason,
      );
    };

    for (const op of ["lt", "lte", "gt", "gte"] as const) {
      await checkRefusal(
        { edgePredicates: [{ key: "flag", op, value: true }] },
        "graph_edge_predicates_invalid",
      );
      await checkRefusal(
        { targetPredicates: [{ key: "flag", op, value: false }] },
        "graph_target_predicates_invalid",
      );
    }

    const tooMany: PropertyPredicate[] = Array.from({ length: 33 }, (_, i) => ({
      key: `k${i}`,
      op: "eq",
      value: i,
    }));
    await checkRefusal({ edgePredicates: tooMany }, "graph_edge_predicates_invalid");
    await checkRefusal({ targetPredicates: tooMany }, "graph_target_predicates_invalid");

    const badPreds: unknown[] = [
      { key: "", op: "eq", value: "v" },
      { key: "bad\0key", op: "eq", value: "v" },
      { key: "bad\x1ekey", op: "eq", value: "v" },
      { key: "bad\uD800key", op: "eq", value: "v" },
      { key: "k", op: "eq", value: "bad\0val" },
      { key: "k", op: "eq", value: "bad\x1eval" },
      { key: "k", op: "eq", value: "bad\uD800val" },
      { key: "k", op: "eq", value: Number.NaN },
      { key: "k", op: "eq", value: Number.POSITIVE_INFINITY },
      { key: "k", op: "eq", value: I64_MAX + 1n },
      { key: "k", op: "eq", value: I64_MIN - 1n },
      { key: "k", op: "like", value: "v" },
    ];
    for (const bad of badPreds) {
      await checkRefusal(
        { edgePredicates: [bad as PropertyPredicate] },
        "graph_edge_predicates_invalid",
      );
      await checkRefusal(
        { targetPredicates: [bad as PropertyPredicate] },
        "graph_target_predicates_invalid",
      );
    }

    // Closed object check: unknown field inside predicate object refuses with unknown_field
    await checkRefusal(
      { edgePredicates: [{ key: "k", op: "eq", value: "v", extra: 1 } as unknown as PropertyPredicate] },
      "unknown_field",
    );
  } finally {
    await engine.close();
  }
});

test("slice38 use_graph_arm seeding, provenance, evidence, and TextSearchOptions isolation", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    const srcBody = "quantum observatory telemetry anchor report";
    await engine.write([
      {
        kind: "note",
        body: srcBody,
        sourceId: "src:seed",
        logicalId: "n:anchor",
        provenance: canonicalProv("rev:anchor", "v1"),
      },
      {
        kind: "note",
        body: "celestial neighbor delta",
        sourceId: "src:seed",
        logicalId: "n:bodyless_neighbor",
        provenance: derivedProv("rev:nbr1", "rev:anchor", srcBody),
      },
      {
        kind: "note",
        body: "filtered out neighbor gamma",
        sourceId: "src:seed",
        logicalId: "n:filtered_neighbor",
        provenance: derivedProv("rev:nbr2", "rev:anchor", srcBody),
      },
      {
        kind: "note",
        body: "edge fts companion epsilon",
        sourceId: "src:seed",
        logicalId: "n:edge_fts_target",
        provenance: derivedProv("rev:nbr3", "rev:anchor", srcBody),
      },
      {
        edge: {
          kind: "supports",
          from: "n:anchor",
          to: "n:bodyless_neighbor",
          sourceId: "src:seed",
          logicalId: "e:bodyless_hi",
          properties: { strength: 9, tier: "gold" },
          provenance: derivedProv("rev:e1", "rev:anchor", srcBody),
        },
      },
      {
        edge: {
          kind: "weak_link",
          from: "n:anchor",
          to: "n:filtered_neighbor",
          sourceId: "src:seed",
          logicalId: "e:bodyless_lo",
          properties: { strength: 2, tier: "bronze" },
          provenance: derivedProv("rev:e2", "rev:anchor", srcBody),
        },
      },
      {
        edge: {
          kind: "mentions",
          from: "n:anchor",
          to: "n:edge_fts_target",
          body: "observatory telemetry citation link",
          sourceId: "src:seed",
          logicalId: "e:fts",
          properties: { strength: 7, tier: "silver" },
          provenance: derivedProv("rev:e3", "rev:anchor", srcBody),
        },
      },
    ]);

    const mode: GraphSeedMode = "fused_top_k";
    const expectedSeedSource: GraphArmSeedSource = "fused_top_k";

    // Default graph_fts seeds via EntityFts on lexical node match -> "entity_fts"
    const defaultRes = await engine.search(
      "observatory telemetry",
      undefined,
      0,
      true,
      undefined,
      undefined,
      true,
      { graphSeed: "graph_fts", graphEdgeKinds: ["supports"] },
    );
    assert.ok(defaultRes.explanation !== null);
    const defNbrIdx = defaultRes.results.findIndex((h) => h.id.value === "n:bodyless_neighbor");
    assert.ok(defNbrIdx >= 0);
    assert.equal(defaultRes.explanation.perHit[defNbrIdx]!.graphSeedSource, "entity_fts");
    assert.equal(defaultRes.explanation.perHit[defNbrIdx]!.graphHopDepth, 1);
    defaultRes.results.forEach((hit, idx) => {
      if (hit.branch !== "graph_arm") {
        assert.equal(defaultRes.explanation!.perHit[idx]!.graphSeedSource, null);
        assert.equal(defaultRes.explanation!.perHit[idx]!.graphHopDepth, null);
      }
    });

    // Opt-in fused_top_k seeds body-less property-graph edge
    const fusedRes = await engine.search(
      "observatory telemetry",
      undefined,
      0,
      true,
      undefined,
      undefined,
      true,
      {
        graphSeed: mode,
        graphSeedK: 5,
        graphEdgeKinds: ["supports", "weak_link"],
        graphEdgePredicates: [{ key: "strength", op: "gte", value: 5 }],
      },
    );
    assert.ok(fusedRes.explanation !== null);
    const hitIds = fusedRes.results.map((h) => h.id.value);
    assert.ok(hitIds.includes("n:bodyless_neighbor"));
    assert.equal(hitIds.includes("n:filtered_neighbor"), false);
    const nbrIdx = hitIds.indexOf("n:bodyless_neighbor");
    assert.equal(fusedRes.results[nbrIdx]!.branch, "graph_arm");
    assert.equal(fusedRes.explanation.perHit[nbrIdx]!.graphSeedSource, expectedSeedSource);
    assert.equal(fusedRes.explanation.perHit[nbrIdx]!.graphHopDepth, 1);

    // graphSeed: "both" discovers both body-less and edge-FTS neighbors
    const bothRes = await engine.search(
      "observatory telemetry",
      undefined,
      0,
      true,
      undefined,
      undefined,
      true,
      {
        graphSeed: "both",
        graphSeedK: 5,
        graphEdgeKinds: ["supports", "mentions"],
      },
    );
    const bothIds = bothRes.results.map((h) => h.id.value);
    assert.ok(bothIds.includes("n:bodyless_neighbor"));
    assert.ok(bothIds.includes("n:edge_fts_target"));

    // searchFrozen supports graph-seeding options identically
    const frozen = await engine.freezeReadContext({
      schemaVersion: 1,
      view: {},
      eligibility: {},
    });
    const frozenRes = await engine.searchFrozen("observatory telemetry", frozen, {
      useGraphArm: true,
      explain: true,
      graphSeed: "fused_top_k",
      graphSeedK: 5,
      graphEdgeKinds: ["supports"],
      graphEdgePredicates: [{ key: "tier", op: "eq", value: "gold" }],
    });
    assert.ok(frozenRes.explanation !== null);
    const fIdx = frozenRes.results.findIndex((h) => h.id.value === "n:bodyless_neighbor");
    assert.ok(fIdx >= 0);
    assert.equal(frozenRes.explanation.perHit[fIdx]!.graphSeedSource, "fused_top_k");
    assert.equal(frozenRes.explanation.perHit[fIdx]!.graphHopDepth, 1);

    // searchWithEvidence + resolveEvidence exposes EvidenceGraphOriginV1.seedSource
    const evRes = await engine.searchWithEvidence({
      schemaVersion: 1,
      query: "observatory telemetry",
      context: frozen,
      useGraphArm: true,
      includeExplanation: true,
      graphSeed: "fused_top_k",
      graphSeedK: 5,
      graphEdgeKinds: ["supports"],
      graphEdgePredicates: [{ key: "tier", op: "eq", value: "gold" }],
    });
    const evIdx = evRes.searchResult.results.findIndex((h) => h.id.value === "n:bodyless_neighbor");
    assert.ok(evIdx >= 0);
    const resolved = await engine.resolveEvidence({
      schemaVersion: 1,
      evidenceRef: evRes.evidence[evIdx]!.evidenceRef,
      context: frozen,
    });
    assert.equal(resolved.projectionOrigin.representativeArm, "graph_arm");
    assert.ok(resolved.projectionOrigin.graphOrigin !== null);
    assert.equal(resolved.projectionOrigin.graphOrigin.kind, "traversal");
    assert.equal(resolved.projectionOrigin.graphOrigin.hopCount, 1);
    assert.equal(resolved.projectionOrigin.graphOrigin.seedSource, "fused_top_k");

    // EdgeFts seedSource via resolveEvidence
    const evFts = await engine.searchWithEvidence({
      schemaVersion: 1,
      query: "observatory telemetry",
      context: frozen,
      useGraphArm: true,
      graphSeed: "graph_fts",
      graphEdgeKinds: ["mentions"],
    });
    const ftsIdx = evFts.searchResult.results.findIndex((h) => h.id.value === "n:edge_fts_target");
    assert.ok(ftsIdx >= 0);
    const resolvedFts = await engine.resolveEvidence({
      schemaVersion: 1,
      evidenceRef: evFts.evidence[ftsIdx]!.evidenceRef,
      context: frozen,
    });
    assert.equal(resolvedFts.projectionOrigin.graphOrigin?.seedSource, "edge_fts");

    // Validation errors
    await assert.rejects(
      () =>
        engine.search("observatory", undefined, 0, true, undefined, undefined, false, {
          graphSeed: "bogus" as GraphSeedMode,
        }),
      (err: unknown) => err instanceof InvalidArgumentError,
    );
    for (const badK of [0, -1, 101]) {
      await assert.rejects(
        () =>
          engine.search("observatory", undefined, 0, true, undefined, undefined, false, {
            graphSeedK: badK,
          }),
        (err: unknown) => err instanceof InvalidArgumentError,
      );
      await assert.rejects(
        () => engine.searchFrozen("observatory", frozen, { graphSeedK: badK }),
        (err: unknown) => err instanceof InvalidArgumentError,
      );
      await assert.rejects(
        () =>
          engine.searchWithEvidence({
            schemaVersion: 1,
            query: "observatory",
            context: frozen,
            graphSeedK: badK,
          }),
        (err: unknown) => err instanceof InvalidArgumentError,
      );
    }
    await assert.rejects(
      () =>
        engine.search("observatory", undefined, 0, true, undefined, undefined, false, {
          graphEdgeKinds: [""],
        }),
      (err: unknown) => err instanceof InvalidFilterError,
    );
    await assert.rejects(
      () =>
        engine.search("observatory", undefined, 0, true, undefined, undefined, false, {
          graphEdgeKinds: ["bad\0kind"],
        }),
      (err: unknown) => err instanceof WriteValidationError,
    );
    await assert.rejects(
      () =>
        engine.search("observatory", undefined, 0, true, undefined, undefined, false, {
          graphEdgePredicates: [{ key: "flag", op: "gt", value: true }],
        }),
      (err: unknown) => err instanceof InvalidFilterError,
    );

    // Runtime isolation: passing graphSeed to searchTextOnly fails closed with InvalidArgumentError
    await assert.rejects(
      () =>
        engine.searchTextOnly("observatory", {
          graphSeed: "fused_top_k",
        } as unknown as TextSearchOptions),
      (err: unknown) => err instanceof InvalidArgumentError,
    );
  } finally {
    await engine.close();
  }
});
