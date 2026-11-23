// 0.8.31 Slice 40 (`HISTORY-GRAPH`) — Layer 3 TypeScript SDK parity tests.

import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";

import {
  Engine,
  graph,
  type GraphExpandRequestV1,
  type PropertyPredicate,
} from "../src/index.js";
import { EvidenceError, GraphExpansionError } from "../src/errors.js";
import { canonicalGraphRequest } from "../src/graph.js";
import { freshDbPath } from "./helpers.js";

const U64_MAX_STR = "18446744073709551615";
const U64_OVERFLOW_STR = "18446744073709551616";

function baseRequest(overrides: Partial<GraphExpandRequestV1> = {}): GraphExpandRequestV1 {
  return {
    schemaVersion: 1,
    seed: {
      schemaVersion: 1,
      type: "explicit",
      logicalIds: [{ space: "logical", value: "root" }],
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
        eligibility: {},
      },
    },
    maxDepth: 2,
    resultLimit: 20,
    maxWorkUnits: "1000",
    includeExplanation: false,
    ...overrides,
  };
}

function canonicalProv(rev: string, ver: string) {
  return {
    schemaVersion: 1 as const,
    role: "canonical" as const,
    artifactRevisionId: rev,
    sourceVersionId: ver,
  };
}

function derivedProv(rev: string, srcRev: string, body: string, ver = "sv-1") {
  return {
    schemaVersion: 1 as const,
    role: "derived" as const,
    artifactRevisionId: rev,
    sourceVersionId: ver,
    sourceRevisionId: srcRev,
    sourceLocator: { kind: "whole_body" as const },
    canonicalSourceHash: {
      algorithm: "sha256" as const,
      digestHex: createHash("sha256").update(body, "utf8").digest("hex"),
    },
  };
}

function queryCommitTimestampMs(dbPath: string, cursor: number): number {
  const out = execFileSync(
    "python3",
    [
      "-c",
      "import sqlite3, sys; c = sqlite3.connect(sys.argv[1]); row = c.execute('SELECT committed_at_ms FROM _fathomdb_commit_timestamps WHERE write_cursor = ?', (int(sys.argv[2]),)).fetchone(); print(row[0])",
      dbPath,
      String(cursor),
    ],
    { encoding: "utf8" },
  ).trim();
  return Number(out);
}

test("slice38 historical graph bitemporal 4-quadrant, hydration, and cursor bounds", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    const r1 = await engine.write([
      {
        kind: "doc",
        body: "root alpha v1",
        sourceId: "src:bi",
        logicalId: "root",
        validFrom: 50,
        validUntil: 500,
      },
      {
        kind: "claim",
        body: "target alpha v1",
        sourceId: "src:bi",
        logicalId: "target_a",
        validFrom: 100,
        validUntil: 200,
        properties: { version: 1, label: "alpha-v1" },
      },
      {
        edge: {
          kind: "supports",
          from: "root",
          to: "target_a",
          sourceId: "src:bi",
          logicalId: "e-root-a",
          tValid: 100,
          tInvalid: 200,
          properties: { weight: 10 },
        },
      },
    ]);
    const c1 = String(r1.cursor);

    const r2 = await engine.write([
      {
        kind: "finding",
        body: "target alpha v2",
        sourceId: "src:bi",
        logicalId: "target_a",
        validFrom: 200,
        validUntil: 300,
        properties: { version: 2, label: "alpha-v2" },
      },
      {
        edge: {
          kind: "supports",
          from: "root",
          to: "target_a",
          sourceId: "src:bi",
          logicalId: "e-root-a",
          tValid: 200,
          tInvalid: 300,
          properties: { weight: 20 },
        },
      },
    ]);
    const c2 = String(r2.cursor);

    // Default current-time wire omits asOfWriteCursor and asOfEpochMs.
    const defaultWire = canonicalGraphRequest(baseRequest());
    assert.equal("asOfWriteCursor" in defaultWire, false);
    assert.equal("asOfEpochMs" in defaultWire, false);

    const reqAt = (validAsOf: number, asOfWriteCursor: string) =>
      baseRequest({
        context: {
          schemaVersion: 1,
          type: "current",
          context: {
            schemaVersion: 1,
            view: { validAsOf },
            eligibility: {},
          },
        },
        asOfWriteCursor,
      });

    // Quadrant 1: (asOf=c1, validAsOf=150) -> v1
    const q1 = await graph.expand(engine, reqAt(150, c1));
    assert.equal(q1.targets.length, 1);
    assert.equal(q1.targets[0]!.logicalId, "target_a");
    assert.equal(q1.targets[0]!.kind, "claim");
    assert.equal(q1.targets[0]!.body, "target alpha v1");
    assert.deepEqual(q1.targets[0]!.properties, { version: 1, label: "alpha-v1" });
    assert.deepEqual(q1.targets[0]!.origin.terminalEdgeProperties, { weight: 10 });

    // Quadrant 2: (asOf=c1, validAsOf=250) -> empty
    const q2 = await graph.expand(engine, reqAt(250, c1));
    assert.deepEqual(q2.targets, []);

    // Quadrant 3: (asOf=c2, validAsOf=150) -> empty
    const q3 = await graph.expand(engine, reqAt(150, c2));
    assert.deepEqual(q3.targets, []);

    // Quadrant 4: (asOf=c2, validAsOf=250) -> v2
    const q4 = await graph.expand(engine, reqAt(250, c2));
    assert.equal(q4.targets.length, 1);
    assert.equal(q4.targets[0]!.kind, "finding");
    assert.equal(q4.targets[0]!.body, "target alpha v2");
    assert.deepEqual(q4.targets[0]!.properties, { version: 2, label: "alpha-v2" });
    assert.deepEqual(q4.targets[0]!.origin.terminalEdgeProperties, { weight: 20 });

    // Future cursor (u64::MAX) acts as snapshot-bounded upper bound.
    const future = await graph.expand(engine, reqAt(250, U64_MAX_STR));
    assert.equal(future.targets.length, 1);
    assert.equal(future.targets[0]!.body, "target alpha v2");

    // Cursor "0": empty query seeds, explicit seed unavailable.
    const qZero = await graph.expand(
      engine,
      baseRequest({
        seed: { schemaVersion: 1, type: "query", text: "root alpha", rankedLimit: 5 },
        asOfWriteCursor: "0",
      }),
    );
    assert.deepEqual(qZero.seeds, []);
    assert.deepEqual(qZero.targets, []);

    await assert.rejects(
      graph.expand(engine, reqAt(150, "0")),
      (err: unknown) =>
        err instanceof GraphExpansionError &&
        err.reason === "graph_seed_unavailable" &&
        err.fieldPath === "/seed/logicalIds/0",
    );
  } finally {
    await engine.close();
  }
});

test("slice38 historical graph asOfEpochMs resolution and declared Filterable inline eligibility", async () => {
  const dbPath = freshDbPath();
  const engine = await Engine.open(dbPath, { useDefaultEmbedder: false });
  try {
    await engine.configureProjections([
      { name: "tier", roles: ["filterable"], fts: false, vector: false, entityClass: "node" },
      { name: "region", roles: ["filterable"], fts: false, vector: false, entityClass: "node" },
      { name: "weight", roles: ["filterable"], fts: false, vector: false, entityClass: "edge" },
    ]);

    const r1 = await engine.write([
      {
        kind: "entity",
        body: "epoch root",
        sourceId: "src:ep",
        logicalId: "root",
        properties: { tier: "gold", region: "us" },
      },
      {
        kind: "entity",
        body: "target gold v1",
        sourceId: "src:ep",
        logicalId: "t_gold",
        properties: { tier: "gold", region: "us" },
      },
      {
        edge: {
          kind: "rel",
          from: "root",
          to: "t_gold",
          sourceId: "src:ep",
          logicalId: "e_gold",
          properties: { weight: 10 },
        },
      },
    ]);
    await new Promise((resolve) => setTimeout(resolve, 15));
    const r2 = await engine.write([
      {
        kind: "entity",
        body: "target bronze v2",
        sourceId: "src:ep",
        logicalId: "t_gold",
        properties: { tier: "bronze", region: "eu" },
      },
      {
        edge: {
          kind: "rel",
          from: "root",
          to: "t_gold",
          sourceId: "src:ep",
          logicalId: "e_gold",
          properties: { weight: 1 },
        },
      },
    ]);
    await engine.drain(30_000);

    const ts1 = queryCommitTimestampMs(dbPath, r1.cursor);
    const ts2 = queryCommitTimestampMs(dbPath, r2.cursor);
    assert.ok(ts1 < ts2);

    const goldPred: PropertyPredicate = { key: "tier", op: "eq", value: "gold" };
    const weightPred: PropertyPredicate = { key: "weight", op: "gte", value: 10 };

    const req = (asOfEpochMs?: number) =>
      baseRequest({
        context: {
          schemaVersion: 1,
          type: "current",
          context: {
            schemaVersion: 1,
            view: {},
            eligibility: {
              attributes: [["region", "us"]],
              propertyPredicates: [goldPred],
            },
          },
        },
        edgePredicates: [weightPred],
        targetPredicates: [goldPred],
        ...(asOfEpochMs !== undefined ? { asOfEpochMs } : {}),
      });

    // Current time -> empty (superseded to bronze/eu/1)
    assert.deepEqual((await graph.expand(engine, req())).targets, []);

    // Before ts1 -> cursor 0 (empty query seeds)
    const before = await graph.expand(
      engine,
      baseRequest({
        seed: { schemaVersion: 1, type: "query", text: "epoch root", rankedLimit: 5 },
        asOfEpochMs: ts1 - 1,
      }),
    );
    assert.deepEqual(before.seeds, []);
    assert.deepEqual(before.targets, []);

    // At ts1 -> historical v1 matches declared Filterable inline eligibility
    const atV1 = await graph.expand(engine, req(ts1));
    assert.equal(atV1.targets.length, 1);
    assert.equal(atV1.targets[0]!.body, "target gold v1");
    assert.deepEqual(atV1.targets[0]!.properties, { tier: "gold", region: "us" });
    assert.deepEqual(atV1.targets[0]!.origin.terminalEdgeProperties, { weight: 10 });

    // At ts2 -> v2 does not match gold/us/10
    assert.deepEqual((await graph.expand(engine, req(ts2))).targets, []);
  } finally {
    await engine.close();
  }
});

test("slice38 historical frozen evidence superseded flag and erasure/transition revocation", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    const srcBodyV1 = "ts canonical source v1";
    const srcBodyV2 = "ts canonical source v2";

    const r1 = await engine.write([
      {
        kind: "document",
        body: srcBodyV1,
        sourceId: "src:ev",
        logicalId: "src_doc",
        provenance: canonicalProv("ts-src-r1", "sv-1"),
      },
      {
        kind: "claim",
        body: "root v1",
        sourceId: "src:ev",
        logicalId: "root",
        provenance: derivedProv("ts-root-r1", "ts-src-r1", srcBodyV1),
      },
      {
        kind: "claim",
        body: "target v1",
        sourceId: "src:ev",
        logicalId: "target",
        properties: { stage: "v1" },
        provenance: derivedProv("ts-target-r1", "ts-src-r1", srcBodyV1),
      },
      {
        edge: {
          kind: "supports",
          from: "root",
          to: "target",
          sourceId: "src:ev",
          logicalId: "edge_rt",
          properties: { score: 1 },
          provenance: derivedProv("ts-edge-r1", "ts-src-r1", srcBodyV1),
        },
      },
      {
        kind: "claim",
        body: "erased target v1",
        sourceId: "src:erase_me",
        logicalId: "erased_target",
        provenance: canonicalProv("ts-erased-r1", "sv-1"),
      },
      {
        edge: {
          kind: "supports",
          from: "root",
          to: "erased_target",
          sourceId: "src:erase_me",
          logicalId: "edge_erased",
          provenance: derivedProv("ts-edge-erased-r1", "ts-erased-r1", "erased target v1"),
        },
      },
    ]);
    const c1 = String(r1.cursor);

    await engine.registerSourceDependency({
      schemaVersion: 1,
      dependencyId: "ts-dep-target",
      sourceRevisionId: "ts-src-r1",
      derivedRevisionId: "ts-target-r1",
    });
    await engine.registerSourceDependency({
      schemaVersion: 1,
      dependencyId: "ts-dep-edge",
      sourceRevisionId: "ts-src-r1",
      derivedRevisionId: "ts-edge-r1",
    });

    await engine.write([
      {
        kind: "document",
        body: srcBodyV2,
        sourceId: "src:ev",
        logicalId: "src_doc_b",
        provenance: canonicalProv("ts-src-b-r1", "sv-b1"),
      },
      {
        kind: "claim",
        body: "target v2",
        sourceId: "src:ev",
        logicalId: "target",
        properties: { stage: "v2" },
        provenance: derivedProv("ts-target-r2", "ts-src-b-r1", srcBodyV2, "sv-b1"),
      },
      {
        edge: {
          kind: "supports",
          from: "root",
          to: "target",
          sourceId: "src:ev",
          logicalId: "edge_rt",
          properties: { score: 2 },
          provenance: derivedProv("ts-edge-r2", "ts-src-b-r1", srcBodyV2, "sv-b1"),
        },
      },
    ]);
    await engine.write([
      {
        kind: "document",
        body: srcBodyV2,
        sourceId: "src:ev",
        logicalId: "src_doc",
        provenance: canonicalProv("ts-src-r2", "sv-2"),
      },
    ]);
    await engine.drain(30_000);

    const frozen = await engine.freezeReadContext({ schemaVersion: 1, view: {}, eligibility: {} });
    const res = await graph.expand(
      engine,
      baseRequest({
        edgeKinds: ["supports"],
        targetKinds: ["claim"],
        context: { schemaVersion: 1, type: "frozen", context: frozen },
        includeExplanation: true,
        includeEvidence: true,
        asOfWriteCursor: c1,
      }),
    );

    const targetIdx = res.targets.findIndex((t) => t.logicalId === "target");
    const erasedIdx = res.targets.findIndex((t) => t.logicalId === "erased_target");
    assert.ok(targetIdx >= 0 && erasedIdx >= 0);
    assert.equal(res.targets[targetIdx]!.body, "target v1");
    assert.equal(res.explanation!.perTarget[targetIdx]!.dependencyState, "registered");

    const targetEntry = res.evidence!.entries[targetIdx]!;
    const erasedEntry = res.evidence!.entries[erasedIdx]!;
    assert.equal(targetEntry.targetEvidenceRef.length, 704);
    assert.ok(targetEntry.targetEvidenceRef.startsWith("fdbgev1."));

    const resolvedTarget = await engine.resolveGraphEvidence({
      schemaVersion: 1,
      evidenceRef: targetEntry.targetEvidenceRef,
      context: frozen,
    });
    assert.equal(resolvedTarget.artifact.artifactClass, "node");
    assert.equal(resolvedTarget.artifact.body, "target v1");
    assert.deepEqual(resolvedTarget.artifact.properties, { stage: "v1" });
    assert.equal(resolvedTarget.canonicalSourceBody, srcBodyV1);
    assert.equal(resolvedTarget.artifactLifecycle.superseded, true);

    const resolvedEdge = await engine.resolveGraphEvidence({
      schemaVersion: 1,
      evidenceRef: targetEntry.terminalEdgeEvidenceRef,
      context: frozen,
    });
    assert.equal(resolvedEdge.artifact.artifactClass, "edge");
    assert.deepEqual(resolvedEdge.artifact.properties, { score: 1 });
    assert.equal(resolvedEdge.artifactLifecycle.superseded, true);

    // Transition active head of target -> deleted revokes historical replay & evidence
    await engine.transition("target", "deleted");
    const afterDelete = await graph.expand(
      engine,
      baseRequest({
        edgeKinds: ["supports"],
        targetKinds: ["claim"],
        asOfWriteCursor: c1,
      }),
    );
    assert.deepEqual(
      afterDelete.targets.map((t) => t.logicalId),
      ["erased_target"],
    );
    const remintedDel = await engine.freezeReadContext({
      schemaVersion: 1,
      view: {},
      eligibility: {},
    });
    for (const ctx of [frozen, remintedDel]) {
      await assert.rejects(
        engine.resolveGraphEvidence({
          schemaVersion: 1,
          evidenceRef: targetEntry.targetEvidenceRef,
          context: ctx,
        }),
        (err: unknown) => err instanceof EvidenceError && err.reason === "evidence_unavailable",
      );
    }

    // Physical eraseSource revokes historical replay & evidence
    await engine.eraseSource("src:erase_me");
    const afterErase = await graph.expand(
      engine,
      baseRequest({
        edgeKinds: ["supports"],
        targetKinds: ["claim"],
        asOfWriteCursor: c1,
      }),
    );
    assert.deepEqual(afterErase.targets, []);
    const remintedErase = await engine.freezeReadContext({
      schemaVersion: 1,
      view: {},
      eligibility: {},
    });
    for (const ctx of [frozen, remintedErase]) {
      await assert.rejects(
        engine.resolveGraphEvidence({
          schemaVersion: 1,
          evidenceRef: erasedEntry.targetEvidenceRef,
          context: ctx,
        }),
        (err: unknown) => err instanceof EvidenceError && err.reason === "evidence_unavailable",
      );
    }
  } finally {
    await engine.close();
  }
});

test("slice38 historical graph validation rejects mutual exclusion, null, and non-safe integers", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    await engine.write([{ kind: "doc", body: "seed", sourceId: "src:val", logicalId: "root" }]);

    // Mutual exclusion -> GraphContextInvalid at /asOfEpochMs
    await assert.rejects(
      graph.expand(engine, baseRequest({ asOfWriteCursor: "1", asOfEpochMs: 1_700_000_000_000 })),
      (err: unknown) =>
        err instanceof GraphExpansionError &&
        err.reason === "graph_context_invalid" &&
        err.fieldPath === "/asOfEpochMs",
    );

    // Invalid asOfWriteCursor (including null, number, negative, leading zero, overflow)
    for (const badCursor of [
      null,
      123,
      true,
      "",
      "01",
      "-1",
      "1\n",
      U64_OVERFLOW_STR,
    ] as unknown[]) {
      await assert.rejects(
        graph.expand(engine, baseRequest({ asOfWriteCursor: badCursor as string })),
        (err: unknown) =>
          err instanceof GraphExpansionError &&
          err.reason === "graph_context_invalid" &&
          err.fieldPath === "/asOfWriteCursor",
      );
    }

    // Invalid asOfEpochMs (including null, float, NaN, Infinity, > MAX_SAFE_INTEGER, string)
    for (const badEpoch of [
      null,
      1.5,
      Number.NaN,
      Number.POSITIVE_INFINITY,
      Number.NEGATIVE_INFINITY,
      Number.MAX_SAFE_INTEGER + 1,
      Number.MIN_SAFE_INTEGER - 1,
      "100",
      true,
    ] as unknown[]) {
      await assert.rejects(
        graph.expand(engine, baseRequest({ asOfEpochMs: badEpoch as number })),
        (err: unknown) =>
          err instanceof GraphExpansionError &&
          err.reason === "graph_context_invalid" &&
          err.fieldPath === "/asOfEpochMs",
      );
    }

    // ReadView.includeSuperseded / includeInactive remain refused
    for (const view of [{ includeSuperseded: true }, { includeInactive: true }]) {
      await assert.rejects(
        graph.expand(
          engine,
          baseRequest({
            context: {
              schemaVersion: 1,
              type: "current",
              context: { schemaVersion: 1, view, eligibility: {} },
            },
            asOfWriteCursor: "1",
          }),
        ),
        (err: unknown) =>
          err instanceof GraphExpansionError &&
          err.reason === "graph_context_invalid" &&
          err.fieldPath === "/context",
      );
    }
  } finally {
    await engine.close();
  }
});
