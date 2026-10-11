import type { NativeEngine } from "./binding.js";
import { intercept } from "./native-call.js";
import { validateFfiTree } from "./validation.js";
// Private projection owner for the TypeScript SDK package root.

export type ProjectionReadinessV1 = "ready" | "processing" | "blocked" | "deferred" | "degraded";
export type ProjectionRuntimeStateV1 = "absent" | "usable" | "refused";
export type ProjectionGenerationOriginV1 =
  "fresh" | "legacy_unverified" | "configuration" | "rebuild";

export interface ProjectionGenerationStatusV1 {
  readonly schemaVersion: 1;
  readonly generationId: string;
  readonly declarationSha256: string;
  readonly origin: ProjectionGenerationOriginV1;
  readonly transitionBoundary: string;
  readonly effectiveAtEpochS: number;
  readonly observedBoundary: string;
  readonly readyThrough: string;
  readonly readiness: ProjectionReadinessV1;
  readonly runtimeState: ProjectionRuntimeStateV1;
  readonly pendingCount: string;
  readonly failedCount: string;
}

export interface MutationProjectionStatusRequestV1 {
  readonly schemaVersion: 1;
  readonly operationId: string;
  readonly writeCursor: string;
  readonly expectedGenerationId: string;
}

export interface MutationProjectionStatusV1 {
  readonly schemaVersion: 1;
  readonly operationId: string;
  readonly writeCursor: string;
  readonly generationId: string;
  readonly effectiveAtEpochS: number;
  readonly observedBoundary: string;
  readonly readyThrough: string;
  readonly readiness: ProjectionReadinessV1;
  readonly runtimeState: ProjectionRuntimeStateV1;
  readonly pendingCount: string;
  readonly failedCount: string;
}

/**
 * 0.8.20 Slice 15d (R-20-PR) — the three projection roles (set members).
 * `searchable→FTS` and `searchable→vector` are NOT roles — they are tier labels
 * carried by the `fts`/`vector` sub-objects of a {@link ProjectionSpec}.
 */
export type ProjectionRole = "filterable" | "rankable" | "searchable";

/**
 * 0.8.20 Slice 15d (R-20-PR / C-1) — a declarative projection declaration.
 * HITL-ratified shape `{ name, roles, fts?, vector? }`; `roles` carries SET
 * semantics. `fts` selects the `searchable→FTS` sub-target (optional custom
 * `ftsTokenizer`); `vector` selects `searchable→vector` (optional
 * `vectorEmbedder`). The `vector` sub-object is stored by Slice 15d; Slice 20
 * (R-20-DR) hangs the engine-set `vectorDenseReadiness` off it. Mirrors the
 * Python `ProjectionSpec` (cross-binding parity).
 */
export interface ProjectionSpec {
  name: string;
  roles: ProjectionRole[];
  /** `true` when the `searchable→FTS` sub-target is declared. */
  fts: boolean;
  /** Optional custom tokenizer; omitted = engine default (only with `fts`). */
  ftsTokenizer?: string | null;
  /** `true` when the `searchable→vector` sub-target is declared. */
  vector: boolean;
  /** Optional embedder override; omitted = engine default (only with `vector`). */
  vectorEmbedder?: string | null;
  /** Literal canonical-body member path; omitted retains top-level `name`. */
  source?: string[] | null;
  /**
   * 0.8.22 Slice 21 F5 — **READ METADATA, engine-set.** The signed closed
   * vocabulary is `"unavailable"` / `"embedding"` / `"ready"`; `null` is on
   * every caller-authored spec. Caller input is accept-inert; on reads the
   * engine selects `"unavailable"` when no usable dense runtime exists, or
   * `"embedding"` / `"ready"` from the shared outstanding-work predicate.
   *
   * `filterable` / `searchable→FTS` are same-transaction (non-stale on commit)
   * so they carry no readiness; `searchable→vector` is async and
   * rebuild-durable, so it does. The value is DERIVED from outstanding
   * projection work, never stored — which is what makes
   * `{vector-insert ∧ readiness := ready}` atomic by construction: a `"ready"`
   * reading can never be observed with the vector row absent.
   *
   * `"pending"` is NOT a value here: that token is reserved for the orthogonal
   * ADMISSION axis (quarantine/trust, an app judgment).
   *
   * Supplying it to `configureProjections` is INERT — it is not part of the
   * declaration and the engine always reports the derived truth — so
   * `read.projections` output still re-applies as a no-op. Supplying it with
   * `vector: false`, or any spelling outside
   * `{"unavailable", "embedding", "ready"}`, throws a
   * typed `FDB_INVALID_ARGUMENT` (it could not round-trip).
   */
  vectorDenseReadiness?: DenseReadiness | null;
}

/**
 * 0.8.22 Slice 21 F5 — the signed, closed readiness vocabulary of the
 * `searchable→vector` projection. It is engine-selected and accept-inert on
 * caller input: `"unavailable"` means no usable dense runtime, while
 * `"embedding"` / `"ready"` describe work under one. Mirrors Rust and Python;
 * `"pending"` is deliberately absent because it is admission-only.
 */
export type DenseReadiness = "unavailable" | "embedding" | "ready";

/**
 * 0.8.20 Slice 15d (R-20-PR) — the diff `configureProjections` applied.
 * Idempotent re-registration yields `unchanged: true` with all arrays empty; a
 * destructive change without an explicit drop throws instead of returning.
 */
export interface ProjectionDelta {
  built: string[];
  dropped: string[];
  deferred: string[];
  unchanged: boolean;
  /**
   * 0.8.20 Slice 22 (R-20-VC / TC-67) — node **kinds**, not attribute names: the
   * vector-eligible kinds present in the corpus that the vector writer can NEVER
   * commit, so no `searchable→vector` declaration will ever produce an embedding
   * for them. Such rows remain fully FTS/lexically searchable.
   *
   * This is what distinguishes "`deferred` because the embedder is still working
   * / absent this session" (transient) from "this kind will never be embedded"
   * (permanent). It is a STATE report, not a diff: it is populated on an
   * idempotent re-apply too (`unchanged: true`), which is also how you refresh it
   * after writing new kinds. Empty (never absent) when there is nothing to
   * report. Output-only — `configureProjections` accepts specs, not deltas.
   */
  vectorUnsupportedKinds: string[];
}

/** Reason an open engine session has no usable dense runtime. */
export type ProjectionRuntimeUnavailabilityReason =
  "none" | "no_runtime" | "vector_equivalence_disabled";

/** Dense status for one declared projection in {@link ProjectionRuntimeStatus}. */
export type ProjectionStatusDenseReadiness = "not_declared" | "unavailable" | "embedding" | "ready";

/** One declared projection's current dense status. */
export interface ProjectionRuntimeStatusEntry {
  name: string;
  denseReadiness: ProjectionStatusDenseReadiness;
}

/**
 * Pure current projection-runtime facts for an open {@link Engine}.
 *
 * This is not a configuration echo. `projections` contains every declaration
 * in ascending name order. `vectorUnsupportedKinds` is empty unless a
 * declaration has an effective `searchable→vector` arm.
 */
export interface ProjectionRuntimeStatus {
  runtimeEmbedderAvailable: boolean;
  runtimeUnavailabilityReason: ProjectionRuntimeUnavailabilityReason;
  projections: ProjectionRuntimeStatusEntry[];
  vectorUnsupportedKinds: string[];
}

export type EmbeddingReadinessState = "ready" | "processing" | "deferred" | "blocked";
export type EmbeddingOperation = "graph_edge_body_projection" | "vector_projection";
export interface EmbeddingReadiness {
  state: EmbeddingReadinessState;
  usableEmbedder: boolean;
  pendingCount: number;
  affectedKinds: string[];
  code: "FDB_EMBEDDER_REQUIRED" | null;
  operation: EmbeddingOperation | null;
  remediations: string[];
  documentationUrl: string | null;
}

export async function configureProjections(nativeEngine: NativeEngine, specs: ProjectionSpec[], drop?: string[]): Promise<ProjectionDelta> {
    // TC-47 (keystone terminal codex P2) — every string in the spec/drop tree
    // (projection name, each role, ftsTokenizer, vectorEmbedder, each drop entry)
    // crosses to native. napi-rs silently replaces a lone UTF-16 surrogate with
    // U+FFFD BEFORE the Rust-side guard runs, so — exactly like `write` — the
    // surrogate check must happen JS-side or the mangled U+FFFD is persisted
    // instead of raising WriteValidationError. (A NUL survives the napi UTF-8
    // path as a real byte and is already caught Rust-side; the surrogate is not.)
    validateFfiTree(specs);
    if (drop !== undefined) validateFfiTree(drop);
    // 0.8.20 keystone closeout fix-4 — normalize an explicit `null` sub-field to
    // `undefined` (⇒ napi `None`). `read.projections` EMITS `ftsTokenizer: null`
    // / `vectorEmbedder: null` for a spec with no custom sub-field, but napi-rs
    // rejects an explicit `null` for an `Option<String>` field with an opaque
    // `StringExpected` — so feeding read output straight back into
    // `configureProjections` threw, diverging from pyo3 (which accepts `None`)
    // and breaking the read→configure round-trip. Mapping `null → undefined`
    // here makes the two bindings behave identically and keeps the caller's
    // objects untouched (a shallow copy per spec).
    // 0.8.20 Slice 20 (R-20-DR) — `vectorDenseReadiness` gets the SAME
    // null→undefined normalization: `read.projections` emits an explicit `null`
    // for a spec with no vector sub-object, and napi-rs would reject that for an
    // `Option<String>` field. Its non-null value is carried through unchanged
    // (it is inert engine-side, so the read→configure round-trip stays a no-op).
    const nativeSpecs = specs.map((s) => ({
      ...s,
      ftsTokenizer: s.ftsTokenizer ?? undefined,
      vectorEmbedder: s.vectorEmbedder ?? undefined,
      vectorDenseReadiness: s.vectorDenseReadiness ?? undefined,
      source: s.source ?? undefined,
    }));
    return intercept(() => nativeEngine.configureProjections(nativeSpecs, drop ?? null));
  }
