import type { NativeEngine } from "./binding.js";
import { intercept, interceptSync } from "./native-call.js";
import { validateFfiString } from "./validation.js";
import type { CounterSnapshot, SubscriberCallback } from "./open.js";
// Private instrumentation owner for the TypeScript SDK package root.

/**
 * 0.8.8 Slice 15 — validate a relevance-label id array before the native call
 * (mirrors the Python `_validate_id_list` guard for cross-SDK parity). Ids are
 * non-negative integers (the stable `SearchHit.id` identity carrier).
 */
export function validateIdArray(name: string, value: number[]): void {
  if (!Array.isArray(value)) {
    throw new TypeError(`${name} must be an array of non-negative integers`);
  }
  for (const item of value) {
    if (!Number.isInteger(item)) {
      throw new RangeError(`${name} must contain only integers, got ${typeof item}`);
    }
    if (item < 0) {
      throw new RangeError(`${name} must contain only non-negative integers, got ${item}`);
    }
  }
}

export async function enableTelemetry(nativeEngine: NativeEngine, sinkPath: string): Promise<void> {
    validateFfiString(sinkPath);
    await intercept(() => nativeEngine.enableTelemetry(sinkPath));
  }

export function lastTelemetryQueryId(nativeEngine: NativeEngine): string | null {
    return interceptSync(() => nativeEngine.lastTelemetryQueryId());
  }

export async function recordFeedback(nativeEngine: NativeEngine, queryId: string, relevantIds: number[], irrelevantIds: number[], labelSource: string): Promise<void> {
    validateFfiString(queryId);
    validateFfiString(labelSource);
    validateIdArray("relevantIds", relevantIds);
    validateIdArray("irrelevantIds", irrelevantIds);
    await intercept(() =>
      nativeEngine.recordFeedback(queryId, relevantIds, irrelevantIds, labelSource),
    );
  }

export function counters(nativeEngine: NativeEngine): CounterSnapshot {
    return interceptSync(() => nativeEngine.counters());
  }

export function setProfiling(nativeEngine: NativeEngine, enabled: boolean): void {
    interceptSync(() => nativeEngine.setProfiling(enabled));
  }

export function setSlowThresholdMs(nativeEngine: NativeEngine, value: number): void {
    interceptSync(() => nativeEngine.setSlowThresholdMs(value));
  }

export function attachSubscriber(nativeEngine: NativeEngine, callback: SubscriberCallback): void {
    interceptSync(() => nativeEngine.attachSubscriber(callback));
  }
