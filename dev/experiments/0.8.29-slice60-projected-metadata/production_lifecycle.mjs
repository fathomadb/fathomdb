// Supplemental production N-API lifecycle; never imports test hooks.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync, realpathSync } from "node:fs";
import { createRequire } from "node:module";
import { join, relative, isAbsolute } from "node:path";
import { pathToFileURL } from "node:url";
const source = realpathSync(process.env.S60_SOURCE);
const task = realpathSync(process.env.S60_TASK);
const database = process.argv[2];
const relation = relative(task, database);
assert(!relation.startsWith("..") && !isAbsolute(relation));
const { Engine } = await import(pathToFileURL(join(source, "src/ts/dist/index.js")));
for (let index = 0; index < 2; index += 1) {
  const engine = await Engine.open(database, { useDefaultEmbedder: false });
  assert.equal(engine.openReport().embedderDeviceResolution, null);
  await engine.close();
}
const require = createRequire(import.meta.url);
const loaded = Object.keys(require.cache).filter((path) => path.endsWith(".node") && path.includes("fathomdb."));
assert.equal(loaded.length, 1);
assert(realpathSync(loaded[0]).startsWith(join(source, "src/ts/")));
const binding = require(loaded[0]);
const names = [...Object.keys(binding), ...Object.getOwnPropertyNames(binding.Engine.prototype)];
assert.deepEqual(names.filter((name) => /(?:write|configure)VectorForTest|setLegacyProjectionSearchSubobjectsForTest|forcePanicForTest/.test(name)), []);
console.log(JSON.stringify({ kind: "production-typescript", module: loaded[0], sha256: createHash("sha256").update(readFileSync(loaded[0])).digest("hex"), open_close_cycles: 2, use_default_embedder: false }));
