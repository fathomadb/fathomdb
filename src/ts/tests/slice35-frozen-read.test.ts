import assert from "node:assert/strict";
import test from "node:test";

import {
  Engine,
  FrozenReadError,
  type ReadContextV1,
} from "../src/index.js";
import { freshDbPath } from "./helpers.js";

const SOURCE_ID = "ts-test:slice35-frozen";

test("frozen read context binds eligibility and rejects drift", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    await engine.write([
      { kind: "doc", body: "needle allowed", logicalId: "allowed", sourceId: SOURCE_ID },
      { kind: "note", body: "needle excluded", logicalId: "excluded", sourceId: SOURCE_ID },
    ]);
    const requested: ReadContextV1 = { schemaVersion: 1, view: {}, eligibility: { kind: "doc" } };
    const frozen = await engine.freezeReadContext(requested);

    assert.equal(frozen.context.view.validAsOf, frozen.effectiveValidAt);
    const first = await engine.searchFrozen("needle", frozen, { limit: 1 });
    assert.deepEqual(first.results.map((hit) => hit.id.value), ["allowed"]);

    await engine.write([
      { kind: "doc", body: "needle later", logicalId: "later", sourceId: SOURCE_ID },
    ]);
    await assert.rejects(
      () => engine.searchFrozen("needle", frozen),
      (error: unknown) => error instanceof FrozenReadError && error.reason === "state_drifted",
    );
  } finally {
    await engine.close();
  }
});

test("frozen search expansion returns the governed union", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    await engine.write([
      { kind: "doc", body: "expand needle", logicalId: "root", sourceId: SOURCE_ID },
    ]);
    const frozen = await engine.freezeReadContext({
      schemaVersion: 1,
      view: {},
      eligibility: { kind: "doc" },
    });

    const result = await engine.searchExpandFrozen("expand needle", frozen, 0);
    assert.deepEqual(result.allLogicalIds, ["root"]);
    assert.deepEqual(result.expanded, []);
  } finally {
    await engine.close();
  }
});
