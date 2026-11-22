import assert from "node:assert/strict";
import { setTimeout as delay } from "node:timers/promises";

export async function clipboardEquals(page, expected) {
  const deadline = performance.now() + 5000;
  let actual;
  while (performance.now() < deadline) {
    let timer;
    try {
      actual = await Promise.race([
        page.evaluate(() => navigator.clipboard.readText()),
        new Promise((_, reject) => {
          timer = setTimeout(
            () => reject(Error("clipboard read deadline")),
            Math.max(1, deadline - performance.now()),
          );
        }),
      ]);
    } finally {
      clearTimeout(timer);
    }
    if (actual === expected) {
      assert.equal(actual, expected);
      return;
    }
    await delay(Math.min(10, Math.max(0, deadline - performance.now())));
  }
  assert.equal(actual, expected, "clipboard must contain the exact copied value before deadline");
}
