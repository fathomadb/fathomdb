// Slice 30 public TypeScript readiness/error contract through the real napi engine.

import test from "node:test";
import assert from "node:assert/strict";

import { EmbedderRequiredError, Engine, read } from "../src/index.js";
import { freshDbPath } from "./helpers.js";

function bodyEdge(body: string): object {
  return {
    edge: {
      kind: "relates_to",
      from: "memex-a",
      to: "memex-b",
      logicalId: "memex-edge-a-b",
      sourceId: "ts-test:slice30-readiness",
      body,
    },
  };
}

test("read.embeddingReadiness and immediate typed drain rejection keep edge bodies private", async () => {
  const secret = "slice30-private-edge-body-must-not-cross-readiness";
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    await engine.write([bodyEdge(secret)]);

    const readiness = await read.embeddingReadiness(engine);
    assert.equal(readiness.state, "blocked");
    assert.equal(readiness.usableEmbedder, false);
    assert.equal(readiness.pendingCount, 1);
    assert.deepEqual(readiness.affectedKinds, ["edge_fact"]);
    assert.equal(readiness.code, "FDB_EMBEDDER_REQUIRED");
    assert.equal(readiness.operation, "graph_edge_body_projection");
    assert.deepEqual(readiness.remediations, [
      "configure_default_embedder",
      "configure_caller_embedder",
      "submit_non_embedding_input",
    ]);
    assert.equal(readiness.documentationUrl, "https://fathomdb.dev/errors/FDB_EMBEDDER_REQUIRED");
    assert.equal(JSON.stringify(readiness).includes(secret), false);

    await assert.rejects(
      () => engine.drain(30_000),
      (error: unknown) => {
        assert.ok(error instanceof EmbedderRequiredError);
        assert.equal(error.code, "FDB_EMBEDDER_REQUIRED");
        assert.equal(error.operation, "graph_edge_body_projection");
        assert.equal(error.state, "blocked");
        assert.deepEqual(error.remediations, readiness.remediations);
        assert.equal(error.documentationUrl, readiness.documentationUrl);
        assert.equal(String(error).includes(secret), false);
        assert.equal(JSON.stringify(error).includes(secret), false);
        return true;
      },
    );
  } finally {
    await engine.close();
  }
});
