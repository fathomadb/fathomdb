import test from "node:test";
import assert from "node:assert/strict";
import { Engine, InvalidFilterError, WriteValidationError } from "../src/index.js";
import { freshDbPath } from "./helpers.js";

test("direct body filter precedes cap and composes with the existing options", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    await engine.configureProjections([{ name: "group", roles: ["filterable"], fts: false, vector: false }]);
    await engine.write([
      ...Array.from({ length: 120 }, (_, i) => ({kind: "doc", logicalId: `n${i}`, sourceId: "slice90", body: JSON.stringify({ text: "s90binding", group: "other", i })})),
      {kind: "note", logicalId: "target", sourceId: "slice90", body: JSON.stringify({ text: "s90binding " + "padding ".repeat(100), group: "wanted" })},
    ]);
    const view = { validAsOf: 1_700_000_000 };
    assert.equal((await engine.searchTextOnly("s90binding", view)).results.length, 10);
    const result = await engine.searchTextOnly("s90binding", { ...view, limit: 1, filter: { sourceType: "note", kind: "note", attributes: [["group", "wanted"]] } });
    assert.deepEqual(result.results.map(h => h.id.value), ["target"]);
    assert.equal(result.softFallback, null);
    assert.deepEqual(await engine.searchTextOnly("s90binding", { ...view, filter: {} }), await engine.searchTextOnly("s90binding", view));
    for (const filter of [{ status: "" }, { createdAfter: 0 }]) {
      assert.deepEqual((await engine.searchTextOnly("s90binding", { filter })).results, []);
    }
    await assert.rejects(() => engine.searchTextOnly("absent", { filter: { attributes: [["unknown", "x"]] } }), InvalidFilterError);
    await assert.rejects(() => engine.searchTextOnly("s90binding", { filter: { kind: "bad\u0000kind" } }), WriteValidationError);
    await assert.rejects(() => engine.searchTextOnly("s90binding", { filter: { kind: "bad\ud800kind" } }), WriteValidationError);
  } finally {
    await engine.close();
  }
});
