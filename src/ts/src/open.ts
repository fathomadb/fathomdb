import type { NativeEngine } from "./binding.js";
import { interceptSync } from "./native-call.js";
// Private open owner for the TypeScript SDK package root.
import {
  type NativeCudaAllocatorReport,
  type NativeCudaDeviceInfo,
  type NativeCudaVisibleDevice,
  type NativeEffectiveEmbedDevice,
  type NativeEmbedderDeviceResolution,
  type NativeEmbedderEvent,
  type NativeGpuAllocationWitness,
  type NativeOpenReport,
} from "./binding.js";

export interface MigrationStepReport {
  readonly stepId: number;
  readonly durationMs: number | null;
  readonly failed: boolean;
}

export interface EmbedderIdentity {
  readonly name: string;
  readonly revision: string;
  readonly dimension: number;
}

/**
 * EU-6 FIX-2 — discriminated-union shape for `OpenReport.embedderEvents`.
 *
 * Each variant interface carries a closed `kind` literal + the
 * variant-specific payload fields (non-optional). Callers pattern-match
 * with `if (event.kind === "...")` and tsc narrows the payload access
 * accordingly. See `dev/design/0.7.1-EU-6-FIX-2-design.md` §6.3.
 */
export interface DefaultEmbedderDownloadEvent {
  readonly kind: "DefaultEmbedderDownload";
  readonly file: string;
  readonly url: string;
  readonly bytes: number;
  readonly sha256: string;
  readonly cachePath: string;
  readonly durationMs: number;
}

export interface DefaultEmbedderCacheHitEvent {
  readonly kind: "DefaultEmbedderCacheHit";
  readonly file: string;
  readonly sha256: string;
  readonly cachePath: string;
}

export interface MeanVecPinnedEvent {
  readonly kind: "MeanVecPinned";
  readonly dim: number;
  readonly docCount: number;
}

/**
 * Forward-compat fallback for `kind` values not known to this build.
 * Part of the public `EmbedderEvent` union for soundness: a future or
 * replaced native extension may emit kinds this build does not know
 * about, and exposing them under a typed fallback member is more honest
 * than pretending the runtime is exhaustive at compile time. Because
 * `kind` here is the open type `string`, tsc cannot exclude this member
 * purely from a literal `event.kind === "..."` check on the bare union
 * — wrap such checks in {@link isKnownEmbedderEvent} first to recover
 * precise narrowing on the three known variants.
 */
export interface UnknownEmbedderEvent {
  readonly kind: string;
  readonly [field: string]: unknown;
}

export type EmbedderEvent =
  | DefaultEmbedderDownloadEvent
  | DefaultEmbedderCacheHitEvent
  | MeanVecPinnedEvent
  | UnknownEmbedderEvent;

/**
 * Type guard that narrows an {@link EmbedderEvent} to the three known
 * variants, excluding {@link UnknownEmbedderEvent}. Use as a gate before
 * discriminating on `event.kind`:
 *
 * ```ts
 * if (isKnownEmbedderEvent(event)) {
 *   if (event.kind === "DefaultEmbedderDownload") {
 *     const bytes: number = event.bytes; // narrowed precisely
 *   }
 * }
 * ```
 *
 * Without this guard, the open `kind: string` on `UnknownEmbedderEvent`
 * prevents tsc from removing it from the union on a literal-equality
 * check, so payload field access widens to `unknown`.
 */
export function isKnownEmbedderEvent(
  event: EmbedderEvent,
): event is DefaultEmbedderDownloadEvent | DefaultEmbedderCacheHitEvent | MeanVecPinnedEvent {
  return (
    event.kind === "DefaultEmbedderDownload" ||
    event.kind === "DefaultEmbedderCacheHit" ||
    event.kind === "MeanVecPinned"
  );
}

/**
 * @internal — maps the wide napi-rs `NativeEmbedderEvent` into the
 * narrow discriminated `EmbedderEvent` union at the binding → SDK
 * seam. The non-null assertions are sound under the Rust emitter
 * invariant codified by AC-FIX2-6's runtime shape consistency test:
 * for each known `kind`, the emitter populates exactly the variant-
 * appropriate fields. Unknown `kind` values pass through as
 * `UnknownEmbedderEvent` so a forward-compatible variant addition
 * remains a strict refinement, not a breaking change.
 */
export function mapEmbedderEvent(n: NativeEmbedderEvent): EmbedderEvent {
  switch (n.kind) {
    case "DefaultEmbedderDownload":
      return {
        kind: "DefaultEmbedderDownload",
        file: n.file!,
        url: n.url!,
        bytes: n.bytes!,
        sha256: n.sha256!,
        cachePath: n.cachePath!,
        durationMs: n.durationMs!,
      };
    case "DefaultEmbedderCacheHit":
      return {
        kind: "DefaultEmbedderCacheHit",
        file: n.file!,
        sha256: n.sha256!,
        cachePath: n.cachePath!,
      };
    case "MeanVecPinned":
      return {
        kind: "MeanVecPinned",
        dim: n.dim!,
        docCount: n.docCount!,
      };
    default: {
      // Forward-compat: surface unknown kinds verbatim, dropping any
      // nullish wide-shape fields so the resulting object has only the
      // keys the emitter actually populated. `UnknownEmbedderEvent` is
      // part of the declared `EmbedderEvent` union, so no cast through
      // `unknown` is required — callers recover precise narrowing on
      // the known variants via `isKnownEmbedderEvent`.
      const out: Record<string, unknown> = { kind: n.kind };
      for (const [k, v] of Object.entries(n)) {
        if (k !== "kind" && v !== null && v !== undefined) out[k] = v;
      }
      return out as UnknownEmbedderEvent;
    }
  }
}

export interface OpenReport {
  readonly schemaVersionBefore: number;
  readonly schemaVersionAfter: number;
  readonly migrationSteps: ReadonlyArray<MigrationStepReport>;
  readonly embedderWarmupMs: number;
  readonly queryBackend: string;
  readonly defaultEmbedder: EmbedderIdentity;
  /** EU-5b — wall-time ms the loader spent fetching default-embedder
   *  weights, or `null` on full cache hit / caller-supplied embedder. */
  readonly embedderDownloadMs: number | null;
  /** EU-5b — structured loader events (downloads, cache hits,
   *  mean-vec pin). */
  readonly embedderEvents: ReadonlyArray<EmbedderEvent>;
  /** EU-5b — static identity capability (mean-centering required for
   *  bge-small). */
  readonly embedderMeanCenteringRequired: boolean;
  /** EU-5a2 — dynamic workspace state (`mean_vec IS NOT NULL` after the
   *  256-doc threshold crossing). */
  readonly embedderMeanVecPinned: boolean;
  /** 0.8.18 Slice 5 (#5 vector-equivalence probe, R-VEQ-6) — `true` iff the
   *  open-time self-check found a vector-equivalence divergence and every
   *  vector-dependent arm now refuses at query time with
   *  `VectorEquivalenceMismatchError`. The `searchTextOnly` path stays
   *  serviceable. */
  readonly denseDisabled: boolean;
  /** R-VEQ-6 — reason for `denseDisabled`, or `null` when dense is healthy. */
  readonly denseDisabledReason: string | null;
  /** Strict CPU/CUDA selection used to construct the embedder, or `null` when
   *  no embedder was configured. */
  readonly embedderDeviceResolution: DeviceResolution | null;
  /** Independent cross-encoder CPU/CUDA selection. It never attests SQLite or
   * embedding work and is `null` when this artifact lacks the reranker. */
  readonly rerankerDeviceResolution: DeviceResolution | null;
  /** 0.8.23 Slice 80.6 (D-80.6-6, AC80-6) — the in-process GPU allocation
   *  witness measured during this open, or `null` when none was measured.
   *
   *  `null` means **no witness was taken**, never "a witness measured
   *  nothing": a zero, negative, or below-floor allocation delta is a typed
   *  failure inside the witness and fails the open, so a zero-valued record is
   *  not reachable here. */
  readonly embedderGpuAllocationWitness: GpuAllocationWitness | null;
}

/**
 * 0.8.23 Slice 80.6 (D-80.6-6, R80-13) — the retained
 * `fathomdb.tegra-gpu-allocation-witness/v1` record, measured in the
 * artifact's own process.
 *
 * Every number the verdict used is present, so a reader re-derives the verdict
 * instead of trusting it: `freeBeforeBytes - freeAfterBytes` is `deltaBytes`,
 * which must be at least `deltaFloorBytes`, and the deliberate control
 * allocation shows the shared iGPU memory counter was live and attributable at
 * the time. Byte counts are JavaScript numbers, which are exact for every
 * physically reachable device-memory value.
 */
export interface GpuAllocationWitness {
  /** Schema string of the retained record. */
  readonly schema: string;
  /** The precondition the witness run states rather than assumes. */
  readonly soleGpuConsumerPrecondition: string;
  readonly deviceOrdinalRequested: number;
  readonly deviceOrdinalActual: number;
  readonly deviceUuid: string;
  readonly deviceName: string;
  readonly computeCapability: string;
  readonly freeBeforeBytes: number;
  readonly freeAfterBytes: number;
  readonly totalBytes: number;
  readonly deltaBytes: number;
  readonly deltaFloorBytes: number;
  readonly controlAllocationRequestBytes: number;
  readonly controlBlockCount: number;
  readonly controlFreeBeforeBytes: number;
  readonly controlFreeAfterBytes: number;
  readonly controlDeltaBytes: number;
  readonly embeddedVectorDim: number;
}

/**
 * The CUDA allocator decision behind one device. Each string is the stable
 * core name: `path` is `private`, `default_pool` or `synchronous` (`null`
 * when unknown); `poolMaxSizeBytes` and `releaseThreshold` are `null` unless
 * the path is private.
 */
export interface CudaAllocatorReport {
  readonly path: string | null;
  readonly reason: string;
  readonly poolMaxSizeBytes: number | null;
  readonly releaseThreshold: "0" | "max" | null;
  readonly moduleLoadInit: string;
}

/**
 * Safe CUDA provider facts associated with an effective CUDA selection.
 * `cudaAllocator` is `null` off aarch64 Linux CUDA builds.
 */
export interface CudaDeviceInfo {
  readonly ordinal: number;
  readonly uuid: string | null;
  readonly name: string | null;
  readonly driverVersion: string | null;
  readonly computeCapability: string | null;
  readonly cudaToolkitVersion: string | null;
  readonly cudaAllocator: CudaAllocatorReport | null;
}

/** One CUDA device visible to the process after `CUDA_VISIBLE_DEVICES`. */
export interface CudaVisibleDevice {
  readonly visibleOrdinal: number;
  readonly uuid: string;
  readonly name: string;
  readonly computeCapability: string | null;
}

/** The CPU or CUDA backend selected for one embedder device policy. */
export type EffectiveEmbedDevice =
  | { readonly kind: "cpu"; readonly cudaDevice: null }
  | { readonly kind: "cuda"; readonly cudaDevice: CudaDeviceInfo };

/**
 * Strict CPU/CUDA policy outcome captured when an embedder was constructed.
 * `requestedPolicy` is exactly `auto`, `cpu`, or `cuda:N`; `reason` explains
 * an automatic CPU fallback and is `null` for an explicitly selected device.
 */
export interface DeviceResolution {
  readonly requestedPolicy: string;
  readonly cudaCompiled: boolean;
  readonly effectiveDevice: EffectiveEmbedDevice;
  readonly visibleCudaDevices: readonly CudaVisibleDevice[];
  readonly selectedCudaUuid: string | null;
  readonly reason: string | null;
}

export function mapCudaAllocatorReport(report: NativeCudaAllocatorReport): CudaAllocatorReport {
  const threshold = report.releaseThreshold ?? null;
  if (threshold !== null && threshold !== "0" && threshold !== "max") {
    throw new Error(`invalid native CUDA release threshold: ${threshold}`);
  }
  return {
    path: report.path ?? null,
    reason: report.reason,
    poolMaxSizeBytes: report.poolMaxSizeBytes ?? null,
    releaseThreshold: threshold,
    moduleLoadInit: report.moduleLoadInit,
  };
}

export function mapCudaDeviceInfo(info: NativeCudaDeviceInfo): CudaDeviceInfo {
  return {
    ordinal: info.ordinal,
    uuid: info.uuid ?? null,
    name: info.name ?? null,
    driverVersion: info.driverVersion ?? null,
    computeCapability: info.computeCapability ?? null,
    cudaToolkitVersion: info.cudaToolkitVersion ?? null,
    cudaAllocator: info.cudaAllocator ? mapCudaAllocatorReport(info.cudaAllocator) : null,
  };
}

export function mapCudaVisibleDevice(device: NativeCudaVisibleDevice): CudaVisibleDevice {
  return {
    visibleOrdinal: device.visibleOrdinal,
    uuid: device.uuid,
    name: device.name,
    computeCapability: device.computeCapability ?? null,
  };
}

export function mapEffectiveEmbedDevice(device: NativeEffectiveEmbedDevice): EffectiveEmbedDevice {
  if (device.kind === "cpu") return { kind: "cpu", cudaDevice: null };
  if (device.kind === "cuda" && device.cudaDevice) {
    return { kind: "cuda", cudaDevice: mapCudaDeviceInfo(device.cudaDevice) };
  }
  throw new Error(`invalid native embedder effective device: ${device.kind}`);
}

export function mapDeviceResolution(resolution: NativeEmbedderDeviceResolution): DeviceResolution {
  return {
    requestedPolicy: resolution.requestedPolicy,
    cudaCompiled: resolution.cudaCompiled,
    effectiveDevice: mapEffectiveEmbedDevice(resolution.effectiveDevice),
    visibleCudaDevices: resolution.visibleCudaDevices.map(mapCudaVisibleDevice),
    selectedCudaUuid: resolution.selectedCudaUuid ?? null,
    reason: resolution.reason ?? null,
  };
}

export function mapGpuAllocationWitness(witness: NativeGpuAllocationWitness): GpuAllocationWitness {
  // Field-for-field, deliberately: R80-13 requires the record stay
  // re-derivable, so nothing here summarizes or drops a number.
  return {
    schema: witness.schema,
    soleGpuConsumerPrecondition: witness.soleGpuConsumerPrecondition,
    deviceOrdinalRequested: witness.deviceOrdinalRequested,
    deviceOrdinalActual: witness.deviceOrdinalActual,
    deviceUuid: witness.deviceUuid,
    deviceName: witness.deviceName,
    computeCapability: witness.computeCapability,
    freeBeforeBytes: witness.freeBeforeBytes,
    freeAfterBytes: witness.freeAfterBytes,
    totalBytes: witness.totalBytes,
    deltaBytes: witness.deltaBytes,
    deltaFloorBytes: witness.deltaFloorBytes,
    controlAllocationRequestBytes: witness.controlAllocationRequestBytes,
    controlBlockCount: witness.controlBlockCount,
    controlFreeBeforeBytes: witness.controlFreeBeforeBytes,
    controlFreeAfterBytes: witness.controlFreeAfterBytes,
    controlDeltaBytes: witness.controlDeltaBytes,
    embeddedVectorDim: witness.embeddedVectorDim,
  };
}

/**
 * @internal Map the native open-time snapshot into the public SDK shape.
 * Kept separate so the binding contract is testable without a CUDA host.
 */
export function mapOpenReport(r: NativeOpenReport): OpenReport {
  return {
    schemaVersionBefore: r.schemaVersionBefore,
    schemaVersionAfter: r.schemaVersionAfter,
    migrationSteps: r.migrationSteps,
    embedderWarmupMs: r.embedderWarmupMs,
    queryBackend: r.queryBackend,
    defaultEmbedder: r.defaultEmbedder,
    embedderDownloadMs: r.embedderDownloadMs,
    embedderEvents: r.embedderEvents.map(mapEmbedderEvent),
    embedderMeanCenteringRequired: r.embedderMeanCenteringRequired,
    embedderMeanVecPinned: r.embedderMeanVecPinned,
    denseDisabled: r.denseDisabled,
    denseDisabledReason: r.denseDisabledReason ?? null,
    embedderDeviceResolution: r.embedderDeviceResolution
      ? mapDeviceResolution(r.embedderDeviceResolution)
      : null,
    rerankerDeviceResolution: r.rerankerDeviceResolution
      ? mapDeviceResolution(r.rerankerDeviceResolution)
      : null,
    embedderGpuAllocationWitness: r.embedderGpuAllocationWitness
      ? mapGpuAllocationWitness(r.embedderGpuAllocationWitness)
      : null,
  };
}

export interface CounterSnapshot {
  queries: number;
  writes: number;
  writeRows: number;
  adminOps: number;
  cacheHit: number;
  cacheMiss: number;
}

export type SubscriberEvent =
  | {
      kind: "event";
      phase: "started" | "slow" | "heartbeat" | "finished" | "failed";
      source: "engine" | "sqlite_internal";
      category: "writer" | "search" | "admin" | "error" | "corruption" | "recovery" | "io";
      code?: string;
      droppedRecordsTotal: string;
    }
  | {
      kind: "profile";
      wallClockMs: string;
      stepCount: string;
      cacheDelta: string;
      droppedRecordsTotal: string;
    }
  | {
      kind: "slowStatement";
      statement: string;
      wallClockMs: string;
      droppedRecordsTotal: string;
    }
  | {
      kind: "stressFailure";
      threadGroupId: string;
      opKind: string;
      lastErrorChain: string[];
      projectionState: string;
      droppedRecordsTotal: string;
    };

export type SubscriberCallback = (event: SubscriberEvent) => void;

export function denseDisabled(nativeEngine: NativeEngine): boolean {
    return nativeEngine.denseDisabled();
  }

export function denseDisabledReason(nativeEngine: NativeEngine): string | null {
    return nativeEngine.denseDisabledReason() ?? null;
  }

export function vectorEquivalenceRefusalCount(nativeEngine: NativeEngine): number {
    return nativeEngine.vectorEquivalenceRefusalCount();
  }

export function openReport(nativeEngine: NativeEngine): OpenReport {
    return interceptSync(() => mapOpenReport(nativeEngine.openReport()));
  }
