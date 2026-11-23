import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";

import {
  EVIDENCE_RESOLVE_MANY_MAX,
  Engine,
  EvidenceError,
  InvalidArgumentError,
  WriteValidationError,
} from "../src/index.js";
import type {
  EvidenceResolveManyItemV1,
  EvidenceResolveManyRequestV1,
  Hint,
} from "../src/index.js";
import { freshDbPath } from "./helpers.js";

function sha256Hex(value: string): string {
  return createHash("sha256").update(value, "utf8").digest("hex");
}

function tamperToken(token: string): string {
  const last = token.endsWith("0") ? "1" : "0";
  return token.slice(0, -1) + last;
}

async function seedProvenancedPair(engine: Engine): Promise<{ bodyA: string; bodyB: string }> {
  const bodyA = "canonical alpha evidence text for ts slice50";
  const bodyB = "canonical beta evidence text for ts slice50";
  await engine.write([
    {
      kind: "document",
      body: bodyA,
      sourceId: "ts-slice50-src-a",
      logicalId: "ts-slice50-doc-a",
      provenance: {
        schemaVersion: 1,
        role: "canonical",
        artifactRevisionId: "ts-slice50-doc-a-r1",
        sourceVersionId: "ts-slice50-ver-a-1",
      },
    },
    {
      kind: "fact",
      body: "tsslice50needle alpha fact",
      sourceId: "ts-slice50-src-a",
      logicalId: "ts-slice50-fact-a",
      provenance: {
        schemaVersion: 1,
        role: "derived",
        artifactRevisionId: "ts-slice50-fact-a-r1",
        sourceVersionId: "ts-slice50-ver-a-1",
        sourceRevisionId: "ts-slice50-doc-a-r1",
        sourceLocator: { kind: "whole_body" },
        canonicalSourceHash: { algorithm: "sha256", digestHex: sha256Hex(bodyA) },
      },
    },
    {
      kind: "document",
      body: bodyB,
      sourceId: "ts-slice50-src-b",
      logicalId: "ts-slice50-doc-b",
      provenance: {
        schemaVersion: 1,
        role: "canonical",
        artifactRevisionId: "ts-slice50-doc-b-r1",
        sourceVersionId: "ts-slice50-ver-b-1",
      },
    },
    {
      kind: "fact",
      body: "tsslice50needle beta fact",
      sourceId: "ts-slice50-src-b",
      logicalId: "ts-slice50-fact-b",
      provenance: {
        schemaVersion: 1,
        role: "derived",
        artifactRevisionId: "ts-slice50-fact-b-r1",
        sourceVersionId: "ts-slice50-ver-b-1",
        sourceRevisionId: "ts-slice50-doc-b-r1",
        sourceLocator: { kind: "whole_body" },
        canonicalSourceHash: { algorithm: "sha256", digestHex: sha256Hex(bodyB) },
      },
    },
  ]);
  return { bodyA, bodyB };
}

test("resolveEvidenceMany enforces schemaVersion, type, NUL, unknown-key, and 1..=50 bounds", async () => {
  assert.equal(EVIDENCE_RESOLVE_MANY_MAX, 50);
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    await seedProvenancedPair(engine);
    const frozen = await engine.freezeReadContext({
      schemaVersion: 1,
      view: {},
      eligibility: {},
    });
    const ev = await engine.searchWithEvidence({
      schemaVersion: 1,
      query: "tsslice50needle",
      context: frozen,
      limit: 10,
    });
    const validRef = ev.evidence[0]!.evidenceRef;

    await assert.rejects(
      () =>
        engine.resolveEvidenceMany({
          schemaVersion: 2 as 1,
          evidenceRefs: [validRef],
          context: frozen,
        }),
      (err: unknown) =>
        err instanceof EvidenceError &&
        err.reason === "unsupported_schema_version" &&
        err.fieldPath === "/schemaVersion",
    );

    await assert.rejects(
      () =>
        engine.resolveEvidenceMany({
          schemaVersion: 1,
          evidenceRefs: [validRef],
          context: frozen,
          extraField: true,
        } as unknown as EvidenceResolveManyRequestV1),
      (err: unknown) => err instanceof EvidenceError && err.reason === "unknown_field",
    );

    await assert.rejects(
      () =>
        engine.resolveEvidenceMany({
          schemaVersion: 1,
          evidenceRefs: "not-an-array" as unknown as string[],
          context: frozen,
        }),
      TypeError,
    );

    await assert.rejects(
      () =>
        engine.resolveEvidenceMany({
          schemaVersion: 1,
          evidenceRefs: [validRef, 123 as unknown as string],
          context: frozen,
        }),
      TypeError,
    );

    await assert.rejects(
      () =>
        engine.resolveEvidenceMany({
          schemaVersion: 1,
          evidenceRefs: [validRef, "bad\0token"],
          context: frozen,
        }),
      WriteValidationError,
    );

    await assert.rejects(
      () =>
        engine.resolveEvidenceMany({
          schemaVersion: 1,
          evidenceRefs: [],
          context: frozen,
        }),
      InvalidArgumentError,
    );

    await assert.rejects(
      () =>
        engine.resolveEvidenceMany({
          schemaVersion: 1,
          evidenceRefs: Array.from({ length: EVIDENCE_RESOLVE_MANY_MAX + 1 }, () => validRef),
          context: frozen,
        }),
      InvalidArgumentError,
    );
  } finally {
    await engine.close();
  }
});

test("resolveEvidenceMany preserves duplicates, per-slot nondisclosure, and whole-call drift refusal", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  const other = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    const { bodyA, bodyB } = await seedProvenancedPair(engine);
    await seedProvenancedPair(other);

    const frozen = await engine.freezeReadContext({
      schemaVersion: 1,
      view: {},
      eligibility: {},
    });
    const otherFrozen = await other.freezeReadContext({
      schemaVersion: 1,
      view: {},
      eligibility: {},
    });
    const filteredFrozen = await engine.freezeReadContext({
      schemaVersion: 1,
      view: {},
      eligibility: { kind: "fact" },
    });

    const ev = await engine.searchWithEvidence({
      schemaVersion: 1,
      query: "tsslice50needle",
      context: frozen,
      limit: 10,
    });
    assert.equal(ev.evidence.length, 2);
    const ref0 = ev.evidence[0]!.evidenceRef;
    const ref1 = ev.evidence[1]!.evidenceRef;

    const wrongDbRef = (
      await other.searchWithEvidence({
        schemaVersion: 1,
        query: "tsslice50needle",
        context: otherFrozen,
        limit: 10,
      })
    ).evidence[0]!.evidenceRef;

    const wrongCtxRef = (
      await engine.searchWithEvidence({
        schemaVersion: 1,
        query: "tsslice50needle",
        context: filteredFrozen,
        limit: 10,
      })
    ).evidence[0]!.evidenceRef;

    const dupItems: EvidenceResolveManyItemV1[] = await engine.resolveEvidenceMany({
      schemaVersion: 1,
      evidenceRefs: [ref0, ref1, ref0],
      context: frozen,
    });
    assert.equal(dupItems.length, 3);
    assert.equal(dupItems[0]!.status, "resolved");
    assert.equal(dupItems[1]!.status, "resolved");
    assert.equal(dupItems[2]!.status, "resolved");
    assert.deepEqual(dupItems[0], dupItems[2]);
    const bodies = new Set([
      dupItems[0]!.status === "resolved" ? dupItems[0]!.resolved.canonicalSourceBody : "",
      dupItems[1]!.status === "resolved" ? dupItems[1]!.resolved.canonicalSourceBody : "",
    ]);
    assert.deepEqual(bodies, new Set([bodyA, bodyB]));

    const mixed = await engine.resolveEvidenceMany({
      schemaVersion: 1,
      evidenceRefs: [
        ref0,
        "",
        "malformed-token",
        tamperToken(ref0),
        "x".repeat(2049),
        wrongDbRef,
        wrongCtxRef,
        ref1,
      ],
      context: frozen,
    });
    assert.equal(mixed.length, 8);
    assert.deepEqual(mixed[0], dupItems[0]);
    assert.deepEqual(mixed[7], dupItems[1]);
    for (let i = 1; i <= 6; i += 1) {
      const slot = mixed[i]!;
      assert.equal(slot.status, "refused");
      if (slot.status === "refused") {
        assert.equal(slot.reason, "evidence_unavailable");
        assert.equal(slot.fieldPath, `/evidenceRefs/${i}`);
      }
    }

    // Mutate to drift the frozen context.
    await engine.write([
      {
        kind: "fact",
        body: "tsslice50needle alpha fact v2",
        sourceId: "ts-slice50-src-a",
        logicalId: "ts-slice50-fact-a",
        provenance: {
          schemaVersion: 1,
          role: "derived",
          artifactRevisionId: "ts-slice50-fact-a-r2",
          sourceVersionId: "ts-slice50-ver-a-1",
          sourceRevisionId: "ts-slice50-doc-a-r1",
          sourceLocator: { kind: "whole_body" },
          canonicalSourceHash: { algorithm: "sha256", digestHex: sha256Hex(bodyA) },
        },
      },
    ]);
    await assert.rejects(
      () =>
        engine.resolveEvidenceMany({
          schemaVersion: 1,
          evidenceRefs: [ref0],
          context: frozen,
        }),
      (err: unknown) => err instanceof EvidenceError && err.reason === "evidence_unavailable",
    );

    const freshFrozen = await engine.freezeReadContext({
      schemaVersion: 1,
      view: {},
      eligibility: {},
    });
    const freshRef = (
      await engine.searchWithEvidence({
        schemaVersion: 1,
        query: "tsslice50needle",
        context: freshFrozen,
        limit: 10,
      })
    ).evidence[0]!.evidenceRef;
    const postDrift = await engine.resolveEvidenceMany({
      schemaVersion: 1,
      evidenceRefs: [freshRef, ref0],
      context: freshFrozen,
    });
    assert.equal(postDrift[0]!.status, "resolved");
    assert.deepEqual(postDrift[1], {
      status: "refused",
      reason: "evidence_unavailable",
      fieldPath: "/evidenceRefs/1",
    });
  } finally {
    await other.close();
    await engine.close();
  }
});

test("search, searchFrozen, and searchWithEvidence support opt-in hints and reject non-boolean values", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    await engine.write(
      Array.from({ length: 4 }, (_, idx) => ({
        kind: "doc",
        body: `tshintstarget document number ${idx}`,
        sourceId: `ts-hints-src-${idx}`,
        logicalId: `ts-hints-doc-${idx}`,
        provenance: {
          schemaVersion: 1,
          role: "canonical" as const,
          artifactRevisionId: `ts-hints-doc-${idx}-r1`,
          sourceVersionId: `ts-hints-ver-${idx}-1`,
        },
      })),
    );
    const frozen = await engine.freezeReadContext({
      schemaVersion: 1,
      view: {},
      eligibility: {},
    });

    const defaultRes = await engine.search(
      "tshintstarget",
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      { limit: 2 },
    );
    const falseRes = await engine.search(
      "tshintstarget",
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      { limit: 2, hints: false },
    );
    assert.equal(defaultRes.hints, undefined);
    assert.equal(falseRes.hints, undefined);
    assert.deepEqual(defaultRes.results, falseRes.results);

    const moreRes = await engine.search(
      "tshintstarget",
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      { limit: 2, hints: true },
    );
    assert.deepEqual(moreRes.results, defaultRes.results);
    assert.ok(Array.isArray(moreRes.hints));
    assert.ok(
      moreRes.hints.some(
        (h: Hint) => h.action === "search" && h.reason === "more_results" && h.limit === 4,
      ),
    );

    const emptyRes = await engine.search(
      "definitelyabsenttsword",
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      { limit: 5, hints: true },
    );
    assert.equal(emptyRes.results.length, 0);
    assert.ok(Array.isArray(emptyRes.hints));
    assert.ok(emptyRes.hints.some((h: Hint) => h.action === "search" && h.reason === "no_results"));

    const frozenDefault = await engine.searchFrozen("tshintstarget", frozen, { limit: 2 });
    assert.equal(frozenDefault.hints, undefined);
    const frozenHints = await engine.searchFrozen("tshintstarget", frozen, {
      limit: 2,
      hints: true,
    });
    assert.deepEqual(frozenHints.results, frozenDefault.results);
    assert.ok(frozenHints.hints?.some((h) => h.reason === "more_results" && h.limit === 4));

    const evDefault = await engine.searchWithEvidence({
      schemaVersion: 1,
      query: "tshintstarget",
      context: frozen,
      limit: 2,
    });
    assert.equal(evDefault.searchResult.hints, undefined);
    const evHints = await engine.searchWithEvidence({
      schemaVersion: 1,
      query: "tshintstarget",
      context: frozen,
      limit: 2,
      hints: true,
    });
    assert.deepEqual(evHints.searchResult.results, evDefault.searchResult.results);
    assert.ok(
      evHints.searchResult.hints?.some((h) => h.reason === "more_results" && h.limit === 4),
    );

    for (const badHints of [1, 0, "true", null]) {
      await assert.rejects(
        () =>
          engine.search(
            "tshintstarget",
            undefined,
            undefined,
            undefined,
            undefined,
            undefined,
            undefined,
            { hints: badHints as unknown as boolean },
          ),
        TypeError,
      );
      await assert.rejects(
        () =>
          engine.searchFrozen("tshintstarget", frozen, {
            hints: badHints as unknown as boolean,
          }),
        TypeError,
      );
      await assert.rejects(
        () =>
          engine.searchWithEvidence({
            schemaVersion: 1,
            query: "tshintstarget",
            context: frozen,
            hints: badHints as unknown as boolean,
          }),
        TypeError,
      );
    }
  } finally {
    await engine.close();
  }
});

test("OpenReport exposes capability booleans on real Engine.open", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    const report = engine.openReport();
    assert.equal(typeof report.rerankerCompiled, "boolean");
    assert.equal(typeof report.cudaCompiled, "boolean");
    assert.equal(typeof report.rerankAvailable, "boolean");
    if (!report.rerankerCompiled) {
      assert.equal(report.rerankAvailable, false);
    }
  } finally {
    await engine.close();
  }
});
