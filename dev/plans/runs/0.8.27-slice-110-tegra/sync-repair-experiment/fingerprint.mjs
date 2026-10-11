// Embedding fingerprint: compares numeric output across allocator paths.
import { Engine } from "<worktree>/src/ts/dist/index.js";
import { mkdtempSync } from "node:fs";
import { createHash } from "node:crypto";
const d = mkdtempSync("<scratch>/repair/dbs/fp-");
try {
  const e = await Engine.open(d + "/t.fdb", { useDefaultEmbedder: true });
  const v = await e.embed("fathomdb sync allocator repair experiment");
  const h = createHash("sha256").update(Buffer.from(new Float64Array(v).buffer)).digest("hex");
  console.log(JSON.stringify({ outcome: "pass", dim: v.length, sha: h.slice(0, 16), head: v.slice(0, 3) }));
  await e.close();
} catch (err) {
  console.log(JSON.stringify({ outcome: "fail", message: String(err.message).slice(0, 160) }));
}
