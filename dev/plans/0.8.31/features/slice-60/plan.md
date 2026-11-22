---
title: "Slice 60 — FathomDB Inspector Schema 38, Historical Graph, Scoped CI & Release Packaging"
status: DRAFT
target_release: 0.8.31
slice: 60
date: 2026-11-22
---

# Slice 60 — FathomDB Inspector (`0.3.0`) for Schema 38, `as_of` Historical Graph, Scoped Local CI, Timeout Hardening & Release Packaging · `[implementation]` · depends-on: `30 (STRICT-GRAPH), 40 (HISTORY-GRAPH), 50 (SDK-CLI-UPGRADE)`

> **Phase C Draft Plan (`ROLE_PLANNER`) — serialized after Slice 50 (`SDK-CLI-UPGRADE`) for single-writer `.github/workflows/release.yml` edits.**
> Prior prework triage (`P2-13`, `P3-16`, `P3-17`, `P4-13`, `P5-10`, `P6-11`, `P7-01`, `P7-02`, `P7-04`, `P7-06`) was relocated from pre-triage Slice 50 to Slice 60 at HITL Gate 1 (`2026-11-22`). Reconcile exact `fathomdb-engine` / `fathomdb-schema` Rust API shapes landed by Slices 20, 30, 40, and 50 immediately before executing Wave 1.

---

## 1. Goals & Non-Goals

### 1.1 Goals

1. **Schema 38 admission & reserved `scope` alignment (`src/inspector/src/admission.rs`, `storage.rs`):**
   - Admit Schema `38` databases (`compiled_schema = 38`), validate the Schema 38 columns on `canonical_nodes`, `canonical_edges`, `operational_mutations`, `operational_state`, `_fathomdb_projection_registry`, plus the new Schema 38 metadata/index tables (`canonical_property_values`, `_fathomdb_commit_timestamps`, `_fathomdb_vector_kinds`), and surface explicit `scope` column inspection (empty/reserved in `0.8.31`).
2. **Outside-`body` node & edge `properties` UI + strict XSS & erasure non-disclosure (`storage.rs`, `tree.rs`, `assets/app.js`):**
   - Inspect, expand, and exactly copy outside-`body` node and edge `properties` (preserving typed primitives, empty strings, `SQL NULL`, and 64-bit integer precision outside `2^53`) alongside plain-text `body` documents, with all user/database keys and values rendered purely via `.textContent` (`el()` / `svgElement()`) and with `properties` strictly withheld unless `Session::policy.permits(classification)` authorizes access across all `D-01` 8-flag combinations.
   - Surface declared filterable property metadata (`canonical_property_values` + `_fathomdb_projection_registry`) for both `Class::Node` and `Class::Edge` in `associations` and `catalog`.
3. **Historical `as_of` graph inspection & `P4-13` `INDEXED BY` lift (`graph.rs`, `worker.rs`, `app.js`, `comparison.mjs`):**
   - Expose mutually exclusive `as_of_write_cursor` and `as_of_epoch_ms` (resolved via `_fathomdb_commit_timestamps`) historical snapshot controls for both stored-adjacency (`op: "graph"`) and native engine traversal (`op: "graph_serving"`).
   - Lift `INDEXED BY canonical_nodes_logical_active_idx` and `superseded_at IS NULL` in `graph_serving` whenever `as_of` is active so pre-revision and superseded origin/predecessor nodes resolve accurately against historical snapshots, while subordinating every stored and native `as_of` record, origin, and inline property payload to the `D-01` 8-flag erasure non-disclosure policy.
4. **Scoped `local-ci.sh` default (~5 min) retaining full gate & fail-closed path guard (`P2-13`, `P7-01`):**
   - Change `src/inspector/tools/local-ci.sh` default mode to `--scoped` (~5 min Inspector-scoped lint, typecheck, default + `cached-cpu` Inspector Rust tests, tool unittests, Playwright UI tests, packaging, and installed qualification), retain `--full-workspace` (full `scripts/agent-verify.sh`) and `--installed-only`, enforce flag mutual exclusion (exit `2`), and fail closed when uncommitted or branch diffs against `origin/main` touch files outside `src/inspector/`, `dev/inspector/`, or `docs/inspector/`.
5. **Bounded timeouts & non-blocking process cleanup (`P7-02`):**
   - Enforce `timeout --kill-after=10s`, 2s `SIGKILL` escalation on `Xvfb` teardown, unblocked `strace` stdout pipe handling, 5s deadlines on `browser.close()` and post-`SIGKILL` teardown with `/proc` descendant-tree reaping, explicit timeouts on `execFileSync` / `spawnSync` across all browser/qualification harnesses, and a 30s `try_wait()` + `SIGKILL` bound in `src/inspector/tests/launch.rs`.
6. **Retained `D4 = B` Docker-free Ubuntu 24.04 qualification & `compiled_schema = 38` (`P7-04`):**
   - Lock `local-release.py` and `test_local_release.py` to `D4 = B` (native Ubuntu 24.04 / `glibc >= 2.39`, WebKitGTK native window), update `compiled_schema` from `37` to `38`, replace the stale `0.8.30` hardcoded Engine commit SHA with workspace-consistent `0.8.31` release metadata, bump Inspector from `0.2.1` to `0.3.0` (`src/inspector/Cargo.toml` as single source of truth), and enforce a static zero-`docker` / `openbox` / `xdotool` / `sudo` recurrence guard.
7. **`build-inspector` release packaging in `.github/workflows/release.yml` (`P7-06`, serialized after Slice 50) & Docs (`P5-10`, `P6-11`):**
   - Add `build-inspector` (Ubuntu 24.04 native package build + upload via `upload-artifact-retry`, gated in `all-builds-passed`, attached to `github-release`) and update `check-release-contract-truth.py`, `test_release_workflow_scope.sh`, and `test_runtime_release_alignment.sh` in lockstep.
   - Update `dev/inspector/{needs,requirements,ROADMAP,BACKLOG}.md`, `docs/inspector/` (including `docs/inspector/releases/0.3.0.md` and `mkdocs.yml`), and `docs/contributing/index.md` (`local-ci.sh --scoped` contributor workflow).

### 1.2 Non-Goals

- **No multi-tenant `scope` enforcement or `scope`-filtering UI:** `0.8.31` reserves the schema `scope TEXT NOT NULL DEFAULT ''` column; tenant isolation policies belong to `0.8.32`. Inspector displays the stored `scope` string read-only without synthesizing scope semantics.
- **No `D4` host expansion (Docker, Wayland-only, macOS, Windows):** `D4 = B` remains locked; Inspector stays Linux `x86_64` Ubuntu `24.04` (`glibc >= 2.39`) qualified.
- **No modifications to `dev/acceptance.md`:** Repo-level `dev/acceptance.md` remains **LOCKED**; all new criteria use slice-local `AC31-60-*` and Inspector-local `INS-*` identifiers under `R31-08`.
- **No engine/SDK schema or query changes:** Slices 20–50 own Schema 38 and Engine/SDK/CLI behavior; Slice 60 adapts Inspector and release CI wiring to match what Slices 20–50 landed.
- **No consumer-specific terminology:** All schemas, fixtures, docs, and UI labels remain strictly product-generic FathomDB terminology.

---

## 2. Draft Needs, Requirements & Acceptance Criteria

### 2.1 Release Requirement Linkage (`dev/plans/0.8.31/plan.md`)

- **`R31-08` (FathomDB Inspector `0.8.31` readiness):** Schema 38 admission, outside-`body` `properties` display with XSS protection, historical `as_of` graph exploration subordinate to 8-flag erasure non-disclosure, scoped `local-ci.sh` default, bounded harness timeouts and cleanup, `D4 = B` `local-release.py` alignment, `.github/workflows/release.yml` `build-inspector` artifact publishing, and `docs/inspector/` + `dev/inspector/` documentation currency.

### 2.2 Slice-Local & Inspector Requirements (`AC31-60-*`, `INS-*`)

> **Constraint:** `dev/acceptance.md` stays **LOCKED**. New criteria below live in Slice 60 and `dev/inspector/`.

| ID | Prework Ref | Requirement & Observable Acceptance Criterion |
| --- | --- | --- |
| `AC31-60-01` | `R31-08` | **Schema 38 strict admission (`INS-01`, `V01-01`):** Opening a Schema `38` database succeeds in both default and `cached-cpu` modes and validates `properties`, `scope`, `canonical_property_values`, `_fathomdb_commit_timestamps`, `_fathomdb_vector_kinds`, and `_fathomdb_projection_registry` columns; Schema `37` or missing required Schema 38 tables/columns fail closed with named admission diagnostics (`unsupported_schema` / `unsupported_layout`). |
| `AC31-60-02` | `P3-16` | **Outside-`body` node/edge `properties` & reserved `scope` inspection (`INS-02`, `INS-03`):** Nodes and edges with plain-text `body` + JSON `properties` render bounded `properties` metadata, expandable typed key/value inspection (`detail_field`, `chunk`, `tree`), exact copy preserving `abs(i64) > 2^53` / `-0` / bool / string / empty / `NULL`, visible reserved `scope`, and declared `canonical_property_values` coverage across both nodes and edges in `associations` and `catalog`. |
| `AC31-60-03` | `P3-16` | **XSS & 8-flag erasure non-disclosure over `properties` (`INS-02`, `D-01`):** Hostile HTML/SVG/JS strings in `properties` keys, string values, or `scope` render inertly via DOM `.textContent` with zero script execution or attribute injection; across all `2^3 = 8` policy flag combinations, queued erasure-pending rows withhold both `body` AND `properties` previews/bytes/values on `page`, `detail`, `chunk`, `tree`, `graph`, `investigate`, `graph_serving`, `lexical`, `vectors`, `serving`, `associations`, and `export` when `--show-erased` is off. |
| `AC31-60-04` | `P3-17`, `P4-13` | **Historical `as_of` graph exploration + 8-flag erasure subordination (`INS-03`, `V01-01`):** Stored (`graph`) and native (`graph_serving`) graph inspection accept mutually exclusive `as_of_write_cursor` or `as_of_epoch_ms` (resolving via `_fathomdb_commit_timestamps`), reject simultaneous cursor+epoch inputs cleanly, lift `INDEXED BY canonical_nodes_logical_active_idx` and `superseded_at IS NULL` in `graph_serving` origin hydration when `as_of` is active, honor active-revision `write_cursor <= cursor AND (superseded_at IS NULL OR superseded_at > cursor)` boundaries, report explicit `as_of` scope metadata, preserve `as_of` parameters across comparison pins, and strictly enforce all 8 `D-01` disclosure flags over historical targets, terminal edges, origins, and inline properties. |
| `AC31-60-05` | `P2-13`, `P7-01` | **Scoped `local-ci.sh` default & fail-closed diff guard (`INS-10`):** `src/inspector/tools/local-ci.sh` defaults to `--scoped` (~5 min Inspector-scoped source check + default/`cached-cpu` Inspector Rust tests + tool unittests + Playwright UI + packaging + installed suite); `--full-workspace` runs full `scripts/agent-verify.sh`; `--installed-only` preserves installed qualification; combining mode flags exits `2`; any staged/unstaged/untracked or `origin/main` diff outside `src/inspector/`, `dev/inspector/`, `docs/inspector/` causes `--scoped` to fail closed before running stages and direct the user to `--full-workspace`. |
| `AC31-60-06` | `P7-02` | **Bounded subprocess timeouts & non-blocking cleanup (`INS-10`):** `run-installed.sh` (`timeout --kill-after=10s`, 2s `Xvfb` `SIGKILL` escalation), `qualify-installed-network.sh`, `installed-harness.mjs` (non-blocking `strace` stdout handling, 5s `browser.close()` and post-`SIGKILL` wait deadlines, `/proc` descendant-tree `SIGKILL` reaping), browser/qualification Node scripts (`execFileSync`/`spawnSync`/`browser.close`/`child` teardown timeouts), and `tests/launch.rs` (30s `try_wait()` + `SIGKILL` escalation) never wedge indefinitely on stalled children or orphan processes. |
| `AC31-60-07` | `P7-04` | **Retained `D4 = B` native Linux qualification & `compiled_schema = 38` (`INS-10`, `INS-11`):** `local-release.py` and `build-description` agree on `compiled_schema = 38`, Inspector `0.3.0`, and Ubuntu 24.04 (`glibc >= 2.39`) WebKitGTK support without stale `0.8.30` commit pins; a static guard in `test_local_release.py` enforces zero `docker`, `openbox`, `xdotool`, or `sudo` invocations across `src/inspector/tools/` and `src/inspector/tests/` (excluding explicit negative test shims in `test_local_ci.py`). |
| `AC31-60-08` | `P7-06` | **`release.yml` `build-inspector` job & structural contract parity:** Serialized after Slice 50, `.github/workflows/release.yml` adds `build-inspector` on `ubuntu-24.04` (checking out `RELEASE_CHECKOUT_REF`, building with pinned Rust `1.95.0` via `package.sh`, uploading `.tar.gz` + `.sha256` via `upload-artifact-retry`), gates `all-builds-passed` on `build-inspector`, attaches the tarball + checksum in `github-release`, and passes `actionlint`, `check-release-contract-truth.py`, `test_release_workflow_scope.sh`, and `test_runtime_release_alignment.sh`. |
| `AC31-60-09` | `P5-10`, `P6-11` | **Engineering specs & public documentation currency (`INS-01`..`INS-12`):** `dev/inspector/{needs,requirements,ROADMAP,BACKLOG}.md`, `docs/inspector/` (including `docs/inspector/releases/0.3.0.md` and `mkdocs.yml`), and `docs/contributing/index.md` reflect Schema 38, `properties`/`scope` UI, `as_of` graph exploration, `local-ci.sh --scoped`, and `D4 = B` release packaging, passing `agent-lint-md.sh`, `agent-lint-docs.sh`, `check-public-doc-truth.py`, and `mkdocs build --strict` without claiming `0.8.31` is already published. |

---

## 3. Architecture & Draft Design

### 3.1 Schema 38 Admission, Reserved `scope`, & `properties` Storage (`admission.rs`, `storage.rs`, `tree.rs`, `vector.rs`, `lexical.rs`, `worker.rs`, `failure.rs`)

1. **`src/inspector/src/admission.rs` (`validate_layout`):**
   - Enforce Schema `38` exact match against `fathomdb_schema::SCHEMA_VERSION`.
   - Extend `validate_layout` column probes (`SELECT {cols} FROM {table} LIMIT 0`):
     - `canonical_nodes`: append `,properties,scope`.
     - `canonical_edges`: append `,properties,scope`.
     - `operational_mutations` and `operational_state`: append `,scope`.
     - Verify `_fathomdb_projection_registry` columns match Schema 38 DDL (e.g., `entity_class` / typed declaration columns landed in Slice 20).
     - Probe new Schema 38 tables: `canonical_property_values` (`entity_class,write_cursor,attr_name,value_type,text_val,num_val,bool_val`), `_fathomdb_commit_timestamps` (`write_cursor,committed_at_ms`), and `_fathomdb_vector_kinds` (`kind,source_type,declared_at` or exact Slice 20 schema columns).

2. **`src/inspector/src/storage.rs` (`Class::fields`, `summary`, `page`, `display_preview`, `detail_field`, `tree`, `associations`, `catalog`):**
   - **Critical Erasure Non-Disclosure Design (`properties` vs `Class::fields`):**
     - `Reader::page` currently populates every item via `Self::summary(&c, &locator)` even when `self.session.policy.permits(class)` is `false` (`payload_status == "refused"`, queued erasure-pending rows).
     - Therefore **do NOT blindly expose inline `properties` bytes/previews in unpermitted `summary()` responses!**
     - Design:
       - Keep non-payload metadata `"scope"` safe in `Class::fields` for `Node`, `Edge`, `OperationalMutation`, and `OperationalState` (visible on summary and detail, since `scope` is a structural namespace boundary empty in `0.8.31`).
       - Add `"properties"` to `Self::field(&locator, field)` for `Class::Node` and `Class::Edge` so `detail_field(handle, "properties")` and `chunk(handle, "properties", offset, limit)` stream exact UTF-8 slices under the existing `if !self.session.policy.permits(class) { return Err("refused"); }` gate (`storage.rs:1141, 1545`).
       - In `Self::summary` / `summary_fields_budget`: include `properties`, `properties_bytes`, and `properties_storage` **only when `policy.permits(class)` evaluates `true`**, and explicitly emit `"properties": null, "properties_bytes": null, "properties_storage": "refused"` (or omit `properties*` on unpermitted rows) whenever `permits(class)` is `false`! Pass `policy` (or `permitted: bool`) explicitly into `summary_with_policy` / `summary` callers across `page`, `sample`, `detail_field`, `graph_record`, `investigate`, `lexical`, `vector`, and `associations` so no path can accidentally return withheld `properties` content.
   - **Plain-text `body` + `properties` `display_preview` fallback:**
     - In `display_preview` (`storage.rs:419–460`), if `body` is non-JSON plain text (or JSON without `"title"` / `"name"`), check `properties` (bounded <= 4096 bytes) for `"title"` or `"name"` **only after verifying `permits(class)`**; never propagate JSON parse failures from plain-text `body`.
   - **Structured typed property tree & 64-bit integer precision (`tree.rs`, `worker.rs`, `storage.rs`):**
     - Extend `worker::Request::Tree` (`#[serde(deny_unknown_fields)]`) with `#[serde(default)] field: Option<String>` accepting `None | Some("body") | Some("properties")` (rejecting any other column name cleanly with `"invalid_field"` / `"invalid_request"`). `storage::tree` opens `Self::field(&locator, field)` (preserving the `policy.permits(class)` refusal check!) and delegates to `tree::children`.
     - Because `tree::children` extracts raw byte `lexeme` strings directly from SQLite blob slices without going through JavaScript floating-point `JSON.parse`, 64-bit integers outside IEEE-754 safe integer range (`abs(x) > 2^53`, e.g. `9007199254740993123456789`) and `-0` survive inspection and exact copy verbatim!
   - **Node & edge property materialization telemetry (`catalog`, `associations`):**
     - Extend `catalog` (`storage.rs:1032–1064`) projection counts with `property_value_entries` (`COUNT(*) FROM canonical_property_values`) and `commit_timestamp_entries` (`COUNT(*) FROM _fathomdb_commit_timestamps`).
     - Extend `associations` (`storage.rs:1231–1298`) so `Class::Node` AND `Class::Edge` inspect `canonical_property_values WHERE entity_class=?1 AND write_cursor=?2` (gated on `permits(class)`) and match against declared Filterable properties in `_fathomdb_projection_registry`, showing declared vs materialized status for both node and edge properties.
   - **`vector.rs` custom enrolled vector kinds & `lexical.rs` parity:**
     - Keep `vector.rs:252` `source_type == "edge_fact"` vs `Class::Node` routing and add coverage validating custom enrolled node kinds from `_fathomdb_vector_kinds` (and verifying reserved `"edge_fact"` rejection holds on Schema 38).
     - Keep `lexical.rs` (`ReadView` / `LexicalView`) aligned with Slice 20/30 struct signatures landed prior to Slice 60.
   - **`failure.rs` closed error whitelist:**
     - Any new worker error codes (e.g., `"ambiguous_as_of"` / `"invalid_as_of"` or reusing `"invalid_range"`) MUST be registered in `Failure::decode`'s `CODES` slice (`failure.rs:72–120`) and mapped in `app.js` human-readable status formatting.

### 3.2 Safe UI Rendering for `properties`, `scope`, & `as_of` Telemetry (`src/inspector/assets/app.js`, `presentation.mjs`)

- **Strict `.textContent` XSS boundary:**
  - Construct every DOM element for node/edge `properties` summaries, `Open full properties` tree/text inspector, `Copy full properties` exact payload button, `scope` badge, `associations` property entries (`Node` + `Edge`), and `graph` target/edge property panels exclusively using `el(tag, text, attrs)` (`n.textContent = String(text)`) and `svgElement(...)` (`node.textContent = text`).
  - Zero use of `innerHTML`, `insertAdjacentHTML`, `document.write`, or untrusted `href`/`src` attributes. Render `<img onerror=...>`, `<script>`, SVG `onload`, empty string `""`, and `"null"` strictly as literal text.
- **Outside-`body` `properties` workspace UX:**
  - Record detail panel displays:
    - Existing `body` workspace (now explicitly handles plain-text `body` alongside JSON),
    - A dedicated `Properties` metadata/action block (when `properties_bytes != null` and permitted: byte size, storage type, bounded preview or `tree` key-type-lexeme rows when JSON object <= 16 KiB, `Open full properties`, `Copy full properties`; explicit `"null"` / `"refused (erasure-pending)"` states when null or withheld by policy),
    - Read-only `Scope` field (`""` rendered clearly as empty/default namespace).

### 3.3 Historical `as_of` Graph Inspection, `P4-13` `INDEXED BY` Lift & 8-Flag Erasure Matrix (`graph.rs`, `worker.rs`, `app.js`, `comparison.mjs`)

1. **Worker IPC contract (`worker.rs`):**
   - Extend `worker::Request::Graph` and `worker::Request::GraphServing` (`#[serde(deny_unknown_fields)]`) with:
     - `#[serde(default)] as_of_write_cursor: Option<u64>`
     - `#[serde(default)] as_of_epoch_ms: Option<i64>`
   - Enforce mutual exclusion at worker validation: if BOTH `as_of_write_cursor.is_some()` AND `as_of_epoch_ms.is_some()`, return named error (`"invalid_as_of"` registered in `Failure::decode::CODES`, or `"invalid_range"`) cleanly without dropping worker session state.
   - Commit-timestamp resolution helper (`resolve_as_of_cursor`):
     - If `as_of_write_cursor = Some(c)`: `resolved_cursor = c`.
     - If `as_of_epoch_ms = Some(ms)`: query `SELECT MAX(write_cursor) FROM _fathomdb_commit_timestamps WHERE committed_at_ms <= ?1`; if `NULL` (timestamp precedes every commit), `resolved_cursor = 0` (explicitly matching zero committed revisions rather than falling back to current!).
     - Record `{ mode: "current" | "write_cursor" | "epoch_ms", requested_as_of_write_cursor, requested_as_of_epoch_ms, resolved_as_of_write_cursor }` in `query_context` and `scope` of both `graph` and `graph_serving` responses.

2. **`src/inspector/src/graph.rs` — `P4-13` `INDEXED BY` Lift & Historical Resolution:**
   - **Native `Reader::graph_serving`:**
     - Construct Slice 30/40 `GraphExpandRequestV1` carrying the historical `as_of` read context (`GraphReadContextV1` / `known_as_of` matching Slices 30–40 Engine types) when `as_of` is active, and `Frozen` current snapshot context when `as_of` is `None`.
     - **`P4-13` Origin/Predecessor lookup (`graph.rs:614`):**
       - When `as_of` is **None** (current mode): preserve `SELECT rowid,write_cursor FROM canonical_nodes INDEXED BY canonical_nodes_logical_active_idx WHERE logical_id=?1 AND superseded_at IS NULL`.
       - When `as_of` is **Some(resolved_cursor)**: **LIFT `INDEXED BY canonical_nodes_logical_active_idx` and `superseded_at IS NULL`**! Resolve `origin.seed_logical_id` and `origin.predecessor_logical_id` at `resolved_cursor` via historical revision visibility: `write_cursor <= ?2 AND (superseded_at IS NULL OR superseded_at > ?2)` using `graph_scan` / unforced logical-id scan bounded by `budget` (ordering/selecting the single visible snapshot row at `resolved_cursor`), leaving `origin` handle unresolved with explicit coverage telemetry only if `budget` exhausts rather than failing SQLite query planning!
     - **Native inline property & payload policy guard:** Whenever Engine `GraphTargetV1` / `GraphOriginV1` returns inline `properties` or `terminal_edge_properties`, check `self.session.policy.permits(classification)` against the hydrated `target_record` and `terminal_edge_record`; **strip/omit native inline `properties`, `payload`, and hydrated `record` metadata whenever `permits` is `false`!**
   - **Stored-adjacency `Reader::graph`:**
     - Reconcile `graph.rs:232` edge index selection against Schema 38 (`P4-10`):
       - Current mode (`as_of = None`): inspect edges using Schema 38 `canonical_edges_{from,to}_active_idx` or `canonical_edges_{from,to}_sup_kind_idx` (per Schema 38 DDL landed in Slice 20; replace stale `canonical_edges_{column}_idx` reference if Slice 20 replaced those indexes!).
       - Historical mode (`as_of = Some(resolved_cursor)`): query edges using `canonical_edges_{from,to}_sup_kind_idx` / bounded cursor scan matching `write_cursor <= ?cursor AND (superseded_at IS NULL OR superseded_at > ?cursor)`, and filter endpoint node hydration from `graph_scan` strictly to revisions satisfying `write_cursor <= resolved_cursor AND (superseded_at IS NULL OR superseded_at > resolved_cursor)`.
     - **8-flag disclosure parity (`D-01`):** Pass every historical node and edge revision through `self.graph_record` -> `self.session.policy.permits(class)` (`no_show_inactive`, `show_erased`, `show_payloads`). Historical `as_of` NEVER resurrects physically erased rows (already deleted in SQLite), NEVER reveals queued erasure-pending payloads/properties when `show_erased == false`, and NEVER overrides `--no-show-inactive`!

3. **UI Controls & Comparison Pins (`app.js`, `comparison.mjs`):**
   - Add `Snapshot` (`Current` / `At write cursor` / `At commit time (ms)`) selector + bounded non-negative integer input (`as_of_write_cursor` or `as_of_epoch_ms`) in the Graph workspace (shared across Stored graph and Native graph traversal).
   - Validate integer syntax strictly before dispatch and display backend errors clearly inside the graph feedback status region.
   - Include `as_of_write_cursor` / `as_of_epoch_ms` in `observationSummary`, `pinDescriptor`, `openPinned`, and `comparison` side A/B diffs so users can compare historical vs current graphs without silent snapshot drift, and render `target.properties` / `terminal_edge.properties` safely via `el()`.

### 3.4 Bounded Scoped `local-ci.sh` (`P2-13`, `P7-01`)

- Refactor `src/inspector/tools/local-ci.sh` CLI & execution:
  - Mode flags (mutually exclusive; combining >= 2 flags or passing unknown `--*` exits `2` with usage message on stderr):
    - `--scoped` (**default** when zero mode flags passed)
    - `--full-workspace`
    - `--installed-only` (requires 3 positional path arguments as today)
  - Optional positional args (`CANDIDATE FIXTURES BUILD_TARGET`) behavior preserved cleanly across modes.
  - **Fail-closed diff check inside `--scoped` (before running heavy stages):**
    - Enumerate modified, added, deleted, and untracked paths via `git diff --name-only <base>` (prefer `origin/main...HEAD` or `origin/main` working-tree diff; fallback to `HEAD~1` / `git status --porcelain` when `origin/main` is unavailable in shallow/isolated test clones).
    - Allow only paths under `src/inspector/`, `dev/inspector/`, or `docs/inspector/`.
    - If ANY changed path lies outside those three prefixes, fail immediately (non-zero) printing the out-of-scope file list and instructing `--full-workspace`.
  - **`--scoped` stage execution (~5 min target):**
    - `source`:
      - `bash scripts/agent-lint.sh` (covers shellcheck, clippy, rustfmt, release-contract-truth, public-doc-truth, actionlint, markdownlint, lychee)
      - `bash scripts/agent-typecheck.sh`
      - `cargo test --locked --offline -p fathomdb-inspector --jobs 1 -- --test-threads=1`
      - `cargo test --locked --offline -p fathomdb-inspector --no-default-features --features cached-cpu --jobs 1 -- --test-threads=1`
      - `python3 -m unittest discover -s src/inspector/tests/tools`
    - Follow with existing Inspector stages (`tooling`, `unit`, `native-build`, `seal`, `candidate`, `abi`, `installed`).
  - **`--full-workspace` stage execution:**
    - `source` runs `bash scripts/agent-verify.sh` followed by all Inspector stages.
  - **Single version source of truth:** Derive Inspector version from `src/inspector/Cargo.toml` (`metadata` / TOML read) in `local-ci.sh`, `package.sh`, and test helpers instead of repeating hardcoded `'0.2.1'`.

### 3.5 Bounded Timeouts & Non-Blocking Cleanup (`P7-02`)

| File | Vulnerability / Unbounded Wait | Bounded Replacement |
| --- | --- | --- |
| `src/inspector/tools/run-installed.sh` | `kill "$display_pid"; wait "$display_pid"` hangs if `Xvfb` ignores `SIGTERM`; un-timed `node "$@"` | Wait loop up to 2s (`20 * 0.1s`) polling `kill -0 "$display_pid"`, escalate `kill -KILL "$display_pid"`, `wait ... \|\| true`; execute `timeout --kill-after=10s "${INSPECTOR_RUN_TIMEOUT_SECS:-900}" node "$@"` |
| `src/inspector/tools/qualify-installed-network.sh` | Invokes `run-installed.sh` without outer bound | Wrap invocation in `timeout --kill-after=10s "${INSPECTOR_NETWORK_TIMEOUT_SECS:-900}"` |
| `src/inspector/tests/browser/installed-harness.mjs` | L39 `strace` `stdio: ['ignore','pipe','pipe']` leaves `child.stdout` unconsumed (pipe deadlock); L72/L75 `execFileSync` untimed; L78 `browser.close()` and L83 `await exited` untimed; orphan descendants after `SIGKILL` | Set `stdio: ['ignore','ignore','pipe']` (or drain stdout); add `timeout:` to `execFileSync`; race `browser.close()` with 5s timer; add `/proc/<pid>/task/*/children` + `/proc` PPID tree `SIGKILL` reaper (`killDescendantTree`) and race post-`SIGKILL` `exited` against 5s deadline |
| `src/inspector/tools/qualify-startup.mjs` | Unbounded `execFileSync` (L54, L77, L101) & `spawnSync` (L129) | Add explicit `{ timeout: ... }` (e.g., 120s build / 30s inspect / 60s startup calibration) and fail loudly on non-zero `status`, `signal`, or `error` |
| `src/inspector/tools/qualify-{graph,lexical,vector,scale}.mjs`, `clipboard-diagnostic.mjs` & `src/inspector/tests/browser/*.mjs` | Several `execFileSync`, `browser.close()`, and `child.on('exit')` calls lack timeouts or `SIGKILL` escalation | Extract/use a small shared cleanup/exec helper bounding `execFileSync` (`timeout`), `browser.close()` (5s–10s `Promise.race`), and `driver` child teardown (`child.stdin.end('close\n')` -> 5s `SIGTERM` -> 2s `SIGKILL` + `/proc` descendant reap) |
| `src/inspector/tests/launch.rs` | L70 `child.wait()` blocks forever if child stalls after `SIGTERM` | Replace with `Duration::from_secs(30)` bounded poll loop (`try_wait()` + 25ms sleep); on expiry invoke `child.kill()` + `child.wait()` and panic with diagnostic |

### 3.6 Retained `D4 = B` Lock & `local-release.py` (`P7-04`)

- Lock `D4 = B` (Docker-free native Ubuntu 24.04 / `glibc >= 2.39`, WebKitGTK native window, no `docker` / `openbox` / `xdotool` / `sudo`).
- Update `src/inspector/tools/local-release.py`:
  - `compiled_schema`: **`37 -> 38`**.
  - Read Inspector version (`0.3.0`) cleanly from `src/inspector/Cargo.toml` (fail closed if drifted).
  - Replace the stale `0.8.30` `ENGINE` commit SHA (`72fe9363...`) with dynamic workspace release provenance matching `0.8.31` (root `Cargo.toml` package version + `git rev-parse HEAD` / commit match against `build-description` provenance) instead of pinning an immutable `0.8.30` hash inside the file.
  - Align `SUPPORT` (`distribution` / release channel metadata) with the `github-release` artifact published by `build-inspector`, retaining `minimum_ubuntu: "24.04"`, `minimum_glibc: "2.39"`, and `ui_stack: "native_gtk_webkit"`.
- Update `src/inspector/tests/tools/test_local_release.py`:
  - Expect `compiled_schema: 38` and version `0.3.0`.
  - Add a **static `D4 = B` recurrence guard** scanning `src/inspector/tools/**` and `src/inspector/tests/**` (excluding the intentional negative `docker` poison-PATH test stub inside `test_local_ci.py`) asserting zero `docker`, `openbox`, `xdotool`, or `sudo` subprocess calls or directives.

### 3.7 Release Workflow `build-inspector` Job (`P7-06`, Serialized After Slice 50)

> **Single-Writer Serialization:** Edit `.github/workflows/release.yml`, `scripts/check-release-contract-truth.py`, `scripts/tests/test_release_workflow_scope.sh`, and `scripts/tests/test_runtime_release_alignment.sh` **only after Slice 50 lands** its CLI/release-workflow changes.

- **New job `build-inspector` in `.github/workflows/release.yml`:**
  - `runs-on: ubuntu-24.04` (matches `D4 = B` glibc `2.39` floor and Ubuntu 24.04 GTK4/WebKitGTK environment).
  - `if: ${{ !cancelled() && needs.verify-release.result == 'success' }}` (consistent with sibling `build-python` / `build-rust` / `build-napi` lanes; `needs: [verify-release]`).
  - Checkout step: `uses: actions/checkout@9c091bb21b7c1c1d1991bb908d89e4e9dddfe3e0` with `ref: ${{ env.RELEASE_CHECKOUT_REF }}` and `fetch-depth: 0` (preserving `test_runtime_release_alignment.sh` checkout invariant).
  - Toolchain: `uses: dtolnay/rust-toolchain@fa04a1451ff1842e2626ccb99004d0195b455a88` with `toolchain: "1.95.0"` (exact pin required by `test_runtime_release_alignment.sh`).
  - Install Ubuntu 24.04 native Inspector build/qualification system dependencies (`libgtk-4-dev` / `libwebkitgtk-6.0-dev` / `libsqlite3-dev` / `pkg-config` / `patchelf` / etc. as verified against `src/inspector/Cargo.toml`), build and package via `bash src/inspector/tools/package.sh "$RUNNER_TEMP/inspector-dist"`, verify `--build-description` `compiled_schema == 38`, `version == 0.3.0`, and glibc floor check against `2.39`.
  - Upload tarball + `.sha256` exclusively via `uses: ./.github/actions/upload-artifact-retry` (e.g., `name: inspector-linux-x64-gnu`).
- **Release Gate & Attachment Wiring:**
  - Add `build-inspector` to `all-builds-passed.needs` (and require `needs.build-inspector.result == 'success'` inside `all-builds-passed`'s gate expression so crates/PyPI/npm never publish if Inspector packaging breaks).
  - Download `inspector-linux-x64-gnu` inside `github-release` and attach the Inspector tarball and `.sha256` alongside the core release assets (preserving `RELEASE_CHECKOUT_REF` and `RELEASE_JOB_NEEDS`).
- **Lockstep Workflow Structural Tests:**
  - Update `scripts/check-release-contract-truth.py` (`RELEASE_JOB_NEEDS` / `build-inspector` / `all-builds-passed` / `github-release` artifact contract checks).
  - Update `scripts/tests/test_release_workflow_scope.sh`: add `"build-inspector"` to `abp` required set (`GATE` check) and update the exact expected count in `[ "$(grep -c 'uses: .*/upload-artifact-retry' "$WF")" -eq <N> ]` (`baseline 13` + Slice 50 additions + `1` for `build-inspector`).
  - Re-verify `scripts/tests/test_runtime_release_alignment.sh` (`dtolnay/rust-toolchain` pin count, `actions/checkout` `RELEASE_CHECKOUT_REF` allowlist) and `actionlint` `1.7.12`.

### 3.8 Engineering Specs, Public Docs & Contributing Notes (`P5-10`, `P6-11`)

- **`dev/inspector/`:** Update `needs.md`, `requirements.md`, `ROADMAP.md`, and `BACKLOG.md` for Schema 38 admission, `properties`/`scope` UI, `as_of` graph exploration + 8-flag subordination, `local-ci.sh --scoped`, `D4 = B` release qualification, and `build-inspector` artifact delivery. (Do **not** touch `dev/acceptance.md`.)
- **`docs/inspector/` + `mkdocs.yml`:**
  - Update `index.md`, `getting-started/index.md`, `concepts/index.md`, `guides/index.md`, `reference/index.md`, `operations/index.md`, `contributing/index.md`, and `ROADMAP.md` for `0.3.0` / FathomDB `0.8.31` (Schema `38`, `properties` & `scope`, `as_of` graph inspection, release tarball download/checksum verification).
  - Add `docs/inspector/releases/0.3.0.md` and register it in `mkdocs.yml` navigation.
  - Phrasing guard (`scripts/check-public-doc-truth.py`): target `0.8.31` compatibility without asserting `0.8.31` is already published prior to release tagging.
- **`docs/contributing/index.md` (`P6-11`):** Add a concise section documenting `bash src/inspector/tools/local-ci.sh` (`--scoped` ~5 min default for Inspector-only diffs vs `--full-workspace` when crossing shared/workspace files vs `--installed-only`), complementing Slice 10's bootstrap updates.

---

## 4. Prerequisites Discovered During Read-Only Audit

1. **`storage.rs` `Reader::page` erasure-leak seam (`storage.rs:661, 720–724, 755`):** `page()` emits `Self::summary` items with all `Class::fields()` columns even when `self.session.policy.permits(class)` is `false` (`payload_status == "refused"`). Adding `"properties"` directly into `Class::fields()` without gating `summary()` on `permits(class)` would leak queued erasure-pending property JSON on `page()`.
2. **`graph.rs:232` edge index naming mismatch with Schema 38 (`P4-10`):** `Reader::graph` currently forces `INDEXED BY canonical_edges_{column}_idx`. Per Schema 38 / Slice 20 (`ADR #73`, `ADR #75`), incident edge indexes are `canonical_edges_{from,to}_active_idx` (partial `WHERE superseded_at IS NULL`) and `canonical_edges_{from,to}_sup_kind_idx` (composite `(from_id|to_id, superseded_at, kind)`). Left un-updated, `r.graph` fails SQLite query planning on Schema 38 stores.
3. **`graph.rs:614` hardcoded `INDEXED BY canonical_nodes_logical_active_idx`:** Hardcoding active-only partial index lookup in `graph_serving` breaks historical `as_of` origin/predecessor resolution whenever nodes are later superseded or revised after `as_of_cursor`.
4. **Post-Slice 20/30/40 struct & inline-property reconciliation (`graph.rs:572–591`, `lexical.rs`, `tests/fixtures/mod.rs`):** Slices 30/40 extend `GraphExpandRequestV1` (`edge_predicates`, `target_predicates`, `as_of` context) and add `GraphTargetV1.properties` / `GraphOriginV1.terminal_edge_properties` (`ADR #74`). Any native inline properties forwarded by `graph_serving` must also obey `policy.permits` erasure withholding.
5. **Closed deserialization / error boundaries (`worker.rs`, `storage.rs:117`, `failure.rs:72–120`):** `worker::Request` and `Filter` enforce `#[serde(deny_unknown_fields)]`, and `Failure::decode` rejects any worker error code absent from `CODES`. New `as_of` / `tree.field` / error tokens require synchronized updates across Rust + JS.
6. **Silent `strace` stdout pipe deadlock & unbounded waits (`installed-harness.mjs:39`, `launch.rs:70`, `run-installed.sh:44`):** `installed-harness.mjs` spawns `strace` with `stdio: ['ignore', 'pipe', 'pipe']` without consuming `child.stdout`; `launch.rs:70` runs bare `child.wait()` before checking elapsed time; `run-installed.sh` waits on `Xvfb` without `SIGKILL` escalation.
7. **Hardcoded `0.2.1` / `37` / `0.8.30` SHA drift:** `'0.2.1'` is duplicated across `src/inspector/Cargo.toml`, `package.sh:37`, `local-ci.sh:71,85,87`, `local-release.py:16`, `tests/browser/package.mjs`, `tests/tools/test_package.py:48`, and `tests/tools/test_local_release.py`; `local-release.py:20` pins a `0.8.30` Engine commit SHA (`72fe9363...`) and `test_local_release.py:34` asserts `compiled_schema: 37`.
8. **Release workflow brittle assertions & Slice 50 serialization (`test_release_workflow_scope.sh:281`, `check-release-contract-truth.py`, `test_runtime_release_alignment.sh`):** `test_release_workflow_scope.sh` enforces exact counts of `upload-artifact-retry` steps and `all-builds-passed` dependencies; `test_runtime_release_alignment.sh` enforces exact `dtolnay/rust-toolchain` `1.95.0` pins and `actions/checkout` `RELEASE_CHECKOUT_REF` usage; `check-release-contract-truth.py` validates `RELEASE_JOB_NEEDS` and `github-release`.
9. **Local dev test harness note (`scripts/agent-test.sh` & `scripts/agent-lint.sh`):** Because `src/inspector` is in root `Cargo.toml` `[workspace.members]`, `scripts/test-rust-workspace.sh --serial` runs `fathomdb-inspector` default-feature tests inside `agent-test.sh` and `agent-lint.sh` runs `clippy --workspace`, `rustfmt`, and `shellcheck` on `src/inspector/tools/*.sh`; however `fathomdb-inspector --features cached-cpu` tests, Playwright browser tests, and `src/inspector/tests/tools/` Python unittests run only via `local-ci.sh` (or explicit invocation), making `local-ci.sh --scoped` mandatory verification for Slice 60.

---

## 5. TDD Implementation & Verification Plan

Follow strict **Red -> Green -> Refactor** per wave. Commit or verify each test **fails first (RED)** before writing implementation code. Do not modify failing tests to pass; fix implementation code.

### Wave 1 (RED -> GREEN): Schema 38 Admission, `properties` / `scope`, Precision, & 8-Flag Erasure Guard

- **RED tests to add first:**
  1. `src/inspector/tests/integration/admission.rs`: reject Schema `37` or missing `canonical_property_values` / `_fathomdb_commit_timestamps` / `_fathomdb_vector_kinds` / `properties` / `scope` columns; admit Schema `38` database in both default and `cached-cpu`.
  2. `src/inspector/tests/integration/records.rs` & `associations.rs`: node and edge outside-`body` `properties` (`detail_field`, `chunk`, `tree` with `field: "properties"`), plain-text `body` with `display_preview` fallback, `scope` field visibility, `64`-bit integer `9007199254740993123456789` + `-0` + bool + empty string + `NULL` exact lexeme preservation, and `canonical_property_values` declarations reported for both `Class::Node` and `Class::Edge` (`catalog` + `associations`).
  3. `src/inspector/tests/integration/disclosure.rs` & `src/inspector/tests/worker.rs`: across all `2^3 = 8` disclosure flag combinations (`--no-show-inactive`, `--show-erased`, `--show-payloads`), prove queued erasure-pending rows **never** expose inline `properties` preview/bytes/storage or `detail_field("properties")` / `chunk("properties")` / `tree("properties")` when `--show-erased` is OFF, while authorized flag sets expose them completely.
- **GREEN:** Implement `admission.rs`, `storage.rs`, `vector.rs` / `lexical.rs` reconciliations, `worker.rs`, and `failure.rs`.
- **Check:**
  - `cargo test --locked --offline -p fathomdb-inspector --jobs 1 -- --test-threads=1`
  - `cargo test --locked --offline -p fathomdb-inspector --no-default-features --features cached-cpu --jobs 1 -- --test-threads=1`

### Wave 2 (RED -> GREEN): Historical `as_of` Graph (`graph.rs`, `P4-13` `INDEXED BY` Lift, & 8-Flag Matrix)

- **RED tests to add first:**
  1. `src/inspector/tests/integration/graph.rs` & `src/inspector/tests/worker.rs`:
     - Construct multi-cursor / commit-timestamped fixture where a node and incident edges evolve across cursors `[c1 < c2 < c3]` (including an active revision superseded after `c1` and another later superseded/purged, plus queued erasure-pending rows).
     - Assert `op: "graph"` and `op: "graph_serving"` with `as_of_write_cursor: c1` and `as_of_epoch_ms: t(c1)` return the exact `c1` snapshot (`graph_serving` resolves superseded `c1` origin/predecessor nodes without SQLite `INDEXED BY` planner/miss errors -> verifies `P4-13`!).
     - Assert timestamp before first commit (`committed_at_ms < t0`) resolves cursor `0` (empty, no silent fallback to current).
     - Assert simultaneous `as_of_write_cursor` + `as_of_epoch_ms` fails cleanly with named worker error while keeping the worker usable.
     - Assert all `8` disclosure flag combinations hold over historical `as_of` stored & native graph targets, terminal edges, origins, and native inline `properties` / `terminal_edge_properties` (physically erased rows absent; erasure-pending rows withheld when `!show_erased`; inactive rows withheld when `no_show_inactive`).
- **GREEN:** Implement `graph.rs` historical index selection, `P4-13` `INDEXED BY` conditional lift, `_fathomdb_commit_timestamps` resolution, native inline property policy filtering, and Schema 38 edge index alignment.
- **Check:** Re-run `fathomdb-inspector` default + `cached-cpu` Rust suites.

### Wave 3 (RED -> GREEN): Playwright Browser UI (Outside-`body` `properties`, XSS Safety, `as_of` Controls, Comparison, Installed)

- **RED tests to add first:**
  1. `src/inspector/tests/browser/attributes.mjs` & `installed.mjs`: inspect node and edge outside-`body` `properties`, expand `tree`, copy full properties to real clipboard asserting exact `9007199254740993123456789` lexeme preservation, verify `scope` rendering, and test hostile XSS strings (`<img src=x onerror=alert(1)>`, `<script>`, `"><svg onload=alert(1)>`) in `properties` keys, values, and graph labels trigger zero `pageerror` / alert / DOM injection.
  2. `src/inspector/tests/browser/graph.mjs` & `comparison.mjs`: select `Current` vs `Write cursor` vs `Commit time (ms)` modes in Graph UI, run both Stored and Native traversal, verify `Back` restoration preserves `as_of` context, verify A/B comparison pins retain distinct `as_of` requests/observations, and extend `D01` 8-flag erasure UI assertions (`comparison.mjs`, `installed.mjs`) to verify zero `properties` or historical erased disclosure leakage.
- **GREEN:** Implement `assets/app.js` and `assets/presentation.mjs` property panels, `as_of` controls, and comparison state propagation.
- **Check:** Playwright unit browser suite (default + `installed.mjs`).

### Wave 4 (RED -> GREEN): Scoped `local-ci.sh`, Timeouts/Cleanup, & `D4 = B` / `local-release.py`

- **RED tests to add first:**
  1. `src/inspector/tests/tools/test_local_ci.py`:
     - Default runs `--scoped` (verifying `agent-lint.sh` + `agent-typecheck.sh` + `cargo test -p fathomdb-inspector` default/`cached-cpu` are invoked, and full `scripts/agent-verify.sh` is NOT invoked);
     - `--full-workspace` invokes `scripts/agent-verify.sh`;
     - `--installed-only` preserves installed behavior;
     - Combining any two mode flags (`--scoped --full-workspace`, `--scoped --installed-only`, `--full-workspace --installed-only`) exits `2`;
     - Non-Inspector diff (e.g. modified `src/rust/crates/...` or `Cargo.toml` in a temp git fixture) causes `--scoped` to fail closed before stages run and name `--full-workspace`.
  2. `src/inspector/tests/tools/test_local_release.py`:
     - Update compiled schema expectation to `38` and version `0.3.0`;
     - Add static `D4 = B` AST/tree grep test failing if `docker`, `openbox`, `xdotool`, or `sudo` appears outside the explicit negative test mock inside `test_local_ci.py`.
  3. Timeout & cleanup regression tests:
     - `src/inspector/tests/launch.rs`: test verifying child ignoring `SIGTERM` is killed via `SIGKILL` within the 30s bound rather than hanging `launch.rs` (using a short-circuit or mock child ignoring `SIGTERM` inside test coverage).
     - `src/inspector/tests/tools/` shell/Node fixture verifying `run-installed.sh` escalates stalled `Xvfb` to `SIGKILL` within ~2s and `installed-harness.mjs` reaps descendants cleanly without stdout pipe deadlock.
- **GREEN:** Implement `local-ci.sh`, timeout/cleanup fixes across shell/Node/`launch.rs`, dynamic version derivation, `local-release.py`, and `package.sh`.
- **Check:** `python3 -m unittest discover -s src/inspector/tests/tools -v` + `cargo test --locked -p fathomdb-inspector --test launch`.

### Wave 5 (RED -> GREEN, Serialized After Slice 50): `release.yml` `build-inspector` & Workflow Guards

- **RED checks (run before editing `release.yml` or right after adding `build-inspector` before updating guards):**
  - Confirm `scripts/check-release-contract-truth.py`, `scripts/tests/test_release_workflow_scope.sh`, `scripts/tests/test_runtime_release_alignment.sh`, and `actionlint` catch unmatched `upload-artifact-retry` count, `all-builds-passed` needs, or `github-release` wiring.
- **GREEN:** Add `build-inspector` + `github-release` artifact upload/download in `.github/workflows/release.yml`, and update `scripts/check-release-contract-truth.py`, `scripts/tests/test_release_workflow_scope.sh`, and `scripts/tests/test_runtime_release_alignment.sh` in lockstep.
- **Check:**
  - `actionlint -config-file .github/actionlint.yaml .github/workflows/*.yml`
  - `python3 scripts/check-release-contract-truth.py`
  - `bash scripts/tests/test_release_workflow_scope.sh`
  - `bash scripts/tests/test_runtime_release_alignment.sh`

### Wave 6: Engineering Docs, Public Docs & Final Proportional Verification

- Update `dev/inspector/{needs,requirements,ROADMAP,BACKLOG}.md`, `docs/inspector/**`, `docs/inspector/releases/0.3.0.md`, `mkdocs.yml`, and `docs/contributing/index.md` (`P5-10`, `P6-11`).
- **Verification commands (run from the target worktree):**
  1. `bash scripts/agent-lint.sh` (includes workspace clippy `-D warnings`, rustfmt, shellcheck over `src/inspector/tools/*.sh`, `check-release-contract-truth.py`, `check-public-doc-truth.py`, `actionlint`, `agent-lint-md.sh`, lychee)
  2. `bash scripts/agent-typecheck.sh`
  3. `bash scripts/agent-lint-docs.sh` and `mkdocs build --strict`
  4. `bash scripts/tests/test_release_workflow_scope.sh` and `bash scripts/tests/test_runtime_release_alignment.sh`
  5. Inspector end-to-end qualification: `bash src/inspector/tools/local-ci.sh --full-workspace` (required whenever Slice 60 diff includes `.github/workflows/release.yml`, `scripts/`, or `docs/contributing/index.md`; `local-ci.sh --scoped` on Inspector-only branches)
  6. Note: Release-level paired benchmarks and full unconfined `scripts/agent-verify.sh` (including `AC-036` `strace`) run in Slice 70 (`QUALIFICATION`).
