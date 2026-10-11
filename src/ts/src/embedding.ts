import type { NativeEngine } from "./binding.js";
// Private embedding owner for the TypeScript SDK package root.
import { native, type NativeRerankPassage } from "./binding.js";

import { validateFfiString } from "./validation.js";

import { intercept } from "./native-call.js";

/**
 * Embed `texts` with the pinned default BGE-small model using CLS pooling.
 *
 * This is the TypeScript peer of Python's `embed_batch_cls`. It returns one
 * L2-normalized vector per input in input order; `[]` returns `[]` without
 * loading weights. It is intentionally distinct from {@link Engine.embed},
 * which uses the engine's Mean-pooling read path. The published native package
 * includes the default embedder; a custom thin build rejects with
 * `EmbedderNotConfiguredError`.
 */
export async function embedBatchCls(texts: readonly string[]): Promise<number[][]> {
  const batch = Array.from(texts);
  for (const text of batch) validateFfiString(text);
  return intercept(() => native.embedBatchCls(batch));
}

/** One caller-supplied passage accepted by {@link rerank}. */
export interface RerankPassage {
  id: number;
  body: string;
  score: number;
}

/** Options for the standalone caller-supplied reranker. */
export interface RerankOptions {
  alpha?: number;
  poolN?: number;
}

/** One result from {@link rerank}, in final ranked order. */
export interface RerankResult {
  id: number;
  score: number;
  ceScore: number | null;
}

export function validateRerankU32(name: string, value: number): void {
  if (!Number.isInteger(value)) {
    throw new RangeError(`${name} must be an integer, got ${typeof value}`);
  }
  if (value < 0) throw new RangeError(`${name} must be >= 0, got ${value}`);
  if (value > 0xffffffff) {
    throw new RangeError(`${name} must be <= 4294967295 (u32 max), got ${value}`);
  }
}

export function validateRerankString(name: string, value: unknown): asserts value is string {
  if (typeof value !== "string") {
    throw new TypeError(`${name} must be a string, got ${typeof value}`);
  }
  validateFfiString(value);
}

/**
 * Rerank an arbitrary caller-supplied passage pool.
 *
 * A depth of zero or an empty pool is a model-free identity path. Builds
 * without the default reranker preserve identity ordering for every depth.
 */
export async function rerank(
  query: string,
  passages: readonly RerankPassage[],
  rerankDepth: number,
  options: RerankOptions = {},
): Promise<RerankResult[]> {
  validateRerankString("query", query);
  validateRerankU32("rerankDepth", rerankDepth);
  if (options.alpha !== undefined && !Number.isFinite(options.alpha)) {
    throw new RangeError(`alpha must be a finite number, got ${options.alpha}`);
  }
  if (options.poolN !== undefined) validateRerankU32("poolN", options.poolN);
  const nativePassages: NativeRerankPassage[] = passages.map((passage) => {
    if (!Number.isSafeInteger(passage.id) || passage.id < 0) {
      throw new RangeError(`passage id must be a non-negative safe integer, got ${passage.id}`);
    }
    validateRerankString("passage body", passage.body);
    if (!Number.isFinite(passage.score)) {
      throw new RangeError(`passage score must be finite, got ${passage.score}`);
    }
    return { id: passage.id, body: passage.body, score: passage.score };
  });
  const values = await intercept(() =>
    native.rerank(query, nativePassages, rerankDepth, options.alpha, options.poolN),
  );
  return values.map((value) => ({
    id: value.id,
    score: value.score,
    ceScore: value.ceScore ?? null,
  }));
}

export async function embed(nativeEngine: NativeEngine, text: string): Promise<number[]> {
    validateFfiString(text);
    return intercept(() => nativeEngine.embed(text));
  }
