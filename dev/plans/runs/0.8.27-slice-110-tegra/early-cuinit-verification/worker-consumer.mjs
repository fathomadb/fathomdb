// Slice 110 early-cuInit verification, fix round 3: the first load of the
// addon happens inside a worker_threads worker. The main thread never imports
// fathomdb. The worker imports it (so module registration, and with it the
// early cuInit, runs in the worker's env), grows its own heap to HEAP_OBJECTS
// live objects, then runs the same full path as consumer.mjs and posts the
// result. Env as for consumer.mjs (FATHOMDB_MODULE, FATHOMDB_DB_SCRATCH,
// HEAP_OBJECTS, EXPECT_DEVICE, device policies). Prints one JSON line.
import { isMainThread, parentPort, Worker } from "node:worker_threads";
import { mkdtempSync, readFileSync } from "node:fs";
import { performance } from "node:perf_hooks";

const vszMiB = () =>
  Number(readFileSync("/proc/self/status", "utf8").match(/^VmSize:\s+(\d+)/m)[1]) / 1024;
const median = (xs) => {
  const s = [...xs].sort((a, b) => a - b);
  return s.length % 2 ? s[(s.length - 1) / 2] : (s[s.length / 2 - 1] + s[s.length / 2]) / 2;
};

if (isMainThread) {
  const out = { node: process.version, mainImportedFathomdb: false };
  const result = await new Promise((resolve) => {
    const worker = new Worker(new URL(import.meta.url));
    worker.on("message", resolve);
    worker.on("error", (err) => resolve({ outcome: "fail", failedStep: "worker", error: { message: String(err) } }));
  });
  Object.assign(out, result);
  if (out.outcome !== "pass") process.exitCode = 1;
  console.log(JSON.stringify(out));
} else {
  const out = { outcome: "fail", failedStep: null, error: null, timingsMs: {}, mem: {} };
  let step = "import";
  let engine = null;
  const keepAlive = [];
  try {
    const before = vszMiB();
    let t = performance.now();
    const { Engine, rerank } = await import(process.env.FATHOMDB_MODULE ?? "fathomdb");
    out.timingsMs.import = performance.now() - t;
    out.mem.importVszDeltaMiB = Math.round(vszMiB() - before);

    step = "heap";
    const objects = Number(process.env.HEAP_OBJECTS ?? 0);
    for (let i = 0; i < objects; i++) keepAlive.push({ i, s: `obj-${i}`, a: [i, i + 1] });
    out.heapObjects = keepAlive.length;
    out.heapUsedMiB = Math.round(process.memoryUsage().heapUsed / 2 ** 20);

    step = "open";
    const dir = mkdtempSync(`${process.env.FATHOMDB_DB_SCRATCH}/db-`);
    t = performance.now();
    engine = await Engine.open(`${dir}/t.fdb`, { useDefaultEmbedder: true });
    out.timingsMs.open = performance.now() - t;
    step = "openReport";
    const report = engine.openReport();
    out.embedderDevice = report.embedderDeviceResolution?.effectiveDevice?.kind ?? null;
    const witness = report.embedderGpuAllocationWitness ?? null;
    out.witnessDeltaMiB = witness ? Math.round(witness.deltaBytes / 2 ** 20) : null;
    const expect = process.env.EXPECT_DEVICE ?? "cuda";
    if (out.embedderDevice !== expect) throw new Error(`embedder device ${out.embedderDevice}, want ${expect}`);
    if (expect === "cuda" && process.env.FATHOMDB_GPU_ALLOCATION_WITNESS === "1" && !witness) {
      throw new Error("no GPU allocation witness in open report");
    }

    step = "embed";
    t = performance.now();
    const vec = await engine.embed("fathomdb early cuInit verification");
    out.timingsMs.embedFirst = performance.now() - t;
    if (vec.length !== 384 || !vec.every(Number.isFinite)) throw new Error(`bad embedding: dim=${vec.length}`);
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
  } catch (err) {
    out.failedStep = step;
    out.error = { name: err?.name ?? null, code: err?.code ?? null, kind: err?.kind ?? null, message: String(err?.message ?? err) };
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
  parentPort.postMessage(out);
}
