// One fresh Node process per row for smoke-tegra-node-package.sh.
//
// The smoke copies this file into the consumer it installed, so the bare
// `fathomdb` import resolves through that consumer's node_modules exactly as
// an application's would. It records facts, not verdicts:
// tegra_node_package_verdict.py decides whether the row passed.
//
// argv: <consumer-name> <select|embed|rerank> <policy|-> <out.json> <db-dir>
// The device policy is the caller's FATHOMDB_EMBED_DEVICE /
// FATHOMDB_RERANK_DEVICE; `policy` only labels the row.
// Exit 0 iff the row ran to completion and its record was written.
import { createHash } from "node:crypto";
import { mkdtempSync, readFileSync, realpathSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const [consumer, component, policyArg, outFile, dbDir] = process.argv.slice(2);
const out = {
  schema: "fathomdb.tegra-node-package-row/v1",
  consumer,
  consumer_root: realpathSync(process.cwd()),
  component,
  policy: policyArg === "-" ? null : policyArg,
  node: process.version,
  outcome: "error",
  step: "import",
  error: null,
};

// The addon the process actually mapped, from the kernel's view rather than
// the loader's own report.
function loadedNativeAddons() {
  const paths = new Set();
  for (const line of readFileSync("/proc/self/maps", "utf8").split("\n")) {
    const start = line.indexOf("/");
    if (start < 0) continue;
    const path = line.slice(start).trim();
    if (path.endsWith(".node")) paths.add(path);
  }
  return [...paths].sort();
}

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function resolution(value) {
  if (!value) return null;
  return {
    requested_policy: value.requestedPolicy,
    cuda_compiled: value.cudaCompiled,
    effective_device: value.effectiveDevice?.kind ?? null,
    selected_cuda_uuid: value.selectedCudaUuid,
    reason: value.reason,
  };
}

let engine = null;
try {
  const fathomdb = await import("fathomdb");
  out.loaded_native_addons = loadedNativeAddons();
  out.loaded_native_sha256 = Object.fromEntries(
    out.loaded_native_addons.map((path) => [path, sha256(path)]),
  );
  if (component === "embed") {
    out.step = "open";
    const dir = mkdtempSync(join(dbDir, "embed-"));
    engine = await fathomdb.Engine.open(join(dir, "smoke.fdb"), { useDefaultEmbedder: true });
    const report = engine.openReport();
    out.resolution = resolution(report.embedderDeviceResolution);
    out.embedder_events = report.embedderEvents.map((event) => event.kind);
    out.witness = report.embedderGpuAllocationWitness;
    out.witness_present = report.embedderGpuAllocationWitness !== null;
    out.step = "embed";
    const vector = await engine.embed("FathomDB installed Jetson Tegra package smoke");
    if (!vector.every(Number.isFinite)) throw new Error("embedding contains a non-finite value");
    out.embedding_dim = vector.length;
  } else if (component === "rerank") {
    // The reranker reports no GPU allocation witness. CUDA is shown only by
    // the open-time device resolution plus finite cross-encoder scores.
    out.step = "open";
    const dir = mkdtempSync(join(dbDir, "rerank-"));
    engine = await fathomdb.Engine.open(join(dir, "smoke.fdb"), { useDefaultEmbedder: false });
    const report = engine.openReport();
    out.resolution = resolution(report.rerankerDeviceResolution);
    out.embedder_events = report.embedderEvents.map((event) => event.kind);
    out.step = "rerank";
    const passages = [
      { id: 1, body: "The Jetson Orin integrates an Ampere GPU with the CPU.", score: 0.5 },
      { id: 2, body: "Bananas are a good source of potassium.", score: 0.5 },
    ];
    const results = await fathomdb.rerank("Which device has an integrated GPU?", passages, passages.length);
    out.passages = passages.length;
    out.ce_scores = results.map((result) => result.ceScore);
  } else if (component !== "select") {
    throw new Error(`unknown row component: ${component}`);
  }
  if (engine) {
    out.step = "close";
    await engine.close();
    engine = null;
  }
  out.step = "done";
  out.outcome = "completed";
} catch (error) {
  out.loaded_native_addons ??= loadedNativeAddons();
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
      // The row already records the original failure.
    }
  }
}
writeFileSync(outFile, `${JSON.stringify(out, null, 2)}\n`);
