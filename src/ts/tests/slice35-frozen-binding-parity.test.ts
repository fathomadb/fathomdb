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

test("malformed frozen token precedes every JavaScript numeric conversion", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    const frozen = await engine.freezeReadContext({ schemaVersion: 1, view: {}, eligibility: {} });
    const malformed = { ...frozen, token: `${frozen.token}0` };
    for (const options of [
      { rerankDepth: 1.5 },
      { rerankDepth: Number.MAX_VALUE },
      { alpha: "high" as unknown as number },
      { poolN: 1.5 },
      { poolN: Number.MAX_VALUE },
      { limit: 1.5 },
      { limit: Number.MAX_VALUE },
    ]) {
      await assert.rejects(
        () => engine.searchFrozen("query", malformed, options),
        (error: unknown) => (error as { reason?: string }).reason === "token_malformed",
      );
    }
    for (const [depth, searchLimit] of [
      [1.5, 10],
      [Number.MAX_VALUE, 10],
      [0, 1.5],
      [0, Number.MAX_VALUE],
    ]) {
      await assert.rejects(
        () => engine.searchExpandFrozen("query", malformed, depth, { searchLimit }),
        (error: unknown) => (error as { reason?: string }).reason === "token_malformed",
      );
    }
  } finally {
    await engine.close();
  }
});
