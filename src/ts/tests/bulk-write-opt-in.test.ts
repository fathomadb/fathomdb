// 0.8.32 Slice 30 (WP-30B) — TypeScript SDK opt-in bulk write mode parity.

import test from "node:test";
import assert from "node:assert/strict";

import * as sdk from "../src/index.js";
import {
  Engine,
  ScopePolicy,
  ScopeViolationError,
  WriteValidationError,
  graph,
  read,
  type EndpointValidationMode,
  type WriteOptions,
} from "../src/index.js";
import { freshDbPath } from "./helpers.js";

function node(logicalId: string, body: string, sourceId = "bulk-src"): object {
  return { kind: "note", body, logicalId, sourceId };
}

function edge(from: string, to: string, sourceId = "bulk-src", logicalId?: string): object {
  return {
    edge: {
      kind: "links",
      from,
      to,
      sourceId,
      ...(logicalId !== undefined ? { logicalId } : {}),
    },
  };
}

test("write namespace stays private and WriteOptions types compile", () => {
  assert.equal("write" in sdk, false);
  const mode: EndpointValidationMode = "bulk_non_strict";
  const opts: WriteOptions = { endpointValidation: mode };
  assert.equal(opts.endpointValidation, "bulk_non_strict");
});

test("strict default on write and writeWithOptions refuses dangling endpoints", async () => {
  const engine = await Engine.open(freshDbPath());
  try {
    await assert.rejects(
      engine.write([edge("n:a", "n:b")]),
      (err: unknown) => err instanceof WriteValidationError,
    );
    await assert.rejects(
      engine.writeWithOptions([edge("n:a", "n:b")]),
      (err: unknown) => err instanceof WriteValidationError,
    );
    await assert.rejects(
      engine.writeWithOptions([edge("n:a", "n:b")], {}),
      (err: unknown) => err instanceof WriteValidationError,
    );
    await assert.rejects(
      engine.writeWithOptions([edge("n:a", "n:b")], { endpointValidation: "strict" }),
      (err: unknown) => err instanceof WriteValidationError,
    );
  } finally {
    await engine.close();
  }
});

test("bulk_non_strict counts dangling endpoints truthfully and resolves later", async () => {
  const engine = await Engine.open(freshDbPath());
  try {
    const bulk: WriteOptions = { endpointValidation: "bulk_non_strict" };

    const r1 = await engine.writeWithOptions([edge("n:a", "n:b", "bulk-src", "e:ab")], bulk);
    assert.equal(r1.danglingEdgeEndpoints, 2);

    const r2 = await engine.writeWithOptions(
      [node("n:a", "alpha"), edge("n:a", "n:c", "bulk-src", "e:ac")],
      bulk,
    );
    assert.equal(r2.danglingEdgeEndpoints, 1);

    const r3 = await engine.writeWithOptions(
      [node("n:b", "beta"), node("n:c", "gamma")],
      bulk,
    );
    assert.equal(r3.danglingEdgeEndpoints, 0);

    const neighbors = await graph.neighbors(engine, "n:a", 1);
    assert.deepEqual(
      neighbors.map((n) => n.logicalId).sort(),
      ["n:b", "n:c"],
    );
  } finally {
    await engine.close();
  }
});

test("bulk_non_strict refuses cross-scope active endpoints and read-only scopes", async () => {
  const dbPath = freshDbPath();
  const bulk: WriteOptions = { endpointValidation: "bulk_non_strict" };

  const scopeA = await Engine.open(dbPath, { scopePolicy: ScopePolicy.single("scope-a") });
  try {
    await scopeA.write([node("shared:a", "alpha in a", "src-a")]);
  } finally {
    await scopeA.close();
  }

  const scopeB = await Engine.open(dbPath, { scopePolicy: ScopePolicy.single("scope-b") });
  try {
    await assert.rejects(
      scopeB.writeWithOptions(
        [node("local:b", "beta in b", "src-b"), edge("local:b", "shared:a", "src-b")],
        bulk,
      ),
      (err: unknown) => err instanceof ScopeViolationError,
    );
    assert.equal(await read.get(scopeB, "local:b"), null);
  } finally {
    await scopeB.close();
  }

  const ro = await Engine.open(dbPath, { scopePolicy: ScopePolicy.readOnly(["scope-a"]) });
  try {
    await assert.rejects(
      ro.writeWithOptions([node("ro:1", "denied")], bulk),
      (err: unknown) => err instanceof ScopeViolationError,
    );
  } finally {
    await ro.close();
  }
});

test("writeWithOptions throws TypeError on invalid options or endpointValidation literal", async () => {
  const engine = await Engine.open(freshDbPath());
  try {
    for (const badOpts of [null, "bulk_non_strict", 42, []] as unknown[]) {
      await assert.rejects(
        engine.writeWithOptions([], badOpts as WriteOptions),
        (err: unknown) => err instanceof TypeError,
      );
    }
    for (const badMode of ["", "non_strict", "Strict", 123, null, true] as unknown[]) {
      await assert.rejects(
        engine.writeWithOptions([], { endpointValidation: badMode as EndpointValidationMode }),
        (err: unknown) => err instanceof TypeError,
      );
    }
  } finally {
    await engine.close();
  }
});
