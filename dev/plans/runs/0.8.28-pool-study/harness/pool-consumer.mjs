// 0.8.28 pool study consumer (protocol section 4.2): one fresh Node process,
// one JSON line on stdout. Derived from the Slice 110 consumers
// (explicit-pool-experiment/consumer.mjs, early-cuinit-verification/consumer.mjs).
//
// Env: FATHOMDB_MODULE (module specifier; default "fathomdb"),
// FATHOMDB_DB_SCRATCH (directory for the per-run database), IMPORT_ORDER
// (early|late), HEAP_OBJECTS (live objects before open), HEAP_GROW_AFTER_OPEN,
// CONSUMER_MODE (import|open|full|perf|ingest|cycles), WARMUP_ITERS (5),
// TIMED_ITERS (50), BATCH_SIZES (1,8,32,128), INGEST_DOCS (10000), CYCLES (50),
// MAPS_DIR, EXPECT_DEVICE (cuda|cpu; default cuda), IDLE_AFTER_CLOSE_S (CB2:
// seconds to idle after the last close, so the exit teardown line shows what
// the pool kept), OVERSIZE_BATCH (CB3/CB4: in full mode, one embedBatchCls of
// that many long passages after the first embed, then a check that embedding
// still runs on CUDA with the same hash). The runner sets the device
// policy, the witness and the FATHOMDB_POOL_* variables, and merges allocMode,
// poolEvents and host into this JSON after the process exits.
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { performance } from "node:perf_hooks";
import { createHash } from "node:crypto";

const env = process.env;
const num = (name, dflt) => Number(env[name] ?? dflt);
const out = {
  outcome: "fail",
  failedStep: null,
  error: null,
  node: process.version,
  variant: env.FATHOMDB_POOL_VARIANT ?? "S",
  maxSize: env.FATHOMDB_POOL_MAXSIZE ?? null,
  releaseThreshold: env.FATHOMDB_POOL_RELEASE_THRESHOLD ?? null,
  order: env.IMPORT_ORDER ?? "early",
  preloaded: process.execArgv.some((arg) => arg.startsWith("--import")),
  consumerMode: env.CONSUMER_MODE ?? "full",
  pid: process.pid,
  heapObjects: null,
  heapUsedMiB: null,
  heapGrowAfterOpen: num("HEAP_GROW_AFTER_OPEN", 0) || null,
  mem: { points: {} },
  oversize: null,
  timingsMs: {
    import: null, open: null, embedFirst: null, embedSteady: [], embedBatch: {},
    rerankFirst: null, rerankSteady: [], ingestTotal: null, cyclesOpen: [], cyclesClose: [],
  },
  embedderDevice: null,
  embedderReason: null,
  rerankerDevice: null,
  witness: null,
  embedSha: null,
  rerankScores: null,
  ingest: null,
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
  return { rssMiB: kib("VmRSS") / 1024, swapMiB: kib("VmSwap") / 1024, vszMiB: kib("VmSize") / 1024 };
};
// VmRSS, VmSwap and VmSize at each named measurement point (protocol 4.2).
const point = (name) => {
  out.mem.points[name] = status();
};
const snapMaps = (stage) => {
  if (env.MAPS_DIR) writeFileSync(`${env.MAPS_DIR}/maps-${process.pid}-${stage}.txt`, readFileSync("/proc/self/maps"));
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
  out.mem.importVszDeltaMiB = after.vszMiB - before.vszMiB;
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
  snapMaps("before-open");
  if (out.consumerMode === "import") {
    out.outcome = "pass";
  } else if (out.consumerMode === "cycles") {
    for (let c = 0; c < num("CYCLES", 50); c++) {
      step = `cycle-${c}-open`;
      const [openMs, e] = await timeIt(openEngine);
      engine = e;
      out.timingsMs.cyclesOpen.push(openMs);
      step = `cycle-${c}-embed`;
      checkVec(await engine.embed(`cycle ${c} embedding`));
      step = `cycle-${c}-close`;
      const [closeMs] = await timeIt(() => engine.close());
      engine = null;
      out.timingsMs.cyclesClose.push(closeMs);
    }
    out.outcome = "pass";
  } else {
    const { rerank, embedBatchCls } = mod;
    const expect = env.EXPECT_DEVICE ?? "cuda";
    step = "open";
    const [openMs, e] = await timeIt(openEngine);
    engine = e;
    out.timingsMs.open = openMs;
    const afterOpen = status();
    out.mem.afterOpenRssMiB = afterOpen.rssMiB;
    out.mem.afterOpenVszMiB = afterOpen.vszMiB;
    point("afterOpen");
    snapMaps("after-open");

    step = "openReport";
    const report = engine.openReport();
    out.embedderDevice = report.embedderDeviceResolution?.effectiveDevice?.kind ?? null;
    out.embedderReason = report.embedderDeviceResolution?.reason ?? null;
    out.rerankerDevice = report.rerankerDeviceResolution?.effectiveDevice?.kind ?? null;
    const w = report.embedderGpuAllocationWitness ?? null;
    out.witness = w ? { deltaBytes: w.deltaBytes ?? null, floorBytes: w.floorBytes ?? null, outcome: w.outcome ?? null } : null;
    if (out.embedderDevice !== expect) throw new Error(`embedder device ${out.embedderDevice}, want ${expect}`);
    if (expect === "cuda" && env.FATHOMDB_GPU_ALLOCATION_WITNESS === "1" && !w) {
      throw new Error("no GPU allocation witness in open report");
    }
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
        const o = { batch: n, ok: null, ms: null, error: null, embedAfterOk: null, deviceAfter: null, hashAfter: null, sameHash: null };
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
        o.deviceAfter = engine.openReport().embedderDeviceResolution?.effectiveDevice?.kind ?? null;
        await embedBatchCls(["a short batch after the oversized one"]);
        out.oversize = o;
      }
      if (out.heapGrowAfterOpen) {
        step = "heapGrowAfterOpen";
        grow(out.heapGrowAfterOpen);
        snapMaps("after-grow");
        step = "embedAfterGrow";
        checkVec(await engine.embed("embedding after the heap grew"));
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
      point("afterRerank");
      if (out.consumerMode === "ingest") {
        step = "ingest";
        const docs = num("INGEST_DOCS", 10000);
        const t = performance.now();
        for (let i = 0; i < docs; i += 100) {
          const batch = [];
          for (let k = i; k < Math.min(docs, i + 100); k++) {
            batch.push({ kind: "doc", body: `ingest document ${k} about memory pools on unified memory`, sourceId: "pool-study" });
          }
          await engine.write(batch);
        }
        out.timingsMs.ingestTotal = performance.now() - t;
        out.ingest = { docs, docsPerSecond: docs / (out.timingsMs.ingestTotal / 1000) };
        point("afterIngest");
      }
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
const end = status();
out.mem.endRssMiB = end.rssMiB;
out.mem.endVszMiB = end.vszMiB;
out.keepAlive = keepAlive.length;
console.log(JSON.stringify(out));
