/** Capture a forced CUDA reranker refusal from an installed N-API candidate. */

import { Engine, RerankerDevicePolicyError } from "fathomdb";

const argv = ["node", "/fathomdb-harness/forced-reranker-napi.mjs"];
const CANONICAL_JSON_KEYS = [
  "argv", "consumer", "effective_device", "error", "kind", "message", "ordinal",
  "reason", "requested_policy", "schema_version", "status", "type",
];

try {
  await Engine.open("/tmp/forced-reranker-cuda-napi.fdb", { useDefaultEmbedder: false });
  throw new Error("forced reranker cuda:0 unexpectedly opened successfully");
} catch (error) {
  if (!(error instanceof RerankerDevicePolicyError)) throw error;
  const message = String(error.message);
  const payload = {
    schema_version: "fathomdb.cuda-forced-reranker-capture/v1",
    consumer: "napi",
    argv,
    requested_policy: "cuda:0",
    status: "cuda_unavailable",
    effective_device: null,
    reason: "no_visible_cuda_device",
    error: {
      type: error.constructor.name,
      kind: error.kind,
      ordinal: error.ordinal,
      message,
    },
  };
  process.stdout.write(`${JSON.stringify(payload, CANONICAL_JSON_KEYS)}\n`);
  process.stderr.write(`${message}\n`);
  process.exitCode = 1;
}
