// EP-03 `embedder_overloaded` at the TypeScript wrapper boundary. A TS test
// cannot drive the engine's embed queue to saturation, so the napi conversion
// of a real overloaded engine result is tested in Rust (`fathomdb-napi`
// `read_search::degradation_tests`) and this file tests the wrapper mapping.
import assert from "node:assert/strict";
import test from "node:test";

import { mapNativeSearchResult } from "../src/index.js";

type NativeSearchResult = Parameters<typeof mapNativeSearchResult>[0];

function native(extra: Partial<NativeSearchResult>): NativeSearchResult {
  return { projectionCursor: 3, softFallback: null, results: [], explanation: null, ...extra };
}

test("typescript search result reports embedder_overloaded", () => {
  const mapped = mapNativeSearchResult(
    native({ degradations: [{ arm: "vector", reason: "embedder_overloaded" }] }),
  );
  assert.deepEqual(mapped.degradations, [{ arm: "vector", reason: "embedder_overloaded" }]);
});

test("typescript search result omits degradations when none are reported", () => {
  for (const extra of [{}, { degradations: null }, { degradations: [] }]) {
    const mapped = mapNativeSearchResult(native(extra));
    assert.equal("degradations" in mapped, false);
  }
});

test("typescript search expand results report embedder_overloaded", async () => {
  const { mapNativeGraphSearchExpandResult } = await import("../src/graph.js");
  const { mapNativeSearchExpandFrozenResult } = await import("../src/search.js");
  const base = { searchHits: [], expanded: [], allLogicalIds: [] };
  const reported = { ...base, degradations: [{ arm: "vector", reason: "embedder_overloaded" }] };
  for (const map of [mapNativeGraphSearchExpandResult, mapNativeSearchExpandFrozenResult]) {
    assert.deepEqual(map(reported).degradations, [{ arm: "vector", reason: "embedder_overloaded" }]);
    for (const absent of [base, { ...base, degradations: null }, { ...base, degradations: [] }]) {
      assert.equal("degradations" in map(absent), false);
    }
  }
});

test("typescript search and searchTextOnly pass degradations through", async () => {
  const { search, searchTextOnly } = await import("../src/search.js");
  type NativeEngine = Parameters<typeof searchTextOnly>[0];
  // A stand-in for the native engine that returns one fixed native result, so
  // the test exercises only the wrapper's mapping.
  const returning = (result: NativeSearchResult): NativeEngine =>
    ({
      search: async () => result,
      searchTextOnly: async () => result,
    }) as unknown as NativeEngine;
  const reported = native({ degradations: [{ arm: "vector", reason: "embedder_overloaded" }] });
  const expected = [{ arm: "vector", reason: "embedder_overloaded" }];
  assert.deepEqual((await search(returning(reported), "q")).degradations, expected);
  assert.deepEqual((await searchTextOnly(returning(reported), "q")).degradations, expected);
  for (const extra of [{}, { degradations: null }, { degradations: [] }]) {
    assert.equal("degradations" in (await search(returning(native(extra)), "q")), false);
    assert.equal("degradations" in (await searchTextOnly(returning(native(extra)), "q")), false);
  }
});
