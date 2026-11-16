// Installed lifecycle only; the observer owns every CUDA availability query.
import fs from "node:fs";
import path from "node:path";
import { Engine } from "fathomdb";

const root = process.env.FATHOMDB_CAPTURE_DIR;
const engine = await Engine.open(path.join(root, "lifecycle.fdb"), { useDefaultEmbedder: true });
const resolution = engine.openReport().embedderDeviceResolution;
const vector = await engine.embed("installed CUDA artifact explicit device lifecycle");
await engine.write([
  { kind: "doc", body: "cuda package rerank smoke one", sourceId: "nonuse" },
  { kind: "doc", body: "cuda package rerank smoke two", sourceId: "nonuse" },
]);
const result = await engine.search("cuda package rerank smoke", undefined, 2, undefined, undefined, undefined, undefined, { limit: 2 });
fs.writeFileSync(path.join(root, "ready.tmp"), JSON.stringify({
  pid: process.pid, policy: resolution.requestedPolicy,
  cuda_compiled: resolution.cudaCompiled,
  effective_device: resolution.effectiveDevice.kind,
  selected_uuid: resolution.selectedCudaUuid,
  module_path: import.meta.resolve("fathomdb"),
  vector_dim: vector.length, result_count: result.results.length,
  reranked: result.results.length === 2 && result.results.every(row => row.ceScore !== null),
}) + "\n");
fs.renameSync(path.join(root, "ready.tmp"), path.join(root, "ready.json"));
const deadline = Date.now() + 30_000;
while (!fs.existsSync(path.join(root, "release"))) {
  if (Date.now() > deadline) throw new Error("observer did not release the live consumer");
  await new Promise(resolve => setTimeout(resolve, 100));
}
await engine.close();
fs.writeFileSync(path.join(root, "done.json"), JSON.stringify({ pid: process.pid, closed: true }) + "\n");
console.log("installed N-API lifecycle closed");
