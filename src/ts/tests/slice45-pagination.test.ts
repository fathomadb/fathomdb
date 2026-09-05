import assert from "node:assert/strict";
import test from "node:test";

import { Engine, read } from "../src/index.js";
import { freshDbPath } from "./helpers.js";

test("canonical and operational pages share frozen authority", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    await engine.write(
      Array.from({ length: 3 }, (_, index) => ({
        kind: "slice45_doc",
        body: JSON.stringify({ n: index }),
        logicalId: `n-${index}`,
        sourceId: "typescript:slice45",
      })),
    );
    await engine.write([
      {
        adminSchema: {
          name: "slice45_state",
          kind: "latest_state",
          schemaJson: "{}",
          retentionJson: "{}",
        },
      },
    ]);
    for (let index = 0; index < 3; index += 1) {
      await engine.write([
        {
          opStore: {
            collection: "slice45_state",
            recordKey: `k-${index}`,
            body: JSON.stringify({ n: index }),
          },
        },
      ]);
    }

    const frozen = await engine.freezeReadContext({ schemaVersion: 1, view: {}, eligibility: {} });
    const first = await read.canonicalPage(engine, "slice45_doc", frozen, {
      schemaVersion: 1,
      limit: 2,
    });
    const second = await read.canonicalPage(engine, "slice45_doc", frozen, {
      schemaVersion: 1,
      limit: 2,
      cursor: first.nextCursor,
    });
    assert.deepEqual(
      [...first.items, ...second.items].map((row) => row.logicalId),
      ["n-0", "n-1", "n-2"],
    );
    assert.equal(second.nextCursor, null);

    const state = await read.operationalState(engine, "slice45_state", "k-0", frozen);
    const statePage = await read.operationalStatePage(engine, "slice45_state", frozen, {
      schemaVersion: 1,
      limit: 2,
    });
    assert.deepEqual(state, statePage.items[0]);
  } finally {
    await engine.close();
  }
});
