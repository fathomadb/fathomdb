// Slice 110 explicit-pool experiment (derived from the sync-repair consumer).
// Extra env: HEAP_OBJECTS=N grows the JS heap with N live objects before Engine.open.
// Env expected from the runner: FATHOMDB_EMBED_DEVICE=cuda:0,
// FATHOMDB_RERANK_DEVICE=cuda:0, FATHOMDB_GPU_ALLOCATION_WITNESS=1,
// CUDARC_SLICE110_ALLOC_MODE=stock|sync|fallback.
import { Engine, rerank } from "<worktree>/src/ts/dist/index.js";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { performance } from "node:perf_hooks";
import { createHash } from "node:crypto";

const SCRATCH = "<scratch>/explicit-pool/dbs";
const out = {
  outcome: "fail",
  failedStep: null,
  error: null,
  node: process.version,
  mode: process.env.CUDARC_SLICE110_ALLOC_MODE ?? "stock",
  timingsMs: {},
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
let step = "heap";
const heapObjects = Number(process.env.HEAP_OBJECTS ?? 0);
const keepAlive = [];
for (let i = 0; i < heapObjects; i++) keepAlive.push({ i, s: `obj-${i}`, a: [i, i + 1] });
out.heapObjects = heapObjects;
out.heapUsedMiB = Math.round(process.memoryUsage().heapUsed / 2 ** 20);
if (process.env.MAPS_DIR) {
  writeFileSync(`${process.env.MAPS_DIR}/maps-${process.pid}-before-open.txt`, readFileSync("/proc/self/maps"));
}
out.pid = process.pid;
step = "open";
let engine = null;
try {
  const dir = mkdtempSync(`${SCRATCH}/db-`);
  let t = performance.now();
  engine = await Engine.open(`${dir}/t.fdb`, { useDefaultEmbedder: true });
  out.timingsMs.open = performance.now() - t;

  step = "openReport";
  const report = engine.openReport();
  out.embedderWarmupMs = report.embedderWarmupMs;
  out.embedderDevice = report.embedderDeviceResolution?.effectiveDevice?.kind ?? null;
  out.rerankerDevice = report.rerankerDeviceResolution?.effectiveDevice?.kind ?? null;
  out.witness = report.embedderGpuAllocationWitness;
  if (!out.witness) throw new Error("no GPU allocation witness in open report");

  step = "embed";
  t = performance.now();
  const vec = await engine.embed("fathomdb sync allocator repair experiment");
  out.timingsMs.embedFirst = performance.now() - t;
  out.dimension = vec.length;
  out.embedSha = createHash("sha256").update(Buffer.from(new Float64Array(vec).buffer)).digest("hex").slice(0, 16);
  if (vec.length !== 384 || !vec.every(Number.isFinite)) {
    throw new Error(`bad embedding: dim=${vec.length}`);
  }
  const steady = [];
  for (let i = 0; i < 10; i++) {
    t = performance.now();
    await engine.embed(`steady state embedding number ${i} about tegra allocators`);
    steady.push(performance.now() - t);
  }
  out.timingsMs.embedSteadyMedian = median(steady);
  out.timingsMs.embedSteadyMax = Math.max(...steady);

  step = "rerank";
  const passages = [
    { id: 1, body: "The Jetson AGX Orin shares DRAM between CPU and GPU.", score: 0.5 },
    { id: 2, body: "Stream-ordered allocation uses a per-device memory pool.", score: 0.4 },
  ];
  t = performance.now();
  const r1 = await rerank("How does CUDA stream-ordered allocation work?", passages, 2);
  out.timingsMs.rerankFirst = performance.now() - t;
  out.rerankScores = r1.map((r) => r.ceScore);
  if (r1.length !== 2 || r1.some((r) => r.ceScore === null || !Number.isFinite(r.ceScore))) {
    throw new Error(`bad rerank result ${JSON.stringify(r1)}`);
  }
  t = performance.now();
  await rerank("Which device shares DRAM?", passages, 2);
  out.timingsMs.rerankSecond = performance.now() - t;

  step = "close";
  await engine.close();
  engine = null;
  out.outcome = "pass";
} catch (err) {
  out.failedStep = step;
  out.error = errInfo(err);
  process.exitCode = 1;
} finally {
  if (engine) {
    try { await engine.close(); } catch { /* reported via failedStep */ }
  }
}
out.keepAlive = keepAlive.length;
console.log(JSON.stringify(out));
