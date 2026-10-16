// 0.8.28 Slice 41 phase 2 unit Q — configurable query stop words, TypeScript SDK.
//
// AC41-36 (open-time refusals), AC41-38 (the hash on the open report and the
// explain trace, and `DEFAULT_STOP_WORDS_SHA256`) and AC41-40 (the shared
// small-corpus case). Loads `src/conformance/query-stop-words-fixtures.json`,
// which the Rust and Python suites also load. Real stores, no mocking.

import test from "node:test";
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { DEFAULT_STOP_WORDS_SHA256, Engine, InvalidArgumentError } from "../src/index.js";
import type { QueryStopWords, SearchResult } from "../src/index.js";
import { freshDbPath } from "./helpers.js";

type Json = any; // eslint-disable-line @typescript-eslint/no-explicit-any

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..", "..", "..");
const fixtures: Json = JSON.parse(
  readFileSync(join(root, "src", "conformance", "query-stop-words-fixtures.json"), "utf8"),
);
const QUERY = "how long had the dentist suggest flossing";

function bits(score: number): bigint {
  const view = new DataView(new ArrayBuffer(8));
  view.setFloat64(0, score);
  return view.getBigUint64(0);
}

function ids(result: SearchResult): string[] {
  return result.results.map((hit) => hit.id.value);
}

async function writeCorpus(engine: Engine): Promise<void> {
  await engine.write(
    fixtures.corpus.map((doc: Json) => ({
      kind: "doc",
      body: doc.body,
      logicalId: doc.logical_id,
      sourceId: "slice41q-fixture",
    })),
  );
  await engine.drain(30_000);
}

test("DEFAULT_STOP_WORDS_SHA256 matches the fixture", () => {
  assert.equal(DEFAULT_STOP_WORDS_SHA256, fixtures.default_sha256);
});

test("the shared fixture ranks as derived and as the harness transform", async () => {
  const path = freshDbPath();
  const transforms: SearchResult[] = [];
  const plain = await Engine.open(path);
  try {
    await writeCorpus(plain);
    assert.equal(plain.openReport().stopWordsSha256, fixtures.default_sha256);
    for (const q of fixtures.queries) {
      assert.deepStrictEqual(ids(await plain.searchTextOnly(q.query)), q.default_ids);
      transforms.push(await plain.searchTextOnly(q.stripped_query));
    }
  } finally {
    await plain.close();
  }

  const setting: QueryStopWords = fixtures.setting;
  const configured = await Engine.open(path, { engineConfig: { queryStopWords: setting } });
  try {
    assert.deepStrictEqual(configured.config.queryStopWords, setting);
    assert.equal(configured.openReport().stopWordsSha256, fixtures.setting_sha256);
    for (const [i, q] of fixtures.queries.entries()) {
      const result = await configured.searchTextOnly(q.query);
      assert.deepStrictEqual(ids(result), q.setting_ids);
      assert.deepStrictEqual(ids(transforms[i]!), q.setting_ids);
      assert.deepStrictEqual(
        result.results.map((h) => bits(h.score)),
        transforms[i]!.results.map((h) => bits(h.score)),
      );
    }
  } finally {
    await configured.close();
  }
});

for (const [name, setting, expected] of [
  ["unset", undefined, "default_sha256"],
  ["explicit default", { base: "fathomdb" }, "default_sha256"],
  ["fixture setting", fixtures.setting, "setting_sha256"],
] as const) {
  test(`the explain trace and open report record the hash (${name})`, async () => {
    const engine = await Engine.open(
      freshDbPath(),
      setting === undefined ? {} : { engineConfig: { queryStopWords: setting as QueryStopWords } },
    );
    try {
      await writeCorpus(engine);
      assert.equal(engine.openReport().stopWordsSha256, fixtures[expected]);
      const result = await engine.search(QUERY, undefined, undefined, undefined, undefined, undefined, true);
      assert.equal(result.explanation?.trace.stopWordsSha256, fixtures[expected]);
    } finally {
      await engine.close();
    }
  });
}

for (const c of fixtures.invalid_settings) {
  test(`invalid setting ${c.name} is InvalidArgumentError at open`, async () => {
    const path = freshDbPath();
    await assert.rejects(
      Engine.open(path, { engineConfig: { queryStopWords: c.setting } }),
      (err: unknown) => err instanceof InvalidArgumentError && err.message.includes(c.message_contains),
    );
    assert.equal(existsSync(path), false);
  });
}

test("wrong setting shapes are TypeError", async () => {
  for (const bad of [
    "had",
    ["had"],
    { extra: "had" },
    { keep: [1] },
    { base: "engine" },
  ]) {
    await assert.rejects(
      Engine.open(freshDbPath(), { engineConfig: { queryStopWords: bad as unknown as QueryStopWords } }),
      TypeError,
      JSON.stringify(bad),
    );
  }
});
