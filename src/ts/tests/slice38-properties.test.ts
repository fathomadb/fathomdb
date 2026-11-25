// 0.8.31 Slice 20 (`PROPERTY-SCHEMA`) — Layer 5 TypeScript SDK parity tests.

import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";

import {
  Engine,
  graph,
  read,
  type OperationalCollectionInfo,
  type PropertyMap,
  type PropertyPredicate,
} from "../src/index.js";
import {
  GraphExpansionError,
  InvalidFilterError,
  WriteValidationError,
} from "../src/errors.js";
import { freshDbPath } from "./helpers.js";

const I64_MIN = -(1n << 63n);
const I64_MAX = (1n << 63n) - 1n;
const BEYOND_F64_POS = (1n << 53n) + 1n;
const BEYOND_F64_NEG = -((1n << 53n) + 1n);

test("slice38 inline property round-trip (bigint, -0, bool, string, empty) and rejections", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    const props: PropertyMap = {
      strVal: "hello world",
      safeInt: 42,
      intMin: I64_MIN,
      intMax: I64_MAX,
      intHi: BEYOND_F64_POS,
      intLo: BEYOND_F64_NEG,
      floatVal: 3.25,
      negZero: -0,
      flagTrue: true,
      flagFalse: false,
    };

    await engine.write([
      {
        kind: "note",
        body: "node with typed properties",
        sourceId: "src:props",
        logicalId: "node:typed",
        properties: props,
      },
      {
        kind: "note",
        body: "node with empty properties",
        sourceId: "src:props",
        logicalId: "node:empty",
        properties: {},
      },
    ]);

    const rec = await read.get(engine, "node:typed");
    assert.ok(rec !== null);
    assert.ok(rec.properties !== null && rec.properties !== undefined);
    assert.equal(rec.properties.strVal, "hello world");
    assert.equal(rec.properties.safeInt, 42);
    assert.equal(rec.properties.intMin, I64_MIN);
    assert.equal(rec.properties.intMax, I64_MAX);
    assert.equal(rec.properties.intHi, BEYOND_F64_POS);
    assert.equal(rec.properties.intLo, BEYOND_F64_NEG);
    assert.equal(rec.properties.floatVal, 3.25);
    assert.equal(rec.properties.negZero, 0);
    assert.equal(Object.is(rec.properties.negZero, -0), false);
    assert.equal(rec.properties.flagTrue, true);
    assert.equal(rec.properties.flagFalse, false);

    const emptyRec = await read.get(engine, "node:empty");
    assert.ok(emptyRec !== null);
    assert.equal(emptyRec.properties, null);

    const invalidMaps: unknown[] = [
      { "": "empty-key" },
      { "bad\0key": "nul" },
      { "bad\x1ekey": "rs" },
      { k: "bad\0val" },
      { k: "bad\x1eval" },
      { k: "lone\uD800surrogate" },
      { k: null },
      { k: { nested: 1 } },
      { k: [1, 2] },
      { k: Number.NaN },
      { k: Number.POSITIVE_INFINITY },
      { k: Number.NEGATIVE_INFINITY },
      { k: I64_MAX + 1n },
      { k: I64_MIN - 1n },
    ];
    for (let i = 0; i < invalidMaps.length; i += 1) {
      await assert.rejects(
        () =>
          engine.write([
            {
              kind: "note",
              body: `invalid-${i}`,
              sourceId: "src:invalid",
              logicalId: `node:invalid:${i}`,
              properties: invalidMaps[i] as PropertyMap,
            },
          ]),
        (err: unknown) => err instanceof WriteValidationError,
      );
      assert.equal(await read.get(engine, `node:invalid:${i}`), null);
    }
  } finally {
    await engine.close();
  }
});

test("slice38 O(1) hydration across read, search, graph.expand, and evidence", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    const srcBody = "anchor source alpha";
    const srcHash = createHash("sha256").update(srcBody).digest("hex");
    const seedProps: PropertyMap = { role: "seed", rank: 1, active: true };
    const targetProps: PropertyMap = { role: "target", bigId: BEYOND_F64_POS, weight: 2.5 };
    const edgeProps: PropertyMap = { relation: "supports", strength: 7, verified: true };

    await engine.write([
      {
        kind: "note",
        body: srcBody,
        sourceId: "src:hydrate",
        logicalId: "node:seed",
        properties: seedProps,
        provenance: {
          schemaVersion: 1,
          role: "canonical",
          artifactRevisionId: "rev-seed-1",
          sourceVersionId: "sv-1",
        },
      },
      {
        kind: "note",
        body: "target beta evidence payload",
        sourceId: "src:hydrate",
        logicalId: "node:target",
        properties: targetProps,
        provenance: {
          schemaVersion: 1,
          role: "derived",
          artifactRevisionId: "rev-target-1",
          sourceVersionId: "sv-1",
          sourceRevisionId: "rev-seed-1",
          sourceLocator: { kind: "whole_body" },
          canonicalSourceHash: { algorithm: "sha256", digestHex: srcHash },
        },
      },
      {
        edge: {
          kind: "supports",
          from: "node:seed",
          to: "node:target",
          sourceId: "src:hydrate",
          logicalId: "edge:seed-target",
          body: "supporting link",
          properties: edgeProps,
          provenance: {
            schemaVersion: 1,
            role: "derived",
            artifactRevisionId: "rev-edge-1",
            sourceVersionId: "sv-1",
            sourceRevisionId: "rev-seed-1",
            sourceLocator: { kind: "whole_body" },
            canonicalSourceHash: { algorithm: "sha256", digestHex: srcHash },
          },
        },
      },
    ]);

    const got = await read.get(engine, "node:target");
    assert.deepEqual(got?.properties, targetProps);

    const searchRes = await engine.searchTextOnly("target beta evidence");
    const hit = searchRes.results.find((h) => h.id.value === "node:target");
    assert.ok(hit !== undefined);
    assert.deepEqual(hit.properties, targetProps);

    const frozen = await engine.freezeReadContext({
      schemaVersion: 1,
      view: {},
      eligibility: {},
    });
    const evSearch = await engine.searchWithEvidence({
      schemaVersion: 1,
      query: "target beta evidence",
      context: frozen,
      limit: 5,
    });
    const targetSidecar = evSearch.evidence.find(
      (e) => e.artifactRevisionId === "rev-target-1",
    );
    assert.ok(targetSidecar !== undefined);
    const resolved = await engine.resolveEvidence({
      schemaVersion: 1,
      evidenceRef: targetSidecar.evidenceRef,
      context: frozen,
    });
    assert.deepEqual(resolved.properties, targetProps);

    const expanded = await graph.expand(engine, {
      schemaVersion: 1,
      seed: {
        schemaVersion: 1,
        type: "explicit",
        logicalIds: [{ space: "logical", value: "node:seed" }],
      },
      direction: "outgoing",
      edgeKinds: ["supports"],
      targetKinds: [],
      context: {
        schemaVersion: 1,
        type: "frozen",
        context: frozen,
      },
      maxDepth: 1,
      resultLimit: 10,
      maxWorkUnits: "100",
      includeExplanation: true,
      includeEvidence: true,
    });
    assert.equal(expanded.targets.length, 1);
    assert.deepEqual(expanded.targets[0]?.properties, targetProps);
    assert.deepEqual(expanded.targets[0]?.origin.terminalEdgeProperties, edgeProps);

    assert.ok(expanded.evidence !== undefined && expanded.evidence.entries.length === 1);
    const entry = expanded.evidence.entries[0]!;
    const nodeEv = await engine.resolveGraphEvidence({
      schemaVersion: 1,
      evidenceRef: entry.targetEvidenceRef,
      context: frozen,
    });
    assert.equal(nodeEv.artifact.artifactClass, "node");
    assert.deepEqual(nodeEv.artifact.properties, targetProps);

    const edgeEv = await engine.resolveGraphEvidence({
      schemaVersion: 1,
      evidenceRef: entry.terminalEdgeEvidenceRef,
      context: frozen,
    });
    assert.equal(edgeEv.artifact.artifactClass, "edge");
    assert.deepEqual(edgeEv.artifact.properties, edgeProps);
  } finally {
    await engine.close();
  }
});

test("slice38 SearchFilter.propertyPredicates, ProjectionSpec.entityClass & vectorKinds", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    const delta = await engine.configureProjections([
      { name: "priority", roles: ["filterable"], fts: false, vector: false, entityClass: "node" },
      { name: "verified", roles: ["filterable"], fts: false, vector: false, entityClass: "node" },
      { name: "edge_tag", roles: ["filterable"], fts: false, vector: false, entityClass: "edge" },
      {
        name: "body",
        roles: ["searchable"],
        fts: false,
        vector: true,
        vectorKinds: ["custom:memory"],
      },
    ]);
    assert.equal(delta.unchanged, false);

    const specs = new Map((await read.projections(engine)).map((s) => [s.name, s]));
    assert.equal(specs.get("priority")?.entityClass, "node");
    assert.equal(specs.get("edge_tag")?.entityClass, "edge");
    assert.deepEqual(specs.get("body")?.vectorKinds, ["custom:memory"]);

    await assert.rejects(
      () =>
        engine.configureProjections([
          {
            name: "bad_vec",
            roles: ["searchable"],
            fts: false,
            vector: true,
            vectorKinds: ["edge_fact"],
          },
        ]),
      (err: unknown) => err instanceof WriteValidationError,
    );

    await engine.write([
      {
        kind: "custom:memory",
        body: "shared search needle alpha",
        sourceId: "src:pred",
        logicalId: "n:1",
        properties: { priority: 10, verified: true },
      },
      {
        kind: "custom:memory",
        body: "shared search needle beta",
        sourceId: "src:pred",
        logicalId: "n:2",
        properties: { priority: 20, verified: false },
      },
      {
        kind: "invoice",
        body: "shared search needle gamma",
        sourceId: "src:pred",
        logicalId: "n:3",
        properties: { priority: 30, verified: true },
      },
    ]);

    const reapply = await engine.configureProjections([
      { name: "priority", roles: ["filterable"], fts: false, vector: false, entityClass: "node" },
      { name: "verified", roles: ["filterable"], fts: false, vector: false, entityClass: "node" },
      { name: "edge_tag", roles: ["filterable"], fts: false, vector: false, entityClass: "edge" },
      {
        name: "body",
        roles: ["searchable"],
        fts: false,
        vector: true,
        vectorKinds: ["custom:memory"],
      },
    ]);
    assert.ok(reapply.vectorUnsupportedKinds.includes("invoice"));
    assert.ok(!reapply.vectorUnsupportedKinds.includes("custom:memory"));

    const hitIds = async (propertyPredicates: PropertyPredicate[]): Promise<string[]> => {
      const res = await engine.searchTextOnly("shared search needle", {
        filter: { propertyPredicates },
      });
      return res.results.map((h) => h.id.value).sort();
    };

    assert.deepEqual(await hitIds([{ key: "priority", op: "eq", value: 20 }]), ["n:2"]);
    assert.deepEqual(await hitIds([{ key: "priority", op: "ne", value: 20 }]), ["n:1", "n:3"]);
    assert.deepEqual(await hitIds([{ key: "priority", op: "lt", value: 20 }]), ["n:1"]);
    assert.deepEqual(await hitIds([{ key: "priority", op: "lte", value: 20 }]), ["n:1", "n:2"]);
    assert.deepEqual(await hitIds([{ key: "priority", op: "gt", value: 20 }]), ["n:3"]);
    assert.deepEqual(await hitIds([{ key: "priority", op: "gte", value: 20 }]), ["n:2", "n:3"]);
    assert.deepEqual(await hitIds([{ key: "verified", op: "eq", value: true }]), ["n:1", "n:3"]);
    assert.deepEqual(await hitIds([{ key: "verified", op: "ne", value: true }]), ["n:2"]);

    for (const op of ["lt", "lte", "gt", "gte"] as const) {
      await assert.rejects(
        () =>
          engine.searchTextOnly("shared search needle", {
            filter: { propertyPredicates: [{ key: "verified", op, value: true }] },
          }),
        (err: unknown) => err instanceof InvalidFilterError,
      );
    }

    await assert.rejects(
      () =>
        engine.searchTextOnly("shared search needle", {
          filter: { propertyPredicates: [{ key: "undeclared", op: "eq", value: 1 }] },
        }),
      (err: unknown) => err instanceof InvalidFilterError,
    );
    await assert.rejects(
      () =>
        engine.searchTextOnly("shared search needle", {
          filter: { propertyPredicates: [{ key: "edge_tag", op: "eq", value: "x" }] },
        }),
      (err: unknown) => err instanceof InvalidFilterError,
    );
  } finally {
    await engine.close();
  }
});

test("slice38 operationalCollections, AdminSchema kind conflict, and EraseReport.completedPendingErasure", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    const initial: OperationalCollectionInfo[] = await read.operationalCollections(engine);
    assert.deepEqual(
      initial.map((c) => c.name),
      ["projection_failures"],
    );
    assert.equal(await read.operationalCollection(engine, "missing"), null);

    await engine.write([
      {
        adminSchema: {
          name: "alpha_log",
          kind: "append_only_log",
          schemaJson: '{"type":"object"}',
          retentionJson: '{"max_rows":10}',
        },
      },
    ]);

    const cols = await read.operationalCollections(engine);
    assert.deepEqual(
      cols.map((c) => c.name),
      ["alpha_log", "projection_failures"],
    );
    const alpha = await read.operationalCollection(engine, "alpha_log");
    assert.ok(alpha !== null);
    assert.equal(alpha.name, "alpha_log");
    assert.equal(alpha.kind, "append_only_log");
    assert.equal(alpha.schemaJson, '{"type":"object"}');
    assert.equal(alpha.retentionJson, '{"max_rows":10}');
    assert.equal(alpha.formatVersion, 1);

    await assert.rejects(
      () =>
        engine.write([
          {
            adminSchema: {
              name: "alpha_log",
              kind: "latest_state",
              schemaJson: "{}",
              retentionJson: "{}",
            },
          },
        ]),
      (err: unknown) => err instanceof WriteValidationError,
    );

    await engine.write([{ kind: "note", body: "erase check", sourceId: "src:erase" }]);
    const report = await engine.eraseSource("src:erase");
    assert.equal(report.nodesExcised, 1);
    assert.equal(report.completedPendingErasure, false);

    const noop = await engine.eraseSource("src:erase");
    assert.equal(noop.nodesExcised, 0);
    assert.equal(noop.completedPendingErasure, false);
  } finally {
    await engine.close();
  }
});

test("slice38 frozen context with eligibility.propertyPredicates works on evidence and traceDependency", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    await engine.configureProjections([
      { name: "tier", roles: ["filterable"], fts: false, vector: false, entityClass: "node" },
    ]);
    const srcBody = "frozen predicate source alpha";
    const srcHash = createHash("sha256").update(srcBody).digest("hex");

    await engine.write([
      {
        kind: "note",
        body: srcBody,
        sourceId: "src:fpred",
        logicalId: "fp:src",
        properties: { tier: 1 },
        provenance: {
          schemaVersion: 1,
          role: "canonical",
          artifactRevisionId: "rev-fp-src",
          sourceVersionId: "sv-fp-1",
        },
      },
      {
        kind: "note",
        body: "frozen predicate derived target beta",
        sourceId: "src:fpred",
        logicalId: "fp:derived",
        properties: { tier: 2 },
        provenance: {
          schemaVersion: 1,
          role: "derived",
          artifactRevisionId: "rev-fp-derived",
          sourceVersionId: "sv-fp-1",
          sourceRevisionId: "rev-fp-src",
          sourceLocator: { kind: "whole_body" },
          canonicalSourceHash: { algorithm: "sha256", digestHex: srcHash },
        },
      },
      {
        edge: {
          kind: "supports",
          from: "fp:src",
          to: "fp:derived",
          sourceId: "src:fpred",
          logicalId: "fp:edge",
          body: "supporting frozen link",
          provenance: {
            schemaVersion: 1,
            role: "derived",
            artifactRevisionId: "rev-fp-edge",
            sourceVersionId: "sv-fp-1",
            sourceRevisionId: "rev-fp-src",
            sourceLocator: { kind: "whole_body" },
            canonicalSourceHash: { algorithm: "sha256", digestHex: srcHash },
          },
        },
      },
    ]);

    await engine.registerSourceDependency({
      schemaVersion: 1,
      dependencyId: "dep-fp-1",
      sourceRevisionId: "rev-fp-src",
      derivedRevisionId: "rev-fp-derived",
    });

    const frozen = await engine.freezeReadContext({
      schemaVersion: 1,
      view: {},
      eligibility: {
        propertyPredicates: [{ key: "tier", op: "gte", value: 1 }],
      },
    });

    const evSearch = await engine.searchWithEvidence({
      schemaVersion: 1,
      query: "frozen predicate derived target beta",
      context: frozen,
      limit: 5,
    });
    const sidecar = evSearch.evidence.find((e) => e.artifactRevisionId === "rev-fp-derived");
    assert.ok(sidecar !== undefined);

    const resolvedEv = await engine.resolveEvidence({
      schemaVersion: 1,
      evidenceRef: sidecar.evidenceRef,
      context: frozen,
    });
    assert.deepEqual(resolvedEv.properties, { tier: 2 });

    const expanded = await graph.expand(engine, {
      schemaVersion: 1,
      seed: {
        schemaVersion: 1,
        type: "explicit",
        logicalIds: [{ space: "logical", value: "fp:src" }],
      },
      direction: "outgoing",
      edgeKinds: ["supports"],
      targetKinds: [],
      context: {
        schemaVersion: 1,
        type: "frozen",
        context: frozen,
      },
      maxDepth: 1,
      resultLimit: 10,
      maxWorkUnits: "100",
      includeExplanation: false,
      includeEvidence: true,
    });
    assert.equal(expanded.targets.length, 1);
    const gEntry = expanded.evidence!.entries[0]!;
    const resolvedGraphEv = await engine.resolveGraphEvidence({
      schemaVersion: 1,
      evidenceRef: gEntry.targetEvidenceRef,
      context: frozen,
    });
    assert.equal(resolvedGraphEv.artifact.artifactClass, "node");
    assert.deepEqual(resolvedGraphEv.artifact.properties, { tier: 2 });

    const traced = await engine.traceDependency({
      schemaVersion: 1,
      rootRevisionId: "rev-fp-derived",
      direction: "to_source",
      context: frozen,
    });
    assert.equal(traced.rootRevisionId, "rev-fp-derived");
    assert.equal(traced.dependencyEdges.length, 1);
  } finally {
    await engine.close();
  }
});

test("slice38 graph.expand rejects lone surrogate and NUL in eligibility.propertyPredicates with GraphExpansionError", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    await engine.configureProjections([
      { name: "tag", roles: ["filterable"], fts: false, vector: false, entityClass: "node" },
    ]);
    await engine.write([
      {
        kind: "note",
        body: "graph expand ffi string seed",
        sourceId: "src:gffi",
        logicalId: "gffi:1",
        properties: { tag: "ok" },
      },
    ]);

    const badPredicates: Array<{ pred: PropertyPredicate; fieldPath: string }> = [
      { pred: { key: "tag\uD800", op: "eq", value: "ok" }, fieldPath: "/context/context/eligibility/propertyPredicates/0/key" },
      { pred: { key: "tag\0", op: "eq", value: "ok" }, fieldPath: "/context/context/eligibility/propertyPredicates/0/key" },
      { pred: { key: "tag", op: "eq", value: "bad\uD800val" }, fieldPath: "/context/context/eligibility/propertyPredicates/0/value" },
      { pred: { key: "tag", op: "eq", value: "bad\0val" }, fieldPath: "/context/context/eligibility/propertyPredicates/0/value" },
      { pred: { key: "tag", op: "eq", value: Number.NaN }, fieldPath: "/context/context/eligibility/propertyPredicates/0/value" },
    ];

    for (const { pred, fieldPath } of badPredicates) {
      await assert.rejects(
        () =>
          graph.expand(engine, {
            schemaVersion: 1,
            seed: {
              schemaVersion: 1,
              type: "explicit",
              logicalIds: [{ space: "logical", value: "gffi:1" }],
            },
            direction: "outgoing",
            edgeKinds: [],
            targetKinds: [],
            context: {
              schemaVersion: 1,
              type: "current",
              context: {
                schemaVersion: 1,
                view: {},
                eligibility: {
                  propertyPredicates: [pred],
                },
              },
            },
            maxDepth: 1,
            resultLimit: 10,
            maxWorkUnits: "100",
            includeExplanation: false,
          }),
        (err: unknown) =>
          err instanceof GraphExpansionError &&
          err.reason === "graph_context_invalid" &&
          err.fieldPath === fieldPath,
      );
    }
  } finally {
    await engine.close();
  }
});

test("slice40: simultaneous (node, status) + (edge, status), isolated drop, ProjectionRuntimeStatusEntry.entityClass, and TypeError on bare-string drop", async () => {
  const path = freshDbPath();
  const engine = await Engine.open(path, { useDefaultEmbedder: false });
  try {
    await engine.write([
      {
        kind: "note",
        body: '{"status":"open","text":"node status body"}',
        sourceId: "src:s40",
        logicalId: "ts-s40:n1",
        properties: { status: "open" },
      },
      {
        kind: "note",
        body: '{"status":"closed","text":"node closed body"}',
        sourceId: "src:s40",
        logicalId: "ts-s40:n2",
        properties: { status: "closed" },
      },
      {
        type: "edge",
        kind: "links",
        from: "ts-s40:n1",
        to: "ts-s40:n2",
        sourceId: "src:s40",
        logicalId: "ts-s40:e1",
        properties: { status: "verified" },
      },
    ]);

    const delta = await engine.configureProjections([
      {
        name: "status",
        roles: ["filterable", "searchable"],
        fts: true,
        vector: false,
        entityClass: "node",
      },
      {
        name: "status",
        roles: ["filterable"],
        fts: false,
        vector: false,
        entityClass: "edge",
      },
    ]);
    assert.equal(delta.unchanged, false);
    assert.deepEqual(delta.built, [
      { name: "status", entityClass: "node" },
      { name: "status", entityClass: "edge" },
    ]);

    const status = await read.projectionStatus(engine);
    assert.deepEqual(
      status.projections.map((p) => ({ name: p.name, entityClass: p.entityClass })),
      [
        { name: "status", entityClass: "node" },
        { name: "status", entityClass: "edge" },
      ],
    );

    // Strictly reject bare strings in drop with TypeError.
    await assert.rejects(
      () => engine.configureProjections([], ["status"] as unknown as Array<{ name: string }>),
      (err: unknown) => err instanceof TypeError,
    );

    // Drop ONLY the edge projection; node projection stays intact and searchable.
    const dropDelta = await engine.configureProjections([], [
      { name: "status", entityClass: "edge" },
    ]);
    assert.deepEqual(dropDelta.dropped, [{ name: "status", entityClass: "edge" }]);
    const remaining = await read.projections(engine);
    assert.deepEqual(
      remaining.map((s) => ({ name: s.name, entityClass: s.entityClass })),
      [{ name: "status", entityClass: "node" }],
    );

    const hits = await engine.searchProjectedText("open", "status");
    assert.deepEqual(
      hits.results.map((h) => h.id.value),
      ["ts-s40:n1"],
    );
  } finally {
    await engine.close();
  }
});
