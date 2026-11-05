import { WriteValidationError } from "./errors.js";
import { validateFfiString, validateSourceIdFfiString } from "./validation.js";
import type { NativeErasureMaintenance } from "./binding.js";
import { intercept } from "./native-call.js";

/** Persisted scheduling; explicit intent leaves read visibility unchanged. */
export type PhysicalErasurePolicy = "autonomous" | "explicit_maintenance";
/** Availability after releasing temporary ownership, independent of completion. */
export type ErasureServiceState = "normal_with_queued_restrictions" | "maintenance_owned" | "reads_allowed_writes_fenced" | "closed_or_unavailable";

/** Immutable source or deleted-logical selector; acceptance captures its cut. */
export type PhysicalErasureSelector =
  | { readonly kind: "source_all_at_cut"; readonly sourceId: string }
  | { readonly kind: "deleted_logical_all_at_cut"; readonly logicalId: string };
export type ErasureLogicalState = "unchanged_by_this_request" | "removed";
export type ErasurePhase = "queued" | "executing" | "sanitizing" | "finalizing" | "complete" | "withdrawn_before_removal";
export type ErasureBlocker = "resource_envelope_changed" | "original_sink_unavailable" | "wal_checkpoint" | "owned_readers";
/** Identity-free status; counts imply no completion deadline. */
export interface ErasureStatus {
  readonly phase: ErasurePhase;
  readonly logicalState: ErasureLogicalState;
  readonly blocker: ErasureBlocker | null;
  readonly serviceState: ErasureServiceState;
  readonly targetCount: number;
}
export type QueueResult =
  | { readonly outcome: "accepted"; readonly ticket: string; readonly logicalState: ErasureLogicalState }
  | { readonly outcome: "needs_maintenance_admission" };
export type WithdrawResult = "withdrawn_before_removal" | "already_activated" | "unknown";
export type CleanupResult =
  | { readonly outcome: "complete"; readonly receipt: ErasureStatus }
  | { readonly outcome: "incomplete"; readonly status: ErasureStatus };

export interface NativeErasureStatus {
  phase: ErasurePhase;
  logical_state: ErasureLogicalState;
  blocker: ErasureBlocker | null;
  service_state: ErasureServiceState;
  target_count: number;
}
export type NativeQueueResult =
  | { outcome: "accepted"; ticket: string; logical_state: ErasureLogicalState }
  | { outcome: "needs_maintenance_admission" };

export type NativeCleanupResult =
  | { outcome: "complete"; receipt: NativeErasureStatus }
  | { outcome: "incomplete"; status: NativeErasureStatus };

export function erasureSelector(selector: PhysicalErasureSelector): unknown {
  if (selector === null || typeof selector !== "object" || Array.isArray(selector)) {
    throw new WriteValidationError("invalid erasure selector");
  }
  const keys = Object.keys(selector);
  if (selector.kind === "source_all_at_cut" && typeof selector.sourceId === "string" &&
      keys.length === 2 && keys.includes("sourceId")) {
    validateSourceIdFfiString(selector.sourceId);
    return { kind: selector.kind, source_id: selector.sourceId };
  }
  if (selector.kind === "deleted_logical_all_at_cut" && typeof selector.logicalId === "string" &&
      keys.length === 2 && keys.includes("logicalId")) {
    validateSourceIdFfiString(selector.logicalId);
    return { kind: selector.kind, logical_id: selector.logicalId };
  }
  throw new WriteValidationError("invalid erasure selector");
}

export function queueResult(result: NativeQueueResult): QueueResult {
  return result.outcome === "accepted"
    ? { outcome: result.outcome, ticket: result.ticket, logicalState: result.logical_state }
    : result;
}

export function erasureStatus(status: NativeErasureStatus): ErasureStatus {
  return { phase: status.phase, logicalState: status.logical_state, blocker: status.blocker,
    serviceState: status.service_state, targetCount: status.target_count };
}

/** Exclusive window; dropping it never forgives durable physical obligations. */
export class ErasureMaintenance {
  constructor(private readonly native: NativeErasureMaintenance) {}
  /** Heavy non-destructive admission inside this exclusive window. */
  async queuePhysicalErasure(selector: PhysicalErasureSelector, idempotencyKey: string): Promise<QueueResult> {
    const lowered = erasureSelector(selector);
    return queueResult(await intercept(() => this.native.queuePhysicalErasure(lowered, idempotencyKey)));
  }
  /** Attempt exact physical completion. Incomplete retains durable owed work. */
  async completePhysicalErasure(ticket: string): Promise<CleanupResult> {
    const result = await intercept(() => this.native.completePhysicalErasure(ticket));
    return result.outcome === "complete"
      ? { outcome: result.outcome, receipt: erasureStatus(result.receipt) }
      : { outcome: result.outcome, status: erasureStatus(result.status) };
  }
  /** Reattach only a recorded original. Never registers or substitutes a sink. */
  async reattachOriginalErasureSink(ticket: string, path: string): Promise<void> {
    validateFfiString(path);
    await intercept(() => this.native.reattachOriginalErasureSink(ticket, path));
  }
  /** Release temporary admission gates. This is not a physical completion receipt. */
  async close(): Promise<ErasureServiceState> {
    return await intercept(() => this.native.close()) as ErasureServiceState;
  }
}
