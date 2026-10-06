// Slice 110 sync-repair experiment: full-path forced-CUDA consumer check.
// Env expected from the runner: FATHOMDB_EMBED_DEVICE=cuda:0,
// FATHOMDB_RERANK_DEVICE=cuda:0, FATHOMDB_GPU_ALLOCATION_WITNESS=1,
// CUDARC_SLICE110_ALLOC_MODE=stock|sync|fallback.
import { Engine, rerank } from "<worktree>/src/ts/dist/index.js";
import { mkdtempSync } from "node:fs";
import { performance } from "node:perf_hooks";

const SCRATCH = "<scratch>/repair/dbs";
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
let step = "open";
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
console.log(JSON.stringify(out));
