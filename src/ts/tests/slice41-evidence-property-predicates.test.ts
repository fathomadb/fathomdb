// 0.8.32 Slice 41 (`WP-41C`, `#269` Option A) — TypeScript cross-binding parity tests.

import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import test from "node:test";

import {
  Engine,
  ScopePolicy,
  graph,
  type PropertyMap,
  type PropertyPredicate,
  type SearchFilter,
} from "../src/index.js";
import { DependencyTraceError, EvidenceError } from "../src/errors.js";
import { freshDbPath } from "./helpers.js";

function sha256Hex(body: string): string {
  return createHash("sha256").update(body).digest("hex");
}

async function configureAclAndConfidence(engine: Engine): Promise<void> {
  await engine.configureProjections([
    { name: "acl", roles: ["filterable"], fts: false, vector: false },
    { name: "confidence", roles: ["filterable"], fts: false, vector: false },
  ]);
}

function canonicalNode(args: {
  logicalId: string;
  kind: string;
  revisionId: string;
  sourceVersionId: string;
  sourceId: string;
  body: string;
  properties: PropertyMap;
}) {
  return {
    kind: args.kind,
    body: args.body,
    sourceId: args.sourceId,
    logicalId: args.logicalId,
    properties: args.properties,
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
  properties: PropertyMap;
}) {
  return {
    kind: args.kind,
    body: args.body,
    sourceId: args.sourceId,
    logicalId: args.logicalId,
    properties: args.properties,
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

test("AC32-EVIDENCE-PROP-01: artifact-scoped propertyPredicates across search, resolve, resolveMany, and graph evidence", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    await configureAclAndConfidence(engine);
    const sourceBody = "canonical team-a raw source without confidence property";
    await engine.write([
      canonicalNode({
        logicalId: "src-team-a",
        kind: "SourceDoc",
        revisionId: "src-team-a-r1",
        sourceVersionId: "src-team-a-v1",
        sourceId: "team-a-bucket",
        body: sourceBody,
        properties: { acl: "team-a" },
      }),
      derivedNode({
        logicalId: "doc-root",
        kind: "Doc",
        revisionId: "doc-root-r1",
        sourceVersionId: "src-team-a-v1",
        sourceRevisionId: "src-team-a-r1",
        sourceId: "team-a-bucket",
        sourceBody,
        body: "root anchor document",
        properties: { acl: "team-a", confidence: 0.99 },
      }),
      derivedNode({
        logicalId: "doc-low",
        kind: "Doc",
        revisionId: "doc-low-r1",
        sourceVersionId: "src-team-a-v1",
        sourceRevisionId: "src-team-a-r1",
        sourceId: "team-a-bucket",
        sourceBody,
        body: "tsheliosneedle",
        properties: { acl: "team-a", confidence: 0.25 },
      }),
      derivedNode({
        logicalId: "doc-high",
        kind: "Doc",
        revisionId: "doc-high-r1",
        sourceVersionId: "src-team-a-v1",
        sourceRevisionId: "src-team-a-r1",
        sourceId: "team-a-bucket",
        sourceBody,
        body: "tsheliosneedle high confidence extracted claim with extra context words",
        properties: { acl: "team-a", confidence: 0.95 },
      }),
      derivedEdge({
        logicalId: "edge-high",
        kind: "supports",
        from: "doc-root",
        to: "doc-high",
        revisionId: "edge-high-r1",
        sourceVersionId: "src-team-a-v1",
        sourceRevisionId: "src-team-a-r1",
        sourceId: "team-a-bucket",
        sourceBody,
      }),
      derivedEdge({
        logicalId: "edge-low",
        kind: "weakly_supports",
        from: "doc-root",
        to: "doc-low",
        revisionId: "edge-low-r1",
        sourceVersionId: "src-team-a-v1",
        sourceRevisionId: "src-team-a-r1",
        sourceId: "team-a-bucket",
        sourceBody,
      }),
    ]);
    await engine.drain(10_000);

    const predicate: PropertyPredicate = { key: "confidence", op: "gte", value: 0.8 };
    const eligibility: SearchFilter = {
      kind: "Doc",
      attributes: [["acl", "team-a"]],
      propertyPredicates: [predicate],
    };
    const frozen = await engine.freezeReadContext({
      schemaVersion: 1,
      view: {},
      eligibility,
    });

    // 1. searchWithEvidence (limit=1 proves low-confidence doc excluded before truncation)
    const searchRes = await engine.searchWithEvidence({
      schemaVersion: 1,
      query: "tsheliosneedle",
      context: frozen,
      limit: 1,
    });
    assert.equal(searchRes.searchResult.results.length, 1);
    assert.equal(searchRes.searchResult.results[0]?.id.value, "doc-high");
    assert.equal(searchRes.evidence.length, 1);
    assert.equal(searchRes.evidence[0]?.artifactRevisionId, "doc-high-r1");

    // 2. resolveEvidence
    const resolved = await engine.resolveEvidence({
      schemaVersion: 1,
      evidenceRef: searchRes.evidence[0]!.evidenceRef,
      context: frozen,
    });
    assert.equal(resolved.artifactRevisionId, "doc-high-r1");
    assert.equal(resolved.sourceRevisionId, "src-team-a-r1");
    assert.equal(resolved.canonicalSourceBody, sourceBody);

    // 3. resolveEvidenceMany
    const many = await engine.resolveEvidenceMany({
      schemaVersion: 1,
      evidenceRefs: [searchRes.evidence[0]!.evidenceRef],
      context: frozen,
    });
    assert.equal(many.length, 1);
    assert.equal(many[0]?.status, "resolved");
    if (many[0]?.status === "resolved") {
      assert.equal(many[0].resolved.sourceRevisionId, "src-team-a-r1");
      assert.equal(many[0].resolved.canonicalSourceBody, sourceBody);
    }

    // 4. graph.expand(includeEvidence=true) + resolveGraphEvidence
    const expanded = await graph.expand(engine, {
      schemaVersion: 1,
      seed: {
        schemaVersion: 1,
        type: "explicit",
        logicalIds: [{ space: "logical", value: "doc-root" }],
      },
      direction: "outgoing",
      edgeKinds: ["supports", "weakly_supports"],
      targetKinds: [],
      context: {
        schemaVersion: 1,
        type: "frozen",
        context: frozen,
      },
      maxDepth: 1,
      resultLimit: 10,
      maxWorkUnits: "10000",
      includeExplanation: false,
      includeEvidence: true,
    });
    assert.deepEqual(
      expanded.targets.map((target) => target.logicalId),
      ["doc-high"],
    );
    assert.ok(expanded.evidence !== null && expanded.evidence !== undefined);
    const entry = expanded.evidence.entries[0]!;

    const resolvedTarget = await engine.resolveGraphEvidence({
      schemaVersion: 1,
      evidenceRef: entry.targetEvidenceRef,
      context: frozen,
    });
    assert.equal(resolvedTarget.artifactRevisionId, "doc-high-r1");
    assert.equal(resolvedTarget.sourceRevisionId, "src-team-a-r1");
    assert.equal(resolvedTarget.canonicalSourceBody, sourceBody);

    const resolvedEdge = await engine.resolveGraphEvidence({
      schemaVersion: 1,
      evidenceRef: entry.terminalEdgeEvidenceRef,
      context: frozen,
    });
    assert.equal(resolvedEdge.artifactRevisionId, "edge-high-r1");
    assert.equal(resolvedEdge.sourceRevisionId, "src-team-a-r1");
  } finally {
    await engine.close();
  }
});

test("AC32-EVIDENCE-PROP-02: source attributes and ScopePolicy remain fail-closed", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    await configureAclAndConfidence(engine);
    const srcABody = "team-a canonical source";
    const srcBBody = "team-b restricted canonical source";
    await engine.write([
      canonicalNode({
        logicalId: "src-a",
        kind: "SourceDoc",
        revisionId: "src-a-r1",
        sourceVersionId: "src-a-v1",
        sourceId: "bucket-a",
        body: srcABody,
        properties: { acl: "team-a" },
      }),
      canonicalNode({
        logicalId: "src-b",
        kind: "SourceDoc",
        revisionId: "src-b-r1",
        sourceVersionId: "src-b-v1",
        sourceId: "bucket-b",
        body: srcBBody,
        properties: { acl: "team-b" },
      }),
      derivedNode({
        logicalId: "doc-root",
        kind: "Doc",
        revisionId: "doc-root-r1",
        sourceVersionId: "src-a-v1",
        sourceRevisionId: "src-a-r1",
        sourceId: "bucket-a",
        sourceBody: srcABody,
        body: "root anchor",
        properties: { acl: "team-a", confidence: 0.99 },
      }),
      derivedNode({
        logicalId: "doc-from-b",
        kind: "Doc",
        revisionId: "doc-from-b-r1",
        sourceVersionId: "src-b-v1",
        sourceRevisionId: "src-b-r1",
        sourceId: "bucket-b",
        sourceBody: srcBBody,
        body: "tsrestrictedneedle",
        properties: { acl: "team-a", confidence: 0.95 },
      }),
      derivedEdge({
        logicalId: "edge-to-b",
        kind: "supports",
        from: "doc-root",
        to: "doc-from-b",
        revisionId: "edge-to-b-r1",
        sourceVersionId: "src-a-v1",
        sourceRevisionId: "src-a-r1",
        sourceId: "bucket-a",
        sourceBody: srcABody,
      }),
    ]);
    await engine.drain(10_000);

    const frozen = await engine.freezeReadContext({
      schemaVersion: 1,
      view: {},
      eligibility: {
        kind: "Doc",
        attributes: [["acl", "team-a"]],
        propertyPredicates: [{ key: "confidence", op: "gte", value: 0.8 }],
      },
    });

    await assert.rejects(
      () =>
        engine.searchWithEvidence({
          schemaVersion: 1,
          query: "tsrestrictedneedle",
          context: frozen,
        }),
      (err: unknown) =>
        err instanceof EvidenceError &&
        err.reason === "evidence_unavailable" &&
        err.fieldPath === "/evidenceRef",
    );

    await assert.rejects(
      () =>
        graph.expand(engine, {
          schemaVersion: 1,
          seed: {
            schemaVersion: 1,
            type: "explicit",
            logicalIds: [{ space: "logical", value: "doc-root" }],
          },
          direction: "outgoing",
          edgeKinds: ["supports"],
          targetKinds: [],
          context: { schemaVersion: 1, type: "frozen", context: frozen },
          maxDepth: 1,
          resultLimit: 10,
          maxWorkUnits: "10000",
          includeExplanation: false,
          includeEvidence: true,
        }),
      (err: unknown) =>
        err instanceof EvidenceError &&
        err.reason === "evidence_unavailable" &&
        err.fieldPath === "/evidence",
    );
  } finally {
    await engine.close();
  }

  // Schema 39 ScopePolicy cross-scope isolation under Option A
  const scopedPath = freshDbPath();
  let refA = "";
  const engA = await Engine.open(scopedPath, {
    useDefaultEmbedder: false,
    scopePolicy: ScopePolicy.single("tenant-a"),
  });
  try {
    await configureAclAndConfidence(engA);
    const tenantABody = "tenant-a canonical source bytes";
    await engA.write([
      canonicalNode({
        logicalId: "src-ta",
        kind: "SourceDoc",
        revisionId: "src-ta-r1",
        sourceVersionId: "src-ta-v1",
        sourceId: "bucket-ta",
        body: tenantABody,
        properties: { acl: "team-a" },
      }),
      derivedNode({
        logicalId: "doc-ta",
        kind: "Doc",
        revisionId: "doc-ta-r1",
        sourceVersionId: "src-ta-v1",
        sourceRevisionId: "src-ta-r1",
        sourceId: "bucket-ta",
        sourceBody: tenantABody,
        body: "tsscopedoptionaneedle",
        properties: { acl: "team-a", confidence: 0.95 },
      }),
    ]);
    await engA.drain(10_000);
    const frozenA = await engA.freezeReadContext({
      schemaVersion: 1,
      view: {},
      eligibility: {
        kind: "Doc",
        attributes: [["acl", "team-a"]],
        propertyPredicates: [{ key: "confidence", op: "gte", value: 0.8 }],
      },
    });
    const resA = await engA.searchWithEvidence({
      schemaVersion: 1,
      query: "tsscopedoptionaneedle",
      context: frozenA,
    });
    refA = resA.evidence[0]!.evidenceRef;
    const resolvedA = await engA.resolveEvidence({
      schemaVersion: 1,
      evidenceRef: refA,
      context: frozenA,
    });
    assert.equal(resolvedA.canonicalSourceBody, tenantABody);
  } finally {
    await engA.close();
  }

  const engB = await Engine.open(scopedPath, {
    useDefaultEmbedder: false,
    scopePolicy: ScopePolicy.single("tenant-b"),
  });
  try {
    const frozenB = await engB.freezeReadContext({
      schemaVersion: 1,
      view: {},
      eligibility: {
        kind: "Doc",
        attributes: [["acl", "team-a"]],
        propertyPredicates: [{ key: "confidence", op: "gte", value: 0.8 }],
      },
    });
    const resB = await engB.searchWithEvidence({
      schemaVersion: 1,
      query: "tsscopedoptionaneedle",
      context: frozenB,
    });
    assert.equal(resB.searchResult.results.length, 0);
    await assert.rejects(
      () =>
        engB.resolveEvidence({
          schemaVersion: 1,
          evidenceRef: refA,
          context: frozenB,
        }),
      (err: unknown) =>
        err instanceof EvidenceError &&
        err.reason === "evidence_unavailable" &&
        err.fieldPath === "/evidenceRef",
    );
  } finally {
    await engB.close();
  }
});

test("AC32-EVIDENCE-PROP-03: traceDependency symmetry and GraphArm SearchHit.sourceId vs ResolvedEvidenceV1.sourceId", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    await configureAclAndConfidence(engine);
    const nodeSrcBody = "canonical node provenance without confidence";
    const edgeSrcBody = "canonical edge provenance without confidence";
    await engine.write([
      canonicalNode({
        logicalId: "node-src",
        kind: "SourceDoc",
        revisionId: "node-src-r1",
        sourceVersionId: "node-src-v1",
        sourceId: "node-source-bucket",
        body: nodeSrcBody,
        properties: { acl: "team-a" },
      }),
      canonicalNode({
        logicalId: "edge-src",
        kind: "SourceDoc",
        revisionId: "edge-src-r1",
        sourceVersionId: "edge-src-v1",
        sourceId: "edge-source-bucket",
        body: edgeSrcBody,
        properties: { acl: "team-a" },
      }),
      derivedNode({
        logicalId: "ent-seed",
        kind: "Doc",
        revisionId: "ent-seed-r1",
        sourceVersionId: "node-src-v1",
        sourceRevisionId: "node-src-r1",
        sourceId: "node-source-bucket",
        sourceBody: nodeSrcBody,
        body: "tsquasarseedtoken",
        properties: { acl: "team-a", confidence: 0.95 },
      }),
      derivedNode({
        logicalId: "ent-neighbor",
        kind: "Doc",
        revisionId: "ent-neighbor-r1",
        sourceVersionId: "node-src-v1",
        sourceRevisionId: "node-src-r1",
        sourceId: "node-source-bucket",
        sourceBody: nodeSrcBody,
        body: "unrelated neighbor text",
        properties: { acl: "team-a", confidence: 0.92 },
      }),
      derivedEdge({
        logicalId: "ent-link",
        kind: "links",
        from: "ent-seed",
        to: "ent-neighbor",
        revisionId: "ent-link-r1",
        sourceVersionId: "edge-src-v1",
        sourceRevisionId: "edge-src-r1",
        sourceId: "edge-source-bucket",
        sourceBody: edgeSrcBody,
      }),
    ]);
    await engine.registerSourceDependency({
      schemaVersion: 1,
      dependencyId: "dep-asym",
      sourceRevisionId: "node-src-r1",
      derivedRevisionId: "ent-seed-r1",
    });
    await engine.drain(10_000);

    // traceDependency preserves symmetric endpoint enforcement
    const traceFrozen = await engine.freezeReadContext({
      schemaVersion: 1,
      view: {},
      eligibility: {
        attributes: [["acl", "team-a"]],
        propertyPredicates: [{ key: "confidence", op: "gte", value: 0.8 }],
      },
    });
    const traceRes = await engine.traceDependency({
      schemaVersion: 1,
      rootRevisionId: "ent-seed-r1",
      direction: "to_source",
      context: traceFrozen,
    });
    assert.deepEqual(
      traceRes.nodes.map((node) => node.artifactRevisionId),
      ["ent-seed-r1"],
    );
    assert.equal(traceRes.dependencyEdges.length, 0);

    await assert.rejects(
      () =>
        engine.traceDependency({
          schemaVersion: 1,
          rootRevisionId: "node-src-r1",
          direction: "to_dependents",
          context: traceFrozen,
        }),
      (err: unknown) => err instanceof DependencyTraceError && err.reason === "trace_unavailable",
    );

    // GraphArm SearchHit.sourceId vs ResolvedEvidenceV1.sourceId
    const graphArmFrozen = await engine.freezeReadContext({
      schemaVersion: 1,
      view: {},
      eligibility: {
        kind: "Doc",
        attributes: [["acl", "team-a"]],
        propertyPredicates: [{ key: "confidence", op: "gte", value: 0.8 }],
      },
    });
    const gaRes = await engine.searchWithEvidence({
      schemaVersion: 1,
      query: "tsquasarseedtoken",
      context: graphArmFrozen,
      useGraphArm: true,
    });
    const neighborIdx = gaRes.searchResult.results.findIndex(
      (hit) => hit.id.value === "ent-neighbor" && hit.branch === "graph_arm",
    );
    assert.notEqual(neighborIdx, -1);
    const neighborHit = gaRes.searchResult.results[neighborIdx]!;
    assert.equal(neighborHit.sourceId, "edge-source-bucket");

    const resolvedNeighbor = await engine.resolveEvidence({
      schemaVersion: 1,
      evidenceRef: gaRes.evidence[neighborIdx]!.evidenceRef,
      context: graphArmFrozen,
    });
    assert.equal(resolvedNeighbor.artifactRevisionId, "ent-neighbor-r1");
    assert.equal(resolvedNeighbor.sourceId, "node-source-bucket");
    assert.notEqual(resolvedNeighbor.sourceId, neighborHit.sourceId);
    assert.equal(resolvedNeighbor.canonicalSourceBody, nodeSrcBody);
  } finally {
    await engine.close();
  }
});
