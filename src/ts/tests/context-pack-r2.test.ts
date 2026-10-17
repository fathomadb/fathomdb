// 0.8.28 Slice 41 R2 code-review fix: integer fields that are not safe
// integers (NaN, Infinity, beyond 2^53) are refused host-side with
// InvalidArgumentError naming the field, before JSON.stringify can turn them
// into null or round them.

import test from "node:test";
import assert from "node:assert/strict";

import { InvalidArgumentError, packContext } from "../src/index.js";
import type { ContextItem, ContextPolicy } from "../src/index.js";

const part = [{ role: "user", text: "x" }];
const bad = [Number.NaN, Number.POSITIVE_INFINITY, 2 ** 53 + 1];

function refusedNaming(fn: () => unknown, field: string): void {
  assert.throws(fn, (err: unknown) => err instanceof InvalidArgumentError && err.message.includes(field));
}

for (const value of bad) {
  test(`hit position ${value} is refused naming the field`, () => {
    const hits: ContextItem[] = [{ id: "a", parts: part, position: value }];
    refusedNaming(() => packContext(hits), "hits[0].position");
  });

  test(`context position ${value} is refused naming the field`, () => {
    const hits: ContextItem[] = [{ id: "a", parts: part, group: "g", position: 0 }];
    const context: ContextItem[] = [{ id: "b", parts: part, group: "g", position: value }];
    refusedNaming(() => packContext(hits, {}, context), "context[0].position");
  });

  for (const field of ["budgetChars", "itemChars", "fullItems", "compactChars"] as const) {
    test(`policy ${field} ${value} is refused naming the field`, () => {
      const policy: ContextPolicy = { [field]: value };
      refusedNaming(() => packContext([], policy), `policy.${field}`);
    });
  }
}

test("a non-array keepRoles is a TypeError, not split into characters", () => {
  const hits: ContextItem[] = [{ id: "a", parts: part }];
  assert.throws(
    () => packContext(hits, { keepRoles: "assistant" as unknown as string[] } as ContextPolicy),
    (err: unknown) => err instanceof TypeError && err.message.includes("policy.keepRoles"),
  );
});
