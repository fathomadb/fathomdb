// 0.8.32 Slice 20 (WP-20D) — TypeScript SDK caller-scope isolation & view parity.

import test from "node:test";
import assert from "node:assert/strict";

import {
  Engine,
  InvalidArgumentError,
  ScopePolicy,
  ScopeViolationError,
  graph,
  read,
} from "../src/index.js";
import { freshDbPath } from "./helpers.js";

function node(
  logicalId: string,
  body: string,
  sourceId: string,
  validFrom?: number,
  validUntil?: number,
): object {
  return {
    kind: "note",
    body,
    logicalId,
    sourceId,
    ...(validFrom !== undefined ? { validFrom } : {}),
    ...(validUntil !== undefined ? { validUntil } : {}),
  };
}

function edge(logicalId: string, from: string, to: string, sourceId: string): object {
  return {
    edge: {
      kind: "links",
      from,
      to,
      logicalId,
      sourceId,
    },
  };
}

test("invalid ScopePolicy fails Engine.open with InvalidArgumentError", async () => {
  const path = freshDbPath();
  const invalidPolicies = [
    ScopePolicy.readOnly([]),
    ScopePolicy.single(""),
    ScopePolicy.single("*"),
    ScopePolicy.readWrite(["agent-a"], "agent-b"),
  ];
  for (const policy of invalidPolicies) {
    await assert.rejects(
      Engine.open(path, { scopePolicy: policy }),
      (error: unknown) => error instanceof InvalidArgumentError,
    );
  }
});

test("two callers are isolated and readOnly rejects writes with ScopeViolationError", async () => {
  const path = freshDbPath();
  const aWriter = await Engine.open(path, { engineConfig: { scopePolicy: ScopePolicy.single("a") } });
  await aWriter.write([node("a:only", "scopedneedle alpha private", "src-a")]);
  await aWriter.drain(10_000);
  await aWriter.close();

  const b = await Engine.open(path, { scopePolicy: ScopePolicy.single("b") });
  await b.write([node("b:only", "scopedneedle beta private", "src-b")]);
  await b.drain(10_000);
  assert.equal(await read.get(b, "a:only"), null);
  assert.deepEqual(
    (await graph.searchExpand(b, "scopedneedle", 1)).allLogicalIds,
    ["b:only"],
  );
  await b.close();

  const a = await Engine.open(path, { engineConfig: { scopePolicy: ScopePolicy.single("a") } });
  assert.equal(await read.get(a, "b:only"), null);
  assert.deepEqual(
    (await a.searchTextOnly("scopedneedle")).results.map((hit) => hit.id.value),
    ["a:only"],
  );
  await a.close();

  const ro = await Engine.open(path, { scopePolicy: ScopePolicy.readOnly(["a"]) });
  try {
    assert.notEqual(await read.get(ro, "a:only"), null);
    assert.equal(await read.get(ro, "b:only"), null);
    await assert.rejects(
      ro.write([node("a:blocked", "cannot write", "src-ro")]),
      (error: unknown) =>
        error instanceof ScopeViolationError && error.code === "FDB_SCOPE_VIOLATION",
    );
  } finally {
    await ro.close();
  }
});

test("graph.searchExpand honors view option and preserves caller-scope isolation", async () => {
  const path = freshDbPath();
  const aWriter = await Engine.open(path, { scopePolicy: ScopePolicy.single("a") });
  await aWriter.write([
    node("a:root", "expandanchor alpha", "src-a"),
    node("a:past", "past leaf alpha", "src-a", 1, 2),
    edge("e:a", "a:root", "a:past", "src-a"),
  ]);
  await aWriter.drain(10_000);
  await aWriter.close();

  const b = await Engine.open(path, { scopePolicy: ScopePolicy.single("b") });
  await b.write([
    node("b:root", "expandanchor beta", "src-b"),
    node("b:past", "past leaf beta", "src-b", 1, 2),
    edge("e:b", "b:root", "b:past", "src-b"),
  ]);
  await b.drain(10_000);
  await b.close();

  const a = await Engine.open(path, { scopePolicy: ScopePolicy.single("a") });
  try {
    const strict = await graph.searchExpand(a, "expandanchor", 1);
    assert.deepEqual(strict.allLogicalIds, ["a:root"]);

    const relaxed = await graph.searchExpand(a, "expandanchor", 1, undefined, {
      view: { includeOutOfWindow: true },
    });
    assert.deepEqual(
      relaxed.searchHits.map((hit) => hit.id.value),
      ["a:root"],
    );
    assert.deepEqual(
      relaxed.expanded.map((item) => item.node.logicalId),
      ["a:past"],
    );
    assert.equal(
      relaxed.allLogicalIds.some((id) => id.startsWith("b:")),
      false,
    );

    const frozen = await a.freezeReadContext({ schemaVersion: 1, view: {}, eligibility: {} });
    await assert.rejects(
      a.searchExpandFrozen("expandanchor", frozen, 1, {
        view: { includeOutOfWindow: true },
      } as never),
      InvalidArgumentError,
    );
  } finally {
    await a.close();
  }
});
