// EP-17 match-centred snippets through the TypeScript SDK (T-12).
//
// The engine owns snippet selection
// (dev/design/agent-access/match-centred-snippets.md § 7.2); these tests check
// that the binding forwards the options and returns the sidecar unchanged.
// Offsets in SnippetV1 are UTF-8 byte offsets, as in Rust.

import assert from "node:assert/strict";
import test from "node:test";

import { Engine, type SearchResult, type SnippetV1 } from "../src/index.js";
import { freshDbPath } from "./helpers.js";

const TERM = "zephyrine";
const SOURCE = "ts-test:ep17";
// The only query term sits at byte 2990 of a ~4 KiB ASCII record, far past
// any record-start prefix.
const FAR_BODY = "alpha beta gamma delta ".repeat(130) + TERM + " " + "omega psi chi ".repeat(80);
const SHORT_BODY = `${TERM} short note`;

function writes(): object[] {
  return [
    { kind: "doc", body: FAR_BODY, logicalId: "far", sourceId: SOURCE },
    { kind: "doc", body: SHORT_BODY, logicalId: "short", sourceId: SOURCE },
  ];
}

function byId(result: SearchResult): Map<string, SnippetV1> {
  assert.ok(result.snippets, "snippets is present when requested");
  assert.equal(result.snippets.length, result.results.length);
  return new Map(result.results.map((hit, i) => [hit.id.value, result.snippets![i]!]));
}

function assertFarAndShort(result: SearchResult, maxBytes: number): void {
  const snippets = byId(result);
  assert.deepStrictEqual([...snippets.keys()].sort(), ["far", "short"]);

  const far = snippets.get("far")!;
  assert.equal(far.anchor, "match");
  assert.ok(far.text.includes(TERM));
  assert.ok(Buffer.byteLength(far.text, "utf8") <= maxBytes);
  assert.equal(far.cut, true);
  assert.ok(far.spans.length > 0, "a matched long body carries its body-relative spans");
  const [start, end] = far.spans[0]!;
  assert.equal(Buffer.from(FAR_BODY, "utf8").subarray(start, end).toString("utf8"), TERM);
  assert.ok(far.windows.every((w) => Array.isArray(w) && w.length === 2));
  assert.deepStrictEqual(far.highlights, []);

  const short = snippets.get("short")!;
  assert.equal(short.anchor, "whole");
  assert.equal(short.text, SHORT_BODY);
  assert.equal(short.cut, false);
  assert.deepStrictEqual(short.windows, [[0, Buffer.byteLength(SHORT_BODY, "utf8")]]);
}

test("searchTextOnly returns the snippet sidecar", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    await engine.write(writes());
    const withSnippets = await engine.searchTextOnly(TERM, { snippet: { maxBytes: 120 } });
    assertFarAndShort(withSnippets, 120);

    const plain = await engine.searchTextOnly(TERM);
    assert.equal("snippets" in plain, false, "the sidecar is absent when not requested");
    assert.deepStrictEqual(plain.results, withSnippets.results);
  } finally {
    await engine.close();
  }
});
