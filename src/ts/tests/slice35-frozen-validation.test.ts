import assert from "node:assert/strict";
import test from "node:test";

import { Engine, InvalidArgumentError, WriteValidationError } from "../src/index.js";
import { freshDbPath } from "./helpers.js";

test("valid frozen token rejects invalid query and ranking controls", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    const frozen = await engine.freezeReadContext({ schemaVersion: 1, view: {}, eligibility: {} });
    await assert.rejects(
      () => engine.searchFrozen("bad\0query", frozen),
      (error: unknown) => error instanceof WriteValidationError,
    );
    await assert.rejects(
      () => engine.searchFrozen("query", frozen, { rerankDepth: -1 }),
      (error: unknown) => error instanceof InvalidArgumentError,
    );
    await assert.rejects(
      () => engine.searchFrozen("query", frozen, { poolN: -1 }),
      (error: unknown) => error instanceof InvalidArgumentError,
    );
  } finally {
    await engine.close();
  }
});
