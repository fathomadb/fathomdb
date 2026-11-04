import type { NativeErasureMaintenance } from "./binding.js";
import { intercept } from "./native-call.js";

/** Persisted scheduling; explicit intent leaves read visibility unchanged. */
export type PhysicalErasurePolicy = "autonomous" | "explicit_maintenance";
/** Availability after releasing temporary ownership, independent of completion. */
export type ErasureServiceState = "normal_with_queued_restrictions" | "maintenance_owned" | "reads_allowed_writes_fenced" | "closed_or_unavailable";

/** Exclusive window; dropping it never forgives durable physical obligations. */
export class ErasureMaintenance {
  constructor(private readonly native: NativeErasureMaintenance) {}
  /** Release temporary admission gates. This is not a physical completion receipt. */
  async close(): Promise<ErasureServiceState> {
    return await intercept(() => this.native.close()) as ErasureServiceState;
  }
}
