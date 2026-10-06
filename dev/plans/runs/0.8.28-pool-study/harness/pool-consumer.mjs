// 0.8.28 pool study consumer (protocol section 4.2): one fresh Node process,
// one JSON line on stdout. Derived from the Slice 110 consumers
// (explicit-pool-experiment/consumer.mjs, early-cuinit-verification/consumer.mjs).
//
// Env: FATHOMDB_MODULE (module specifier; default "fathomdb"),
// FATHOMDB_DB_SCRATCH (directory for the per-run database), IMPORT_ORDER
// (early|late), HEAP_OBJECTS (live objects before open), HEAP_GROW_AFTER_OPEN,
// CONSUMER_MODE (import|open|full|perf|ingest|cycles|soak), SOAK_SECONDS and
// SOAK_MEM_FLOOR_MIB (soak mode), WARMUP_ITERS (5),
// TIMED_ITERS (50), BATCH_SIZES (1,8,32,128), INGEST_DOCS (10000), CYCLES (50),
// MAPS_DIR, EXPECT_DEVICE (cuda|cpu; default cuda), IDLE_AFTER_CLOSE_S (CB2:
// seconds to idle after the last close, so the exit teardown line shows what
// the pool kept), OVERSIZE_BATCH (CB3/CB4: in full mode, one embedBatchCls of
// that many long passages after the first embed, then a check that embedding
// still runs on CUDA with the same hash), GC_AFTER_CLOSE (cycles mode: collect
// garbage after each close; needs --expose-gc), TRIM_WAIT_MS and CYCLES (trimcycle
// mode: idle long enough for the trim arm, then embed/rerank/batch again and
// check hashes; reopen each cycle), STRESS_ITERS, STRESS_CONCURRENCY and
// STRESS_WORKERS (trimstress mode: concurrent embeds on the libuv pool and in
// Node worker threads while the trim arm trims every tick). The runner sets the device
// policy, the witness and the FATHOMDB_POOL_* variables, and merges allocMode,
// poolEvents and host into this JSON after the process exits.
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { performance } from "node:perf_hooks";
import { createHash } from "node:crypto";
import { Worker, isMainThread, parentPort, workerData } from "node:worker_threads";

const env = process.env;

// trimstress worker: its own import and engine; embeds and reports hashes.
if (!isMainThread) {
  const { modulePath, dbDir, iters, texts } = workerData;
  const m = await import(modulePath);
  const e = await m.Engine.open(`${dbDir}/w.fdb`, { useDefaultEmbedder: true });
  const hashes = [];
  for (let i = 0; i < iters; i++) {
    const v = await e.embed(texts[i % texts.length]);
    hashes.push(createHash("sha256").update(Buffer.from(new Float64Array(v).buffer)).digest("hex").slice(0, 16));
  }
  await e.close();
  parentPort.postMessage(hashes);
}
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
// System-wide MemAvailable (MiB), for the CB1 sanity check and the soak floor.
const memAvailMiB = () => {
  const m = readFileSync("/proc/meminfo", "utf8").match(/^MemAvailable:\s+(\d+)/m);
  return m ? Number(m[1]) / 1024 : null;
};
const status = () => {
  const s = readFileSync("/proc/self/status", "utf8");
  const kib = (key) => Number((s.match(new RegExp(`^${key}:\\s+(\\d+)`, "m")) ?? [0, 0])[1]);
  return { rssMiB: kib("VmRSS") / 1024, swapMiB: kib("VmSwap") / 1024, vszMiB: kib("VmSize") / 1024, memAvailMiB: memAvailMiB() };
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

if (isMainThread) try {
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
  } else if (out.consumerMode === "trimcycle") {
    const { rerank, embedBatchCls } = mod;
    const text = "fathomdb sync allocator repair experiment";
    out.trim = { cycles: [], hashes: new Set(), clsHashes: new Set(), scores: new Set() };
    for (let c = 0; c < num("CYCLES", 5); c++) {
      step = `trimcycle-${c}-open`;
      engine = await openEngine();
      const cyc = {};
      step = `trimcycle-${c}-warm`;
      out.trim.hashes.add(sha(await engine.embed(text)));
      out.trim.scores.add(JSON.stringify((await rerank("Which device shares DRAM?", passages, 2)).map((r) => r.ceScore)));
      out.trim.clsHashes.add(sha((await embedBatchCls([text]))[0]));
      point(`trimcycle${c}Warm`);
      step = `trimcycle-${c}-idle`;
      await new Promise((resolve) => setTimeout(resolve, num("TRIM_WAIT_MS", 1000)));
      point(`trimcycle${c}Idle`);
      step = `trimcycle-${c}-regrow`;
      let [ms, v] = await timeIt(() => engine.embed(text));
      cyc.embedMs = ms;
      out.trim.hashes.add(sha(v));
      [ms, v] = await timeIt(() => rerank("Which device shares DRAM?", passages, 2));
      cyc.rerankMs = ms;
      out.trim.scores.add(JSON.stringify(v.map((r) => r.ceScore)));
      [ms, v] = await timeIt(() => embedBatchCls([text]));
      cyc.clsMs = ms;
      out.trim.clsHashes.add(sha(v[0]));
      [ms] = await timeIt(() => engine.embed(text));
      cyc.embedSteadyMs = ms;
      step = `trimcycle-${c}-close`;
      await engine.close();
      engine = null;
      out.trim.cycles.push(cyc);
    }
    out.trim.hashes = [...out.trim.hashes];
    out.trim.clsHashes = [...out.trim.clsHashes];
    out.trim.scores = [...out.trim.scores];
    out.outcome = out.trim.hashes.length === 1 && out.trim.clsHashes.length === 1 && out.trim.scores.length === 1 ? "pass" : "fail";
    if (out.outcome === "fail") out.failedStep = "trimcycle-identity";
  } else if (out.consumerMode === "trimstress") {
    const texts = Array.from({ length: 8 }, (_, k) => `stress text ${k} about stream-ordered pools`);
    step = "trimstress-open";
    engine = await openEngine();
    const expect = [];
    for (const t of texts) expect.push(sha(await engine.embed(t)));
    const iters = num("STRESS_ITERS", 200);
    const conc = num("STRESS_CONCURRENCY", 4);
    const nWorkers = num("STRESS_WORKERS", 2);
    step = "trimstress-run";
    const workers = Array.from({ length: nWorkers }, (_, k) => {
      const dbDir = mkdtempSync(`${env.FATHOMDB_DB_SCRATCH}/db-w${k}-`);
      return new Promise((resolve, reject) => {
        const w = new Worker(new URL(import.meta.url), {
          workerData: { modulePath: env.FATHOMDB_MODULE ?? "fathomdb", dbDir, iters: Math.floor(iters / 2), texts },
        });
        w.once("message", resolve);
        w.once("error", reject);
      });
    });
    let mismatches = 0;
    const lanes = Array.from({ length: conc }, async (_, lane) => {
      for (let i = lane; i < iters; i += conc) {
        const h = sha(await engine.embed(texts[i % texts.length]));
        if (h !== expect[i % texts.length]) mismatches++;
      }
    });
    await Promise.all(lanes);
    const workerHashes = await Promise.all(workers);
    let workerMismatches = 0;
    for (const hs of workerHashes) hs.forEach((h, i) => { if (h !== expect[i % texts.length]) workerMismatches++; });
    out.stress = { iters, conc, nWorkers, mismatches, workerMismatches, workerEmbeds: workerHashes.reduce((a, h) => a + h.length, 0) };
    step = "trimstress-close";
    await engine.close();
    engine = null;
    out.outcome = mismatches === 0 && workerMismatches === 0 ? "pass" : "fail";
    if (out.outcome === "fail") out.failedStep = "trimstress-identity";
  } else if (out.consumerMode === "soak") {
    // R6 (protocol 4.7, ruling 4): loop for SOAK_SECONDS; every 60 s grow and
    // drop 200k JS objects, collect garbage (--expose-gc) and record one
    // sample. Stops itself, failing, if MemAvailable falls below
    // SOAK_MEM_FLOOR_MIB (the study's 8 GiB floor), so no watcher has to.
    const { rerank, embedBatchCls } = mod;
    const seconds = num("SOAK_SECONDS", 1200);
    const floorMiB = num("SOAK_MEM_FLOOR_MIB", 8192);
    step = "soak-open";
    engine = await openEngine();
    const text = "fathomdb sync allocator repair experiment";
    const want = sha(await engine.embed(text));
    out.embedSha = want;
    out.soak = { samples: [], iterations: 0, mismatches: 0, docs: 0, aborted: null };
    const t0 = performance.now();
    let next = t0 + 60_000;
    let embedMs = [];
    let rerankMs = [];
    let batchMs = [];
    let i = 0;
    const med = (xs) => (xs.length ? [...xs].sort((a, b) => a - b)[xs.length >> 1] : null);
    while (performance.now() - t0 < seconds * 1000) {
      step = "soak-embed";
      let [ms, v] = await timeIt(() => engine.embed(text));
      embedMs.push(ms);
      if (sha(v) !== want) out.soak.mismatches++;
      const batch = Array.from({ length: 32 }, (_, k) => "soak " + i + " item " + k + " about pools");
      [ms] = await timeIt(() => embedBatchCls(batch));
      batchMs.push(ms);
      step = "soak-rerank";
      [ms] = await timeIt(() => rerank("Which device shares DRAM?", passages, 2));
      rerankMs.push(ms);
      step = "soak-write";
      await engine.write(
        Array.from({ length: 50 }, (_, k) => ({ kind: "doc", body: "soak " + i + " document " + k + " about memory pools", sourceId: "pool-study" })),
      );
      out.soak.docs += 50;
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
          vszMiB: st.vszMiB,
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
          out.soak.aborted = "MemAvailable " + Math.round(st.memAvailMiB) + " MiB below " + floorMiB + " MiB";
          break;
        }
      }
    }
    out.soak.iterations = i;
    step = "soak-close";
    await engine.close();
    engine = null;
    out.outcome = out.soak.mismatches === 0 && out.soak.aborted === null ? "pass" : "fail";
    if (out.outcome === "fail") out.failedStep = out.soak.aborted ? "soak-floor" : "soak-identity";
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
      // C2 diagnosis: a closed engine keeps its embedder until the wrapper
      // is garbage-collected; GC_AFTER_CLOSE=1 (with --expose-gc) collects it.
      if (env.GC_AFTER_CLOSE === "1" && globalThis.gc) {
        globalThis.gc();
        await new Promise((resolve) => setImmediate(resolve));
      }
      (out.cyclesRssMiB ??= []).push(Math.round(status().rssMiB));
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
if (isMainThread) {
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
}
