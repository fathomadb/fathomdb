import { createHash } from "node:crypto";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import assert from "node:assert/strict";
import test from "node:test";

import { Engine, EvidenceError, InvalidArgumentError } from "../src/index.js";

test("search and resolve exact source evidence", async () => {
  const directory = await mkdtemp(join(tmpdir(), "fathomdb-slice50-"));
  try {
    const engine = await Engine.open(join(directory, "evidence.fathom"), {
      useDefaultEmbedder: false,
    });
    const sourceBody = "canonical evidence bytes";
    await engine.write([
      {
        kind: "document",
        body: sourceBody,
        sourceId: "ts-evidence-source",
        logicalId: "ts-source",
        provenance: {
          schemaVersion: 1,
          role: "canonical",
          artifactRevisionId: "ts-source-r1",
          sourceVersionId: "ts-version-r1",
        },
      },
      {
        kind: "fact",
        body: "tsevidenceneedle",
        sourceId: "ts-evidence-source",
        logicalId: "ts-claim",
        provenance: {
          schemaVersion: 1,
          role: "derived",
          artifactRevisionId: "ts-claim-r1",
          sourceVersionId: "ts-version-r1",
          sourceRevisionId: "ts-source-r1",
          sourceLocator: { kind: "whole_body" },
          canonicalSourceHash: {
            algorithm: "sha256",
            digestHex: createHash("sha256").update(sourceBody).digest("hex"),
          },
        },
      },
    ]);
    const context = await engine.freezeReadContext({
      schemaVersion: 1,
      view: {},
      eligibility: {},
    });
    const result = await engine.searchWithEvidence({
      schemaVersion: 1,
      query: "tsevidenceneedle",
      context,
      includeExplanation: false,
    });
    assert.equal(result.searchResult.results[0]?.body, "tsevidenceneedle");
    assert.equal(result.searchResult.explanation, null);
    assert.equal(result.evidence[0]?.artifactRevisionId, "ts-claim-r1");

    const resolved = await engine.resolveEvidence({
      schemaVersion: 1,
      evidenceRef: result.evidence[0]!.evidenceRef,
      context,
    });
    assert.equal(resolved.canonicalSourceBody, sourceBody);
    assert.equal(resolved.sourceRevisionId, "ts-source-r1");
    assert.equal(resolved.projectionOrigin.representativeArm, "text");
    await engine.close();
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("evidence request schema and unknown fields are typed", async () => {
  const directory = await mkdtemp(join(tmpdir(), "fathomdb-slice50-errors-"));
  try {
    const engine = await Engine.open(join(directory, "evidence.fathom"), {
      useDefaultEmbedder: false,
    });
    const context = await engine.freezeReadContext({
      schemaVersion: 1,
      view: {},
      eligibility: {},
    });

    await assert.rejects(
      engine.searchWithEvidence({
        schemaVersion: 2,
        query: "needle",
        context,
      } as never),
      (error: unknown) =>
        error instanceof EvidenceError &&
        error.reason === "unsupported_schema_version" &&
        error.fieldPath === "/schemaVersion",
    );
    await assert.rejects(
      engine.resolveEvidence({
        schemaVersion: 1,
        evidenceRef: "opaque",
        context,
        unexpected: true,
      } as never),
      (error: unknown) =>
        error instanceof EvidenceError &&
        error.reason === "unknown_field" &&
        error.fieldPath === "/unexpected",
    );
    await assert.rejects(
      engine.searchWithEvidence({
        schemaVersion: 1,
        query: "needle",
        context: { ...context, unexpected: true },
      } as never),
      (error: unknown) =>
        error instanceof EvidenceError &&
        error.reason === "unknown_field" &&
        error.fieldPath === "/context/unexpected",
    );
    for (const [nestedContext, expectedPath] of [
      [
        { ...context, context: { ...context.context, unexpected: true } },
        "/context/context/unexpected",
      ],
      [
        {
          ...context,
          context: {
            ...context.context,
            view: { ...context.context.view, unexpected: true },
          },
        },
        "/context/context/view/unexpected",
      ],
      [
        {
          ...context,
          context: {
            ...context.context,
            eligibility: { ...context.context.eligibility, unexpected: true },
          },
        },
        "/context/context/eligibility/unexpected",
      ],
    ] as const) {
      await assert.rejects(
        engine.searchWithEvidence({
          schemaVersion: 1,
          query: "needle",
          context: nestedContext,
        } as never),
        (error: unknown) =>
          error instanceof EvidenceError &&
          error.reason === "unknown_field" &&
          error.fieldPath === expectedPath,
      );
    }
    await assert.rejects(
      engine.searchWithEvidence({
        schemaVersion: 1,
        query: "needle",
        context: {
          ...context,
          context: { ...context.context, schemaVersion: 2 },
        },
      } as never),
      (error: unknown) =>
        error instanceof EvidenceError &&
        error.reason === "evidence_unavailable" &&
        error.fieldPath === "/evidenceRef",
    );
    await assert.rejects(
      engine.resolveEvidence({
        schemaVersion: 1,
        evidenceRef: "opaque",
        context,
        "unexpected~/field": true,
      } as never),
      (error: unknown) =>
        error instanceof EvidenceError &&
        error.reason === "unknown_field" &&
        error.fieldPath === "/unexpected~0~1field",
    );
    await assert.rejects(
      engine.searchWithEvidence({
        schemaVersion: 1,
        query: "needle",
        context: { ...context, token: `${context.token}0` },
      }),
      (error: unknown) =>
        error instanceof EvidenceError &&
        error.reason === "evidence_unavailable" &&
        error.fieldPath === "/evidenceRef",
    );
    await assert.rejects(
      engine.searchWithEvidence({
        schemaVersion: 1,
        query: "needle",
        context,
        rerankDepth: -1,
      }),
      (error: unknown) =>
        error instanceof InvalidArgumentError &&
        error.message.includes("rerankDepth must be >= 0"),
    );
    await engine.close();
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
