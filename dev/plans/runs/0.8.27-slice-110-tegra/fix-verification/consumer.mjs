// Slice 110 allocator-fix verification: full-path forced-CUDA consumer.
// Derived from sync-repair-experiment/consumer.mjs; the module under test is
// FATHOMDB_MODULE (an in-tree dist/index.js path, or "fathomdb" when run from
// an installed consumer directory) and databases go under FATHOMDB_DB_SCRATCH.
// Runner sets FATHOMDB_EMBED_DEVICE / FATHOMDB_RERANK_DEVICE (cuda:0 or cpu)
// and, for CUDA, FATHOMDB_GPU_ALLOCATION_WITNESS=1.
import { mkdtempSync, readFileSync } from "node:fs";
import { performance } from "node:perf_hooks";

const { Engine, rerank } = await import(process.env.FATHOMDB_MODULE ?? "fathomdb");
const SCRATCH = process.env.FATHOMDB_DB_SCRATCH;
const expectCuda = process.env.FATHOMDB_EMBED_DEVICE !== "cpu";
const out = {
  outcome: "fail",
  failedStep: null,
  error: null,
  node: process.version,
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
// Largest anonymous PROT_NONE mapping and largest unmapped hole inside
// [8 GiB, 128 GiB) right after open (driver-isolation-evidence/results.md).
const vaWindow = () => {
  const GiB = 2 ** 30, LO = 8 * GiB, HI = 128 * GiB;
  const maps = readFileSync("/proc/self/maps", "utf8").trim().split("\n").map((line) => {
    const f = line.split(/\s+/);
    const [a, b] = f[0].split("-").map((x) => parseInt(x, 16));
    return { a, b, perms: f[1], name: f.slice(5).join(" ") };
  }).sort((x, y) => x.a - y.a);
  let cur = LO, gap = 0, piece = 0;
  for (const m of maps) {
    if (m.perms === "---p" && !m.name && m.a >= LO && m.a < HI) piece = Math.max(piece, m.b - m.a);
    if (m.b <= cur) continue;
    if (m.a >= HI) break;
    if (m.a > cur) gap = Math.max(gap, Math.min(m.a, HI) - cur);
    cur = Math.max(cur, m.b);
  }
  if (cur < HI) gap = Math.max(gap, HI - cur);
  return { maxNoneMappingGiB: +(piece / GiB).toFixed(2), maxHoleGiB: +(gap / GiB).toFixed(2) };
};
let step = "open";
let engine = null;
try {
  const dir = mkdtempSync(`${SCRATCH}/db-`);
  let t = performance.now();
  engine = await Engine.open(`${dir}/t.fdb`, { useDefaultEmbedder: true });
  out.timingsMs.open = performance.now() - t;
  out.va = vaWindow();

  step = "openReport";
  const report = engine.openReport();
  out.embedderDevice = report.embedderDeviceResolution?.effectiveDevice?.kind ?? null;
  out.rerankerDevice = report.rerankerDeviceResolution?.effectiveDevice?.kind ?? null;
  out.witness = report.embedderGpuAllocationWitness ?? null;
  const want = expectCuda ? "cuda" : "cpu";
  if (out.embedderDevice !== want) throw new Error(`embedder device ${out.embedderDevice}, want ${want}`);
  if (expectCuda && !out.witness) throw new Error("no GPU allocation witness in open report");

  step = "embed";
  t = performance.now();
  const vec = await engine.embed("fathomdb sync allocator repair experiment");
  out.timingsMs.embedFirst = performance.now() - t;
  out.dimension = vec.length;
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
