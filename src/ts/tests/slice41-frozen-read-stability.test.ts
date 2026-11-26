// 0.8.32 Slice 41 (`WP-41D`, `ADR-0.8.32 #103`) — TypeScript cross-binding tests.
//
// Covers `AC32-FROZEN-STAB-01..03`, `AC32-FROZEN-STAB-05`, and `AC32-FROZEN-STAB-06`:
// - Disjoint-kind additive `engine.write` and `engine.actuate` produce zero drift on
//   frozen `kind="Doc"` search, evidence, pagination, and graph expansion contexts.
// - `AppendOnlyLog` (`operational_mutations`) appends produce zero drift across all
//   surfaces including `read.operationalState`; `LatestState` (`operational_state`)
//   upserts produce zero drift on canonical/search/evidence/graph (including
//   unfiltered `eligibility: {}`) while drifting `read.operationalState`.
// - Cross-scope `tenant-b` additive writes, `transition`, `purge`, and `eraseSource`
//   produce zero drift on `tenant-a` `FrozenReadContextV1` across close/reopen.
// - Fail-closed checks: same-`(scope, kind)` write, same-scope edge write on graph
//   surfaces, and destructive mutations on any kind in `scope` fail closed.
// - Schema 39 `fdbfr1` token lengths (`492 B` unscoped / `556 B` scoped, `<= 1024 B`).

import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import test from "node:test";

import { Engine, ScopePolicy, graph, read } from "../src/index.js";
import { EvidenceError, FrozenReadError } from "../src/errors.js";
import { freshDbPath } from "./helpers.js";

function sha256Hex(body: string): string {
  return createHash("sha256").update(body).digest("hex");
}

function canonicalNode(args: {
  logicalId: string;
  kind: string;
  revisionId: string;
  sourceVersionId: string;
  sourceId: string;
  body: string;
}) {
  return {
    kind: args.kind,
    body: args.body,
    sourceId: args.sourceId,
    logicalId: args.logicalId,
    provenance: {
      schemaVersion: 1 as const,
      role: "canonical" as const,
      artifactRevisionId: args.revisionId,
      sourceVersionId: args.sourceVersionId,
    },
  };
}

function derivedNode(args: {
  logicalId: string;
  kind: string;
  revisionId: string;
  sourceVersionId: string;
  sourceRevisionId: string;
  sourceId: string;
  sourceBody: string;
  body: string;
}) {
  return {
    kind: args.kind,
    body: args.body,
    sourceId: args.sourceId,
    logicalId: args.logicalId,
    provenance: {
      schemaVersion: 1 as const,
      role: "derived" as const,
      artifactRevisionId: args.revisionId,
      sourceVersionId: args.sourceVersionId,
      sourceRevisionId: args.sourceRevisionId,
      sourceLocator: { kind: "whole_body" as const },
      canonicalSourceHash: {
        algorithm: "sha256" as const,
        digestHex: sha256Hex(args.sourceBody),
      },
    },
  };
}

function derivedEdge(args: {
  logicalId: string;
  kind: string;
  from: string;
  to: string;
  revisionId: string;
  sourceVersionId: string;
  sourceRevisionId: string;
  sourceId: string;
  sourceBody: string;
}) {
  return {
    edge: {
      from: args.from,
      to: args.to,
      kind: args.kind,
      sourceId: args.sourceId,
      logicalId: args.logicalId,
      confidence: 0.95,
      provenance: {
        schemaVersion: 1 as const,
        role: "derived" as const,
        artifactRevisionId: args.revisionId,
        sourceVersionId: args.sourceVersionId,
        sourceRevisionId: args.sourceRevisionId,
        sourceLocator: { kind: "whole_body" as const },
        canonicalSourceHash: {
          algorithm: "sha256" as const,
          digestHex: sha256Hex(args.sourceBody),
        },
      },
    },
  };
}

async function registerCollection(
  engine: Engine,
  name: string,
  kind: "append_only_log" | "latest_state",
): Promise<void> {
  await engine.write([
    {
      adminSchema: {
        name,
        kind,
        schemaJson: "{}",
        retentionJson: "{}",
      },
    },
  ]);
}

async function seedDocCorpus(engine: Engine, prefix: string): Promise<string> {
  const sourceBody = `${prefix} canonical aurora telemetry source`;
  await engine.write([
    canonicalNode({
      logicalId: `${prefix}-src-1`,
      kind: "SourceDoc",
      revisionId: `${prefix}-src-r1`,
      sourceVersionId: `${prefix}-src-v1`,
      sourceId: `${prefix}-source`,
      body: sourceBody,
    }),
    derivedNode({
      logicalId: `${prefix}-doc-1`,
      kind: "Doc",
      revisionId: `${prefix}-doc-r1`,
      sourceVersionId: `${prefix}-src-v1`,
      sourceRevisionId: `${prefix}-src-r1`,
      sourceId: `${prefix}-source`,
      sourceBody,
      body: `${prefix} aurora doc primary needle`,
    }),
    derivedNode({
      logicalId: `${prefix}-doc-2`,
      kind: "Doc",
      revisionId: `${prefix}-doc-r2`,
      sourceVersionId: `${prefix}-src-v1`,
      sourceRevisionId: `${prefix}-src-r1`,
      sourceId: `${prefix}-source`,
      sourceBody,
      body: `${prefix} aurora doc linked target`,
    }),
    derivedEdge({
      logicalId: `${prefix}-edge-1`,
      kind: "supports",
      from: `${prefix}-doc-1`,
      to: `${prefix}-doc-2`,
      revisionId: `${prefix}-edge-r1`,
      sourceVersionId: `${prefix}-src-v1`,
      sourceRevisionId: `${prefix}-src-r1`,
      sourceId: `${prefix}-source`,
      sourceBody,
    }),
  ]);
  return sourceBody;
}

test("AC32-FROZEN-STAB-01: disjoint-kind write and actuate produce zero drift", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    await seedDocCorpus(engine, "ts1");
    const ctx = await engine.freezeReadContext({
      schemaVersion: 1,
      view: {},
      eligibility: { kind: "Doc" },
    });
    // AC32-FROZEN-STAB-06: Schema 39 unscoped token length is 492 B (<= 1,024 B)
    assert.equal(ctx.token.length, 492);
    assert.ok(ctx.token.length <= 1024);

    // Disjoint-kind additive write
    await engine.write([
      canonicalNode({
        logicalId: "ts1-turn-w1",
        kind: "AppTurn",
        revisionId: "ts1-turn-w1-r1",
        sourceVersionId: "ts1-turn-w1-v1",
        sourceId: "ts1-turns",
        body: "ts1 disjoint write turn",
      }),
    ]);

    // Disjoint-kind additive actuate
    const receipt = await engine.actuate({
      schemaVersion: 1,
      operationId: "ts1-act-turn-1",
      operations: [
        {
          type: "put_canonical_node",
          record: canonicalNode({
            logicalId: "ts1-turn-a1",
            kind: "AppTurn",
            revisionId: "ts1-turn-a1-r1",
            sourceVersionId: "ts1-turn-a1-v1",
            sourceId: "ts1-turns",
            body: "ts1 disjoint actuated turn",
          }),
        },
      ],
    });
    assert.equal(receipt.outcome, "committed");

    const searchRes = await engine.searchFrozen("aurora", ctx);
    assert.ok(searchRes.results.length >= 1);

    const evRes = await engine.searchWithEvidence({
      schemaVersion: 1,
      query: "aurora",
      context: ctx,
    });
    assert.ok(evRes.evidence.length >= 1);

    const single = await engine.resolveEvidence({
      schemaVersion: 1,
      evidenceRef: evRes.evidence[0].evidenceRef,
      context: ctx,
    });
    assert.ok(single.artifactRevisionId.startsWith("ts1-doc-r"));

    const many = await engine.resolveEvidenceMany({
      schemaVersion: 1,
      evidenceRefs: evRes.evidence.map((e) => e.evidenceRef),
      context: ctx,
    });
    assert.ok(many.every((item) => item.status === "resolved"));

    const page = await read.canonicalPage(engine, "Doc", ctx, {
      schemaVersion: 1,
      limit: 10,
    });
    assert.equal(page.items.length, 2);

    const gRes = await graph.expand(engine, {
      schemaVersion: 1,
      seed: {
        schemaVersion: 1,
        type: "explicit",
        logicalIds: [{ space: "logical", value: "ts1-doc-1" }],
      },
      direction: "outgoing",
      edgeKinds: ["supports"],
      targetKinds: [],
      context: { schemaVersion: 1, type: "frozen", context: ctx },
      maxDepth: 1,
      resultLimit: 10,
      maxWorkUnits: "10000",
      includeExplanation: false,
      includeEvidence: true,
    });
    assert.ok(gRes.targets.length >= 1);
    assert.ok(gRes.evidence !== undefined);
    const gEv = await engine.resolveGraphEvidence({
      schemaVersion: 1,
      evidenceRef: gRes.evidence.entries[0].targetEvidenceRef,
      context: ctx,
    });
    assert.equal(gEv.artifactRevisionId, "ts1-doc-r2");
  } finally {
    await engine.close();
  }
});

test("AC32-FROZEN-STAB-02: AppendOnlyLog and LatestState isolation", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    await seedDocCorpus(engine, "ts2");
    await registerCollection(engine, "audit_log", "append_only_log");
    await registerCollection(engine, "checkpoints", "latest_state");
    await engine.write([
      {
        opStore: {
          collection: "checkpoints",
          recordKey: "cursor-1",
          body: '{"step":1}',
        },
      },
    ]);

    const ctxDoc = await engine.freezeReadContext({
      schemaVersion: 1,
      view: {},
      eligibility: { kind: "Doc" },
    });
    const ctxAll = await engine.freezeReadContext({
      schemaVersion: 1,
      view: {},
      eligibility: {},
    });

    // AppendOnlyLog appends produce zero drift on all surfaces including operationalState
    for (let i = 0; i < 10; i += 1) {
      await engine.write([
        {
          opStore: {
            collection: "audit_log",
            recordKey: `evt-${i}`,
            body: JSON.stringify({ seq: i }),
          },
        },
      ]);
    }
    assert.ok((await engine.searchFrozen("aurora", ctxDoc)).results.length >= 1);
    assert.ok((await engine.searchFrozen("aurora", ctxAll)).results.length >= 1);
    const opBefore = await read.operationalState(engine, "checkpoints", "cursor-1", ctxAll);
    assert.ok(opBefore !== null);
    assert.equal(opBefore.payload, '{"step":1}');

    // LatestState upserts (write + actuate PutLatestState): zero drift on canonical/search/evidence/graph,
    // while drifting ReadSurface::OperationalState
    await engine.write([
      {
        opStore: {
          collection: "checkpoints",
          recordKey: "cursor-1",
          body: '{"step":2}',
        },
      },
    ]);
    const actReceipt = await engine.actuate({
      schemaVersion: 1,
      operationId: "ts2-ls-act-1",
      operations: [
        {
          type: "put_latest_state",
          collection: "checkpoints",
          recordKey: "cursor-1",
          payloadJson: '{"step":3}',
        },
      ],
    });
    assert.equal(actReceipt.outcome, "committed");

    assert.ok((await engine.searchFrozen("aurora", ctxDoc)).results.length >= 1);
    assert.ok((await engine.searchFrozen("aurora", ctxAll)).results.length >= 1);
    const evAll = await engine.searchWithEvidence({
      schemaVersion: 1,
      query: "aurora",
      context: ctxAll,
    });
    assert.ok(evAll.evidence.length >= 1);
    assert.equal(
      (await read.canonicalPage(engine, "Doc", ctxDoc, { schemaVersion: 1, limit: 10 })).items.length,
      2,
    );

    await assert.rejects(
      () => read.operationalState(engine, "checkpoints", "cursor-1", ctxAll),
      (err: unknown) => err instanceof FrozenReadError,
    );
  } finally {
    await engine.close();
  }
});

test("AC32-FROZEN-STAB-03: cross-scope isolation including destructive ops and reopen", async () => {
  const dbPath = freshDbPath();
  const policyA = ScopePolicy.single("tenant-a");
  const policyB = ScopePolicy.single("tenant-b");

  let ctxA: Awaited<ReturnType<Engine["freezeReadContext"]>>;
  let savedRef: string;
  {
    const engA = await Engine.open(dbPath, {
      useDefaultEmbedder: false,
      scopePolicy: policyA,
    });
    try {
      await seedDocCorpus(engA, "ta");
      ctxA = await engA.freezeReadContext({
        schemaVersion: 1,
        view: {},
        eligibility: { kind: "Doc" },
      });
      // AC32-FROZEN-STAB-06: Schema 39 scoped token length is 556 B (<= 1,024 B)
      assert.equal(ctxA.token.length, 556);
      assert.ok(ctxA.token.length <= 1024);
      const ev = await engA.searchWithEvidence({
        schemaVersion: 1,
        query: "aurora",
        context: ctxA,
      });
      assert.ok(ev.evidence.length >= 1);
      savedRef = ev.evidence[0].evidenceRef;
    } finally {
      await engA.close();
    }
  }

  // Mutate tenant-b: additive write, transition, purge, eraseSource
  {
    const engB = await Engine.open(dbPath, {
      useDefaultEmbedder: false,
      scopePolicy: policyB,
    });
    try {
      await seedDocCorpus(engB, "tb");
      await engB.write([
        canonicalNode({
          logicalId: "tb-extra",
          kind: "Doc",
          revisionId: "tb-extra-r1",
          sourceVersionId: "tb-extra-v1",
          sourceId: "tb-extra-src",
          body: "tb extra doc to transition and purge",
        }),
      ]);
      await engB.transition("tb-extra", "deleted", "tenant-b churn");
      await engB.purge("tb-extra");
      await engB.eraseSource("tb-source");
    } finally {
      await engB.close();
    }
  }

  // Reopen tenant-a and verify zero drift
  {
    const engA2 = await Engine.open(dbPath, {
      useDefaultEmbedder: false,
      scopePolicy: policyA,
    });
    try {
      assert.ok((await engA2.searchFrozen("aurora", ctxA)).results.length >= 1);
      const resolved = await engA2.resolveEvidence({
        schemaVersion: 1,
        evidenceRef: savedRef,
        context: ctxA,
      });
      assert.ok(resolved.artifactRevisionId.startsWith("ta-doc-r"));
      const page = await read.canonicalPage(engA2, "Doc", ctxA, {
        schemaVersion: 1,
        limit: 10,
      });
      assert.equal(page.items.length, 2);
    } finally {
      await engA2.close();
    }
  }
});

test("AC32-FROZEN-STAB-05: fail-closed on same-kind write, edge write on graph, and destructive ops", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    const sourceBody = await seedDocCorpus(engine, "ts5");
    const ctxDoc = await engine.freezeReadContext({
      schemaVersion: 1,
      view: {},
      eligibility: { kind: "Doc" },
    });
    const evRes = await engine.searchWithEvidence({
      schemaVersion: 1,
      query: "aurora",
      context: ctxDoc,
    });
    const docRef = evRes.evidence[0].evidenceRef;

    // Disjoint-kind edge in same scope drifts graph surfaces while keeping node-only surfaces stable
    await engine.write([
      derivedEdge({
        logicalId: "ts5-edge-2",
        kind: "cites",
        from: "ts5-doc-1",
        to: "ts5-doc-2",
        revisionId: "ts5-edge-r2",
        sourceVersionId: "ts5-src-v1",
        sourceRevisionId: "ts5-src-r1",
        sourceId: "ts5-source",
        sourceBody,
      }),
    ]);
    assert.ok((await engine.searchFrozen("aurora", ctxDoc)).results.length >= 1);
    assert.ok(
      (
        await engine.resolveEvidence({
          schemaVersion: 1,
          evidenceRef: docRef,
          context: ctxDoc,
        })
      ).artifactRevisionId.startsWith("ts5-doc-r"),
    );
    await assert.rejects(
      () =>
        graph.expand(engine, {
          schemaVersion: 1,
          seed: {
            schemaVersion: 1,
            type: "explicit",
            logicalIds: [{ space: "logical", value: "ts5-doc-1" }],
          },
          direction: "outgoing",
          edgeKinds: ["supports"],
          targetKinds: [],
          context: { schemaVersion: 1, type: "frozen", context: ctxDoc },
          maxDepth: 1,
          resultLimit: 10,
          maxWorkUnits: "10000",
          includeExplanation: false,
          includeEvidence: false,
        }),
      (err: unknown) => err instanceof FrozenReadError,
    );
    await assert.rejects(
      () => engine.searchExpandFrozen("aurora", ctxDoc, 1),
      (err: unknown) => err instanceof FrozenReadError,
    );

    // Same-(scope, kind) write fails closed
    const ctxDoc2 = await engine.freezeReadContext({
      schemaVersion: 1,
      view: {},
      eligibility: { kind: "Doc" },
    });
    const evRes2 = await engine.searchWithEvidence({
      schemaVersion: 1,
      query: "aurora",
      context: ctxDoc2,
    });
    await engine.write([
      canonicalNode({
        logicalId: "ts5-doc-new",
        kind: "Doc",
        revisionId: "ts5-doc-new-r1",
        sourceVersionId: "ts5-doc-new-v1",
        sourceId: "ts5-source",
        body: "ts5 aurora same-kind doc mutation",
      }),
    ]);
    await assert.rejects(
      () => engine.searchFrozen("aurora", ctxDoc2),
      (err: unknown) => err instanceof FrozenReadError,
    );
    await assert.rejects(
      () =>
        engine.resolveEvidence({
          schemaVersion: 1,
          evidenceRef: evRes2.evidence[0].evidenceRef,
          context: ctxDoc2,
        }),
      (err: unknown) => err instanceof EvidenceError,
    );
    await assert.rejects(
      () => read.canonicalPage(engine, "Doc", ctxDoc2, { schemaVersion: 1, limit: 10 }),
      (err: unknown) => err instanceof FrozenReadError,
    );

    // Destructive transition on disjoint kind in SAME scope fails closed
    await engine.write([
      canonicalNode({
        logicalId: "ts5-turn-dest",
        kind: "AppTurn",
        revisionId: "ts5-turn-dest-r1",
        sourceVersionId: "ts5-turn-dest-v1",
        sourceId: "ts5-turns",
        body: "ts5 turn to deprecate",
      }),
    ]);
    const ctxDoc3 = await engine.freezeReadContext({
      schemaVersion: 1,
      view: {},
      eligibility: { kind: "Doc" },
    });
    await engine.transition("ts5-turn-dest", "deleted", "destructive test");
    await assert.rejects(
      () => engine.searchFrozen("aurora", ctxDoc3),
      (err: unknown) => err instanceof FrozenReadError,
    );
  } finally {
    await engine.close();
  }
});
