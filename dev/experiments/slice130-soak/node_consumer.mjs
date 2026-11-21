#!/usr/bin/env node
// Installed Node worker; stdout is reserved for bounded supervisor replies.
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";
import { realpathSync, mkdtempSync } from "node:fs";
import { pathToFileURL } from "node:url";
import { createInterface } from "node:readline";
import { createHash } from "node:crypto";
import { performance } from "node:perf_hooks";
import { getHeapStatistics, getHeapSpaceStatistics } from "node:v8";

const args = process.argv.slice(2);
const lifecycle = args.includes("--lifecycle");
const defaultProfile = args.includes("--node-default-profile");
const rootIndex = args.indexOf("--consumer-root");
if (rootIndex < 0 || !args[rootIndex + 1]) throw new Error("--consumer-root must name the fresh installed consumer");
const root = realpathSync(resolve(args[rootIndex + 1]));
const require = createRequire(join(root, "package.json"));
const main = realpathSync(require.resolve("fathomdb"));
if (!main.startsWith(`${root}/node_modules/`)) throw new Error("main package escaped installed consumer");
for (const policy of ["FATHOMDB_EMBED_DEVICE", "FATHOMDB_RERANK_DEVICE"]) {
  if (process.env[policy] !== "cuda:0") throw new Error(`${policy}=cuda:0 must be fixed before import`);
}
const sdk = await import(pathToFileURL(main).href);
const { native } = await import(pathToFileURL(join(dirname(main), "binding.js")).href);
const addon = realpathSync(require.resolve("fathomdb-linux-arm64-gnu-tegra"));
const loaded = Object.keys(require.cache).filter((name) => name.endsWith(".node")).map((name) => realpathSync(name));
if (!addon.startsWith(`${root}/node_modules/`) || loaded.length !== 1 || loaded[0] !== addon) {
  throw new Error("actual loaded addon differs from installed Tegra counterpart");
}
const TEXT = "fathomdb sync allocator repair experiment";
const PASSAGES = [
  { id: 1, body: "The Jetson AGX Orin shares DRAM between CPU and GPU.", score: 0.5 },
  { id: 2, body: "Stream-ordered allocation uses a per-device memory pool.", score: 0.4 },
];
const engines = [];
const expected = [];
let expectedScores;
let expectedBatch;
const FIXED_BATCH = Array.from({ length: 32 }, (_, k) => `fixed item ${k} about memory pools`);
let iteration = 0;
function hash(vector) {
  if (vector.length !== 384 || !vector.every(Number.isFinite)) throw new Error("invalid model embedding");
  const bytes = Buffer.alloc(vector.length * 8);
  vector.forEach((v, i) => bytes.writeDoubleLE(v, i * 8));
  return createHash("sha256").update(bytes).digest("hex");
}
async function openEngine() {
  const dir = mkdtempSync(join(process.env.FATHOMDB_DB_SCRATCH, "engine-"));
  const engine = await sdk.Engine.open(join(dir, "db.fdb"), { useDefaultEmbedder: true });
  if (engine.openReport().embedderDeviceResolution?.effectiveDevice?.kind !== "cuda") throw new Error("forced CUDA did not resolve CUDA");
  await engine.write([{ kind: "doc", body: TEXT, sourceId: "slice130-lifecycle" }]);
  await engine.drain(60_000);
  engines.push(engine);
  expected.push(hash(await engine.embed(TEXT)));
}
const lines = createInterface({ input: process.stdin, crlfDelay: Infinity });
for await (const line of lines) {
  const { operation } = JSON.parse(line);
  try {
    let result;
    switch (operation) {
      case "ready":
        if (!lifecycle) await openEngine();
        result = { pid: process.pid, package: main, native: addon, ...(defaultProfile ? { profile: "node-default-10m", exec_argv: process.execArgv, node_options: process.env.NODE_OPTIONS ?? null, node_version: process.version } : {}) };
        if (engines.length) {
          const resolution = engines[0].openReport().embedderDeviceResolution;
          result.allocator = resolution.effectiveDevice.cudaDevice?.cudaAllocator ?? null;
          result.selected_cuda_uuid = resolution.selectedCudaUuid;
        }
        break;
      case "open-one":
      case "open-two":
        for (let i = 0; i < (operation === "open-two" ? 2 : 1); i++) await openEngine();
        result = { outputs: expected };
        break;
      case "snapshot":
        result = { raw_observation: JSON.parse(native.cudaPoolObservationForQualification()), node_heap: process.memoryUsage(), ...(defaultProfile ? { node_v8: { heap: getHeapStatistics(), spaces: getHeapSpaceStatistics() } } : {}) };
        break;
      case "iteration": {
        const latency = {};
        let start = performance.now();
        const single = hash(await engines[0].embed(TEXT));
        latency.embed = performance.now() - start;
        start = performance.now();
        const batch = await sdk.embedBatchCls(defaultProfile ? FIXED_BATCH : Array.from({ length: 32 }, (_, k) => `soak ${iteration} item ${k} about pools`));
        latency.batch32 = performance.now() - start;
        if (batch.length !== 32) throw new Error("batch32 output count changed");
        const batchHashes = batch.map(hash);
        if (defaultProfile) expectedBatch ??= batchHashes;
        start = performance.now();
        const ranked = await sdk.rerank("Which device shares DRAM?", PASSAGES, 2);
        latency.rerank = performance.now() - start;
        const scores = ranked.map((r) => r.ceScore);
        if (scores.length !== 2 || !scores.every((v) => v !== null && Number.isFinite(v))) throw new Error("reranker output missing/nonfinite");
        expectedScores ??= scores;
        // Retain the historical Node workload's writes, without sampler churn.
        start = performance.now();
        await engines[0].write(Array.from({ length: 50 }, (_, k) => ({ kind: "doc", body: `soak ${iteration} document ${k} about memory pools`, sourceId: "slice30-qual" })));
        latency.write = performance.now() - start;
        result = { latency_ms: latency, mismatches: Number(single !== expected[0]) + Number(JSON.stringify(scores) !== JSON.stringify(expectedScores)) + Number(defaultProfile && JSON.stringify(batchHashes) !== JSON.stringify(expectedBatch)),
          outputs: { embed_sha256: single, batch32_sha256: batchHashes, rerank: ranked }, input_iteration: iteration++ };
        break;
      }
      case "survivor":
        result = { unchanged: hash(await engines.at(-1).embed(TEXT)) === expected.at(-1) };
        break;
      case "singleton-cls":
        result = { output_sha256: hash((await sdk.embedBatchCls([TEXT]))[0]) };
        break;
      case "singleton-reranker":
        result = { outputs: await sdk.rerank("Which device shares DRAM?", PASSAGES, 2) };
        break;
      case "close-first":
        await engines[0].close();
        result = { closed_monotonic_s: Number(process.hrtime.bigint()) / 1e9 };
        break;
      case "close-last":
        await engines.at(-1).close();
        result = { closed_monotonic_s: Number(process.hrtime.bigint()) / 1e9 };
        break;
      case "drop-all":
        throw new Error("Node scope exit has no deterministic native collection contract");
      case "close":
        for (const engine of engines) await engine.close();
        result = { closed_monotonic_s: Number(process.hrtime.bigint()) / 1e9 };
        break;
      case "exit":
        result = { unclosed_owners: engines.length };
        break;
      default:
        throw new Error(`unknown operation ${operation}`);
    }
    console.log(JSON.stringify({ operation, ok: true, result }));
    if (operation === "close" || operation === "exit") {
      lines.close();
      break;
    }
  } catch (error) {
    console.log(JSON.stringify({ operation, ok: false, error: String(error) }));
    process.exitCode = 1;
    lines.close();
    break;
  }
}
