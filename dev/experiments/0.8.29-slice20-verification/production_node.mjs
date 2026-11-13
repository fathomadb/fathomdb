import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { join, resolve, sep } from "node:path";
import { pathToFileURL } from "node:url";

const root = resolve(process.env.S20_SOURCE);
const entry = join(root, "src/ts/dist/index.js");
const { Engine } = await import(pathToFileURL(entry));
const require = createRequire(import.meta.url);
const native = Object.keys(require.cache).filter((path) => path.endsWith(".node"));
assert.equal(native.length, 1);
assert.ok(resolve(native[0]).startsWith(join(root, "src/ts") + sep));
const directory = mkdtempSync(join(process.env.TMPDIR, "s20-production-ts-"));
const database = join(directory, "store.sqlite");
let engine = await Engine.open(database, { useDefaultEmbedder: false });
try {
  await engine.write([{ kind: "doc", body: "s20harborcanary local proof", logicalId: "s20-proof", sourceId: "s20-proof-source" }]);
  const result = await engine.searchTextOnly("s20harborcanary");
  assert.deepEqual(result.results.map((hit) => [hit.id.space, hit.id.value]), [["logical", "s20-proof"]]);
} finally {
  await engine.close();
}
engine = await Engine.open(database, { useDefaultEmbedder: false });
try {
  const result = await engine.searchTextOnly("s20harborcanary");
  assert.deepEqual(result.results.map((hit) => [hit.id.space, hit.id.value]), [["logical", "s20-proof"]]);
} finally {
  await engine.close();
}
console.log(JSON.stringify({ source_sha: execFileSync("git", ["rev-parse", "HEAD"], { cwd: root, encoding: "utf8" }).trim(), entry,
  native: native[0], native_sha256: createHash("sha256").update(readFileSync(native[0])).digest("hex"),
  node: process.version, lifecycle: "write/text-search/close/reopen/text-search/close/normal-exit" }));
