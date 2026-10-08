// 0.8.28 Slice 30 qualification Node consumer: one fresh Node process, one
// JSON line on stdout. Adapted from the pool study's pool-consumer.mjs; the
// product reports its allocator decision in the open report (`cudaAllocator`
// on the CUDA device facts), so there is no stderr event to parse.
//
// Env: FATHOMDB_MODULE (module specifier; default "fathomdb"),
// FATHOMDB_DB_SCRATCH (directory for the per-run database), IMPORT_ORDER
// (early|late), HEAP_OBJECTS (live objects before open), HEAP_GROW_AFTER_OPEN,
// CONSUMER_MODE (import|open|full|perf|cycles|soak), SOAK_SECONDS and
// SOAK_MEM_FLOOR_MIB (soak), WARMUP_ITERS (5), TIMED_ITERS (50), BATCH_SIZES
// (1,8,32,128), CYCLES (cycles), IDLE_AFTER_CLOSE_S (seconds to idle after the
// last close), OVERSIZE_BATCH (full: one embedBatchCls of that many long
// passages after the first embed, then a check that embedding still runs on
// CUDA with the same hash), EXPECT_DEVICE (cuda|cpu; default cuda),
// EXPECT_PATH (private|default_pool|synchronous|any; default any). The runner
// sets the device policy and the FATHOMDB_POOL_* variables and merges the host
// record after the process exits.
import { mkdtempSync, readFileSync } from "node:fs";
import { performance } from "node:perf_hooks";
import { createHash } from "node:crypto";

const env = process.env;
const num = (name, dflt) => Number(env[name] ?? dflt);
const out = {
  outcome: "fail",
  failedStep: null,
  error: null,
  node: process.version,
  label: env.QUAL_LABEL ?? null,
  poolMode: env.FATHOMDB_POOL_MODE ?? null,
  earlyInit: env.FATHOMDB_CUDA_EARLY_INIT ?? null,
  order: env.IMPORT_ORDER ?? "early",
  consumerMode: env.CONSUMER_MODE ?? "full",
  pid: process.pid,
  heapObjects: null,
  heapUsedMiB: null,
  mem: { points: {} },
  oversize: null,
  timingsMs: {
    import: null, open: null, embedFirst: null, embedSteady: [], embedBatch: {},
    rerankFirst: null, rerankSteady: [], cyclesOpen: [], cyclesClose: [],
  },
  embedderDevice: null,
  embedderReason: null,
  rerankerDevice: null,
  allocator: null,
  rerankerAllocator: null,
  allocatorsLater: [],
  embedSha: null,
  rerankScores: null,
};
const errInfo = (err) => ({
  name: err?.name ?? null,
  code: err?.code ?? null,
  ordinal: err?.ordinal ?? null,
  maxSizeBytes: err?.maxSizeBytes ?? null,
  message: String(err?.message ?? err).slice(0, 300),
});
const memAvailMiB = () => {
  const m = readFileSync("/proc/meminfo", "utf8").match(/^MemAvailable:\s+(\d+)/m);
  return m ? Number(m[1]) / 1024 : null;
};
const status = () => {
  const s = readFileSync("/proc/self/status", "utf8");
  const kib = (key) => Number((s.match(new RegExp(`^${key}:\\s+(\\d+)`, "m")) ?? [0, 0])[1]);
  return { rssMiB: kib("VmRSS") / 1024, swapMiB: kib("VmSwap") / 1024, vszMiB: kib("VmSize") / 1024, memAvailMiB: memAvailMiB() };
};
const point = (name) => {
  out.mem.points[name] = status();
};
const keepAlive = [];
const grow = (n) => {
  for (let i = 0; i < n; i++) keepAlive.push({ i, s: `obj-${i}`, a: [i, i + 1] });
};
const sha = (vec) =>
  createHash("sha256").update(Buffer.from(new Float64Array(vec).buffer)).digest("hex").slice(0, 16);
const passages = [
  { id: 1, body: "The Jetson AGX Orin shares DRAM between CPU and GPU.", score: 0.5 },
  { id: 2, body: "Stream-ordered allocation uses a per-device memory pool.", score: 0.4 },
];
const timeIt = async (fn) => {
  const t = performance.now();
  const value = await fn();
  return [performance.now() - t, value];
};
const allocatorOf = (resolution) => resolution?.effectiveDevice?.cudaDevice?.cudaAllocator ?? null;
const allocKey = (a) => (a ? `${a.path}/${a.reason}` : "none");

let step = "import";
let mod = null;
let engine = null;
const doImport = async () => {
  step = "import";
  const before = status();
  const [ms, m] = await timeIt(() => import(env.FATHOMDB_MODULE ?? "fathomdb"));
  mod = m;
  out.timingsMs.import = ms;
  const after = status();
  out.mem.importRssDeltaMiB = after.rssMiB - before.rssMiB;
};
const openEngine = async () => {
  const dir = mkdtempSync(`${env.FATHOMDB_DB_SCRATCH}/db-`);
  return mod.Engine.open(`${dir}/t.fdb`, { useDefaultEmbedder: true });
};
const checkVec = (vec) => {
  if (vec.length !== 384 || !vec.every(Number.isFinite)) throw new Error(`bad embedding: dim=${vec.length}`);
};
const checkRanked = (ranked) => {
  if (ranked.length !== 2 || ranked.some((r) => r.ceScore === null || !Number.isFinite(r.ceScore))) {
    throw new Error(`bad rerank ${JSON.stringify(ranked)}`);
  }
};
// Reads the decision from the open report; with EXPECT_PATH set, a different
// path fails the run (G1: every FathomDB context is on the expected path).
const readReport = (first) => {
  const report = engine.openReport();
  const e = report.embedderDeviceResolution;
  const r = report.rerankerDeviceResolution;
  if (first) {
    out.embedderDevice = e?.effectiveDevice?.kind ?? null;
    out.embedderReason = e?.reason ?? null;
    out.rerankerDevice = r?.effectiveDevice?.kind ?? null;
    out.allocator = allocatorOf(e);
    out.rerankerAllocator = allocatorOf(r);
  } else {
    out.allocatorsLater.push(allocKey(allocatorOf(e)));
  }
  return report;
};
const expectPath = env.EXPECT_PATH ?? "any";
const checkPath = () => {
  if (expectPath !== "any" && out.allocator?.path !== expectPath) {
    throw new Error(`allocator path ${out.allocator?.path}, want ${expectPath}`);
  }
};

try {
  out.mem.startVszMiB = status().vszMiB;
  point("start");
  if (out.order !== "late") await doImport();
  step = "heap";
  grow(num("HEAP_OBJECTS", 0));
  if (out.order === "late") await doImport();
  out.heapObjects = keepAlive.length;
  out.heapUsedMiB = Math.round(process.memoryUsage().heapUsed / 2 ** 20);
  point("beforeOpen");
  if (out.consumerMode === "import") {
    out.outcome = "pass";
  } else if (out.consumerMode === "soak") {
    // G8: loop for SOAK_SECONDS; every 60 s grow and drop 200k JS objects,
    // collect garbage (--expose-gc) and record one sample. Stops itself,
    // failing, if MemAvailable falls below SOAK_MEM_FLOOR_MIB.
    const { rerank, embedBatchCls } = mod;
    const seconds = num("SOAK_SECONDS", 1200);
    const floorMiB = num("SOAK_MEM_FLOOR_MIB", 8192);
    step = "soak-open";
    engine = await openEngine();
    readReport(true);
    const text = "fathomdb sync allocator repair experiment";
    const want = sha(await engine.embed(text));
    out.embedSha = want;
    out.soak = { samples: [], iterations: 0, mismatches: 0, errors: [], aborted: null };
    const t0 = performance.now();
    let next = t0 + 60_000;
    let embedMs = [];
    let rerankMs = [];
    let batchMs = [];
    let i = 0;
    const med = (xs) => (xs.length ? [...xs].sort((a, b) => a - b)[xs.length >> 1] : null);
    while (performance.now() - t0 < seconds * 1000) {
      try {
        step = "soak-embed";
        let [ms, v] = await timeIt(() => engine.embed(text));
        embedMs.push(ms);
        if (sha(v) !== want) out.soak.mismatches++;
        const batch = Array.from({ length: 32 }, (_, k) => `soak ${i} item ${k} about pools`);
        [ms] = await timeIt(() => embedBatchCls(batch));
        batchMs.push(ms);
        step = "soak-rerank";
        [ms] = await timeIt(() => rerank("Which device shares DRAM?", passages, 2));
        rerankMs.push(ms);
        step = "soak-write";
        await engine.write(
          Array.from({ length: 50 }, (_, k) => ({ kind: "doc", body: `soak ${i} document ${k} about memory pools`, sourceId: "slice30-qual" })),
        );
      } catch (err) {
        out.soak.errors.push({ step, ...errInfo(err) });
      }
      i++;
      if (performance.now() >= next) {
        step = "soak-gc";
        grow(200_000);
        keepAlive.length = 0;
        if (globalThis.gc) globalThis.gc();
        const st = status();
        out.soak.samples.push({
          minute: Math.round((performance.now() - t0) / 60_000),
          rssMiB: st.rssMiB,
          swapMiB: st.swapMiB,
          memAvailMiB: st.memAvailMiB,
          heapUsedMiB: process.memoryUsage().heapUsed / 2 ** 20,
          embedMs: med(embedMs),
          batch32Ms: med(batchMs),
          rerankMs: med(rerankMs),
          iterations: embedMs.length,
        });
        embedMs = [];
        rerankMs = [];
        batchMs = [];
        next += 60_000;
        if (st.memAvailMiB !== null && st.memAvailMiB < floorMiB) {
          out.soak.aborted = `MemAvailable ${Math.round(st.memAvailMiB)} MiB below ${floorMiB} MiB`;
          break;
        }
      }
    }
    out.soak.iterations = i;
    readReport(false);
    step = "soak-close";
    await engine.close();
    engine = null;
    out.outcome = out.soak.mismatches === 0 && out.soak.aborted === null && out.soak.errors.length === 0 ? "pass" : "fail";
    if (out.outcome === "fail") out.failedStep = out.soak.aborted ? "soak-floor" : "soak-identity-or-error";
  } else if (out.consumerMode === "cycles") {
    // G3: CYCLES open/embed/close cycles, no garbage collection; the report of
    // every cycle is read, so each cycle's allocator path is recorded.
    const paths = new Set();
    for (let c = 0; c < num("CYCLES", 100); c++) {
      step = `cycle-${c}-open`;
      const [openMs, e] = await timeIt(openEngine);
      engine = e;
      out.timingsMs.cyclesOpen.push(openMs);
      const rep = engine.openReport().embedderDeviceResolution;
      paths.add(allocKey(allocatorOf(rep)));
      step = `cycle-${c}-embed`;
      checkVec(await engine.embed(`cycle ${c} embedding`));
      step = `cycle-${c}-close`;
      const [closeMs] = await timeIt(() => engine.close());
      engine = null;
      out.timingsMs.cyclesClose.push(closeMs);
      (out.cyclesRssMiB ??= []).push(Math.round(status().rssMiB * 100) / 100);
    }
    out.cyclePaths = [...paths];
    out.outcome = "pass";
  } else {
    const { rerank, embedBatchCls } = mod;
    const expect = env.EXPECT_DEVICE ?? "cuda";
    step = "open";
    const [openMs, e] = await timeIt(openEngine);
    engine = e;
    out.timingsMs.open = openMs;
    point("afterOpen");
    step = "openReport";
    readReport(true);
    if (expect !== "any" && out.embedderDevice !== expect) throw new Error(`embedder device ${out.embedderDevice}, want ${expect}`);
    checkPath();
    if (out.consumerMode !== "open") {
      step = "embed";
      const [firstMs, vec] = await timeIt(() => engine.embed("fathomdb sync allocator repair experiment"));
      out.timingsMs.embedFirst = firstMs;
      checkVec(vec);
      out.embedSha = sha(vec);
      const perf = out.consumerMode === "perf";
      const warm = perf ? num("WARMUP_ITERS", 5) : 0;
      const timed = perf ? num("TIMED_ITERS", 50) : 10;
      for (let i = 0; i < warm + timed; i++) {
        const [ms] = await timeIt(() => engine.embed(`steady state embedding number ${i} about tegra allocators`));
        if (i >= warm) out.timingsMs.embedSteady.push(ms);
      }
      point("afterEmbed");
      if (num("OVERSIZE_BATCH", 0) > 0) {
        step = "oversize";
        const n = num("OVERSIZE_BATCH", 0);
        const long = Array.from({ length: 400 }, (_, k) => `token${k}`).join(" ");
        const texts = Array.from({ length: n }, (_, k) => `${k} ${long}`);
        const o = { batch: n, ok: null, ms: null, error: null, embedAfterOk: null, deviceAfter: null, pathAfter: null, hashAfter: null, sameHash: null };
        const t = performance.now();
        try {
          await embedBatchCls(texts);
          o.ok = true;
        } catch (err) {
          o.ok = false;
          o.error = errInfo(err);
        }
        o.ms = performance.now() - t;
        point("afterOversize");
        step = "embedAfterOversize";
        const again = await engine.embed("fathomdb sync allocator repair experiment");
        checkVec(again);
        o.embedAfterOk = true;
        o.hashAfter = sha(again);
        o.sameHash = o.hashAfter === out.embedSha;
        const after = engine.openReport().embedderDeviceResolution;
        o.deviceAfter = after?.effectiveDevice?.kind ?? null;
        o.pathAfter = allocKey(allocatorOf(after));
        await embedBatchCls(["a short batch after the oversized one"]);
        out.oversize = o;
      }
      if (num("HEAP_GROW_AFTER_OPEN", 0) > 0) {
        step = "heapGrowAfterOpen";
        grow(num("HEAP_GROW_AFTER_OPEN", 0));
        readReport(false);
        step = "embedAfterGrow";
        const again = await engine.embed("fathomdb sync allocator repair experiment");
        checkVec(again);
        out.embedShaAfterGrow = sha(again);
      }
      if (perf) {
        step = "embedBatch";
        for (const size of (env.BATCH_SIZES ?? "1,8,32,128").split(",").map(Number)) {
          const texts = Array.from({ length: size }, (_, k) => `batch ${size} item ${k} about unified memory`);
          out.timingsMs.embedBatch[size] = [];
          for (let i = 0; i < warm + timed; i++) {
            const [ms, vecs] = await timeIt(() => embedBatchCls(texts));
            if (vecs.length !== size) throw new Error(`batch ${size} returned ${vecs.length}`);
            if (i >= warm) out.timingsMs.embedBatch[size].push(ms);
          }
        }
      }
      step = "rerank";
      const [rrMs, ranked] = await timeIt(() => rerank("How does CUDA stream-ordered allocation work?", passages, 2));
      out.timingsMs.rerankFirst = rrMs;
      checkRanked(ranked);
      out.rerankScores = ranked.map((r) => r.ceScore);
      const rWarm = perf ? warm : 0;
      const rTimed = perf ? timed : 1;
      for (let i = 0; i < rWarm + rTimed; i++) {
        const [ms, r] = await timeIt(() => rerank("Which device shares DRAM?", passages, 2));
        checkRanked(r);
        if (i >= rWarm) out.timingsMs.rerankSteady.push(ms);
      }
      if (num("HEAP_GROW_AFTER_OPEN", 0) > 0) {
        step = "rerankAfterGrow";
        checkRanked(await rerank("How does CUDA stream-ordered allocation work?", passages, 2));
        readReport(false);
      }
      point("afterRerank");
    }
    step = "close";
    await engine.close();
    engine = null;
    point("afterClose");
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
const idle = num("IDLE_AFTER_CLOSE_S", 0);
if (idle > 0) {
  await new Promise((resolve) => setTimeout(resolve, idle * 1000));
  point("afterIdle");
}
point("end");
out.keepAlive = keepAlive.length;
console.log(JSON.stringify(out));
