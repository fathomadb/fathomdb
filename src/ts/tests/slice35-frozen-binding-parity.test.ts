import assert from "node:assert/strict";
import test from "node:test";

import { Engine } from "../src/index.js";
import { freshDbPath } from "./helpers.js";

test("frozen search rejects JavaScript coercions after authentication", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    const frozen = await engine.freezeReadContext({ schemaVersion: 1, view: {}, eligibility: {} });
    for (const options of [
      { rerankDepth: 1.5 },
      { rerankDepth: 0x1_0000_0000 },
      { useGraphArm: 1 as unknown as boolean },
      { alpha: true as unknown as number },
      { poolN: 1.5 },
      { poolN: 0x1_0000_0000 },
      { explain: 1 as unknown as boolean },
      { limit: 1.5 },
    ]) {
      await assert.rejects(() => engine.searchFrozen("query", frozen, options));
    }
  } finally {
    await engine.close();
  }
});

test("frozen expansion rejects fractional depth and limit after authentication", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    const frozen = await engine.freezeReadContext({ schemaVersion: 1, view: {}, eligibility: {} });
    await assert.rejects(() => engine.searchExpandFrozen("query", frozen, 1.5));
    await assert.rejects(() => engine.searchExpandFrozen("query", frozen, 0, { searchLimit: 1.5 }));
  } finally {
    await engine.close();
  }
});
