---
title: Slice 50 (SDK-CLI-UPGRADE) — SDK Ergonomics, OpenReport Capability Booleans, 37-to-38 Migrate Root, and CLI PyPI Wheel
status: CLOSED
target_release: 0.8.31
slice: 50
date: 2026-11-22
---

# Slice 50 (`SDK-CLI-UPGRADE`) — Draft Plan

Execution contract for FathomDB `0.8.31` Slice 50 covering SDK batch evidence resolution (`resolve_evidence_many`), opt-in search hints (`search(hints=)`), `OpenReport` runtime capability booleans, the offline `fathomdb migrate` (`37 -> 38`) CLI root, lockstep `fathomdb-cli` PyPI binary wheel distribution, and targeted public/interface documentation fixes.

## 1. Goals & Non-Goals

### Goals

1. **Batch evidence resolution (`resolve_evidence_many`):** Expose `resolve_evidence_many` (`resolveEvidenceMany` in TypeScript) across `fathomdb-engine`, `fathomdb`, `fathomdb-sdk`, `fathomdb-py` / Python SDK, and `fathomdb-napi` / TypeScript SDK under a single frozen read context (`1..=50` bound, single-lock/snapshot execution, duplicate preservation, per-reference resolved-or-refused partial-result outcomes per accepted ADR `#76` with strict nondisclosure per slot and whole-call refusal on request/context failure), registered cleanly in `src/conformance/governed-operation-parity.json`, `src/conformance/governed-surface-allowlist.json`, and `scripts/governed-surface-pin.json` (advancing `55 -> 56` live ops and `91 -> 93` allowlist members).
2. **Search-time agent hints (`hints`):** Add opt-in `hints` (`bool` defaulting to `false`/`False`) to `search`, `search_frozen` (`searchFrozen`), and `EvidenceSearchRequestV1` across Rust (`fathomdb-engine`, `fathomdb`, `fathomdb-sdk`), Python, and TypeScript, attaching optional `SearchResult.hints` computed from the engine's existing pure `search_hints` planner in `src/rust/crates/fathomdb-engine/src/hints.rs` with zero overhead or output change when omitted/`false`.
3. **`OpenReport` capability booleans:** Add `reranker_compiled`, `cuda_compiled`, and `rerank_available` (`rerankerCompiled`, `cudaCompiled`, `rerankAvailable` in TypeScript) to `OpenReport` across Rust, Python, and TypeScript with zero network I/O and zero GPU allocation during `Engine::open` while preserving `ADR-0.8.23` `#43` fail-closed open on forced CUDA failure.
4. **Minimal offline `fathomdb migrate` (`37 -> 38`) CLI root:** Add `Command::Migrate` (`fathomdb migrate <db_path> --backup-dir <dir> [--json]`) alongside `Recover` and `Doctor` in `fathomdb-cli`, backed by `fathomdb_schema::STEP_38_SQL` and `fathomdb-engine::operator` (re-exported via `fathomdb` under `#[cfg(feature = "operator")]`), executing the 7-step fail-closed upgrade flow (exclusive lock, Schema 37 check, unfinished-erasure refusal, fail-closed dangling-endpoint preflight admitting valid superseded edges, WAL-checkpointed SHA-256-verified backup, single `BEGIN IMMEDIATE` `execute_batch(STEP_38_SQL)` transaction, and post-upgrade `check-integrity --full`), keeping `Engine::open` strictly Schema-38-only and preserving all 17 `DoctorCommand` variants.
5. **Lockstep `fathomdb-cli` PyPI binary wheel & `fathomdb[cli]` extra:** Add `src/python-cli/` (`pyproject.toml` with `maturin>=1.9,<2` `bindings = "bin"` selecting only the `fathomdb` binary target and excluding `perf-regression-check`, initialized at live Axis W `0.8.30` managed by `scripts/set-version.sh`, plus `LICENSE` and `README.md`), add `cli = ["fathomdb-cli==0.8.30"]` (lockstep Axis W) optional extra to `src/python/pyproject.toml` while keeping zero `[project.scripts]` on `fathomdb` (`AC-041`), and update `.github/workflows/release.yml`, version/license/registry/smoke scripts, and contract tests in lockstep.
6. **Targeted contract, public & interface docs (`P3-12`–`P3-15`, `P6-02`–`P6-05`):** Update `README.md`, `src/ts/README.md`, `docs/`, `dev/interfaces/`, and regenerate `docs/generated/sdk-operation-map.md` for all Slice 50 surfaces while honoring `scripts/check-public-doc-truth.py` pre-publish rules.

### Non-Goals

- **Slice 60 (`INSPECTOR`):** Standalone inspector CLI/package and `build-inspector` workflow wiring belong exclusively to Slice 60 (serialized after Slice 50 merges `.github/workflows/release.yml`).
- **Slice 70 (`QUALIFICATION`):** Cross-platform qualification runs, Tegra CUDA hardware qualification, and authoring `docs/release-notes/0.8.31.md` belong exclusively to Slice 70.
- **`0.8.32+` full `export` / `import` upgrade path:** Schema migrations from `< 37` or general multi-hop rebuild tooling are out of scope; `fathomdb migrate` strictly upgrades committed Schema `37` databases to Schema `38`.
- **Automatic migration in `Engine::open`:** `Engine::open` never auto-migrates and continues rejecting `user_version != 38` with `IncompatibleSchemaVersion`.
- **Console scripts on base `fathomdb` (`AC-041`) or recovery/SQL leakage onto SDKs:** `src/python/pyproject.toml` must keep zero `[project.scripts]` entries, and `migrate` remains CLI-only (`fathomdb-sdk` / Python / TypeScript do not expose `migrate`).
- **Mutating `dev/acceptance.md`:** `dev/acceptance.md` stays **LOCKED**; all slice criteria below use slice-local `AC31-50-*` identifiers mapped to release requirements `R31-05` and `R31-06`.

---

## 2. Draft Needs, Requirements & Acceptance Criteria

### Program Requirements Traced

| Req ID | Release Need | Slice 50 Scope |
| --- | --- | --- |
| `R31-05` | Three-SDK ergonomic & observability parity (`resolve_evidence_many`, opt-in `search(hints=)`, `OpenReport` reranker/CUDA booleans) + governed conformance pin & accurate contract docs | `AC31-50-01` .. `AC31-50-07`, `AC31-50-15`, `AC31-50-16` |
| `R31-06` | Safe offline Schema `37 -> 38` database upgrade path (`fathomdb migrate`) + zero-toolchain `pip install 'fathomdb[cli]'` distribution via lockstep `fathomdb-cli` PyPI wheel | `AC31-50-08` .. `AC31-50-14`, `AC31-50-16` |

### Slice-Local Acceptance Criteria (`dev/acceptance.md` Remains LOCKED)

| AC ID | Requirement | Verification Target |
| --- | --- | --- |
| `AC31-50-01` | `resolve_evidence_many` enforces `1..=50` input length (`EVIDENCE_RESOLVE_MANY_MAX = 50`) prior to database lookup, rejects `0` or `> 50` (including repeated duplicates totaling `> 50`) with `InvalidArgument` / `InvalidArgumentError` across Rust, Python, and TypeScript, and rejects `schema_version != 1` with `EvidenceError(UnsupportedSchemaVersion, "/schemaVersion")`. | Unit & integration tests across Rust Engine, `fathomdb-sdk`, Python, TypeScript |
| `AC31-50-02` | `resolve_evidence_many` preserves 1:1 request order and arity when valid duplicate tokens are submitted (`len(out) == len(in)` with identical resolved items per duplicate slot) inside one lock/snapshot, and validates the frozen read context both before resolution and after `evidence_before_resolve_return_hook` (collapsing invalid/stale/drifted context failures into whole-call `EvidenceError(EvidenceUnavailable)` identically to `resolve_evidence`). | Duplicate-token & snapshot-race round-trip tests in Rust, Python, TypeScript |
| `AC31-50-03` | `resolve_evidence_many` implements accepted ADR `#76` per-reference partial-result outcomes (`get_many` style): for a batch such as `[valid, bad]`, valid slots return `Resolved(ResolvedEvidenceV1)` while every syntactically invalid, tampered (MAC mismatch), wrong-DB/wrong-context, erased, or superseded/inaccessible slot returns `Refused(EvidenceErrorV1 { reason: EvidenceUnavailable, field_path: "/evidenceRefs/{i}" })` without leaking cause (and corrupt/incomplete authorized storage returns `Refused(EvidenceCorrupt)` / `Refused(EvidenceIncomplete)` at `"/evidenceRefs/{i}"`). | Partial-result, tamper, wrong-context, and per-slot nondisclosure matrix tests in Rust, Python, TypeScript |
| `AC31-50-04` | `resolve_evidence_many` achieves full live parity across Rust (`fathomdb-sdk`, `fathomdb`), Python, and TypeScript; `governed-operation-parity.json`, `governed-surface-allowlist.json` (`56` live ops / `93` allowlist members, preserving `5` core and `5` recovery_denylist), `scripts/governed-surface-pin.json`, and `docs/generated/sdk-operation-map.md` match with zero drift. | `scripts/check-sdk-surface-parity.py`, `scripts/check-governed-surface-pin.sh`, `test_generated_public_docs.py`, `sdk-surface-parity.test.ts` |
| `AC31-50-05` | `search`, `search_frozen`, and `EvidenceSearchRequestV1` accept opt-in `hints: bool` (default `false`/`False`); omitted or `false` sets `SearchResult.hints` to `None`/`undefined` with zero `Hint` allocation and byte-identical ranking/snippets; `true` populates `SearchResult.hints` via `search_hints` across Rust, Python, and TypeScript, and non-boolean inputs in Python/TS fail with `TypeError`. | `hints` on/off/non-bool tests across `search`, `search_frozen`, and `search_with_evidence` in Rust, Python, TypeScript |
| `AC31-50-06` | `OpenReport` exposes `reranker_compiled`, `cuda_compiled`, and `rerank_available` (`rerankerCompiled`, `cudaCompiled`, `rerankAvailable` in TS); `reranker_compiled` equals `cfg!(feature = "default-reranker")`, `cuda_compiled` equals `cfg!(any(feature = "embed-cuda", feature = "rerank-cuda"))`, and `rerank_available` verifies runtime device policy + local SHA-256 weight cache (or loaded singleton) without triggering network I/O or CUDA context creation, while preserving fail-closed `Engine::open` on forced CUDA policy failure (`ADR-0.8.23` `#43`). | Feature-off, `default-reranker` uncached vs warm-cache, `slice71_rerank_policy`, `slice71_open_report`, and `test_openreport.py` / `openreport.test.ts` |
| `AC31-50-07` | All existing `OpenReport` boundary/mock tests (`src/python/tests/test_openreport.py`, `src/ts/tests/openreport.test.ts`) and FFI validation (`src/ts/src/evidence-validation.ts`, `_fathomdb.pyi`) cover the new fields with strict type checks. | Updated `pytest` & Node `node:test` suites |
| `AC31-50-08` | `fathomdb` CLI adds `migrate <db_path> --backup-dir <dir> [--json]` as a third top-level `Command::Migrate` root while preserving all 17 `DoctorCommand` variants (`fathomdb-cli` parser tests + `slice55_composed_recovery.rs`) and `Engine::open` Schema-38-only admission refusal (`user_version == 37` still fails `Engine::open`). | `fathomdb-cli` parser tests, `slice55_composed_recovery.rs`, and `Engine::open` Schema-37 refusal test |
| `AC31-50-09` | `fathomdb migrate` acquires the canonical `.lock` (`exit 71` when held), rejects `user_version != 37` (`exit 70` on `36`, `38`, `39`), and fails closed (`exit 70`) when any pending physical erasure closure, active/queued `_fathomdb_erasure_requests` row, root/FTS erasure debt, or `ERASURE_PENDING_REDACTION_COLLECTION` row exists, before any backup or schema mutation. | CLI integration tests covering lock contention (`71`), wrong `user_version` (`70`), and owed/queued/redaction erasure debt (`70`) |
| `AC31-50-10` | `fathomdb migrate` runs read-only edge endpoint preflight (`from_id`/`to_id` reporting both `dangling_edge_endpoints` and `superseded_edge_endpoints`), **fails closed with exit `70` (`E_MIGRATE_DANGLING_EDGES`) before backup/DDL whenever `dangling_edge_endpoints > 0`** while admitting stores where `dangling_edge_endpoints == 0` and `superseded_edge_endpoints > 0` (per accepted ADR `#76` & `R31-05`), WAL-checkpoints (`TRUNCATE`, `busy == 0`), writes a SHA-256-verified byte-identical backup inside `--backup-dir`, executes `fathomdb_schema::STEP_38_SQL` verbatim via `execute_batch` in one `BEGIN IMMEDIATE` transaction, and rolls back cleanly to byte-identical Schema 37 state with untouched backup on mid-DDL SQLite failure or fault injection. | Dangling fail-closed + superseded admit tests, mid-DDL rollback fault-injection test, and backup SHA-256 parity assertion |
| `AC31-50-11` | Post-upgrade `fathomdb migrate` DB passes `doctor check-integrity --full` (`exit 0`), has `sqlite_master` tables/indexes/triggers structurally equivalent to a freshly initialized Schema 38 DB (including `_fathomdb_commit_timestamps`, `canonical_edges_{from,to}_sup_kind_idx`, and `cpv` coalescing triggers), preserves pre-upgrade erased-row non-resurrection, opens cleanly under `Engine::open`, and returns cursor `0` / empty visibility for `as_of_epoch_ms` timestamps strictly before the first Schema-38 commit. | Golden Schema-37 upgrade test on copied `/mnt/data/worktrees/fathomdb-0.8.31-golden/golden-0.8.30-schema37.sqlite` + fresh-vs-migrated `sqlite_master` parity test |
| `AC31-50-12` | `src/python-cli/pyproject.toml` builds a `fathomdb-cli` wheel via Maturin `bindings = "bin"` + `targets = [{ name = "fathomdb", kind = "bin" }]` containing `.data/scripts/fathomdb` (or `fathomdb.exe`), zero `perf-regression-check` binary, and zero `_fathomdb` extension modules, while `src/python/pyproject.toml` adds `cli = ["fathomdb-cli==<version>"]` with zero `[project.scripts]` (`AC-041` preserved). | `test_slice130_package_boundary.py`, `scripts/check_python_wheel_contents.py`, clean-venv install test on stripped `PATH` without `cargo`/`rustc` |
| `AC31-50-13` | `scripts/set-version.sh`, `scripts/check-license-consistency.sh`, `scripts/release/wait-for-registry-version.py`, `scripts/release/smoke/smoke-pypi-wheel.{sh,ps1}`, and `scripts/check-release-contract-truth.py` enforce lockstep versions, byte-identical MIT `LICENSE`, PyPI visibility for `fathomdb-cli`, and `fathomdb[cli]` wheel smoke checks without breaking `build-python` / CUDA release contracts. | `test_set_version.sh`, `test_release_version_surfaces.sh`, `test_verify_release_gates.sh`, `test_check_license_consistency.sh`, `test_wait_for_registry_visibility.py`, `test_smoke_scripts.sh`, `test_pypi_smoke_shell.sh`, `test_release_contract_truth.sh` |
| `AC31-50-14` | `.github/workflows/release.yml` adds `build-python-cli` across all 5 targets (`x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu` on `manylinux_2_28`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`), gates `all-builds-passed`, downloads `python-cli-dist-*` in `publish-pypi` and `github-release`, and passes updated workflow scope/alignment tests. | `actionlint`, `test_release_workflow_scope.sh`, `test_linux_first_platform_scope.sh`, `test_runtime_release_alignment.sh`, `check-release-contract-truth.py` |
| `AC31-50-15` | Targeted public docs (`README.md`, `src/ts/README.md`, `docs/*`) and interface contracts (`dev/interfaces/*`) accurately document Slice 50 additions, `fathomdb[cli]` install, and `fathomdb migrate`, passing all markdown/doc-truth/link gates cleanly. | `./scripts/agent-lint-md.sh`, `./scripts/agent-lint-docs.sh`, `test_public_doc_truth.sh` |
| `AC31-50-16` | Full workspace verification (`./scripts/agent-verify.sh`) and scoped `default-reranker` / migration-rollback tests pass cleanly before merge, leaving `.github/workflows/release.yml` ready for Slice 60 (`build-inspector`). | `./scripts/agent-verify.sh` + targeted `--features default-reranker` Rust/Py tests |

---

## 3. Architecture & Draft Design

### 3.1 `resolve_evidence_many` (Rust Engine, SDK, Python, TypeScript & Conformance)

- **Core Types & Bounds (`src/rust/crates/fathomdb-engine/src/evidence.rs`, re-exported in `fathomdb-engine/src/lib.rs`, `fathomdb/src/lib.rs`, `fathomdb-sdk/src/lib.rs`):**
  - Define `pub const EVIDENCE_RESOLVE_MANY_MAX: usize = 50;`.
  - Define closed request and per-slot outcome types (no `serde` derives on engine types, matching `EvidenceResolveRequestV1`, `EvidenceRefV1`, and `FrozenReadContextV1`):

    ```rust
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct EvidenceResolveManyRequestV1 {
        pub schema_version: u32,
        pub evidence_refs: Vec<EvidenceRefV1>,
        pub context: FrozenReadContextV1,
    }

    #[derive(Clone, Debug, PartialEq)]
    pub enum EvidenceResolveManyItemV1 {
        Resolved(Box<ResolvedEvidenceV1>),
        Refused(EvidenceErrorV1),
    }
    ```

    - **Why single `context` + `evidence_refs` rather than `Vec<EvidenceResolveRequestV1>`:** All evidence refs from a `search_with_evidence` result share one `FrozenReadContextV1`; validating one frozen context against the DB snapshot once inside a single reader/connection lock prevents cross-item context ambiguity and avoids N redundant context deserialization/HMAC checks.
    - **Why per-reference outcomes (`get_many` partial-result style):** Mandated directly by accepted `ADR-0.8.31-cli-pypi-wheel-and-37-to-38-upgrade.md` (`#76`), allowing callers to hydrate every still-authorized evidence token from a batch while inspecting typed per-slot refusals (`EvidenceUnavailable` at `/evidenceRefs/{i}`) for individually revoked/erased/tampered refs.
  - Engine method signature:
    - `Engine::resolve_evidence_many(&self, request: &EvidenceResolveManyRequestV1) -> Result<Vec<EvidenceResolveManyItemV1>, EngineError>`
  - Bindings signatures:
    - **`fathomdb-sdk`:** `Engine::resolve_evidence_many(&self, request: &EvidenceResolveManyRequestV1) -> Result<Vec<EvidenceResolveManyItemV1>>` (sync, `ErrorKind` mapping).
    - **Python (`types.py`, `_sdk_evidence.py`, `engine.py`, `_fathomdb.pyi`, `__init__.py`):**
      - `@dataclass(frozen=True) class EvidenceResolveManyRequestV1: evidence_refs: list[str] | tuple[str, ...]; context: FrozenReadContextV1; schema_version: int = 1`
      - `@dataclass(frozen=True) class EvidenceResolveManyItemV1: status: Literal["resolved", "refused"]; resolved: ResolvedEvidenceV1 | None = None; reason: str | None = None; field_path: str | None = None`
      - `engine.resolve_evidence_many(request: EvidenceResolveManyRequestV1) -> list[EvidenceResolveManyItemV1]`
    - **TypeScript (`src/ts/src/evidence.ts`, `src/ts/src/evidence-validation.ts`, `src/ts/src/core.ts`, `src/ts/src/index.ts`, `src/ts/src/binding.ts`):**
      - `export interface EvidenceResolveManyRequestV1 { schemaVersion: 1; evidenceRefs: string[]; context: FrozenReadContextV1; }` (validated with `assertKnownEvidenceKeys(request, ["schemaVersion", "evidenceRefs", "context"])` mirroring `EvidenceResolveRequestV1`).
      - `export type EvidenceResolveManyItemV1 = { status: "resolved"; resolved: ResolvedEvidenceV1 } | { status: "refused"; reason: EvidenceErrorReasonV1; fieldPath: string };`
      - `engine.resolveEvidenceMany(request: EvidenceResolveManyRequestV1): Promise<EvidenceResolveManyItemV1[]>`

- **Validation, Bounds, Duplicates, Snapshot Consistency & Per-Slot Nondisclosure Semantics (Strict Precedence):**
  1. **Schema version check (whole-request failure):** If `schema_version != 1`, return `EngineError::Evidence(EvidenceErrorV1::new(EvidenceErrorReasonV1::UnsupportedSchemaVersion, "/schemaVersion"))`.
  2. **Bounds check (before dedup, before DB lock, whole-request failure):** Validate `1..=EVIDENCE_RESOLVE_MANY_MAX` (`1..=50`) on raw `evidence_refs.len()` (rejecting `0` or `> 50` even if entries duplicate the same ref). Maps to `EngineError::InvalidArgument` (`ErrorKind::InvalidArgument` in `fathomdb-sdk`, `InvalidArgumentError` in Python and TypeScript — matching `limit` `1..=100` in `_sdk_evidence.py` and `search.ts`). Check FFI NUL-byte / non-string / type guards at Python & TS boundaries before native dispatch (`TypeError` for non-list/non-str elements; `WriteValidationError` for embedded NUL bytes per AC-068a).
  3. **Frozen context & snapshot validation (whole-request failure, matching `search_api.rs`):**
     - Authenticate `request.context` once and call `frozen_read::validate_snapshot` inside one reader/connection lock (`BEGIN DEFERRED` read transaction). Collapse **both** `authenticate` and `validate_snapshot` failures into whole-call **`EngineError::Evidence(EvidenceErrorV1::unavailable())`** for nondisclosure, identically to `resolve_evidence`.
     - After resolving all slots and firing `evidence_before_resolve_return_hook`, re-validate `frozen_read::validate_snapshot` before returning (collapsing post-resolve snapshot drift into whole-call `EngineError::Evidence(EvidenceErrorV1::unavailable())`).
  4. **Single-lock / single-snapshot deduplicated resolution with per-slot partial results:**
     - Iterate `evidence_refs.iter().enumerate()` inside the single authenticated snapshot:
       - At Python/TS/N-API/PyO3 boundaries, malformed ref strings (e.g., `"bad-ref"`) MUST NOT fail the batch with `InvalidArgumentError`; pass them into per-slot nondisclosure handling so each bad slot yields `Refused(EvidenceErrorV1 { reason: EvidenceUnavailable, field_path: format!("/evidenceRefs/{i}") })` indistinguishable from tampered MAC hex, wrong DB key, wrong context, or erased/superseded rows.
       - Maintain a local cache `resolved_by_token: HashMap<&str, Result<ResolvedEvidenceV1, EvidenceErrorReasonV1>>` keyed by exact token string so duplicate valid or refused refs execute HMAC + SQL resolution once and emit identical outcomes per slot, preserving exact input arity and order (`[r0, bad, r0] -> [Resolved(res0), Refused(EvidenceUnavailable, "/evidenceRefs/1"), Resolved(res0)]`).
       - On first occurrence of a token, run full HMAC decode, DB-scope check, frozen-context digest binding check, canonical source/derived provenance & visibility authorization check, and byte/span reconstruction via the exact shared internal helper used by `resolve_evidence`.
       - Map any per-slot nondisclosure refusal (`EvidenceUnavailable`: malformed syntax, tampered MAC, wrong DB, wrong frozen context, erased source/node/edge, superseded/deleted/out-of-window lifecycle, or attribute eligibility denial) to `EvidenceResolveManyItemV1::Refused(EvidenceErrorV1::new(EvidenceErrorReasonV1::EvidenceUnavailable, format!("/evidenceRefs/{i}")))`. Map corrupt/incomplete authorized storage to `Refused(EvidenceErrorV1::new(reason, format!("/evidenceRefs/{i}")))`.
     - Validate native response arrays in TypeScript via `validateNativeResolvedEvidence` on every `"resolved"` item and strict reason/fieldPath validation on every `"refused"` item (`src/ts/src/evidence-validation.ts`).

- **Governed Surface Parity & Pin (`src/conformance/`, `scripts/`):**
  - Slice 50 adds `engine.resolve_evidence_many` as `"state": "live"` to `src/conformance/governed-operation-parity.json`:
    - `python`: `locator: "engine_instance"`, `spelling: "resolve_evidence_many"`
    - `typescript`: `locator: "engine_instance"`, `spelling: "resolveEvidenceMany"`
    - `rust`: `locator: "engine_instance"`, `spelling: "resolve_evidence_many"`
    - `signed_members`: `["resolve_evidence_many", "resolveEvidenceMany"]` (no `"Engine."` prefix — matching `engine.resolve_evidence` in `src/conformance/governed-operation-parity.json` and `test_surface.py`).
  - Append `["resolve_evidence_many", "resolveEvidenceMany"]` to `src/conformance/governed-surface-allowlist.json` (+1 live canonical operation and +2 signed allowlist members atop live post-Slice-20 HEAD: **`55 -> 56` live ops** and **`91 -> 93` allowlist members**, keeping `core == 5`, `recovery_denylist == 5`, `runtime_controls == 2`).
  - Recompute `scripts/governed-surface-pin.json` (`pinned_at_commit`, `pinned_at_commit_short`, `git_blob_sha1`, `sha256`, `counts.{allowlist, core, recovery_denylist, runtime_controls}`, and full `allowlist`) per `scripts/check-governed-surface-pin.sh`; update exact count assertions in `scripts/tests/test_check_governed_surface_pin.sh`, `scripts/tests/test_generated_public_docs.py`, and `src/ts/tests/sdk-surface-parity.test.ts`.
  - Regenerate `docs/generated/sdk-operation-map.md` via `python3 scripts/generate-public-docs.py --write`.

### 3.2 Opt-In Search Hints (`search(hints=)` across `search`, `search_frozen`, and `EvidenceSearchRequestV1`)

- **Core `SearchResult` & Pure Hint Planner Reuse (`src/rust/crates/fathomdb-engine/src/search_types.rs`, `hints.rs`, `search.rs`, `evidence.rs`):**
  - Add `pub hints: Option<Vec<Hint>>` to `SearchResult` (`#[non_exhaustive]` already enables additive field).
  - Update all internal `SearchResult` constructors in `search.rs`, `search_api.rs`, `evidence.rs` (`build_search_result`), and test fixtures. Keep `search_text_only`, `search_projected_text`, and `search_expand*` unchanged with `hints: None`.
  - Keep `hints: false` default zero-allocation / zero behavior change (`hints: None`, `search_hints` not called).
  - When `hints == true`: construct `SearchFacts::new(results.len(), limit, HintMode::Hybrid)` populated with `.with_degradations(degradation_arm_cause_slice)` (first degradation mapped from `ArmDegradation`, plus `.with_hybrid_available(hybrid_available)` reflecting whether an embedder is configured without dense unavailability) and call `fathomdb_engine::hints::search_hints(&facts, &mut out)` (`HINT_LIMIT_CAP = 100`), setting `hints: Some(out)`.
  - Ensure `SearchReaderWork` remains boxed inside `ReaderRequest` (`size_of::<ReaderRequest>() <= 128` assertion in `src/rust/crates/fathomdb-engine/src/lib.rs` stays invariant).
  - Add `pub hints: bool` (default `false`) and builder `pub fn with_hints(mut self, hints: bool) -> Self` to `#[non_exhaustive]` `EvidenceSearchRequestV1` (`src/rust/crates/fathomdb-engine/src/evidence.rs`; no `serde` derives on engine request structs). Note: `search_text_only` is intentionally omitted per accepted `ADR-0.8.31` `#76` (`search`, `search_frozen`, and `EvidenceSearchRequestV1` only).

- **Rust Facade & Rust SDK (`fathomdb`, `fathomdb-sdk`):**
  - Re-export `Hint`, `HintAction`, `HintArm`, `HintCause`, `HintMode`, `HintReason` from `fathomdb` (mirroring `fathomdb-engine/src/lib.rs` and `fathomdb-sdk/src/lib.rs`).
  - Keep existing `fathomdb::Engine::search*` / `search_frozen*` signatures non-breaking (`hints: false`) and provide additive explicit-hints overloads (`search_with_options` / `search_with_hints` / `search_frozen_with_hints`) or internal options builder consumed by `fathomdb-sdk`, `fathomdb-py`, and `fathomdb-napi`.
  - In `fathomdb-sdk/src/options.rs`, add `pub hints: bool` (default `false`) to `#[non_exhaustive]` `SearchOptions` and `FrozenSearchOptions`, passing `options.hints` to `search`, `search_frozen`, and `EvidenceSearchRequestV1`.

- **Python & TypeScript Bindings:**
  - **Rust bindings (`fathomdb-py`, `fathomdb-napi`):**
    - Add `PyHint` (`action: String`, `reason: String`, `mode: Option<String>`, `limit: Option<usize>`, `arm: Option<String>`, `cause: Option<String>`) and `NativeHint` (with `limit?: number`, optional string properties) mapping `HintReason`/`HintAction`/`HintMode`/`HintArm`/`HintCause` lower-snake strings cleanly (`"search" | "none"`, `"degraded" | "no_results" | "more_results"`, `"text" | "hybrid"`).
    - Wire `hints: bool = false` on native `search`, `search_frozen`, and `search_with_evidence` (`EvidenceSearchRequestV1`), returning `hints: Option<Vec<PyHint>>` / `hints?: NativeHint[]` on `SearchResult`.
  - **Python SDK (`types.py`, `_sdk_search.py`, `_sdk_evidence.py`, `engine.py`, `_fathomdb.pyi`, `__init__.py`):**
    - Export `@dataclass(frozen=True) class Hint: action: str; reason: str; mode: str | None = None; limit: int | None = None; arm: str | None = None; cause: str | None = None` in `types.py` and `__init__.py` (`__all__`), mirroring Rust `Hint` and TypeScript `Hint` (verified zero collisions in `src/python/fathomdb/__init__.py`).
    - Add `hints: list[Hint] | None = None` to `SearchResult`.
    - Add keyword-only `hints: bool = False` to `Engine.search` and `Engine.search_frozen`, and `hints: bool = False` to `EvidenceSearchRequestV1`. Enforce `if not isinstance(hints, bool): raise TypeError("hints must be a bool")` (never accept `int`/`1`/`0` or `None`).
    - Update `src/python/tests/python_surface_additions.json` and `src/python/tests/test_slice130_package_boundary.py` (which pins exact `inspect.signature(fathomdb.Engine.search)` and reverse-patches `_fathomdb.pyi`) alongside every new Python public symbol/signature.
  - **TypeScript SDK (`src/ts/src/search.ts`, `evidence.ts`, `evidence-validation.ts`, `core.ts`, `binding.ts`, `index.ts`):**
    - Export `HintAction`, `HintReason`, `HintMode`, `Hint` (`{ action, reason, mode?, limit?, arm?, cause? }`) and add `hints?: Hint[]` to `SearchResult`.
    - Accept `hints?: boolean` on `SearchCallOptions`, `SearchOptions`, `FrozenSearchOptions`, and `EvidenceSearchRequestV1`; reject non-boolean non-undefined values with `TypeError`. Map `NativeSearchResult.hints` in `mapNativeSearchResult` (`search.ts`) and validate hint variants inside `validateNativeEvidenceSearch` (`evidence-validation.ts`).

### 3.3 `OpenReport` Capability Booleans (`reranker_compiled`, `cuda_compiled`, `rerank_available`)

- **Semantics & Zero-Network Guarantee (`fathomdb-embedder`, `fathomdb-engine`):**
  - Add three boolean fields to `OpenReport` in `src/rust/crates/fathomdb-engine/src/open.rs`:
    1. `pub reranker_compiled: bool` — exact compile-time truth: `cfg!(feature = "default-reranker")` (which is automatically implied by `rerank-cuda` and `rerank-metal`).
    2. `pub cuda_compiled: bool` — exact compile-time truth: `cfg!(any(feature = "embed-cuda", feature = "rerank-cuda"))`.
    3. `pub rerank_available: bool` — open-time runtime readiness on successful open, evaluated **without any network call, model weight load, or CUDA device allocation**:
       - Always `false` when `!cfg!(feature = "default-reranker")`.
       - Preserve `open.rs` fail-closed `Engine::open` on `FATHOMDB_RERANK_DEVICE=cuda` forced-CUDA policy failure (`ADR-0.8.23` `#43`, pinned by `slice71_rerank_policy` and `slice71_open_report`) — do **not** swallow `resolve_default_reranker_device_from_env()` errors.
       - When `cfg!(feature = "default-reranker")` and open succeeds: `reranker_device_resolution.is_some() && fathomdb_embedder::reranker_locally_ready()`, where `fathomdb-embedder` checks **only in-process `module_models::RERANKER` state** (`Loaded` -> `true`, `Unavailable` / `DevicePolicy` -> `false`) **and local disk SHA-256 verification** of `config.json`, `tokenizer.json`, and `model.safetensors` under `FATHOMDB_RERANKER_CACHE` / `dirs::cache_dir()`, with zero HTTP downloads and zero Candle/CUDA tensor instantiation.
- **Cross-Language Propagation & Boundary Fixtures:**
  - **`fathomdb-py` & Python SDK (`src/rust/crates/fathomdb-py/src/types.rs`, `src/python/fathomdb/types.py`, `_sdk_open.py`, `_fathomdb.pyi`):**
    - Expose `reranker_compiled: bool`, `cuda_compiled: bool`, `rerank_available: bool` on `PyOpenReport` and dataclass `OpenReport`. Enforce strict `bool` parsing (no silent default fallback on missing keys) in `_map_open_report` and update every `_map_open_report` / `OpenReport` mock literal across `src/python/tests/` (including `test_openreport.py`).
  - **`fathomdb-napi` & TypeScript SDK (`src/rust/crates/fathomdb-napi/src/types.rs`, `src/ts/src/binding.ts`, `src/ts/src/open.ts`):**
    - Expose `reranker_compiled: bool`, `cuda_compiled: bool`, `rerank_available: bool` in Rust (`rerankerCompiled`, `cudaCompiled`, `rerankAvailable` in TS via napi-rs). Enforce strict boolean validation in `mapOpenReport` and update every `mapOpenReport` / `OpenReport` mock literal across `src/ts/tests/` (including `openreport.test.ts`).

### 3.4 Minimal `fathomdb migrate` (`37 -> 38`) CLI Root

- **CLI Surface (`src/rust/crates/fathomdb-cli/src/lib.rs`, `src/rust/crates/fathomdb-cli/Cargo.toml`, `src/rust/crates/fathomdb/src/lib.rs`):**
  - Extend `pub enum Command` with sibling variant `Migrate(MigrateArgs)`:
    - Synopsis: `fathomdb migrate --backup-dir <dir> [--json] <db_path>` (Clap positional `<DB_PATH>`, required `--backup-dir <DIR>`, optional `--json`).
    - All 17 variants of `DoctorCommand` remain untouched (`fathomdb-cli` parser tests + `slice55_composed_recovery.rs`).
  - **Facade operator re-export & module boundary:** `fathomdb-cli` depends on `fathomdb` (`features = ["operator"]`), NOT `fathomdb-engine` directly. Place `migrate_37_to_38` / `MigrateReport` in `fathomdb-engine::operator::data_plane` (or declare any new `operator::migrate` submodule in `dev/tools/module-boundary-policy.txt` for `scripts/check-module-boundaries.sh`), re-export them behind `#[cfg(feature = "operator")]` in `src/rust/crates/fathomdb/src/lib.rs`, and add a `compile_fail` absence proof to `governed_surface_method_absence_proof` proving `migrate` is absent without `feature = "operator"`.
  - Exit codes strictly align with `dev/interfaces/cli.md` and `exit_code`:
    - `0` (`OK`): upgraded `37 -> 38`, backup SHA-256 verified, `check-integrity --full` clean.
    - `65` (`DOCTOR_FOUND_ISSUES`): DDL committed `37 -> 38` but post-upgrade `check-integrity --full` reported structural/logical/semantic findings (JSON surfaces `backup_path`, `backup_sha256`, and `findings` for operator rollback).
    - `70` (`UNRECOVERABLE`): missing/empty/corrupt DB, `--backup-dir` unusable/unsafe, `user_version != 37` (`36`, `38`, `39`, etc.), unfinished/queued/active erasure or redaction obligation blocking upgrade, **genuinely dangling edge endpoints (`dangling_edge_endpoints > 0`, `E_MIGRATE_DANGLING_EDGES`)**, or DDL failure rolled back to intact Schema 37.
    - `71` (`LOCK_HELD`): active `.lock` held by another process (`DatabaseLocked`) or busy WAL checkpoint (`busy != 0`).

- **Shared Schema 38 DDL Single Source of Truth (`fathomdb-schema` + `fathomdb-engine::operator`):**
  - Reuse the existing `pub const STEP_38_SQL: &str` already landed in Slice 20 (`src/rust/crates/fathomdb-schema/src/lib.rs`) verbatim via `conn.execute_batch(fathomdb_schema::STEP_38_SQL)` (matching `apply_one` in `fathomdb-schema` — **never** split SQL on `;`, which breaks `CREATE TRIGGER ... BEGIN ...; END;`). `STEP_38_SQL` already contains every Schema 38 additive object (`canonical_nodes.{scope,properties}`, `canonical_edges.{scope,properties}`, `operational_{mutations,state}.scope`, `_fathomdb_projection_registry.{entity_class,vector_kinds}`, `_fathomdb_vector_kinds`, `canonical_property_values` + 4 indexes + `visibility_coalescing_trigger_sql!("cpv", "canonical_property_values")` triggers, `_fathomdb_commit_timestamps` + `_fathomdb_commit_timestamps_ms_idx`, `canonical_nodes_logical_sup_idx`, `canonical_edges_{from,to}_active_idx`, `canonical_edges_{from,to}_sup_kind_idx`).
  - Place the test-only fault-injection hook (`#[cfg(any(test, feature = "migration-test-hooks"))]`) right between `conn.execute_batch(fathomdb_schema::STEP_38_SQL)` and `PRAGMA user_version = 38` inside the single `BEGIN IMMEDIATE` transaction so rollback proves atomicity across the entire Schema 38 DDL batch.
  - Note on `scripts/agent-lint-migrations.py`: `fathomdb-schema` defines steps in Rust (`MIGRATIONS`), not `.sql` files; running `check_migration_accretion` and `check_migration_logical_id_pin` in `fathomdb-schema` tests remains green, and `Engine::open` continues to refuse `PRAGMA user_version != 38`.

- **7-Step Fail-Closed Execution Flow (`migrate_37_to_38` in `fathomdb-engine::operator`):**
  1. **Quiescence & Exclusive Product Lock:** Canonicalize `db_path` via `canonical_database_path`; verify regular non-empty file; acquire `<db_path>.lock` exclusively via `acquire_lock_without_metadata_mutation(&canonical_path)?.initialize()?` (`LOCK_HELD` `71` on contention). Re-validate regular file, refuse non-empty `-journal`, capture `ShmSnapshot` (`open.rs`) for restore on preflight refusal, and validate `--backup-dir` (create directory if absent, verify writable directory distinct from colliding with live DB path, construct timestamped unique backup filename `<stem>.schema37.<utc-compact>-<pid>.sqlite` + `.sha256` manifest via `create_new` so prior backups are never overwritten).
  2. **Header, Integrity & Strict Schema-37 Admission:** Open immutable read-only connection (`query_only = ON`), run `probe_database_header`, `probe_open_integrity`, `reject_legacy_shape`, read effective `PRAGMA user_version` (checking both immutable main file and read-only WAL-aware view per `open_recovery_connection` in `src/rust/crates/fathomdb-engine/src/operator/data_plane.rs`). If `user_version != 37`, restore SHM snapshot and fail closed (`exit 70`, typed code `E_MIGRATE_ALREADY_CURRENT` for `38` or `E_MIGRATE_UNSUPPORTED_VERSION` otherwise). Also run `validate_recovery_schema_invariants(&validation, 37)`.
  3. **Fail-Closed Unfinished-Erasure Preflight:** On the read-only snapshot, refuse (`exit 70`, code `E_MIGRATE_PENDING_ERASURE`) if ANY of the following is non-empty/non-zero:
     - `dependency_closure::pending_physical_closures(conn)` (owed physical closures),
     - `physical_erasure::diagnostics(conn)` (unfinished root or FTS retained-term sanitization debt),
     - `_fathomdb_erasure_requests WHERE phase IN ('queued', 'executing', 'sanitizing', 'finalizing')`,
     - `operational_mutations WHERE collection_name = ERASURE_PENDING_REDACTION_COLLECTION`.
     (Message instructs operator to complete erasures via `fathomdb doctor check-integrity` / `fathomdb recover --accept-data-loss --complete-erasures` before running `migrate`.)
  4. **Fail-Closed Read-Only Edge Endpoint Preflight (`dangling` vs `superseded`, ADR `#76` & `R31-05`):** Count and distinguish active edges (`superseded_at IS NULL`) whose `from_id` / `to_id` endpoints:
     - have **zero** rows with matching `logical_id` in `canonical_nodes` (`dangling_edge_endpoints`), vs
     - have rows with matching `logical_id` in `canonical_nodes` where none is currently active / non-superseded (`superseded_edge_endpoints`).
     Record both counts in `MigrateReport`. **Whenever `dangling_edge_endpoints > 0`, restore SHM snapshot and fail closed with exit `70` (`E_MIGRATE_DANGLING_EDGES`) before creating any backup or mutating schema** (never silently dropping edges); admit stores where `dangling_edge_endpoints == 0` even when `superseded_edge_endpoints > 0`.
  5. **WAL-Checkpointed & SHA-256-Verified Backup:** Open RW connection, execute `PRAGMA wal_checkpoint(TRUNCATE)` verifying `busy == 0` (else `LOCK_HELD` `71`) and `-wal` zeroed. Compute `SHA-256` over the quiescent source DB bytes, write copy atomically into `--backup-dir/<backup_name>` (`fsync`), re-read backup bytes from disk and verify `sha256(backup_bytes) == sha256(source_bytes)`, `fsync`, and write sibling `.sha256` receipt file before executing any DDL.
  6. **Single-Transaction `BEGIN IMMEDIATE` Schema 38 DDL:** Inside `BEGIN IMMEDIATE`: execute `conn.execute_batch(fathomdb_schema::STEP_38_SQL)`, optional test-only fault-injection hook (`#[cfg(any(test, feature = "migration-test-hooks"))]`) right before final version stamp, `PRAGMA user_version = 38`, and `COMMIT`. Any SQL error triggers explicit/automatic `ROLLBACK` leaving Schema 37 byte-identical with zero partial Schema 38 columns/tables/triggers/indexes and preserving the verified backup on disk.
  7. **Post-Upgrade Integrity Check & Checkpoint:** Run `PRAGMA wal_checkpoint(TRUNCATE)`, then drop the temporary migrate RW handle cleanly while retaining exclusive lock (or invoke shared `physical_section(..., full=true)` + `logical_section` + `semantic_section` + `pending_erasures` plus verify read/open via `admit_current_database` at version `38`). Emit normative JSON (`verb: "migrate"`, `status`, `from_version: 37`, `to_version: 38`, `db_path`, `backup_path`, `backup_sha256`, `preflight: { dangling_edge_endpoints, superseded_edge_endpoints }`, `integrity: { status, full: true }`) or clean human text summary. Note pre-upgrade historical invariant: rows written before Schema 38 have no `_fathomdb_commit_timestamps` entries, so post-upgrade `as_of_epoch_ms` queries strictly earlier than the first Schema-38 commit resolve deterministically to cursor `0` / empty visible set.

### 3.5 Lockstep `fathomdb-cli` PyPI Binary Wheel & `fathomdb[cli]` Extra

- **New Directory `src/python-cli/`:**
  - `src/python-cli/LICENSE`: byte-identical copy of root `LICENSE` (MIT).
  - `src/python-cli/README.md`: concise PyPI package README describing `fathomdb-cli` (`recover`, `doctor`, `migrate`) and companion `fathomdb[cli]` install.
  - `src/python-cli/pyproject.toml`:
    - `[build-system]` pinned identically to `src/python/pyproject.toml`: **`maturin>=1.9,<2`** (`maturin 1.8.0` fails to parse PEP-639 `license-files = ["LICENSE"]`).
    - `[project]`: `name = "fathomdb-cli"`, `version = "0.8.30"` (initialized at live Axis W to pass `scripts/set-version.sh --check-files` and `scripts/verify-release-gates.sh`, bumped to `0.8.31` together at release cut), `requires-python = ">=3.10"`, `license = "MIT"`, `license-files = ["LICENSE"]`, `readme = "README.md"`, MIT classifier.
    - `[tool.maturin]`:
      - `bindings = "bin"`
      - `manifest-path = "../rust/crates/fathomdb-cli/Cargo.toml"`
      - `targets = [{ name = "fathomdb", kind = "bin" }]` (verified supported by `maturin 1.14.0`: compiles and packages **only** `[[bin]] name = "fathomdb"`, excluding `[[bin]] name = "perf-regression-check"` cleanly!).
- **Python SDK Extra (`src/python/pyproject.toml`):**
  - Keep **zero** `[project.scripts]` section (`AC-041` invariant).
  - Add `[project.optional-dependencies]`:
    - `cli = ["fathomdb-cli==0.8.30"]` (exact Axis-W lockstep maintained by `scripts/set-version.sh`).

### 3.6 Release Workflow, Packaging & Tooling Lockstep

Update every workflow, release script, and gate test referencing PyPI artifacts or governed surfaces:

1. **`.github/workflows/release.yml`:**
   - Add `build-python-cli` job (`needs: [verify-release, verify-cuda-trusted-route]`, matching `build-python`'s SHA-pinned `actions/checkout` ref and SHA-pinned `dtolnay/rust-toolchain` with `toolchain: "1.95.0"` for `scripts/tests/test_runtime_release_alignment.sh`):
     - 5-target matrix matching `test_linux_first_platform_scope.sh`: `x86_64-unknown-linux-gnu` (`ubuntu-latest`, `manylinux: "2_28"`), `aarch64-unknown-linux-gnu` (`ubuntu-24.04-arm`, `manylinux: "2_28"`), `x86_64-apple-darwin` (`macos-15-intel`, `--target x86_64-apple-darwin`), `aarch64-apple-darwin` (`macos-15`), `x86_64-pc-windows-msvc` (`windows-latest`).
     - Run SHA-pinned `PyO3/maturin-action@e83996d129638aa358a18fbd1dfb82f0b0fb5d3b` with `working-directory: src/python-cli`, `command: build`, `args: --release --out dist --locked` (NO CUDA flags on default PyPI wheel; keeps CPU parity with `cargo install fathomdb-cli`).
     - Post-build wheel inspection inside `build-python-cli` (where repo checkout already exists): run `python3 scripts/check_python_wheel_contents.py --kind cli dist/*.whl` verifying ONLY `.data/scripts/fathomdb(.exe)` is packaged and ZERO `perf-regression-check` or `_fathomdb` `.so/.pyd` exists; on Linux targets extract `.data/scripts/fathomdb` and verify glibc ceiling via `scripts/check-glibc-floor.sh`.
     - Upload artifact `name: python-cli-dist-${{ matrix.target }}` via pinned `upload-artifact-retry`.
   - Gate **both** `all-builds-passed.needs` AND `all-builds-passed.if` (`&& needs.build-python-cli.result == 'success'`) on `build-python-cli` so a failed CLI wheel build blocks publication.
   - In `publish-pypi` and `github-release`: download `pattern: python-cli-dist-*` alongside `pattern: python-dist-*`. Since `publish-pypi` has no repo checkout, wheel content validation runs in `build-python` and `build-python-cli` prior to upload (or add a SHA-pinned `actions/checkout` with `ref: ${{ env.RELEASE_CHECKOUT_REF }}`, `persist-credentials: false`, `permissions: contents: read` if re-verifying downloaded wheels). **Slice 80 prerequisite note:** PyPI OIDC Trusted Publishing for the brand-new `fathomdb-cli` project requires configuring a PyPI Pending Publisher (`environment: pypi`) before Slice 80 tag publication.
   - Serialize Slice 60 `build-inspector` additions strictly after Slice 50 lands.
2. **Version & License Gates:**
   - `scripts/set-version.sh`: read/write `src/python-cli/pyproject.toml` `[project].version` and `src/python/pyproject.toml` `cli = ["fathomdb-cli==<ver>"]` in both `--check-files` and mutate modes.
   - `scripts/check-license-consistency.sh`: enforce `src/python-cli/pyproject.toml` (`license = "MIT"`, `license-files = ["LICENSE"]`), byte-identical `src/python-cli/LICENSE`, and `--only python --with-wheel` inspection for both `src/python` and `src/python-cli` wheels.
3. **Registry Visibility & PyPI Smoke Gates:**
   - `scripts/release/wait-for-registry-version.py`: include `RegistryPackage("pypi", "fathomdb-cli", version)` in `wait-release` **gated on `(root / "src/python-cli/pyproject.toml").is_file()`** (preserving historical recovery tags such as `v0.8.20` that lack `src/python-cli/`) and verify its declared version matches `version`.
   - `scripts/release/smoke/smoke-pypi-wheel.sh` & `smoke-pypi-wheel.ps1`: install `"fathomdb[cli]==${PIP_VERSION}"` into the isolated venv, extend the transient CDN-skew retry regex from `fathomdb==` to `(fathomdb|fathomdb-cli)==` in both `.sh` and `.ps1`, assert `python -c "import fathomdb; print(fathomdb.__version__)"` (regression-locking `_coinstall.py` compatibility), `fathomdb --version`, `fathomdb doctor platform --json` (and zero `cargo` required / zero `perf-regression-check` present in venv `bin`/`Scripts`), and check Linux glibc ceiling on both `_fathomdb*.so` and venv `fathomdb` binary.
4. **Release Contract & Script Unit Tests:**
   - Update `scripts/check-release-contract-truth.py`, `scripts/tests/test_release_contract_truth.sh`, `scripts/tests/test_release_workflow_scope.sh` (updating the `upload-artifact-retry` count check `13 -> 14` and adding `build-python-cli` matrix assertions), `scripts/tests/test_linux_first_platform_scope.sh`, `scripts/tests/test_runtime_release_alignment.sh`, `scripts/tests/test_set_version.sh`, `scripts/tests/test_release_version_surfaces.sh`, `scripts/tests/test_verify_release_gates.sh` (`SNAP_VERSIONED` adding `src/python-cli/pyproject.toml`), `scripts/tests/test_check_license_consistency.sh` (`mkfix()` seeding `src/python-cli/`), `scripts/tests/test_wait_for_registry_visibility.py`, `scripts/tests/test_smoke_scripts.sh`, `scripts/tests/test_pypi_smoke_shell.sh`, `scripts/check_python_wheel_contents.py`, and `scripts/tests/test_python_wheel_contents.py`.

### 3.7 Targeted Public, Interface & Generated Documentation Updates (`P3-12`–`P3-15`, `P6-02`–`P6-05`)

- **Interface Specs (`dev/interfaces/`):**
  - `dev/interfaces/cli.md`: add `fathomdb migrate <db_path> --backup-dir <dir> [--json]` root section, JSON schema, 7-step safety guarantees, exit codes (`0`/`65`/`70`/`71`), PyPI `fathomdb-cli` / `fathomdb[cli]` distribution contract, and schema `38` open boundary note.
  - `dev/interfaces/python.md`, `dev/interfaces/typescript.md`, `dev/interfaces/rust-sdk.md`, `dev/interfaces/rust.md`, `dev/interfaces/wire.md`: document `resolve_evidence_many` / `resolveEvidenceMany` (`1..=50`, per-reference partial-result outcomes with per-slot `EvidenceUnavailable` nondisclosure), `hints` parameter & `SearchResult.hints` (`Hint` type), `OpenReport` capability booleans, and `EvidenceSearchRequestV1` / `EvidenceResolveManyRequestV1` wire rules + updated governed op counts (`56` live ops / `93` allowlist members).
- **Public Docs (`README.md`, `src/ts/README.md`, `docs/`):**
  - `README.md`, `docs/install/python.md`, `docs/getting-started/index.md`, `docs/getting-started/quickstart.md`: add `fathomdb[cli]` / `fathomdb-cli` CLI install instructions (note `0.8.31` package posture cleanly per `scripts/check-public-doc-truth.py` pre-publish rules while keeping base `fathomdb` console-script-free), link `fathomdb migrate` for `37 -> 38` upgrades, and fix stale `0.8.20`/`0.8.28`/`37` references scoped to Slice 50 touchpoints.
  - `src/ts/README.md`: sync Slice 50 SDK methods/fields (`resolveEvidenceMany`, `hints`, `OpenReport` flags) and stale version references.
  - `docs/compatibility/index.md`, `docs/reference/cli.md`, `docs/operations/integrity-inspection.md`: document Schema `38`, explicit offline `fathomdb migrate --backup-dir <dir> <db_path>` `37 -> 38` upgrade + `doctor check-integrity --full`, and `fathomdb-cli` PyPI wheel option.
  - `docs/reference/{python-api,typescript-api,rust-sdk,rust-api}.md` & `docs/guides/frozen-evidence.md`: document `resolve_evidence_many`/`resolveEvidenceMany`, `hints` / `SearchResult.hints`, and `OpenReport` booleans.
  - Regenerate `docs/generated/sdk-operation-map.md` using `python3 scripts/generate-public-docs.py --write`.

---

## 4. Prerequisites Discovered

1. **Dual `[[bin]]` trap (`P4-14`) & Maturin 1.14 target filtering:** `src/rust/crates/fathomdb-cli/Cargo.toml` defines both `fathomdb` and `perf-regression-check`. Passing `--bin fathomdb` in `maturin build` CLI positional `[ARGS]...` forwards flags to `rustc`, NOT Cargo, which fails the build. Target filtering MUST use `[tool.maturin] targets = [{ name = "fathomdb", kind = "bin" }]` inside `src/python-cli/pyproject.toml` (verified on installed `maturin 1.14.0`), backed by explicit wheel ZIP assertions (`scripts/check_python_wheel_contents.py`) confirming `perf-regression-check` is absent.
2. **`OpenReport` non-exhaustive absence & test mock literals:** Adding public fields to `OpenReport` requires atomic updates to `src/python/tests/test_openreport.py` (`SimpleNamespace` literals), `src/ts/tests/openreport.test.ts` (mock object literals), and Rust facade/SDK structs/tests so none fail on missing `reranker_compiled`/`cuda_compiled`/`rerank_available` keys.
3. **`test_slice130_package_boundary.py` reverse-patch proof (`AC-041`):** `src/python/tests/test_slice130_package_boundary.py` enforces zero `[project.scripts]` on `src/python/pyproject.toml`. Putting the console binary exclusively in `src/python-cli/` (`fathomdb-cli`) and linking it via `cli = ["fathomdb-cli==<version>"]` optional extra preserves `AC-041` verbatim while allowing boundary tests to prove `fathomdb` alone exposes zero scripts and `fathomdb-cli` exposes only `fathomdb` (no `_fathomdb` native module, no `perf-regression-check`).
4. **Brittle counts & snapshots across release/conformance tests:**
   - `scripts/tests/test_release_workflow_scope.sh` asserts exact `upload-artifact-retry` invocation count `== 13` (must advance cleanly to `14` for `build-python-cli`).
   - `scripts/tests/test_verify_release_gates.sh` (`SNAP_VERSIONED`) backups/restores versioned manifests during synthetic-version mutation tests — must include `src/python-cli/pyproject.toml` or tests dirty the tree.
   - `scripts/tests/test_check_license_consistency.sh` (`mkfix()`) constructs an isolated synthetic repo fixture missing `src/python-cli/` — must seed `src/python-cli/{pyproject.toml,LICENSE}`.
   - `scripts/check-cuda-release-contract.py` inspects `workflow_job("build-python")` specifically; creating `build-python-cli` as a separate job rather than modifying `build-python`'s matrix preserves CUDA contract checks cleanly.
   - `scripts/tests/test_generated_public_docs.py`, `src/ts/tests/sdk-surface-parity.test.ts`, and `scripts/check-governed-surface-pin.sh` / `test_check_governed_surface_pin.sh` check exact governed op/member counts; apply `+1` live op / `+2` members incrementally atop Slice 20's post-`read.operational_collections` baseline.
5. **Verified post-Slice-40 entry state & cross-slice ordering:**
   - **Slices 20 (`975637f3c`), 30 (`8e44a47ad`), and 40 (`fe5e7d8a1`) have landed:** Schema 38 (`fathomdb_schema::STEP_38_SQL`, `_fathomdb_commit_timestamps`, `canonical_edges_{from,to}_sup_kind_idx`, `canonical_nodes_logical_sup_idx`, `cpv` visibility triggers), governed surface baseline (`55` live ops / `91` allowlist members pinned at `36802ba4e`), and bitemporal `graph_expand` (`as_of_epoch_ms` cursor-`0` pre-upgrade semantics) are live; the closed golden Schema-37 fixture is verified present at `/mnt/data/worktrees/fathomdb-0.8.31-golden/golden-0.8.30-schema37.sqlite` (634,880 bytes, SHA-256 `59d56eabb9a6e817168c5c540b4bb2ec6eb7f741fb24a72c7617df4aa5004548`).
   - **Blocks Slice 60 (`INSPECTOR`) release-workflow edits:** Slice 60 adds `build-inspector` to `.github/workflows/release.yml` and explicitly serializes after Slice 50 merges.
6. **Markdown plan-anchor lint discipline (`scripts/lint-plan-anchors.sh`):** When promoting this plan from `DRAFT`, never introduce bare `<file>:<line>` backtick tokens or `...` elisions inside `<needle> in <path>` citations; ensure every symbol/needle cited via `<needle> in <path>` resolves verbatim.

---

## 5. TDD Implementation & Verification Plan

### Wave 1 — Failing RED Tests First (Per-Subsystem Contracts)

Write failing RED tests covering every contract before implementing production code:

1. **Engine / SDK / Py / TS `resolve_evidence_many` tests:**
   - Bounds: `0` refs and `51` refs (including 51 identical duplicate refs) rejected before DB access with `InvalidArgument` / `InvalidArgumentError` across Rust, Python, and TypeScript.
   - `schema_version != 1` fails with `EvidenceError(UnsupportedSchemaVersion, "/schemaVersion")`.
   - Frozen-context failure / snapshot drift (including post-`evidence_before_resolve_return_hook` drift) fails the entire call with `EvidenceError(EvidenceUnavailable)`.
   - Duplicates: `[ref_a, ref_b, ref_a]` returns 3 `Resolved` entries with `[res_a, res_b, res_a]` in exact order inside one read snapshot.
   - Per-slot partial-result nondisclosure: `[valid_0, malformed_1]`, `[valid_0, tampered_1]`, `[valid_0, erased_source_1]`, `[valid_0, wrong_context_1]`, and `[valid_0, superseded_1]` return `[Resolved(res_0), Refused(EvidenceUnavailable, "/evidenceRefs/1")]` with identical per-slot nondisclosure across every failure cause.
2. **Engine / SDK / Py / TS `search(hints=)` tests:**
   - Default omitted `hints` and explicit `hints=False` on `search`, `search_frozen`, and `EvidenceSearchRequestV1` set `SearchResult.hints` to `None`/`undefined` with byte-identical hit rankings/snippets.
   - `hints=True` emits expected hints (`no_results`, `more_results`, `degraded` when dense/embedder degraded) across all three entry points and preserves nested `EvidenceSearchResultV1.search_result.hints`.
   - Non-boolean `hints` (`1`, `"true"`, `None` when passed explicitly in Py, `1` in TS) raises `TypeError` cleanly without native calls.
3. **`OpenReport` booleans & boundary mock tests:**
   - Feature-off build: `reranker_compiled == false`, `cuda_compiled == false`, `rerank_available == false`.
   - `--features default-reranker` with empty/unverified cache: `reranker_compiled == true`, `rerank_available == false`, zero network HTTP calls made during `Engine::open`; forced-CUDA policy failure (`FATHOMDB_RERANK_DEVICE=cuda` without CUDA) still fails `Engine::open` closed (`slice71_rerank_policy` / `slice71_open_report`).
   - `--features default-reranker` with SHA-256-verified cached weights in `FATHOMDB_RERANKER_CACHE` (and under `embed-cuda`/`rerank-cuda` compilation): `reranker_compiled == true`, `rerank_available == true`, `cuda_compiled` truthful.
   - Updated mock tests in `src/python/tests/test_openreport.py` and `src/ts/tests/openreport.test.ts` validating strict boolean mapping.
4. **`fathomdb migrate` CLI & Engine-Operator tests (in `src/rust/crates/fathomdb-cli/tests/` & `fathomdb-engine/tests/`):**
   - `fathomdb-cli` parser tests & `slice55_composed_recovery.rs`: `fathomdb migrate --backup-dir ... <db>` parses cleanly while all 17 `doctor` subcommands stay intact; `Engine::open` on a Schema-37 DB still fails with `IncompatibleSchemaVersion { seen: 37, supported: 38 }`; `governed_surface_method_absence_proof` proves `migrate_37_to_38` is absent without `feature = "operator"`.
   - Lock & version gates: active `.lock` returns exit `71`; `user_version == 38`, `36`, or `39` returns exit `70` leaving DB untouched.
   - Fail-closed erasure gates: Schema-37 DB with pending dependency closure, active/queued `_fathomdb_erasure_requests` (`queued`, `executing`, `sanitizing`, `finalizing`), physical root/FTS debt, or `ERASURE_PENDING_REDACTION_COLLECTION` returns exit `70` before creating backup or touching `user_version`.
   - Edge endpoint preflight (`from_id`/`to_id`): fixture with missing-target (`dangling_edge_endpoints > 0`) **fails closed with exit `70` (`E_MIGRATE_DANGLING_EDGES`) before backup/DDL** while reporting both counts in JSON; fixture with `dangling_edge_endpoints == 0` and `superseded_edge_endpoints > 0` succeeds (`exit 0`) and reports `superseded_edge_endpoints`.
   - **Mid-DDL rollback fault-injection test (`migration-test-hooks` / `test-hooks`):** Inject SQL/hook failure right between `conn.execute_batch(fathomdb_schema::STEP_38_SQL)` and `PRAGMA user_version = 38` inside `BEGIN IMMEDIATE`; assert CLI exits `70`, source DB SHA-256 equals pre-migrate SHA-256 (zero partial `scope`/`properties`/`canonical_property_values`/`_fathomdb_commit_timestamps`/indexes including `canonical_nodes_logical_sup_idx`/triggers), AND verified Schema-37 backup remains intact in `--backup-dir`.
   - **Fresh vs migrated schema equivalence test:** Compare `sqlite_master` objects (tables, columns, indexes, triggers) between a fresh Schema-38 `Engine::open` DB and a migrated `37 -> 38` DB for verbatim structural equality — explicitly asserting presence and exact DDL parity of `canonical_nodes` (`scope`, `properties`), `canonical_nodes_logical_sup_idx(logical_id, superseded_at, write_cursor)`, `canonical_property_values` and its 4 indexes, `cpv` visibility coalescing triggers, `_fathomdb_commit_timestamps` + `_fathomdb_commit_timestamps_ms_idx`, `canonical_edges_{from,to}_active_idx`, and `canonical_edges_{from,to}_sup_kind_idx`.
   - **Golden Schema-37 DB upgrade test:** Copy immutable `/mnt/data/worktrees/fathomdb-0.8.31-golden/golden-0.8.30-schema37.sqlite` (SHA-256 `59d56eabb9a6e817168c5c540b4bb2ec6eb7f741fb24a72c7617df4aa5004548`) into a temp dir (never mutate the golden original), run `fathomdb migrate --backup-dir <tmp_backup> --json <copy>`, assert exit `0`, backup SHA-256 matches golden SHA-256, `doctor check-integrity --full` exits `0`, `Engine::open` succeeds, pre-upgrade erased rows remain un-resurrected, and querying historical state with `as_of_epoch_ms` strictly prior to any Schema-38 commit resolves cleanly to cursor `0` / empty visible set.
5. **CLI PyPI packaging, clean-venv smoke, parity & release-contract tests:**
   - Extend `src/python/tests/test_slice130_package_boundary.py` & `scripts/tests/test_python_wheel_contents.py`: assert `fathomdb` keeps zero `[project.scripts]`, `fathomdb[cli]` depends on `fathomdb-cli==<version>`, and `fathomdb-cli` wheel contains `.data/scripts/fathomdb(.exe)`, zero `perf-regression-check`, and zero `_fathomdb` extension modules.
   - Build a real wheel from `src/python-cli/` via `maturin build` into a temp dir, install alongside built `fathomdb` (`fathomdb[cli]`) into a **fresh venv under a stripped `PATH` with zero `cargo` / `rustc`**, and execute `python -c "import fathomdb; print(fathomdb.__version__)"` (verifying `_coinstall.py` compatibility), `fathomdb --version`, `fathomdb doctor platform --json`, and `fathomdb migrate` end-to-end.
   - Update & run `scripts/tests/` suites (`test_release_workflow_scope.sh`, `test_linux_first_platform_scope.sh`, `test_runtime_release_alignment.sh`, `test_release_contract_truth.sh`, `test_set_version.sh`, `test_release_version_surfaces.sh`, `test_verify_release_gates.sh`, `test_check_license_consistency.sh`, `test_wait_for_registry_visibility.py`, `test_smoke_scripts.sh`, `test_pypi_smoke_shell.sh`, `test_check_governed_surface_pin.sh`, `test_check_sdk_surface_parity.py`, `test_generated_public_docs.py`, `test_public_doc_truth.sh`).

### Wave 2 — GREEN Implementation Order

1. `fathomdb-schema`: verify existing `pub const STEP_38_SQL: &str` (`src/rust/crates/fathomdb-schema/src/lib.rs`) is consumed verbatim via `conn.execute_batch` by both `fathomdb-schema` and `migrate_37_to_38` (zero `;`-splitting).
2. `fathomdb-embedder`: add zero-network, zero-CUDA `reranker_locally_ready()` helper checking `module_models::RERANKER` singleton state + local SHA-256 weight cache.
3. `fathomdb-engine`: implement `resolve_evidence_many` (per-ref `EvidenceResolveManyItemV1`, single-lock snapshot + post-resolve `validate_snapshot`), `SearchResult.hints` + `search_hints` integration (`search`, `search_frozen`, `EvidenceSearchRequestV1::with_hints`), `OpenReport.{reranker_compiled, cuda_compiled, rerank_available}` (preserving fail-closed forced-CUDA open), and `operator::migrate_37_to_38` (7-step flow with fail-closed dangling-endpoint gate + rollback fault-injection hook + `dev/tools/module-boundary-policy.txt` entry if in a new submodule).
4. `fathomdb` & `fathomdb-sdk`: expose facade/SDK types and methods cleanly (`#[cfg(feature = "operator")]` re-export of `migrate_37_to_38` / `MigrateReport` + `compile_fail` absence proof in `fathomdb`); ensure `fathomdb-sdk/src/options.rs` `SearchOptions` and `FrozenSearchOptions` add `hints: bool` (`default = false`).
5. `fathomdb-cli`: wire `Command::Migrate(MigrateArgs)` (`recover` / `doctor` / `migrate`), JSON & human output, and exit codes `0`/`65`/`70`/`71`.
6. `fathomdb-py` + `src/python/` and `fathomdb-napi` + `src/ts/`: implement bindings, Python/TS dataclasses & interfaces (`Hint`, `EvidenceResolveManyRequestV1`, `EvidenceResolveManyItemV1`), `InvalidArgumentError` bounds validation, FFI string/bool guards, `OpenReport` mocks, `python_surface_additions.json` + `test_slice130_package_boundary.py`, and `src/python/pyproject.toml` `cli` extra (`0.8.30` lockstep).
7. `src/python-cli/`: add `pyproject.toml` (`maturin>=1.9,<2`, `version = "0.8.30"`, `bindings = "bin"`, `targets = [{ name = "fathomdb", kind = "bin" }]`), `LICENSE`, and `README.md`.
8. `src/conformance/` + `scripts/governed-surface-pin.json`: update `governed-operation-parity.json`, `governed-surface-allowlist.json` (`56` live ops / `93` allowlist members, tokens `["resolve_evidence_many", "resolveEvidenceMany"]`), pin file, and `docs/generated/sdk-operation-map.md`.
9. `.github/workflows/release.yml` + `scripts/`: add `build-python-cli`, update both `all-builds-passed.needs` & `all-builds-passed.if`, `publish-pypi`, `github-release`, `set-version.sh` (`--check-files`), `check-license-consistency.sh` (`--only python --with-wheel`), manifest-gated `wait-for-registry-version.py`, `smoke-pypi-wheel.{sh,ps1}` (`(fathomdb|fathomdb-cli)==` skew retry + `_coinstall.py` import check), `check-release-contract-truth.py`, `check_python_wheel_contents.py`, and all corresponding `scripts/tests/*`.
10. Targeted docs (`README.md`, `src/ts/README.md`, `docs/*`, `dev/interfaces/*`) and final `scripts/generate-public-docs.py --write`.

### Wave 3 — Proportional Verification & Release-Readiness Gates

Run inside the target checkout with its own `.venv` and `node_modules`:

1. **Workspace Full Gate:** `./scripts/agent-verify.sh` (covers `agent-build`, `agent-lint` incl. clippy/rustfmt/migrations/ruff/actionlint/md/lychee, `agent-typecheck` incl. cargo/pyright/tsc, and `agent-test` incl. Rust unit/integration + pytest + TypeScript + script contract suites).
2. **Feature-Gated Reranker & Migration Rollback Verification:**
   - `cargo test -p fathomdb-engine -p fathomdb-sdk -p fathomdb --features default-reranker` (verifying `reranker_compiled == true`, warm/cold cache `rerank_available` without network access, and `slice71_rerank_policy` / `slice71_open_report`).
   - `cargo test -p fathomdb-engine -p fathomdb-cli --features "operator,migration-test-hooks,test-hooks"` (verifying dangling fail-closed + superseded admit, mid-DDL rollback, golden Schema-37 DB upgrade, and `doctor check-integrity --full`).
3. **CLI Wheel & License Verification (Without `cargo`):**
   - `maturin build --release --manifest-path src/python-cli/pyproject.toml --out <tmp_wheels> --locked` (or `python3 -m pip wheel --no-deps src/python-cli -w <tmp_wheels>`).
   - `python3 scripts/check_python_wheel_contents.py --kind cli <tmp_wheels>/*.whl` (confirming only `fathomdb(.exe)` and zero `perf-regression-check`).
   - `bash scripts/check-license-consistency.sh --only python --with-wheel`.
   - Isolated venv smoke install of built `fathomdb` + `fathomdb-cli` (`fathomdb[cli]`) running under `PATH="$VENV/bin:/usr/bin:/bin"` (explicitly excluding `~/.cargo/bin`) executing `python -c "import fathomdb; print(fathomdb.__version__)"`, `fathomdb --version`, `fathomdb doctor platform --json`, and `fathomdb migrate` on a copied Schema-37 fixture.
4. **Surface Parity, Release Contract & Doc Gates:**
   - `python3 scripts/check-sdk-surface-parity.py`
   - `bash scripts/check-governed-surface-pin.sh`
   - `bash scripts/verify-release-gates.sh --check-files`
   - `python3 scripts/check-release-contract-truth.py`
   - `./scripts/agent-lint-md.sh`
   - `./scripts/agent-lint-docs.sh`
5. **Handoff Note:** Record exact landed SHAs on `dev/plans/runs/STATUS-0.8.31.md` and `dev/plans/release-state-0.8.31.json` so **Slice 60 (`INSPECTOR`)** can immediately build its `build-inspector` workflow job atop the merged `.github/workflows/release.yml`.
</SYSTEM_MESSAGE>
