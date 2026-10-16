// 0.8.28 Slice 41 — context pack through the TypeScript SDK (AC41-10/11/20/21).
//
// Loads the shared hand-derived fixture `src/conformance/context-pack-fixtures.json`
// that the Rust (`fathomdb-engine` and `fathomdb-sdk` `tests/slice41_context_pack.rs`)
// and Python (`src/python/tests/test_context_pack_parity.py`) suites load, and
// compares rendered text and every provenance field. Errors are
// `InvalidArgumentError` whose message names the field.

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import {
  DEFAULT_CONTEXT_POLICY,
  Engine,
  InvalidArgumentError,
  contextItemFromHit,
  decodeRoundBody,
  encodeRoundBody,
  packContext,
} from "../src/index.js";
import type { ContextItem, ContextPack, ContextPart, ContextPolicy, SearchHit } from "../src/index.js";
import { freshDbPath } from "./helpers.js";

type Json = any; // eslint-disable-line @typescript-eslint/no-explicit-any

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..", "..", "..");
const fixtures: Json = JSON.parse(
  readFileSync(join(root, "src", "conformance", "context-pack-fixtures.json"), "utf8"),
);

function part(v: Json): ContextPart {
  return { role: v.role, text: v.text };
}

function item(v: Json): ContextItem {
  return {
    id: v.id,
    parts: v.parts.map(part),
    group: v.group,
    position: v.position,
    orderKey: v.order_key,
    header: v.header,
  };
}

function policy(v: Json): ContextPolicy {
  const p: ContextPolicy = {};
  if (!v) return p;
  if ("budget_chars" in v) p.budgetChars = v.budget_chars;
  if ("item_chars" in v) p.itemChars = v.item_chars;
  if ("full_items" in v) p.fullItems = v.full_items;
  if ("compact_chars" in v) p.compactChars = v.compact_chars;
  if ("keep_roles" in v) p.keepRoles = v.keep_roles;
  if ("expand_groups" in v) p.expandGroups = v.expand_groups;
  if ("order" in v) p.order = v.order;
  return p;
}

function call(request: Json): ContextPack {
  return packContext((request.hits ?? []).map(item), policy(request.policy), (request.context ?? []).map(item));
}

function asWire(pack: ContextPack): Json {
  return {
    text: pack.text,
    chars_used: pack.charsUsed,
    text_chars: pack.textChars,
    items: pack.items.map((e) => ({
      id: e.id,
      source: e.source,
      tier: e.tier,
      chars: e.chars,
      truncated: e.truncated,
      forced: e.forced,
      rendered_index: e.renderedIndex,
      parts: e.parts.map((p) => ({ role: p.role, chars: p.chars, of_chars: p.ofChars })),
    })),
  };
}

for (const c of fixtures.pack_cases) {
  test(`fixture case ${c.name} matches through TypeScript`, () => {
    assert.deepStrictEqual(asWire(call(c.request)), c.expected);
  });
}

test("scalar counts differ from UTF-16 length (AC41-10)", () => {
  const c = fixtures.pack_cases.find((x: Json) => x.name === "scalar_values_on_non_ascii_text");
  const pack = call(c.request);
  assert.equal(pack.textChars, 17);
  assert.equal(pack.text.length, c.utf16_length);
  assert.notEqual(pack.text.length, pack.textChars);
});

test("the default policy is the documented product default", () => {
  assert.deepStrictEqual(
    {
      budget_chars: DEFAULT_CONTEXT_POLICY.budgetChars,
      item_chars: DEFAULT_CONTEXT_POLICY.itemChars,
      full_items: DEFAULT_CONTEXT_POLICY.fullItems,
      compact_chars: DEFAULT_CONTEXT_POLICY.compactChars,
      keep_roles: [...DEFAULT_CONTEXT_POLICY.keepRoles],
      full_ids: [...DEFAULT_CONTEXT_POLICY.fullIds],
      expand_groups: DEFAULT_CONTEXT_POLICY.expandGroups,
      order: DEFAULT_CONTEXT_POLICY.order,
    },
    fixtures.defaults,
  );
  const hits: ContextItem[] = [{ id: "a", parts: [{ role: "user", text: "x" }] }];
  assert.deepStrictEqual(packContext(hits), packContext(hits, { ...DEFAULT_CONTEXT_POLICY }));
});

for (const e of fixtures.pack_errors.filter((x: Json) => !x.json_only)) {
  test(`invalid input ${e.name} is InvalidArgumentError naming the field`, () => {
    assert.throws(
      () => call(e.request),
      (err: unknown) => err instanceof InvalidArgumentError && err.message.includes(e.message_contains),
    );
  });
}

test("a lone surrogate is InvalidArgumentError", () => {
  assert.throws(
    () => packContext([{ id: "a", parts: [{ role: "user", text: "\ud800" }] }]),
    InvalidArgumentError,
  );
  assert.throws(() => decodeRoundBody("\ud800"), InvalidArgumentError);
});

for (const c of fixtures.round_bodies) {
  test(`round body ${c.name}`, () => {
    const parts = c.parts.map(part);
    const body = encodeRoundBody(parts);
    assert.equal(body, c.body);
    assert.deepStrictEqual(decodeRoundBody(body), parts);
  });
}

for (const c of fixtures.untagged_bodies) {
  test(`untagged body ${c.name} decodes to one role-null part`, () => {
    assert.deepStrictEqual(decodeRoundBody(c.body), c.parts.map(part));
  });
}

for (const e of fixtures.encode_errors) {
  test(`encode error ${e.name}`, () => {
    assert.throws(
      () => encodeRoundBody(e.parts.map(part)),
      (err: unknown) => err instanceof InvalidArgumentError && err.message.includes(e.message_contains),
    );
  });
}

for (const e of fixtures.decode_errors) {
  test(`decode error ${e.name}`, () => {
    assert.throws(
      () => decodeRoundBody(e.body),
      (err: unknown) => err instanceof InvalidArgumentError && err.message.includes(e.message_contains),
    );
  });
}

async function searchAfterProjection(engine: Engine, query: string): Promise<SearchHit[]> {
  const deadline = Date.now() + 10_000;
  for (;;) {
    const hits = (await engine.searchTextOnly(query)).results;
    if (hits.length > 0 || Date.now() > deadline) return hits;
    await new Promise((resolve) => setTimeout(resolve, 20));
  }
}

test("contextItemFromHit decodes tagged hits and keeps untagged ones (AC41-20)", async () => {
  const tagged = encodeRoundBody([
    { role: "user", text: "my cat is named tom" },
    { role: "assistant", text: "tom is a fine name" },
  ]);
  const engine = await Engine.open(freshDbPath());
  try {
    await engine.write([
      { kind: "round", body: tagged, logicalId: "round:1", sourceId: "session-7" },
      { kind: "round", body: "plain kettle note", logicalId: "round:2", sourceId: "session-7" },
      { kind: "round", body: "\u001eXbroken kazoo", logicalId: "round:3", sourceId: "session-7" },
    ]);
    const hits = await searchAfterProjection(engine, "cat");
    assert.equal(hits.length, 1);
    const it = contextItemFromHit(hits[0]!, {
      group: "session-7",
      position: 1,
      orderKey: "2023/01/02",
      header: "[Session date: 2023/01/02]",
    });
    assert.equal(it.id, "l:round:1");
    assert.deepStrictEqual(it.parts, [
      { role: "user", text: "my cat is named tom" },
      { role: "assistant", text: "tom is a fine name" },
    ]);
    const p: ContextPolicy = { fullItems: 0, compactChars: 3 };
    assert.equal(packContext([it], p).text, "[Session date: 2023/01/02]\nmy cat is named tom\ntom");

    const plain = contextItemFromHit((await searchAfterProjection(engine, "kettle"))[0]!);
    assert.deepStrictEqual(plain.parts, [{ role: null, text: "plain kettle note" }]);
    assert.equal(packContext([plain], p).text, "plain kettle note");

    const broken = (await searchAfterProjection(engine, "kazoo"))[0]!;
    assert.throws(() => contextItemFromHit(broken), InvalidArgumentError);
  } finally {
    await engine.close();
  }
});
