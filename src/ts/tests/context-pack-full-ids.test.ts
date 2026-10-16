// 0.8.28 Slice 41 phase 2 — `ContextPolicy.fullIds` through the TypeScript SDK.
//
// Loads the hand-derived `full_ids_cases` and `full_ids_errors` from
// `src/conformance/context-pack-fixtures.json` (the Rust and Python suites load
// the same cases) and compares the text and every provenance field, `forced`
// included (AC41-28..31, AC41-33).

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { DEFAULT_CONTEXT_POLICY, InvalidArgumentError, packContext } from "../src/index.js";
import type { ContextItem, ContextPack, ContextPolicy } from "../src/index.js";

type Json = any; // eslint-disable-line @typescript-eslint/no-explicit-any

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..", "..", "..");
const fixtures: Json = JSON.parse(
  readFileSync(join(root, "src", "conformance", "context-pack-fixtures.json"), "utf8"),
);

function item(v: Json): ContextItem {
  return {
    id: v.id,
    parts: v.parts.map((p: Json) => ({ role: p.role, text: p.text })),
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
  if ("full_ids" in v) p.fullIds = v.full_ids;
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

for (const c of fixtures.full_ids_cases) {
  test(`full_ids case ${c.name} matches through TypeScript`, () => {
    assert.deepStrictEqual(asWire(call(c.request)), c.expected);
  });
}

for (const e of fixtures.full_ids_errors.filter((x: Json) => !x.json_only)) {
  test(`invalid full_ids ${e.name} is InvalidArgumentError naming the field`, () => {
    assert.throws(
      () => call(e.request),
      (err: unknown) =>
        err instanceof InvalidArgumentError &&
        err.message.includes(e.message_contains) &&
        (e.message_excludes === undefined || !err.message.includes(e.message_excludes)),
    );
  });
}

test("the default fullIds is empty", () => {
  assert.deepStrictEqual([...DEFAULT_CONTEXT_POLICY.fullIds], []);
});

test("a non-array fullIds is a TypeError", () => {
  const hits: ContextItem[] = [{ id: "h1", parts: [{ role: "user", text: "x" }] }];
  assert.throws(() => packContext(hits, { fullIds: "h1" as unknown as string[] }), TypeError);
});
