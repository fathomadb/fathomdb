// EP-17 match-centred snippets through the TypeScript SDK (T-12).
//
// The engine owns snippet selection
// (dev/design/agent-access/match-centred-snippets.md § 7.2); these tests check
// that the binding forwards the options and returns the sidecar unchanged.
// Offsets in SnippetV1 are UTF-8 byte offsets, as in Rust.

import assert from "node:assert/strict";
import test from "node:test";

import {
  Engine,
  InvalidArgumentError,
  type SearchResult,
  type SnippetOptions,
  type SnippetV1,
} from "../src/index.js";
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

test("search returns the snippet sidecar", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    await engine.write(writes());
    const withSnippets = await engine.search(
      TERM,
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      { snippet: { maxBytes: 120 } },
    );
    assertFarAndShort(withSnippets, 120);

    const plain = await engine.search(TERM);
    assert.equal("snippets" in plain, false, "the sidecar is absent when not requested");
    assert.deepStrictEqual(plain.results, withSnippets.results);
  } finally {
    await engine.close();
  }
});

test("searchFrozen returns the snippet sidecar", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    await engine.write(writes());
    const frozen = await engine.freezeReadContext({ schemaVersion: 1, view: {}, eligibility: {} });
    const withSnippets = await engine.searchFrozen(TERM, frozen, { snippet: { maxBytes: 120 } });
    assertFarAndShort(withSnippets, 120);

    // Snippet options are presentation: the same frozen context serves both
    // calls and the results do not change.
    const plain = await engine.searchFrozen(TERM, frozen);
    assert.equal("snippets" in plain, false, "the sidecar is absent when not requested");
    assert.deepStrictEqual(plain.results, withSnippets.results);
  } finally {
    await engine.close();
  }
});

type Surface = (engine: Engine, snippet: unknown) => Promise<SearchResult>;

const SURFACES: Record<string, Surface> = {
  search: (engine, snippet) =>
    engine.search(TERM, undefined, undefined, undefined, undefined, undefined, undefined, {
      snippet: snippet as SnippetOptions,
    }),
  searchTextOnly: (engine, snippet) =>
    engine.searchTextOnly(TERM, { snippet: snippet as SnippetOptions }),
  searchFrozen: async (engine, snippet) =>
    engine.searchFrozen(
      TERM,
      await engine.freezeReadContext({ schemaVersion: 1, view: {}, eligibility: {} }),
      { snippet: snippet as SnippetOptions },
    ),
};

// Each value is out of its documented range (match-centred-snippets.md
// § 7.2), including values past the native integer types.
const OUT_OF_RANGE: SnippetOptions[] = [
  { maxBytes: 39 },
  { maxBytes: 4001 },
  { maxBytes: -1 },
  { maxBytes: 2 ** 40 },
  { maxWindows: 0 },
  { maxWindows: 4 },
  { maxWindows: 2 ** 40 },
  { maxBytes: 100, headBytes: 51 },
  { headBytes: -1 },
  { maxPointQueries: 256 },
  { maxPointQueries: -1 },
];

const IN_RANGE: SnippetOptions[] = [
  {},
  { maxBytes: 40, maxWindows: 3, headBytes: 20, maxPointQueries: 0 },
  { maxBytes: 4000, highlights: true, maxPointQueries: 255 },
];

const WRONG_TYPE: unknown[] = [
  "300",
  300,
  { maxBytes: 300.5 },
  { maxBytes: Number.NaN },
  { maxBytes: Number.POSITIVE_INFINITY },
  { maxBytes: "300" },
  { maxWindows: true },
  { highlights: 1 },
];

for (const [name, run] of Object.entries(SURFACES)) {
  test(`${name} validates snippet options`, async () => {
    const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
    try {
      await engine.write(writes());
      for (const options of IN_RANGE) {
        const result = await run(engine, options);
        assert.equal(result.snippets?.length, 2, JSON.stringify(options));
      }
      for (const options of OUT_OF_RANGE) {
        await assert.rejects(run(engine, options), InvalidArgumentError, JSON.stringify(options));
      }
      for (const options of WRONG_TYPE) {
        await assert.rejects(run(engine, options), TypeError, String(JSON.stringify(options)));
      }
      // A misspelt key is refused, not ignored.
      await assert.rejects(run(engine, { maxbytes: 300 }), InvalidArgumentError);
    } finally {
      await engine.close();
    }
  });
}

test("searchProjectedText refuses a snippet option", async () => {
  const engine = await Engine.open(freshDbPath(), { useDefaultEmbedder: false });
  try {
    await assert.rejects(
      engine.searchProjectedText(TERM, "facts", undefined, { snippet: { maxBytes: 120 } }),
      InvalidArgumentError,
    );
  } finally {
    await engine.close();
  }
});
