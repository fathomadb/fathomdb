// Per-call absence through the real production N-API binding.
import assert from "node:assert/strict";
import test from "node:test";
import { Engine, graph } from "../src/index.js";
import { freshDbPath } from "./helpers.js";

const QUERY = "sliceseventyneedle";
for (const route of ["search", "frozen", "expand", "frozen_expand"]) {
  test(`None provider reports per-call absence through ${route}`, async () => {
    const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
    try {
      await engine.write([{ kind: "note", body: `${QUERY} note`, logicalId: "s70-note", sourceId: "s70-typescript" }]);
      const text = await engine.searchTextOnly(QUERY);
      assert.equal("degradations" in text, false);
      assert.deepEqual(text.results.map((hit) => hit.id.value), ["s70-note"]);
      const frozen = await engine.freezeReadContext({ schemaVersion: 1, view: {}, eligibility: {} });
      const result = route === "search" ? await engine.search(QUERY)
        : route === "frozen" ? await engine.searchFrozen(QUERY, frozen)
        : route === "expand" ? await graph.searchExpand(engine, QUERY, 1)
        : await engine.searchExpandFrozen(QUERY, frozen, 1);
      assert.deepEqual(result.degradations, [{ arm: "vector", reason: "embedder_absent" }]);
      const hits = "results" in result ? result.results : result.searchHits;
      assert.deepEqual(hits.map((hit) => hit.id.value), ["s70-note"]);
      assert.ok(hits.every((hit) => hit.branch === "text"));
      assert.equal(hits[0]!.score, 3 / 31);
    } finally {
      await engine.close();
    }
  });
}
