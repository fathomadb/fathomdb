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
import { InvalidFilterError, WriteValidationError } from "../src/errors.js";
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
