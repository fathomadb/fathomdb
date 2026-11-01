import type { Engine } from "../src/core.js";
import type { OpenReport } from "../src/open.js";
import type { NativeEngine, NativeOpenReport } from "../src/binding.js";

// Approved failed-open contract removes the previous degraded-handle API.
function surface(engine: Engine, native: NativeEngine, report: OpenReport, nativeReport: NativeOpenReport): void {
  // @ts-expect-error actual refusal has no successful report
  report.denseDisabled;
  // @ts-expect-error actual refusal has no successful report
  report.denseDisabledReason;
  // @ts-expect-error native report also has no refused-open state
  nativeReport.denseDisabled;
  // @ts-expect-error native report also has no refused-open state
  nativeReport.denseDisabledReason;
  // @ts-expect-error no degraded handle is returned
  engine.denseDisabled();
  // @ts-expect-error no degraded handle is returned
  engine.denseDisabledReason();
  // @ts-expect-error no query-time refusal instrumentation remains
  engine.vectorEquivalenceRefusalCount();
  // @ts-expect-error native handle follows same contract
  native.denseDisabled();
  // @ts-expect-error native handle follows same contract
  native.denseDisabledReason();
  // @ts-expect-error native handle follows same contract
  native.vectorEquivalenceRefusalCount();
}
void surface;

import type { ProjectionRuntimeStateV1, ProjectionRuntimeUnavailabilityReason } from "../src/projection.js";
// @ts-expect-error failed opens have no projection runtime state
const refused: ProjectionRuntimeStateV1 = "refused";
// @ts-expect-error actual verification refusal returns no runtime status
const disabled: ProjectionRuntimeUnavailabilityReason = "vector_equivalence_disabled";
void refused;
void disabled;

import type { ProjectionReadinessV1, EmbeddingReadinessState } from "../src/projection.js";
// @ts-expect-error refused-runtime projection deferral no longer exists
const deferredProjection: ProjectionReadinessV1 = "deferred";
// @ts-expect-error refused-runtime embedding deferral no longer exists
const deferredEmbedding: EmbeddingReadinessState = "deferred";
void deferredProjection;
void deferredEmbedding;
