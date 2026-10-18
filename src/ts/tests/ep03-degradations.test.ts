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
