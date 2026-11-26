import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { Engine, FathomDbError, mapPerHitExplain } from "../src/index.js";

type NativePerHitInput = Parameters<typeof mapPerHitExplain>[0];

function fakeNativePerHit(overrides: Record<string, unknown> = {}): NativePerHitInput {
  return {
    id: 1,
    arm: "vector",
    vectorRank: 0,
    textRank: null,
    graphRank: null,
    fusedScore: 0.016,
    ceScore: null,
    blended: 0.016,
    importance: null,
    confidence: null,
    graphSeedSource: null,
    graphHopDepth: null,
    vectorCosine: 0.875,
    ...overrides,
  } as NativePerHitInput;
}

function freshDbPath(name: string): string {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "fathomdb-wp40d-"));
  return path.join(dir, `${name}.fdb`);
}

test("WP-40D: mapPerHitExplain accepts valid vectorCosine values and normalizes undefined to null", () => {
  for (const val of [0, 1, -1, 0.42, -0.75, null]) {
    const mapped = mapPerHitExplain(fakeNativePerHit({ vectorCosine: val }));
    assert.equal(mapped.vectorCosine, val);
  }
  const negZero = mapPerHitExplain(fakeNativePerHit({ vectorCosine: -0 }));
  assert.ok(
    Object.is(negZero.vectorCosine, 0),
    `expected -0 to normalize to +0, got ${Object.is(negZero.vectorCosine, -0) ? "-0" : String(negZero.vectorCosine)}`,
  );
  const omitted = fakeNativePerHit();
  delete (omitted as unknown as Record<string, unknown>).vectorCosine;
  assert.equal(mapPerHitExplain(omitted).vectorCosine, null);
});

test("WP-40D: mapPerHitExplain rejects non-finite, out-of-range, or non-number vectorCosine", () => {
  const badValues: unknown[] = [
    Number.NaN,
    Number.POSITIVE_INFINITY,
    Number.NEGATIVE_INFINITY,
    1.5,
    -1.5,
    1.00001,
    -1.00001,
    "0.5",
    true,
    false,
  ];
  for (const bad of badValues) {
    assert.throws(
      () => mapPerHitExplain(fakeNativePerHit({ vectorCosine: bad })),
      (err: unknown) =>
        err instanceof FathomDbError && err.message.includes("/vectorCosine"),
      `expected FathomDbError(/vectorCosine) for ${String(bad)}`,
    );
  }
});

test("WP-40D: text-only search and searchWithEvidence expose vectorCosine === null", async () => {
  const engine = await Engine.open(freshDbPath("text-only"), { useDefaultEmbedder: false });
  try {
    await engine.write([
      {
        kind: "doc",
        body: "quasar nebula pulsar typescript record",
        sourceId: "src:ts-text",
        logicalId: "ts-text-1",
        provenance: {
          schemaVersion: 1 as const,
          role: "canonical" as const,
          artifactRevisionId: "rev-ts-text-1",
          sourceVersionId: "ver-ts-text-1",
        },
      },
    ]);
    const res = await engine.search("quasar nebula", undefined, 0, false, undefined, undefined, true);
    assert.ok(res.explanation);
    assert.equal(res.explanation.perHit.length, 1);
    assert.equal(res.explanation.perHit[0].vectorRank, null);
    assert.equal(res.explanation.perHit[0].vectorCosine, null);

    const frozen = await engine.freezeReadContext({
      schemaVersion: 1,
      view: {},
      eligibility: {},
    });
    const ev = await engine.searchWithEvidence({
      schemaVersion: 1,
      query: "quasar nebula",
      context: frozen,
      includeExplanation: true,
    });
    assert.ok(ev.searchResult.explanation);
    assert.equal(ev.searchResult.explanation.perHit.length, 1);
    assert.equal(ev.searchResult.explanation.perHit[0].vectorRank, null);
    assert.equal(ev.searchResult.explanation.perHit[0].vectorCosine, null);
  } finally {
    await engine.close();
  }
});

test(
  "WP-40D: vector-scored search and searchWithEvidence expose finite vectorCosine in [-1, 1]",
  { skip: Boolean(process.env.FATHOMDB_SKIP_NETWORK_TESTS) },
  async () => {
    const engine = await Engine.open(freshDbPath("vector"), { useDefaultEmbedder: true });
    try {
      await engine.configureProjections(
        [{ name: "memory", roles: ["searchable"], fts: false, vector: true }],
        [],
      );
      await engine.write([
        {
          kind: "doc",
          body: "distributed database transaction isolation",
          sourceId: "src:ts-vec",
          logicalId: "ts-vec-1",
          provenance: {
            schemaVersion: 1 as const,
            role: "canonical" as const,
            artifactRevisionId: "rev-ts-vec-1",
            sourceVersionId: "ver-ts-vec-1",
          },
        },
      ]);
      await engine.drain(30_000);

      const res = await engine.search(
        "distributed database transaction",
        undefined,
        0,
        false,
        undefined,
        undefined,
        true,
      );
      assert.ok(res.explanation);
      assert.ok(res.explanation.perHit.length >= 1);
      assert.ok(
        res.explanation.perHit.some(
          (p) => p.vectorRank !== null && typeof p.vectorCosine === "number",
        ),
      );
      for (const p of res.explanation.perHit) {
        if (p.vectorRank !== null) {
          assert.equal(typeof p.vectorCosine, "number");
          assert.ok(Number.isFinite(p.vectorCosine));
          assert.ok((p.vectorCosine as number) >= -1.0 && (p.vectorCosine as number) <= 1.0);
        } else {
          assert.equal(p.vectorCosine, null);
        }
      }

      const frozen = await engine.freezeReadContext({
        schemaVersion: 1,
        view: {},
        eligibility: {},
      });
      const ev = await engine.searchWithEvidence({
        schemaVersion: 1,
        query: "distributed database transaction",
        context: frozen,
        includeExplanation: true,
      });
      assert.ok(ev.searchResult.explanation);
      assert.ok(ev.searchResult.explanation.perHit.length >= 1);
      assert.ok(
        ev.searchResult.explanation.perHit.some(
          (p) => p.vectorRank !== null && typeof p.vectorCosine === "number",
        ),
      );
      for (const p of ev.searchResult.explanation.perHit) {
        if (p.vectorRank !== null) {
          assert.equal(typeof p.vectorCosine, "number");
          assert.ok(Number.isFinite(p.vectorCosine));
          assert.ok((p.vectorCosine as number) >= -1.0 && (p.vectorCosine as number) <= 1.0);
        } else {
          assert.equal(p.vectorCosine, null);
        }
      }
    } finally {
      await engine.close();
    }
  },
);
