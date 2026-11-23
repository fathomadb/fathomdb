---
title: "Slice 60 — FathomDB Inspector Schema 38, Historical Graph, Scoped CI & Release Packaging"
status: CLOSED
target_release: 0.8.31
slice: 60
date: 2026-11-23
---

# Slice 60 — FathomDB Inspector (`0.3.0`) for Schema 38, `as_of` Historical Graph, Scoped Local CI, Timeout Hardening & Release Packaging · `[implementation]` · depends-on: `30 (STRICT-GRAPH), 40 (HISTORY-GRAPH), 50 (SDK-CLI-UPGRADE)`

> **Reconciled Post-Slice-50 Plan (HEAD `28f5def55`) — Slices 20, 30, 40, and 50 have landed.**
> Prior prework triage (`P2-13`, `P3-16`, `P3-17`, `P4-13`, `P5-10`, `P6-11`, `P7-01`, `P7-02`, `P7-04`, `P7-06`) was relocated to Slice 60 at HITL Gate 1 (`2026-11-22`). All Schema 38 DDL, `GraphExpandRequestV1::with_as_of_{write_cursor,epoch_ms}`, Chromium + bundled-SQLite runtime stack, and post-Slice-50 `.github/workflows/release.yml` contracts below are reconciled against live HEAD.

---

## 1. Goals & Non-Goals

### 1.1 Goals

1. **Schema 38 admission & reserved `scope` alignment (`src/inspector/src/admission.rs`, `storage.rs`):**
   - Admit Schema `38` databases (`compiled_schema = 38`), reject Schema `37` (`0.8.30`) cleanly, validate Schema 38 columns on `canonical_nodes` (`properties,scope`), `canonical_edges` (`properties,scope`), `operational_mutations` (`scope`), `operational_state` (`scope`), and `_fathomdb_projection_registry` (`entity_class,vector_kinds`), plus Schema 38 tables `canonical_property_values` and `_fathomdb_commit_timestamps` (retaining existing `_fathomdb_vector_kinds(kind,profile,created_at)` probe), and surface explicit read-only `scope` inspection (empty/reserved in `0.8.31`).
   - Preserve Inspector's read/lock posture: preflight uses `SQLITE_OPEN_READ_ONLY` + `immutable=1` (preserving refused files byte-for-byte), all `Reader` SQLite queries use `SQLITE_OPEN_READ_ONLY` + `PRAGMA query_only = true` inside snapshot transactions, and `Session::attach_mode` holds an owned `Engine` session + `.lock` per `dev/inspector/ROADMAP.md`.
2. **Outside-`body` node & edge `properties` UI + strict XSS, memory-admission & erasure non-disclosure (`storage.rs`, `graph.rs`, `lexical.rs`, `tree.rs`, `assets/app.js`, `assets/presentation.mjs`):**
   - Inspect, expand, and exactly copy outside-`body` node and edge `properties` (preserving typed primitives, empty strings, `SQL NULL`, and 64-bit integer precision outside `2^53`) alongside plain-text `body` documents, with all user/database keys and values rendered purely via `.textContent` (`el()` / `svgElement()`), threading `field` through `tree()` / `drawTree()` / copy actions (fixing hardcoded `"body"`), adding `"properties"` to `graph_admission()` and `lexical_admission()` 64 MiB scans, and withholding `properties` AND `properties_bytes` / `typeof(properties)` metadata whenever `!Session::policy.permits(classification)` across all `D-01` 8-flag combinations.
   - Surface declared filterable property metadata (`canonical_property_values` + `_fathomdb_projection_registry.{entity_class,vector_kinds}`) for both `Class::Node` and `Class::Edge` in `associations` and bounded `catalog` (filtering edge-only declarations out of node-only lexical `propertyChoices`).
3. **Historical `as_of` graph inspection & `P4-13` `INDEXED BY` lift (`graph.rs`, `worker.rs`, `app.js`, `comparison.mjs`):**
   - Expose mutually exclusive `as_of_write_cursor` and `as_of_epoch_ms` (`Option<String>` wire encoding matching Inspector's 64-bit integer protocol, resolved via O(log N) `_fathomdb_commit_timestamps` reverse seek) for both stored-adjacency (`op: "graph"`) and native engine traversal (`op: "graph_serving"` using `GraphExpandRequestV1::with_as_of_write_cursor` / `with_as_of_epoch_ms`).
   - Lift `INDEXED BY canonical_nodes_logical_active_idx` and `superseded_at IS NULL` in `graph_serving` whenever `as_of` is active, using the exact O(log N) two-branch `UNION ALL` lookup backed by Schema 38's `canonical_nodes_logical_sup_idx` (and resolving the historical seed revision in stored-adjacency `Reader::graph`), while preserving non-partial `canonical_edges_{from_id,to_id}_idx` in current mode when `!no_show_inactive` so superseded edges stay visible under `D-01` defaults and subordinating every stored and native `as_of` record, origin, and inline property payload to all 8 `D-01` erasure flags.
4. **Scoped `local-ci.sh` default (~5 min) retaining full gate & fail-closed path guard (`P2-13`, `P7-01`):**
   - Change `src/inspector/tools/local-ci.sh` default mode to `--scoped` (~5 min Inspector-scoped lint, typecheck, default + `cached-cpu` Inspector Rust tests, tooling/unit/native-build/seal/candidate/abi/installed stages without duplicate unittest runs), retain `--full-workspace` (full `scripts/agent-verify.sh`) and `--installed-only`, enforce flag mutual exclusion (exit `2`), support explicit `--base <ref>` / `INSPECTOR_SCOPED_BASE` (defaulting to `origin/main` and failing closed if unresolvable) so branch worktrees like `release/0.8.31` can target Inspector-only commits without false positives, and support optional default paths alongside explicit 3 positional args (`CANDIDATE FIXTURES BUILD_TARGET`).
5. **Bounded timeouts & non-blocking process cleanup (`P7-02`):**
   - Enforce `timeout --kill-after=10s`, 2s `SIGKILL` escalation on `Xvfb` teardown, unblocked `strace` stdout pipe handling, 5s deadlines on `browser.close()` and post-`SIGKILL` teardown with `/proc` descendant-tree reaping, explicit timeouts on `execFileSync` / `spawnSync` across all browser/qualification harnesses (including `attributes.mjs`, `graph.mjs`, `installed.mjs`, `package.mjs`), and a bounded `try_wait()` + `SIGKILL` helper in `src/inspector/tests/launch.rs` (30s production default, short-timeout unit test). Keep all `src/inspector/tools/*.sh` edits `SC2312`-clean for `scripts/agent-lint-shell.sh`.
6. **Retained `D4 = B` Docker-free Ubuntu 24.04 Chromium + bundled-SQLite qualification & `compiled_schema = 38` (`P7-04`):**
   - Lock `local-release.py` and `test_local_release.py` to `D4 = B` (native Ubuntu 24.04 / `glibc 2.39`, embedded local HTTP UI in Chromium, bundled SQLite + Landlock — zero GTK4/WebKitGTK), update `compiled_schema` from `37` to `38`, derive `VERSION` dynamically from `src/inspector/Cargo.toml` (`0.3.0`) and engine provenance dynamically from root `Cargo.toml` `[workspace.package].version` + `compiled_schema = 38` (no hardcoded `0.8.30` SHA and no premature `0.8.31` break pre-Slice-70), update `Cargo.lock` + `src/inspector/package/README.md`, and enforce a static zero-`docker` / `openbox` / `xdotool` / `sudo` recurrence guard.
7. **`build-inspector` release packaging in `.github/workflows/release.yml` (`P7-06`, serialized after Slice 50) & Docs (`P5-10`, `P6-11`):**
   - Add `build-inspector` on `ubuntu-24.04` (`needs: [verify-release, verify-cuda-trusted-route]`, `permissions: { contents: read }`, no `!cancelled()`, SHA-pinned `actions/checkout` with `persist-credentials: false`, SHA-pinned `dtolnay/rust-toolchain` `1.95.0`, `cargo fetch --locked`, `grep -Fq` in `package.sh`, `scripts/check-glibc-floor.sh --floor 2.39`, `local-release.py seal` + `verify --source`, staging non-colliding `fathomdb-inspector-0.3.0-linux-x86_64.tar.gz` + `.tar.gz.sha256` via `upload-artifact-retry` advancing count `14 -> 15`, gated in both `all-builds-passed.needs` & `all-builds-passed.if`, attached in `github-release` via SHA-pinned `actions/download-artifact`).
   - Update `check-release-contract-truth.py`, `test_release_contract_truth.sh`, `test_release_workflow_scope.sh`, `test_runtime_release_alignment.sh`, and `test_linux_first_platform_scope.sh` in lockstep (`scripts/set-version.sh` and `scripts/check-license-consistency.sh` require no changes because `src/inspector/Cargo.toml` has `publish = false` and `license.workspace = true`).
   - Update `dev/inspector/{needs,requirements,ROADMAP,BACKLOG}.md`, `src/inspector/tools/README.md`, `src/inspector/package/README.md`, `docs/inspector/` (including `docs/inspector/releases/0.3.0.md` and `mkdocs.yml`), and `docs/contributing/index.md`.

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
| `AC31-60-01` | `R31-08` | **Schema 38 strict admission (`INS-01`, `V01-01`):** Opening a Schema `38` database succeeds in both default and `cached-cpu` modes and validates `properties`, `scope`, `_fathomdb_projection_registry.{entity_class,vector_kinds}`, `canonical_property_values`, `_fathomdb_commit_timestamps`, and existing `_fathomdb_vector_kinds`; Schema `37` or missing required Schema 38 tables/columns fail closed with named admission diagnostics (`unsupported_schema` / `unsupported_layout`). |
| `AC31-60-02` | `P3-16` | **Outside-`body` node/edge `properties` & reserved `scope` inspection (`INS-02`, `INS-03`):** Nodes and edges with plain-text `body` + JSON `properties` render bounded `properties` metadata, expandable typed key/value inspection (`detail_field`, `chunk`, `tree` with threaded `field` through `drawTree()`), exact copy preserving `abs(i64) > 2^53` / `-0` / bool / string / empty / `NULL`, visible reserved `scope`, `"properties"` included in `graph_admission()` / `lexical_admission()` 64 MiB scans, and bounded `canonical_property_values` coverage across both nodes and edges in `associations` and `catalog`. |
| `AC31-60-03` | `P3-16` | **XSS & 8-flag erasure non-disclosure over `properties` (`INS-02`, `D-01`):** Hostile HTML/SVG/JS strings in `properties` keys, string values, or `scope` render inertly via DOM `.textContent` with zero script execution or attribute injection; across all `2^3 = 8` policy flag combinations, queued erasure-pending rows withhold `body`, `properties`, `properties_bytes`, and `typeof(properties)` metadata on `page`, `detail`, `chunk`, `tree`, `graph`, `investigate`, `graph_serving`, `lexical`, `vectors`, `serving`, `associations`, and `export` when `--show-erased` is off. |
| `AC31-60-04` | `P3-17`, `P4-13` | **Historical `as_of` graph exploration + 8-flag erasure subordination (`INS-03`, `V01-01`):** Stored (`graph`) and native (`graph_serving` via `GraphExpandRequestV1::with_as_of_{write_cursor,epoch_ms}`) graph inspection accept mutually exclusive `Option<String>` `as_of_write_cursor` or `as_of_epoch_ms` (resolved via O(log N) `_fathomdb_commit_timestamps` reverse seek), reject simultaneous cursor+epoch inputs cleanly without evicting UI cache, lift `INDEXED BY canonical_nodes_logical_active_idx` in `graph_serving` using two-branch `UNION ALL` over `canonical_nodes_logical_sup_idx`, resolve historical seed revisions in `Reader::graph`, keep non-partial `canonical_edges_{from_id,to_id}_idx` in current mode when `!no_show_inactive`, report explicit `as_of` scope metadata, preserve `as_of` parameters across comparison pins, and strictly enforce all 8 `D-01` disclosure flags over historical targets, terminal edges, origins, and inline properties. |
| `AC31-60-05` | `P2-13`, `P7-01` | **Scoped `local-ci.sh` default & fail-closed diff guard (`INS-10`):** `src/inspector/tools/local-ci.sh` defaults to `--scoped` (~5 min Inspector-scoped source check + default/`cached-cpu` Inspector Rust tests + existing tooling/unit/native-build/seal/candidate/abi/installed stages without duplicate unittest runs); `--full-workspace` runs full `scripts/agent-verify.sh`; `--installed-only` preserves installed qualification; combining mode flags exits `2`; supports optional `--base <ref>` / `INSPECTOR_SCOPED_BASE` (default `origin/main`, failing closed if unresolvable) and fails closed if any diff outside `src/inspector/`, `dev/inspector/`, `docs/inspector/` exists, directing the user to `--full-workspace`. |
| `AC31-60-06` | `P7-02` | **Bounded subprocess timeouts & non-blocking cleanup (`INS-10`):** `run-installed.sh` (`timeout --kill-after=10s`, 2s `Xvfb` `SIGKILL` escalation), `qualify-installed-network.sh`, `installed-harness.mjs` (non-blocking `strace` stdout handling, 5s `browser.close()` and post-`SIGKILL` wait deadlines, `/proc` descendant-tree `SIGKILL` reaping), browser/qualification Node scripts (`attributes.mjs`, `graph.mjs`, `installed.mjs`, `package.mjs`, `qualify-*.mjs`, `clipboard-diagnostic.mjs`), and `tests/launch.rs` (bounded `try_wait()` + `SIGKILL` escalation helper tested with a short non-`#[ignore]` timeout) never wedge indefinitely on stalled children or orphan processes. |
| `AC31-60-07` | `P7-04` | **Retained `D4 = B` native Linux Chromium + bundled-SQLite qualification & `compiled_schema = 38` (`INS-10`, `INS-11`):** `local-release.py` and `build-description` agree on `compiled_schema = 38`, Inspector `0.3.0` (from `src/inspector/Cargo.toml`), workspace engine version, and Ubuntu 24.04 / `glibc 2.39` Chromium `SUPPORT` metadata without stale `0.8.30` commit pins; a static guard in `test_local_release.py` enforces zero `docker`, `openbox`, `xdotool`, or `sudo` invocations across `src/inspector/tools/` and `src/inspector/tests/` (excluding explicit negative test shims in `test_local_ci.py`). |
| `AC31-60-08` | `P7-06` | **`release.yml` `build-inspector` job & structural contract parity:** Serialized after Slice 50, `.github/workflows/release.yml` adds `build-inspector` on `ubuntu-24.04` (`needs: [verify-release, verify-cuda-trusted-route]`, `permissions: { contents: read }`, no `!cancelled()`, SHA-pinned `actions/checkout` with `persist-credentials: false`, SHA-pinned `dtolnay/rust-toolchain` `1.95.0`, `cargo fetch --locked`, `package.sh`, `check-glibc-floor.sh --floor 2.39`, `local-release.py seal` + `verify --source`, uploading non-colliding `.tar.gz` + `.tar.gz.sha256` via `upload-artifact-retry` advancing count `14 -> 15`), gates both `all-builds-passed.needs` and `all-builds-passed.if`, attaches assets in `github-release`, and passes `actionlint`, `check-release-contract-truth.py`, `test_release_contract_truth.sh`, `test_release_workflow_scope.sh`, `test_runtime_release_alignment.sh`, and `test_linux_first_platform_scope.sh`. |
| `AC31-60-09` | `P5-10`, `P6-11` | **Engineering specs & public documentation currency (`INS-01`..`INS-12`):** `dev/inspector/{needs,requirements,ROADMAP,BACKLOG}.md`, `src/inspector/tools/README.md`, `src/inspector/package/README.md`, `docs/inspector/` (including `docs/inspector/releases/0.3.0.md` and `mkdocs.yml`), and `docs/contributing/index.md` reflect Schema 38, `properties`/`scope` UI, `as_of` graph exploration, `local-ci.sh --scoped`, and `D4 = B` release packaging, passing `agent-lint-md.sh`, `agent-lint-docs.sh`, `check-public-doc-truth.py`, and `mkdocs build --strict` without claiming `0.8.31` is already published. |

---

## 3. Architecture & Draft Design

### 3.1 Schema 38 Admission, Reserved `scope`, & `properties` Storage (`admission.rs`, `storage.rs`, `tree.rs`, `vector.rs`, `lexical.rs`, `worker.rs`, `failure.rs`)

1. **`src/inspector/src/admission.rs` (`validate_layout`):**
   - Enforce Schema `38` exact match against `fathomdb_schema::SCHEMA_VERSION`.
   - Extend `validate_layout` column probes (`SELECT {cols} FROM {table} LIMIT 0`):
     - `canonical_nodes`: append `,properties,scope`.
     - `canonical_edges`: append `,properties,scope`.
     - `operational_mutations` and `operational_state`: append `,scope`.
     - `_fathomdb_projection_registry`: append `,entity_class,vector_kinds` (matching live Schema 38 `STEP_38_SQL`).
     - Probe new Schema 38 tables: `canonical_property_values` (`entity_class,write_cursor,attr_name,value_type,text_val,num_val,bool_val`) and `_fathomdb_commit_timestamps` (`write_cursor,committed_at_ms`), while keeping the existing `_fathomdb_vector_kinds` (`kind,profile,created_at`) probe unchanged.

2. **`src/inspector/src/storage.rs`, `graph.rs`, `lexical.rs` (`Class::fields`, `summary`, `page`, `display_preview`, `detail_field`, `tree`, `associations`, `catalog`, memory admission):**
   - **Critical Erasure Non-Disclosure Design (`properties` vs `Class::fields`):**
     - `Reader::page` currently populates every item via `Self::summary(&c, &locator)` even when `self.session.policy.permits(class)` is `false` (`payload_status == "refused"`, queued erasure-pending rows).
     - Therefore **NEVER add `"properties"` to `Class::fields()`** and never read or expose `properties`, `properties_bytes`, or `typeof(properties)` on unpermitted rows!
     - Design:
       - Keep non-payload metadata `"scope"` safe in `Class::fields` for `Node`, `Edge`, `OperationalMutation`, and `OperationalState` (visible on summary and detail, since `scope` is a structural namespace boundary empty in `0.8.31`).
       - Add `"properties"` to `Self::field(&locator, field)` for `Class::Node` and `Class::Edge` so `detail_field(handle, "properties")` and `chunk(handle, "properties", offset, limit)` stream exact UTF-8 slices under the existing `if !self.session.policy.permits(class) { return Err("refused"); }` gate.
       - In `Self::summary` / `summary_fields_budget`: read `properties`, `properties_bytes`, and `properties_storage` **only when `policy.permits(class)` evaluates `true`**; when `permits(class)` is `false`, do not read `properties` or leak `properties_bytes`/`typeof(properties)` (emit `"properties_status": "refused"` and omit `properties*` fields, matching `body`). Pass `policy` (or `permitted: bool`) explicitly into `summary` callers across `page`, `sample`, `detail_field`, `graph_record`, `investigate`, `lexical`, `vector`, and `associations`.
   - **Memory admission parity (`graph.rs`, `lexical.rs`):**
     - Add `"properties"` to both `graph_admission()` and `lexical_admission()` `octet_length(...)` scans on `canonical_nodes` and `canonical_edges` so large inline property payloads obey the 64 MiB admission cap.
   - **Plain-text `body` + `properties` `display_preview` fallback & preview budget:**
     - In `display_preview` (`storage.rs`), if `body` is non-JSON plain text (or JSON without `"title"` / `"name"`), check `properties` (bounded <= 4096 bytes) for `"title"` or `"name"` **only after verifying `permits(class)`**; never propagate JSON parse failures from plain-text `body`, and budget `body` and `properties` cleanly in `detail_field` (e.g., `8192` or separate 4096-byte budgets) so a 3000-byte plain-text `body` does not cause a 2000-byte `properties` JSON to falsely hit `preview_budget_exhausted`.
   - **Structured typed property tree & 64-bit integer precision (`tree.rs`, `worker.rs`, `storage.rs`):**
     - Extend `worker::Request::Tree` (`#[serde(deny_unknown_fields)]`) with `#[serde(default)] field: Option<String>` accepting `None | Some("body") | Some("properties")` (rejecting any other column name with existing `"unsupported_field"` from `Failure::decode::CODES`). `storage::tree` opens `Self::field(&locator, field)` (preserving the `policy.permits(class)` refusal check!) and delegates to `tree::children`.
     - Because `tree::children` extracts raw byte `lexeme` strings directly from SQLite blob slices without going through JavaScript floating-point `JSON.parse`, 64-bit integers outside IEEE-754 safe integer range (`abs(x) > 2^53`, e.g. `9007199254740993123456789`) and `-0` survive inspection and exact copy verbatim!
   - **Node & edge property materialization telemetry (`catalog`, `associations`, `presentation.mjs`):**
     - Extend `catalog` (`storage.rs`) using the same bounded `remaining`-row cap (`100000`) and owner `inventory_summary` / `no_show_inactive` policy checks used for `canonical_attributes` (joining `canonical_nodes` for `'node'` and `canonical_edges` for `'edge'`), plus bounded `_fathomdb_commit_timestamps` counts and `_fathomdb_projection_registry.{entity_class,vector_kinds}` metadata (never an unbounded `COUNT(*)`).
     - Extend `associations` (`storage.rs`) to filter `_fathomdb_projection_registry` declarations by `entity_class = locator.class.label()` (`'node'` vs `'edge'`) and inspect `canonical_property_values WHERE entity_class=?1 AND write_cursor=?2` (gated on `permits(class)`) for both `Class::Node` and `Class::Edge`. Guard `assets/presentation.mjs` `propertyChoices` so edge-only declarations do not appear in node-only lexical property FTS choices.
   - **`vector.rs` custom enrolled vector kinds & `lexical.rs` parity:**
     - Keep `vector.rs` `source_type == "edge_fact"` vs `Class::Node` routing and add coverage validating custom enrolled node kinds from `_fathomdb_vector_kinds` (and verifying reserved `"edge_fact"` rejection holds on Schema 38).
     - Keep `lexical.rs` (`ReadView` / `LexicalView`) aligned with Slice 20/30 struct signatures landed prior to Slice 60.
   - **`failure.rs` closed error whitelist:**
     - Reuse existing `"unsupported_field"` for bad `tree.field` values. Register `"invalid_as_of"` in BOTH `Failure::decode`'s `CODES` slice AND `Failure::read` so it maps cleanly to `"refused"` rather than falling through to `"error"`, and validate `as_of` strictly in the Graph form before calling `api()` (matching `renderLexical`) so user input errors do not trigger `app.js`'s `"refused"` observation-cache wipe.

### 3.2 Safe UI Rendering for `properties`, `scope`, & `as_of` Telemetry (`src/inspector/assets/app.js`, `presentation.mjs`)

- **Strict `.textContent` XSS boundary & threaded `field` in `drawTree()`:**
  - Construct every DOM element for node/edge `properties` summaries, `Open full properties` tree/text inspector, `Copy full properties` exact payload button, `scope` badge, `associations` property entries (`Node` + `Edge`), and `graph` target/edge property panels exclusively using `el(tag, text, attrs)` (`n.textContent = String(text)`) and `svgElement(...)` (`node.textContent = text`).
  - Thread `field` (`"body"` vs `"properties"`) explicitly through `tree()`, `drawTree()`, recursive `Expand`, `"More structured fields"`, `"Copy value"`, and `"Copy raw JSON"` in `assets/app.js` (fixing the existing hardcoded `"body"` in `copyField(handle, "body", ...)` so copying values inside a `properties` tree reads byte offsets from `"properties"`, not `"body"`!).
  - Zero use of `innerHTML`, `insertAdjacentHTML`, `document.write`, or untrusted `href`/`src` attributes. Render `<img onerror=...>`, `<script>`, SVG `onload`, empty string `""`, and `"null"` strictly as literal text.
- **Outside-`body` `properties` workspace UX:**
  - Record detail panel displays:
    - Existing `body` workspace (now explicitly handles plain-text `body` alongside JSON),
    - A dedicated `Properties` metadata/action block (when permitted and `properties_bytes != null`: byte size, storage type, bounded preview or `tree` key-type-lexeme rows when JSON object <= 16 KiB, `Open full properties`, `Copy full properties`; explicit `"null"` when SQL NULL and `"refused (erasure-pending)"` with zero byte/type disclosure when withheld by policy),
    - Read-only `Scope` field (`""` rendered clearly as empty/default namespace).

### 3.3 Historical `as_of` Graph Inspection, `P4-13` `INDEXED BY` Lift & 8-Flag Erasure Matrix (`graph.rs`, `worker.rs`, `app.js`, `comparison.mjs`)

1. **Worker IPC contract (`worker.rs`):**
   - Extend `worker::Request::Graph` and `worker::Request::GraphServing` (`#[serde(deny_unknown_fields)]`) with `Option<String>` fields (matching every other 64-bit integer in `worker.rs` to preserve full precision above `2^53`):
     - `#[serde(default)] as_of_write_cursor: Option<String>` (parsed as canonical `u64`)
     - `#[serde(default)] as_of_epoch_ms: Option<String>` (parsed as signed `i64`)
   - Enforce mutual exclusion and syntax validation at worker entry: invalid integers or simultaneous `as_of_write_cursor` + `as_of_epoch_ms` return `"invalid_as_of"` cleanly without dropping worker session state.
   - Commit-timestamp resolution helper (`resolve_as_of_cursor`):
     - If `as_of_write_cursor = Some(c)`: `resolved_cursor = c`.
     - If `as_of_epoch_ms = Some(ms)`: run the exact O(log N) reverse-index-seek query matching `fathomdb-engine`: `SELECT write_cursor FROM _fathomdb_commit_timestamps WHERE committed_at_ms <= ?1 ORDER BY committed_at_ms DESC, write_cursor DESC LIMIT 1`; if no row matches (timestamp precedes every commit), `resolved_cursor = 0` (explicitly matching zero committed revisions rather than falling back to current!).
     - Record `{ mode: "current" | "write_cursor" | "epoch_ms", requested_as_of_write_cursor, requested_as_of_epoch_ms, resolved_as_of_write_cursor }` in `query_context`, `scope`, and `graph_serving`'s `request` object (with stringified 64-bit cursor/epoch values).

2. **`src/inspector/src/graph.rs` — `P4-13` `INDEXED BY` Lift & Historical Resolution:**
   - **Native `Reader::graph_serving`:**
     - Construct Slice 30/40 `GraphExpandRequestV1` (`GraphReadContextV1::Current` / `Frozen` unchanged) and chain `.with_as_of_write_cursor(c)` or `.with_as_of_epoch_ms(ms)` when `as_of` is active.
     - **`P4-13` Origin/Predecessor lookup (`graph.rs`):**
       - When `as_of` is **None** (current mode): preserve `SELECT rowid,write_cursor FROM canonical_nodes INDEXED BY canonical_nodes_logical_active_idx WHERE logical_id=?1 AND superseded_at IS NULL`.
       - When `as_of` is **Some(resolved_cursor)**: **LIFT `INDEXED BY canonical_nodes_logical_active_idx` and `superseded_at IS NULL`** and use the exact O(log N) two-branch `UNION ALL` lookup backed by Schema 38's `canonical_nodes_logical_active_idx` + `canonical_nodes_logical_sup_idx` (matching `fathomdb-engine`'s historical `load_node`):
         `SELECT rowid, write_cursor FROM canonical_nodes WHERE logical_id=?1 AND superseded_at IS NULL AND write_cursor<=?2 UNION ALL SELECT rowid, write_cursor FROM canonical_nodes WHERE logical_id=?1 AND superseded_at>?2 AND write_cursor<=?2 ORDER BY write_cursor DESC LIMIT 1` (saturating `resolved_cursor` to `i64::MAX`), avoiding any linear `graph_scan`!
     - **Native inline property & payload policy guard:** Whenever Engine `GraphTargetV1` / `GraphOriginV1` returns inline `properties` or `terminal_edge_properties`, check `self.session.policy.permits(classification)` against the hydrated `target_record` and `terminal_edge_record`; **strip/omit native inline `properties`, `payload`, and hydrated `record` metadata whenever `permits` is `false`!**
   - **Stored-adjacency `Reader::graph`:**
     - **Historical seed revision resolution:** In `as_of` mode, resolve the seed's `logical_id` at `resolved_cursor` via the exact O(log N) two-branch `UNION ALL` over `canonical_nodes_logical_active_idx` + `canonical_nodes_logical_sup_idx` (never inserting a future revision `write_cursor > resolved_cursor` into a historical graph; emit empty nodes/edges if no revision existed at `resolved_cursor`).
     - **Edge index selection preserving `D-01` visibility:**
       - Current mode (`as_of = None`): keep non-partial `canonical_edges_from_id_idx` / `canonical_edges_to_id_idx` (retained in Schema 38) when `!no_show_inactive` so superseded edges remain visible under `D-01` defaults (optionally using partial `canonical_edges_{from,to}_active_idx` only when `no_show_inactive == true`).
       - Historical mode (`as_of = Some(resolved_cursor)`): use two-branch `UNION ALL` over Schema 38's `canonical_edges_{from,to}_sup_kind_idx` (`superseded_at IS NULL` + `superseded_at > ?cursor`, filtered by `write_cursor <= ?cursor`), and hydrate neighbor nodes at `resolved_cursor` via the two-branch `canonical_nodes_logical_{active,sup}_idx` query.
     - **8-flag disclosure parity (`D-01`):** Pass every historical node and edge revision through `self.graph_record` -> `self.session.policy.permits(class)` (`no_show_inactive`, `show_erased`, `show_payloads`). Because `storage.rs` classifies any `superseded_at IS NOT NULL` row as `inactive = true`, `--no-show-inactive` withholds superseded revisions even under `as_of` (while default `no_show_inactive = false` displays them unless erased & `!show_erased`). Historical `as_of` NEVER resurrects physically erased rows and NEVER reveals queued erasure-pending payloads/properties when `show_erased == false`.

3. **UI Controls & Comparison Pins (`app.js`, `comparison.mjs`):**
   - Add `Snapshot` (`Current` / `At write cursor` / `At commit time (ms)`) selector + integer input (`as_of_write_cursor` or `as_of_epoch_ms`, sent as strings) in the Graph workspace (shared across Stored graph and Native graph traversal).
   - Validate integer syntax strictly in the Graph form before calling `api()` so invalid user input displays inline status without triggering `"refused"` cache invalidation.
   - Forward `as_of_write_cursor` / `as_of_epoch_ms` in `Run native graph traversal`, `observationSummary`, `pinDescriptor`, `openPinned`, and `comparison` side A/B diffs so users can compare historical vs current graphs without silent snapshot drift, and render `target.properties` / `terminal_edge.properties` safely via `el()`.

### 3.4 Bounded Scoped `local-ci.sh` (`P2-13`, `P7-01`)

- Refactor `src/inspector/tools/local-ci.sh` CLI & execution (keeping all bash `SC2312`-clean for `scripts/agent-lint-shell.sh`):
  - Mode flags (mutually exclusive; combining >= 2 flags or passing unknown `--*` exits `2` with usage message on stderr):
    - `--scoped` (**default** when zero mode flags passed)
    - `--full-workspace`
    - `--installed-only`
  - Optional `--base <ref>` flag (or `INSPECTOR_SCOPED_BASE` env var, defaulting to `origin/main`) to choose the diff base for `--scoped` (essential on release branches such as `release/0.8.31` where prior slices already touched non-Inspector files relative to `origin/main`). Fail closed if the selected base ref is unresolvable (never silently skip the diff check).
  - Positional arguments (`CANDIDATE FIXTURES BUILD_TARGET`): continue accepting 3 explicit absolute positional args, and when omitted in `--scoped` / `--full-workspace`, default cleanly to deterministic temp paths under `${TMPDIR:-/tmp}/fathomdb-inspector-ci-...` (cleaned up on exit) so both `bash src/inspector/tools/local-ci.sh --full-workspace` and explicit 3-arg invocations work seamlessly; `--installed-only` still requires explicit `CANDIDATE FIXTURES BUILD_TARGET`.
  - **Fail-closed diff check inside `--scoped` (before running heavy stages):**
    - Enumerate modified, added, deleted, and untracked paths via `git diff --name-only <base>...HEAD` + working-tree `git status --porcelain`.
    - Allow only paths under `src/inspector/`, `dev/inspector/`, or `docs/inspector/`.
    - If ANY changed path lies outside those three prefixes, fail immediately (non-zero) printing the out-of-scope file list and instructing `--full-workspace` (or `--base <ref>` for branch-local Inspector commits).
  - **`--scoped` stage execution (~5 min target):**
    - `source`:
      - `bash scripts/agent-lint.sh` (covers shellcheck, clippy, rustfmt, release-contract-truth, public-doc-truth, actionlint, markdownlint, lychee)
      - `bash scripts/agent-typecheck.sh`
      - `cargo test --locked --offline -p fathomdb-inspector --jobs 1 -- --test-threads=1`
      - `cargo test --locked --offline -p fathomdb-inspector --no-default-features --features cached-cpu --jobs 1 -- --test-threads=1`
    - Follow with existing Inspector stages (`tooling` which already runs `unittest discover -s src/inspector/tests/tools`, `unit`, `native-build`, `seal`, `candidate`, `abi`, `installed` — no duplicate `unittest` run).
  - **`--full-workspace` stage execution:**
    - `source` runs `bash scripts/agent-verify.sh` followed by all Inspector stages.
  - **Single version source of truth:** Derive Inspector version (`0.3.0`) from `src/inspector/Cargo.toml` in `local-ci.sh`, `package.sh`, `local-release.py`, and test helpers instead of repeating hardcoded `'0.2.1'`. Update `src/inspector/tests/tools/test_package.py` and `test_local_ci.py` temp-repo setups to seed a fixture `src/inspector/Cargo.toml`.

### 3.5 Bounded Timeouts & Non-Blocking Cleanup (`P7-02`)

| File | Vulnerability / Unbounded Wait | Bounded Replacement |
| --- | --- | --- |
| `src/inspector/tools/run-installed.sh` | `kill "$display_pid"; wait "$display_pid"` hangs if `Xvfb` ignores `SIGTERM`; un-timed `node "$@"` | Wait loop up to 2s (`20 * 0.1s`) polling `kill -0 "$display_pid"`, escalate `kill -KILL "$display_pid"`, `wait ... \|\| true`; execute `timeout --kill-after=10s "${INSPECTOR_RUN_TIMEOUT_SECS:-900}" node "$@"` (`SC2312`-clean) |
| `src/inspector/tools/qualify-installed-network.sh` | Invokes `run-installed.sh` without outer bound | Wrap invocation in `timeout --kill-after=10s "${INSPECTOR_NETWORK_TIMEOUT_SECS:-900}"` |
| `src/inspector/tests/browser/installed-harness.mjs` | `strace` `stdio: ['ignore','pipe','pipe']` leaves `child.stdout` unconsumed (pipe deadlock); untimed `execFileSync`, `browser.close()`, and `await exited`; orphan descendants after `SIGKILL` | Set `stdio: ['ignore','ignore','pipe']` (or drain stdout); add `timeout:` to `execFileSync`; race `browser.close()` with 5s timer; add `/proc/<pid>/task/*/children` + `/proc` PPID tree `SIGKILL` reaper (`killDescendantTree`) and race post-`SIGKILL` `exited` against 5s deadline |
| `src/inspector/tools/qualify-startup.mjs` | Unbounded `execFileSync` & `spawnSync` | Add explicit `{ timeout: ... }` (120s build / 30s inspect / 60s startup calibration) and fail loudly on non-zero `status`, `signal`, or `error` |
| `src/inspector/tools/qualify-{graph,lexical,vector,scale}.mjs`, `clipboard-diagnostic.mjs`, and `src/inspector/tests/browser/{attributes,graph,installed,package}.mjs` | Unbounded `execFileSync`, `browser.close()`, and `child.on('exit')` waits | Use a shared cleanup/exec helper bounding `execFileSync` (`timeout`), `browser.close()` (5s–10s `Promise.race`), and child teardown (`child.stdin.end('close\n')` -> 5s `SIGTERM` -> 2s `SIGKILL` + `/proc` descendant reap) |
| `src/inspector/tests/launch.rs` | `child.wait()` blocks forever if child stalls after `SIGTERM` | Factor bounded `wait_with_sigkill_fallback(&mut child, timeout)` (production default `Duration::from_secs(30)`, polled via `try_wait()` + 25ms sleep, escalating to `child.kill()` + `child.wait()`); test with a short `500ms` timeout in a non-`#[ignore]` unit test |

### 3.6 Retained `D4 = B` Lock & `local-release.py` (`P7-04`)

- Lock `D4 = B` (Docker-free native Ubuntu 24.04 / `glibc 2.39`, Chromium embedded local HTTP UI + bundled SQLite + Landlock; zero GTK4/WebKitGTK, zero `docker` / `openbox` / `xdotool` / `sudo`).
- Update `src/inspector/Cargo.toml` (`0.2.1 -> 0.3.0`) and `Cargo.lock` (matching `--locked`).
- Update `src/inspector/tools/local-release.py`:
  - `compiled_schema`: **`37 -> 38`**.
  - Read `VERSION` (`0.3.0`) dynamically from `src/inspector/Cargo.toml` (fail closed if drifted).
  - Replace the stale `0.8.30` `ENGINE` commit SHA (`72fe9363...`) with dynamic workspace provenance read from root `Cargo.toml` `[workspace.package].version` + `compiled_schema = 38` (compatible with both Axis-W `0.8.30` during Slice 60 and `0.8.31` at release cut, and compatible with `test_local_release.py` synthetic `verify` runs without `--source`).
  - Keep existing `SUPPORT` schema keys (`'userland': 'Ubuntu 24.04 / glibc 2.39'`, `'chromium_qualified': '156.0.8078.4'`) and update distribution metadata to reflect `build-inspector` GitHub Release assets.
- Update `src/inspector/tools/package.sh`:
  - Derive version dynamically from `src/inspector/Cargo.toml`, replace `rg -q -F` with POSIX `grep -Fq` (so fresh Ubuntu 24.04 runners without ripgrep succeed), and verify `Cargo.lock` stays `--locked --offline`-clean after `cargo fetch --locked`.
- Update `src/inspector/tests/tools/test_local_release.py`:
  - Expect `compiled_schema: 38` and version `0.3.0`.
  - Add a **static `D4 = B` recurrence guard** scanning `src/inspector/tools/**` and `src/inspector/tests/**` (excluding the intentional negative `docker` poison-PATH test stub inside `test_local_ci.py`) asserting zero `docker`, `openbox`, `xdotool`, or `sudo` subprocess calls or directives.

### 3.7 Release Workflow `build-inspector` Job (`P7-06`, Serialized After Slice 50)

> **Post-Slice-50 Baseline (`fbb6591fc`):** `scripts/tests/test_release_workflow_scope.sh` currently enforces `upload-artifact-retry == 14` (post-`build-python-cli`). Adding `build-inspector` advances this to **`15`**. `scripts/set-version.sh` and `scripts/check-license-consistency.sh` require **no** changes (`src/inspector/Cargo.toml` has `publish = false` and `license.workspace = true`).

- **New job `build-inspector` in `.github/workflows/release.yml`:**
  - `runs-on: ubuntu-24.04` (matches `D4 = B` `glibc 2.39` floor; Inspector uses bundled SQLite and Chromium at runtime, requiring zero GTK4/WebKitGTK apt packages to build).
  - `needs: [verify-release, verify-cuda-trusted-route]` (matching sibling `build-python`, `build-python-cli`, `build-rust` lanes — **no** `!cancelled()`, which `check-release-contract-truth.py` bans as `UNSAFE_STATUS_BYPASS`).
  - `permissions: { contents: read }` (explicit least-privilege override of workflow-level `contents: write`).
  - Checkout step: `uses: actions/checkout@9c091bb21b7c1c1d1991bb908d89e4e9dddfe3e0` with `ref: ${{ env.RELEASE_CHECKOUT_REF }}`, `fetch-depth: 0`, and `persist-credentials: false`.
  - Toolchain: `uses: dtolnay/rust-toolchain@fa04a1451ff1842e2626ccb99004d0195b455a88` with `toolchain: "1.95.0"` (exact pin required by `test_runtime_release_alignment.sh`).
  - Pre-fetch dependencies: `cargo fetch --locked` (required so `package.sh`'s `--offline` build succeeds on fresh runners).
  - Build, verify, and seal:
    - `bash src/inspector/tools/package.sh "$RUNNER_TEMP/inspector-dist"`
    - `bash scripts/check-glibc-floor.sh --floor 2.39 "$RUNNER_TEMP/inspector-dist/fathomdb-inspector-0.3.0-linux-x86_64/fathomdb-inspector"` (or dynamically resolved version from `src/inspector/Cargo.toml`)
    - `python3 src/inspector/tools/local-release.py seal "$RUNNER_TEMP/inspector-dist"` and `python3 src/inspector/tools/local-release.py verify --source "$RUNNER_TEMP/inspector-dist"`
    - Stage non-colliding release assets in `$RUNNER_TEMP/inspector-release-assets/`: `fathomdb-inspector-0.3.0-linux-x86_64.tar.gz` and `fathomdb-inspector-0.3.0-linux-x86_64.tar.gz.sha256` (extracted from `SHA256SUMS` so `github-release`'s shared `dist/` never collides on a bare `SHA256SUMS` filename).
  - Upload staged assets exclusively via `uses: ./.github/actions/upload-artifact-retry` (`name: inspector-linux-x64-gnu`).
- **Release Gate & Attachment Wiring:**
  - Add `build-inspector` to **both** `all-builds-passed.needs` AND `all-builds-passed.if` (`&& needs.build-inspector.result == 'success'`).
  - Download `inspector-linux-x64-gnu` inside `github-release` via SHA-pinned `actions/download-artifact@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c` and attach the Inspector `.tar.gz` and `.tar.gz.sha256` alongside core release assets.
- **Lockstep Workflow Structural Tests:**
  - Update `scripts/check-release-contract-truth.py` and `scripts/tests/test_release_contract_truth.sh` (adding negative mutation tests for missing `build-inspector`, missing `all-builds-passed` gate, and missing `github-release` download, mirroring Slice 50's `build-python-cli` tests).
  - Update `scripts/tests/test_release_workflow_scope.sh`: add `"build-inspector"` to `abp` required set and update `upload-artifact-retry` exact count (`14 -> 15`, both assertion and diagnostic text).
  - Update `scripts/tests/test_runtime_release_alignment.sh` (`all-builds-passed` lane check + `dtolnay/rust-toolchain` pin + `actions/checkout` allowlist).
  - Verify `scripts/tests/test_linux_first_platform_scope.sh` (asserting `build-inspector` runs on `ubuntu-24.04`) and `actionlint -config-file .github/actionlint.yaml .github/workflows/*.yml`.

### 3.8 Engineering Specs, Public Docs & Contributing Notes (`P5-10`, `P6-11`)

- **`dev/inspector/`:** Update `needs.md`, `requirements.md`, `ROADMAP.md`, and `BACKLOG.md` for Schema 38 admission, `properties`/`scope` UI, `as_of` graph exploration + 8-flag subordination, `local-ci.sh --scoped`, `D4 = B` release qualification, and `build-inspector` artifact delivery. (Do **not** touch `dev/acceptance.md`.)
- **`src/inspector/tools/README.md` & `src/inspector/package/README.md`:** Document `--scoped` (non-full-workspace claim per `AGENTS.md §3`) vs `--full-workspace` vs `--installed-only` + `--base <ref>`, and update shipped tarball README from `0.2.1` / `0.8.30` / Schema `37` to `0.3.0` / `0.8.31` / Schema `38` / GitHub Release delivery.
- **`docs/inspector/` + `mkdocs.yml`:**
  - Update `index.md`, `getting-started/index.md`, `concepts/index.md`, `guides/index.md`, `reference/index.md`, `operations/index.md`, `contributing/index.md`, and `ROADMAP.md` for `0.3.0` / FathomDB `0.8.31` (Schema `38`, `properties` & `scope`, `as_of` graph inspection, release tarball download/checksum verification).
  - Add `docs/inspector/releases/0.3.0.md` and register it in `mkdocs.yml` navigation.
  - Phrasing guard (`scripts/check-public-doc-truth.py`): target `0.8.31` compatibility without asserting `0.8.31` is already published prior to release tagging.
- **`docs/contributing/index.md` (`P6-11`):** Add a concise section documenting `bash src/inspector/tools/local-ci.sh` (`--scoped` ~5 min default for Inspector-only diffs vs `--full-workspace` when crossing shared/workspace files vs `--installed-only`), complementing Slice 10's bootstrap updates.

---

## 4. Verified Post-Slice-50 Entry State & Key Code Seams

1. **`storage.rs` `Reader::page` erasure-leak & memory-admission seams:** `page()` emits `Self::summary` items with `Class::fields()` even when `!policy.permits(class)`. Keep `"properties"` out of `Class::fields()`, withhold `properties*` metadata when unpermitted, and add `"properties"` to `graph_admission()` and `lexical_admission()` 64 MiB scans.
2. **`graph.rs` edge & node indexes on Schema 38:** Non-partial `canonical_edges_{from_id,to_id}_idx` remain in Schema 38 (keep for `!no_show_inactive` current mode so superseded edges stay visible); Schema 38 also provides `canonical_nodes_logical_sup_idx`, `canonical_edges_{from,to}_active_idx`, and `canonical_edges_{from,to}_sup_kind_idx` for O(log N) historical two-branch `UNION ALL` lookups in both `Reader::graph` (including seed revision resolution) and `Reader::graph_serving` (`P4-13` `INDEXED BY` lift). Clean up stale format-37 comments in `session.rs` and `graph.rs`.
3. **Slice 30/40 Engine APIs landed:** `GraphExpandRequestV1` uses `.with_as_of_write_cursor(u64)` / `.with_as_of_epoch_ms(i64)` (`GraphReadContextV1` unchanged); native inline `GraphTargetV1.properties` / `GraphOriginV1.terminal_edge_properties` must obey `policy.permits` withholding.
4. **`assets/app.js` `drawTree()` hardcoded `"body"` & `attributes.mjs` harness gap:** Thread `field` through `tree()`, `drawTree()`, and copy actions; wire `attributes.mjs` (with `installedWorkspace` support) into `local-ci.sh` / `run-installed.sh` or cover installed property/XSS/clipboard assertions inside `installed.mjs` + `workspace.mjs`.
5. **Closed deserialization / error boundaries (`worker.rs`, `storage.rs`, `failure.rs`):** Use `Option<String>` for `as_of_write_cursor` / `as_of_epoch_ms`, reuse `"unsupported_field"`, register `"invalid_as_of"` in BOTH `Failure::decode::CODES` and `Failure::read`, and validate `as_of` in the Graph form before `api()` to prevent cache invalidation on user typos.
6. **Silent `strace` stdout pipe deadlock & unbounded waits (`installed-harness.mjs`, `launch.rs`, `run-installed.sh`, `tests/browser/{attributes,graph,installed,package}.mjs`):** Bound every subprocess, `browser.close()`, and `child` teardown with `SIGKILL` escalation and `/proc` descendant reaping.
7. **Hardcoded `0.2.1` / `37` / `0.8.30` SHA sites:** `src/inspector/Cargo.toml`, `Cargo.lock`, `src/inspector/package/README.md`, `package.sh`, `local-ci.sh`, `local-release.py`, `tests/browser/{package,installed}.mjs`, `tests/integration/admission.rs`, `tests/tools/{test_package,test_local_ci,test_local_release}.py`.
8. **Post-Slice-50 release workflow baseline (`fbb6591fc`):** `upload-artifact-retry` count is `14` (Slice 60 advances to `15`); `build-inspector` requires `needs: [verify-release, verify-cuda-trusted-route]`, `permissions: { contents: read }`, no `!cancelled()`, `cargo fetch --locked`, `grep -Fq` in `package.sh`, and updates across `check-release-contract-truth.py`, `test_release_contract_truth.sh`, `test_release_workflow_scope.sh`, `test_runtime_release_alignment.sh`, and `test_linux_first_platform_scope.sh`.
9. **Local dev test harness note:** `scripts/test-rust-workspace.sh --serial` runs `fathomdb-inspector` default-feature tests inside `agent-test.sh` and `agent-lint.sh` runs `clippy --workspace`, `rustfmt`, and `shellcheck` (`SC2312`) on `src/inspector/tools/*.sh`; `fathomdb-inspector --features cached-cpu` tests, Playwright browser tests, and `src/inspector/tests/tools/` Python unittests run via `local-ci.sh` (or explicit invocation).

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
  1. `src/inspector/tests/browser/attributes.mjs` (wired into `local-ci.sh` / `run-installed.sh` with `installedWorkspace` support) & `installed.mjs`: inspect node and edge outside-`body` `properties`, expand `tree` (`field: "properties"` threaded through `drawTree()`), copy values & full properties to real clipboard asserting exact `9007199254740993123456789` lexeme preservation (proving `"properties"` is copied rather than `"body"`), verify `scope` rendering, verify Schema `37` refusal in `installed.mjs`, and test hostile XSS strings (`<img src=x onerror=alert(1)>`, `<script>`, `"><svg onload=alert(1)>`) in `properties` keys, values, and graph labels trigger zero `pageerror` / alert / DOM injection.
  2. `src/inspector/tests/browser/graph.mjs` & `comparison.mjs`: select `Current` vs `Write cursor` vs `Commit time (ms)` modes in Graph UI, verify invalid input fails locally without evicting observation cache, run both Stored and Native traversal, verify `Back` restoration preserves `as_of` context, verify A/B comparison pins retain distinct `as_of` requests/observations, and extend `D01` 8-flag erasure UI assertions (`comparison.mjs`, `installed.mjs`) to verify zero `properties` or historical erased disclosure leakage.
- **GREEN:** Implement `assets/app.js` (including `drawTree()` `field` threading) and `assets/presentation.mjs` property panels, `as_of` controls, and comparison state propagation.
- **Check:** Playwright unit browser suite (default + `installed.mjs` + `attributes.mjs`).

### Wave 4 (RED -> GREEN): Scoped `local-ci.sh`, Timeouts/Cleanup, & `D4 = B` / `local-release.py`

- **RED tests to add first:**
  1. `src/inspector/tests/tools/test_local_ci.py` (seeding fixture `src/inspector/Cargo.toml`):
     - Default runs `--scoped` (verifying `agent-lint.sh` + `agent-typecheck.sh` + `cargo test -p fathomdb-inspector` default/`cached-cpu` are invoked, no duplicate `unittest` run, and full `scripts/agent-verify.sh` is NOT invoked);
     - `--full-workspace` invokes `scripts/agent-verify.sh`;
     - `--installed-only` preserves installed behavior;
     - Combining any two mode flags exits `2`;
     - `--base <ref>` works cleanly and an unresolvable base ref fails closed;
     - Non-Inspector diff causes `--scoped` to fail closed before stages run and name `--full-workspace`.
  2. `src/inspector/tests/tools/test_local_release.py` & `test_package.py` (seeding fixture `src/inspector/Cargo.toml`):
     - Update compiled schema expectation to `38` and version `0.3.0`;
     - Add static `D4 = B` AST/tree grep test failing if `docker`, `openbox`, `xdotool`, or `sudo` appears outside the explicit negative test mock inside `test_local_ci.py`.
  3. Timeout & cleanup regression tests:
     - `src/inspector/tests/launch.rs`: non-`#[ignore]` unit test verifying `wait_with_sigkill_fallback` kills a `SIGTERM`-ignoring child via `SIGKILL` within a short `500ms` timeout (no 30s test stall).
     - `src/inspector/tests/tools/` shell/Node fixture verifying `run-installed.sh` escalates stalled `Xvfb` to `SIGKILL` within ~2s and `installed-harness.mjs` reaps descendants cleanly without stdout pipe deadlock.
- **GREEN:** Implement `local-ci.sh`, timeout/cleanup fixes across shell/Node/`launch.rs`, dynamic version derivation, `Cargo.toml` + `Cargo.lock` `0.3.0`, `local-release.py`, and `package.sh` (`grep -Fq`).
- **Check:** `python3 -m unittest discover -s src/inspector/tests/tools -v` + `cargo test --locked -p fathomdb-inspector --test launch`.

### Wave 5 (RED -> GREEN, Serialized After Slice 50): `release.yml` `build-inspector` & Workflow Guards

- **RED checks (run before editing `release.yml` or right after adding `build-inspector` before updating guards):**
  - Confirm `scripts/check-release-contract-truth.py`, `scripts/tests/test_release_contract_truth.sh`, `scripts/tests/test_release_workflow_scope.sh`, `scripts/tests/test_runtime_release_alignment.sh`, `scripts/tests/test_linux_first_platform_scope.sh`, and `actionlint` catch unmatched `upload-artifact-retry` count (`14 -> 15`), `all-builds-passed` needs/`if`, or `github-release` wiring.
- **GREEN:** Add `build-inspector` (`needs: [verify-release, verify-cuda-trusted-route]`, `permissions: { contents: read }`, `cargo fetch --locked`, `package.sh`, `check-glibc-floor.sh --floor 2.39`, `local-release.py seal` + `verify --source`, non-colliding `.tar.gz` + `.tar.gz.sha256`) + `github-release` artifact download in `.github/workflows/release.yml`, and update `scripts/check-release-contract-truth.py`, `scripts/tests/test_release_contract_truth.sh`, `scripts/tests/test_release_workflow_scope.sh`, `scripts/tests/test_runtime_release_alignment.sh`, and `scripts/tests/test_linux_first_platform_scope.sh` in lockstep.
- **Check:**
  - `actionlint -config-file .github/actionlint.yaml .github/workflows/*.yml`
  - `python3 scripts/check-release-contract-truth.py`
  - `bash scripts/tests/test_release_contract_truth.sh`
  - `bash scripts/tests/test_release_workflow_scope.sh`
  - `bash scripts/tests/test_runtime_release_alignment.sh`
  - `bash scripts/tests/test_linux_first_platform_scope.sh`

### Wave 6: Engineering Docs, Public Docs & Final Proportional Verification

- Update `dev/inspector/{needs,requirements,ROADMAP,BACKLOG}.md`, `src/inspector/tools/README.md`, `src/inspector/package/README.md`, `docs/inspector/**`, `docs/inspector/releases/0.3.0.md`, `mkdocs.yml`, and `docs/contributing/index.md` (`P5-10`, `P6-11`).
- **Verification commands (run from the target worktree):**
  1. `bash scripts/agent-lint.sh` (includes workspace clippy `-D warnings`, rustfmt, shellcheck `SC2312` over `src/inspector/tools/*.sh`, `check-release-contract-truth.py`, `check-public-doc-truth.py`, `actionlint`, `agent-lint-md.sh`, lychee)
  2. `bash scripts/agent-typecheck.sh`
  3. `bash scripts/agent-lint-docs.sh` and `mkdocs build --strict`
  4. `bash scripts/tests/test_release_contract_truth.sh`, `bash scripts/tests/test_release_workflow_scope.sh`, `bash scripts/tests/test_runtime_release_alignment.sh`, and `bash scripts/tests/test_linux_first_platform_scope.sh`
  5. Inspector end-to-end qualification: `bash src/inspector/tools/local-ci.sh --full-workspace` (or with explicit temp `CANDIDATE FIXTURES BUILD_TARGET`; also verify `local-ci.sh --scoped --base <inspector-only-base>` on Inspector-only diffs)
  6. Note: Release-level paired benchmarks and full unconfined `scripts/agent-verify.sh` (including `AC-036` `strace`) run in Slice 70 (`QUALIFICATION`).
