import test from "node:test";
import assert from "node:assert/strict";
import {
  CudaPoolExhaustedError, FathomDbError, rethrowTyped, VectorVerificationError,
  VectorEquivalenceMismatchError,
} from "../src/errors.js";

function translate(code: string, payload: Record<string, unknown>): unknown {
  try { rethrowTyped(new Error(JSON.stringify({ code, message: "verification refused", payload }))); }
  catch (error) { return error; }
}

test("cannot-verify preserves probe context and original CUDA leaf", () => {
  const error = translate("FDB_VECTOR_VERIFICATION", {
    probeOrdinal: 7, causeCode: "provider", causeMessage: "original allocator detail",
    providerKind: "cuda_pool_exhausted",
    providerError: {
      code: "FDB_CUDA_POOL_EXHAUSTED", message: "original allocator detail",
      payload: { ordinal: 2, maxSizeBytes: 12345, message: "original allocator detail" },
    },
  });
  assert.ok(error instanceof VectorVerificationError);
  assert.ok(error instanceof FathomDbError);
  assert.equal(error.probeOrdinal, 7);
  assert.equal(error.causeCode, "provider");
  assert.equal(error.providerKind, "cuda_pool_exhausted");
  assert.equal(error.causeMessage, "original allocator detail");
  assert.ok(error.providerError instanceof CudaPoolExhaustedError);
  assert.equal(error.providerError.ordinal, 2);
  assert.equal(error.providerError.maxSizeBytes, 12345);
  assert.equal(error.providerError.message, "original allocator detail");
});

test("non-provider failures retain machine-readable cause without a provider leaf", () => {
  for (const causeCode of ["not_configured", "saturated", "queued_expired", "started_timeout", "invalid_output", "panic", "closing", "cancelled", "invalid_state"]) {
    const error = translate("FDB_VECTOR_VERIFICATION", { probeOrdinal: 0, causeCode, causeMessage: "original detail", providerError: null });
    assert.ok(error instanceof VectorVerificationError);
    assert.equal(error.probeOrdinal, 0);
    assert.equal(error.causeCode, causeCode);
    assert.equal(error.causeMessage, "original detail");
    assert.equal(error.providerError, null);
  }
});

test("numeric mismatch uses existing leaf and original reason", () => {
  const error = translate("FDB_VECTOR_EQUIVALENCE_MISMATCH", { reason: "P1 flips=3" });
  assert.ok(error instanceof VectorEquivalenceMismatchError);
  assert.equal(error.reason, "P1 flips=3");
});

test("generic provider failure and timeout retain their original kind", () => {
  for (const providerKind of ["failed", "timeout"]) {
    const error = translate("FDB_VECTOR_VERIFICATION", {
      probeOrdinal: 4, causeCode: "provider", causeMessage: "original detail", providerKind,
      providerError: { code: "FDB_EMBEDDER", message: "original detail", payload: null },
    });
    assert.ok(error instanceof VectorVerificationError);
    assert.equal(error.providerKind, providerKind);
    assert.equal(error.providerError?.message, "original detail");
  }
});
