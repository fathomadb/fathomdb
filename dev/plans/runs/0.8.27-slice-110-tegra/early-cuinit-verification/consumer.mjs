// Slice 110 early-cuInit verification consumer: one fresh Node process.
// Derived from fix-verification/consumer.mjs and the early-init experiment's
// heap-growth consumer.
//
// Env:
//   FATHOMDB_MODULE      in-tree dist/index.js path, or "fathomdb" when run
//                        from an installed consumer directory
//   FATHOMDB_DB_SCRATCH  directory for the per-run database
//   IMPORT_ORDER         early (default): import, then grow the heap;
//                        late: grow the heap, then import
//                        (with `node --import fathomdb` the module is already
//                        loaded before this script runs, whatever the order)
//   HEAP_OBJECTS         live objects allocated before Engine.open (default 0)
//   CONSUMER_MODE        full (default): open, report, embed x11, rerank x2,
//                        close; import: import only, report its cost
//   EXPECT_DEVICE        cuda (default) or cpu: the embedder device required
// The runner sets FATHOMDB_EMBED_DEVICE / FATHOMDB_RERANK_DEVICE and, for
// forced CUDA, FATHOMDB_GPU_ALLOCATION_WITNESS=1. Prints one JSON line.
import { mkdtempSync, readFileSync } from "node:fs";
import { performance } from "node:perf_hooks";

const out = {
  outcome: "fail",
  failedStep: null,
  error: null,
  node: process.version,
  order: process.env.IMPORT_ORDER ?? "early",
  preloaded: process.execArgv.some((arg) => arg.startsWith("--import")),
  timingsMs: {},
  mem: {},
};
const median = (xs) => {
  const s = [...xs].sort((a, b) => a - b);
  return s.length % 2 ? s[(s.length - 1) / 2] : (s[s.length / 2 - 1] + s[s.length / 2]) / 2;
};
const errInfo = (err) => ({
  name: err?.name ?? null,
  code: err?.code ?? null,
  kind: err?.kind ?? null,
  message: String(err?.message ?? err),
});
const status = () => {
  const s = readFileSync("/proc/self/status", "utf8");
  const kib = (key) => Number((s.match(new RegExp(`^${key}:\\s+(\\d+)`, "m")) ?? [0, 0])[1]);
  return { rssMiB: kib("VmRSS") / 1024, vszMiB: kib("VmSize") / 1024 };
};
const keepAlive = [];
const grow = (n) => {
  for (let i = 0; i < n; i++) keepAlive.push({ i, s: `obj-${i}`, a: [i, i + 1] });
};

let step = "import";
let mod = null;
let engine = null;
const doImport = async () => {
  step = "import";
  const before = status();
  const t = performance.now();
  mod = await import(process.env.FATHOMDB_MODULE ?? "fathomdb");
  out.timingsMs.import = performance.now() - t;
  const after = status();
  out.mem.importRssDeltaMiB = Math.round(after.rssMiB - before.rssMiB);
  out.mem.importVszDeltaMiB = Math.round(after.vszMiB - before.vszMiB);
};
try {
  out.mem.startVszMiB = Math.round(status().vszMiB);
  const heapObjects = Number(process.env.HEAP_OBJECTS ?? 0);
  if (out.order !== "late") await doImport();
  step = "heap";
  grow(heapObjects);
  if (out.order === "late") await doImport();
  out.heapObjects = keepAlive.length;
  out.heapUsedMiB = Math.round(process.memoryUsage().heapUsed / 2 ** 20);
  if ((process.env.CONSUMER_MODE ?? "full") === "import") {
    out.outcome = "pass";
  } else {
    const { Engine, rerank } = mod;
    const expect = process.env.EXPECT_DEVICE ?? "cuda";
    step = "open";
    const dir = mkdtempSync(`${process.env.FATHOMDB_DB_SCRATCH}/db-`);
    let t = performance.now();
    engine = await Engine.open(`${dir}/t.fdb`, { useDefaultEmbedder: true });
    out.timingsMs.open = performance.now() - t;

    step = "openReport";
    const report = engine.openReport();
    out.embedderDevice = report.embedderDeviceResolution?.effectiveDevice?.kind ?? null;
    out.embedderReason = report.embedderDeviceResolution?.reason ?? null;
    out.rerankerDevice = report.rerankerDeviceResolution?.effectiveDevice?.kind ?? null;
    const witness = report.embedderGpuAllocationWitness ?? null;
    out.witnessDeltaMiB = witness ? Math.round(witness.deltaBytes / 2 ** 20) : null;
    if (out.embedderDevice !== expect) {
      throw new Error(`embedder device ${out.embedderDevice}, want ${expect}`);
    }
    if (expect === "cuda" && process.env.FATHOMDB_GPU_ALLOCATION_WITNESS === "1" && !witness) {
      throw new Error("no GPU allocation witness in open report");
    }

    step = "embed";
    t = performance.now();
    const vec = await engine.embed("fathomdb early cuInit verification");
    out.timingsMs.embedFirst = performance.now() - t;
    if (vec.length !== 384 || !vec.every(Number.isFinite)) {
      throw new Error(`bad embedding: dim=${vec.length}`);
    }
    out.embedHead = vec.slice(0, 3);
    const steady = [];
    for (let i = 0; i < 10; i++) {
      t = performance.now();
      await engine.embed(`steady state embedding number ${i} about tegra allocators`);
      steady.push(performance.now() - t);
    }
    out.timingsMs.embedSteadyMedian = median(steady);

    step = "rerank";
    const passages = [
      { id: 1, body: "The Jetson AGX Orin shares DRAM between CPU and GPU.", score: 0.5 },
      { id: 2, body: "Stream-ordered allocation uses a per-device memory pool.", score: 0.4 },
    ];
    const ranked = await rerank("How does CUDA stream-ordered allocation work?", passages, 2);
    if (ranked.length !== 2 || ranked.some((r) => r.ceScore === null || !Number.isFinite(r.ceScore))) {
      throw new Error(`bad rerank ${JSON.stringify(ranked)}`);
    }
    await rerank("Which device shares DRAM?", passages, 2);

    step = "close";
    await engine.close();
    engine = null;
    out.outcome = "pass";
  }
} catch (err) {
  out.failedStep = step;
  out.error = errInfo(err);
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
out.mem.endVszMiB = Math.round(status().vszMiB);
out.keepAlive = keepAlive.length;
console.log(JSON.stringify(out));
