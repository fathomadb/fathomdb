#!/usr/bin/env node
/** Exercise forced CUDA selection through an installed N-API candidate. */

import { Engine, EmbedDevicePolicyError } from "fathomdb";

const argv = ["node", "/fathomdb-harness/forced-napi-open.mjs"];

try {
  await Engine.open("/tmp/forced-cuda-napi.fdb", { useDefaultEmbedder: true });
} catch (error) {
  if (!(error instanceof EmbedDevicePolicyError)) throw error;
  const message = String(error.message);
  const payload = {
    schema_version: "fathomdb.cuda-forced-device-capture/v1",
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
  process.stdout.write(`${JSON.stringify(payload)}\n`);
  process.stderr.write(`${message}\n`);
  process.exit(1);
}

throw new Error("forced cuda:0 unexpectedly opened successfully");
