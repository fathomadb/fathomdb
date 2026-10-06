// One fresh Node process for scripts/tests/test_tegra_node_early_cuinit.sh.
//
// Imports the fathomdb package FIRST, then grows the JavaScript heap to
// HEAP_OBJECTS live objects, then opens an engine with the default embedder
// under the caller's forced FATHOMDB_EMBED_DEVICE=cuda:0 and embeds. On the
// Jetson AGX Orin 64 GB a heap of about a million objects scatters V8 pages
// through the window the CUDA driver must reserve in cuInit, so without CUDA
// initialisation at module load the open refuses with cuda_probe_failed.
//
// Env: FATHOMDB_TEGRA_NODE_PACKAGE (package root containing dist/index.js),
// HEAP_OBJECTS (default 1000000), DB_DIR (scratch directory).
// Prints exactly one JSON line; exit 0 iff the run passed.
import { mkdtempSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

const out = { outcome: "fail", step: "import", node: process.version };
const keepAlive = [];
let engine = null;
try {
  const pkg = process.env.FATHOMDB_TEGRA_NODE_PACKAGE;
  if (!pkg) throw new Error("FATHOMDB_TEGRA_NODE_PACKAGE is not set");
  const fathomdb = await import(pathToFileURL(join(pkg, "dist", "index.js")).href);

  out.step = "heap";
  const objects = Number(process.env.HEAP_OBJECTS ?? 1_000_000);
  for (let i = 0; i < objects; i++) keepAlive.push({ i, s: `obj-${i}`, a: [i, i + 1] });
  out.heapObjects = keepAlive.length;
  out.heapUsedMiB = Math.round(process.memoryUsage().heapUsed / 2 ** 20);

  out.step = "open";
  const dir = mkdtempSync(join(process.env.DB_DIR ?? process.cwd(), "early-cuinit-"));
  engine = await fathomdb.Engine.open(join(dir, "t.fdb"), { useDefaultEmbedder: true });
  const resolution = engine.openReport().embedderDeviceResolution;
  out.cudaCompiled = resolution?.cudaCompiled ?? null;
  out.device = resolution?.effectiveDevice?.kind ?? null;
  if (out.device !== "cuda") throw new Error(`effective device ${out.device}, expected cuda`);

  out.step = "embed";
  const vector = await engine.embed("early CUDA initialisation on Jetson");
  if (vector.length !== 384 || !vector.every(Number.isFinite)) {
    throw new Error(`bad embedding: dim=${vector.length}`);
  }
  out.step = "close";
  await engine.close();
  engine = null;
  out.outcome = "pass";
} catch (error) {
  out.error = {
    name: error?.name ?? null,
    code: error?.code ?? null,
    kind: error?.kind ?? null,
    message: String(error?.message ?? error),
  };
  process.exitCode = 1;
} finally {
  if (engine) {
    try {
      await engine.close();
    } catch {
      // the failure is already recorded
    }
  }
}
out.keepAlive = keepAlive.length;
console.log(JSON.stringify(out));
