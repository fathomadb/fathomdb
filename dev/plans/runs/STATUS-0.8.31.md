# FathomDB 0.8.31 release status

**CLOSED — historical record. COMPLETE — `v0.8.31` published 2026-11-23.**
Active release work has moved to [STATUS-0.8.32.md](./STATUS-0.8.32.md).
The [state](../release-state-0.8.31.json) owns structured status and delivery
receipts. The [master plan](../0.8.31/plan.md) and [scope](../0.8.31/scope.md)
own settled HITL decisions, requirements, and execution contracts.

## Current state

**Release `0.8.31` is COMPLETE and PUBLISHED (`v0.8.31`, tag object `48a224e5ac691d5b88a7979b920e87352d31e87f` → peeled commit `8965af93d6f42d160474acb382596c5aab6b5c2d`).** All slices (`0, 10, 20, 30, 40, 50, 60, 70, 80`) are landed on `main`; remaining ladder = none.

- **Slice 0 (`PREFLIGHT`) & Phase A Prework Audits (`Slices 1–9`) — LANDED (`bf8e905e3`, `42595212c`):**
  - Merged completed `origin/inspector/dev/0.2.1` (`d6501cc5a`) cleanly at `086d5af0a`.
  - Landed RED→GREEN worktree Python/TS bootstrap fix (`20269cee4` → `db271323f`) and bootstrapped owned `.venv`, `node_modules`, and `src/ts/node_modules`.
  - Authored ADRs `#73`–`#76`, W12 fence review, `v0.8.30` baseline latency measurements, and closed golden schema-37 DB.
  - Adopted `RELEASE-AND-SLICE-PLANNING-TEMPLATE.md` (`6ee16b9e8`), remediated all Slice 0 review findings (`bf8e905e3`), completed Phase A Prework Audits (`Slices 1–8`), and obtained **HITL Gate 1 approval** (`2026-11-22`) on `prework/slice-9-triage.md`:
    - `P4-09`: add `_fathomdb_commit_timestamps(write_cursor, committed_at_ms)` in Schema 38.
    - Renumbered ladder to `Slice 10 = Prework Prep`, feature slices `20–80` (`80 = Publication`, held for separate HITL approval).
    - Pre-authorized `governed-surface-pin.json` updates for `read.operational_collections`/`read.operational_collection` (Slice 20) and `resolve_evidence_many` (Slice 50).
    - Recorded strict project-boundary rule (no Memex-specific information written into tracked FathomDB repo/public files).
- **Slice 10 (`PREWORK-PREP`) — LANDED (`3c4b84439`):**
  - Completed `WP-10A`–`WP-10D` (ADR `#73`/`#75`/`#76` & `ADR-0.8.28` amendments + project-boundary scrub; `markdownlint-cli2 ^0.24.2` & `rustix 1.1.6` lockfile hygiene; pre-publish `0.8.31` public-doc truth gate flip + `P6-01`/`P6-07`/`P6-11` fixes; `P7-03` security timeouts, `P7-07` PEP 691 + CDN-skew retry, `P7-08` CHANGELOG GitHub Release body, `P7-09` Bash 3 sentinel + Windows Slice 55 smoke, `P7-10` dynamic tag route + audited 171-record Gitleaks manifest; all wired in `scripts/agent-test.sh`).
  - Independent `pro` adversarial code review returned `APPROVE` after `FIX-1`; recorded in `features/slice-10/status.md`.
- **Phase C Draft Plans (`Slices 20–70`) — AUTHORED:** Draft plans written for Slices `20`, `30`, `40`, `50`, `60`, and `70`.
- **Slice 20 (`PROPERTY-SCHEMA`) — LANDED (`975637f3c`):**
  - Landed Layers 1–5 (`022b62b49`, `3da57de2e`, `644333afb`, `947398398`, `36802ba4e`, `9ee29cd3e`) plus `FIX-1` (`1c61aea20`) and `FIX-2` (`975637f3c`): Schema 38 (`STEP_38_SQL`), inline typed properties + O(1) hydration, `canonical_property_values` with 4 composite indexes and indexed `INNER JOIN` vector pre-selection, `vec0` 8-slot node budget, `_fathomdb_commit_timestamps`, explicit opt-in `R1` vector kinds, `read.operational_collections`/`collection` (`55/55` governed surface pinned at `36802ba4e`), W12 erasure fixes 3a/3b/3c, `ExportClass` audit, Rust/Python/TypeScript SDK parity, and updated interface docs.
  - Independent adversarial code review returned **`APPROVE`** at HEAD `975637f3c`; recorded in `features/slice-20/status.md`.
- **Slice 30 (`STRICT-GRAPH`) — LANDED (`8e44a47ad`):**
  - Landed Layers 1–4 (`0a43d66b7`, `3f8dbc470`, `87cd6a633`, `eda733775`) plus `FIX-1` (`8e44a47ad`) after moving the worktree to `/mnt/data/worktrees/release-0.8.31`: strict write endpoint integrity + 32 convenience-edge test & 3 Inspector forensic fixture updates, `GraphExpandRequestV1` `#[non_exhaustive]` + `::new(...)` + closed typed predicates + current-time SQL pushdown (`UNION ALL`, `t_valid`, `proptest` equivalence), `search.rs` `SEED_FTS_N` mixed-edge fix + Phase 2 `UNION ALL` + opt-in `graph_seed` (`GraphFts`/`FusedTopK`/`Both`) + single-pass `CeScoreCache` + `PerHitExplain`/`EvidenceGraphOriginV1` provenance + `#[non_exhaustive]` `EvidenceSearchRequestV1::new(...)`, Rust/Python/TypeScript parity (1,789 Python / 653 TS / all Rust crates), and updated interface/public docs.
  - Independent adversarial code review returned **`APPROVE`** at HEAD `8e44a47ad`; recorded in `features/slice-30/status.md`.
- **Slice 40 (`HISTORY-GRAPH`) — LANDED (`fe5e7d8a1`):**
  - Landed Layers 1–3 (`039bc7f88`, `e00a3b359`, `9f362ed1f`) plus `search.rs` `EntityFts` fix (`9ca376e57`) and `FIX-1` (`fe5e7d8a1`): bitemporal `graph_expand` (`as_of_write_cursor` / `as_of_epoch_ms`), `EdgeViewBoundary::AsOf` (`UNION ALL`, derived-edge closure barriers, zero-`SCAN` query plans), two-arm historical `load_node` + `Engine::transition` current-head check, inline `(properties > body)` historical eligibility, `proptest` snapshot BFS oracle, historical frozen graph evidence (`payload[7]`, `artifact_lifecycle.superseded`, erasure/transition/barrier revocation), Rust/Python/TypeScript parity (1,794 Python / 657 TS / all 13 Rust crates), and updated interface/guide/reference docs.
  - Independent adversarial code review returned **`APPROVE`** at HEAD `fe5e7d8a1`; recorded in `features/slice-40/status.md`.
- **Slice 50 (`SDK-CLI-UPGRADE`) — LANDED (`28f5def55`):**
  - Landed Layers 1–3 (`09dbec3bd`, `1b07584e1`, `9780e9181`) plus governed-surface commit pin (`fbb6591fc`, `56` live ops / `93` allowlist) and `FIX-1` (`28f5def55`): `resolve_evidence_many` per-ref partial results, opt-in `SearchResult.hints`, zero-I/O `OpenReport` capability booleans preserving forced-CUDA fail-closed open, `fathomdb migrate` `37 -> 38` (with fail-closed dangling-edge check, in-transaction `cpv` backfill + Schema-38 declaration rehash proven against golden Schema-37 DB), `fathomdb-cli` PyPI wheel + `fathomdb[cli]` extra + `build-python-cli` in `release.yml`, Rust/Python/TypeScript parity (1,800 Python / 662 TS / all 13 Rust crates), module-boundary cleanup, and updated public/interface/generated docs.
  - Independent adversarial code review returned **`APPROVE`** at HEAD `28f5def55`; recorded in `features/slice-50/status.md`.
- **Slice 60 (`INSPECTOR`) — LANDED (`6685b0ccf`):**
  - Landed Layers 1–3 (`35543da54`, `e86100641`, `8689b781d`) plus `FIX-1` (`6685b0ccf`): Inspector `0.3.0` Schema 38 strict admission, erasure-gated outside-`body` `properties` & reserved `scope`, `"properties"` memory admission, bounded `catalog`/`associations`, historical `as_of` graph + `P4-13` `INDEXED BY` lift + O(log N) two-branch `UNION ALL`, `.textContent`-only XSS-safe UI with `drawTree()` `field` threading + `i64` precision + `seedExpanded` preservation, `P7-02` bounded timeouts & `/proc` reaping across all harnesses, scoped `local-ci.sh`, `D4 = B` `local-release.py`, `build-inspector` in `.github/workflows/release.yml` (`upload-artifact-retry` `15`), all 4 workflow contract suites, and Inspector `0.3.0` docs.
  - Independent adversarial code review returned **`APPROVE`** at HEAD `6685b0ccf`; recorded in `features/slice-60/status.md`.
- **Slice 70 (`QUALIFICATION`) — LANDED (`dda61cfae`, frozen product code `f13244689`):**
  - Completed `WP-70A`–`WP-70G` + `FIX-1`: unconfined `agent-verify.sh` (`243/246` suites, `0` failed, `agent-security` `0` violations), all 7 canonical `AGENT_LONG=1` & `--release` perf gates, `WP-70B` `warmup_rounds=5` paired `0.8.30` vs. `0.8.31` `10k`/`50k`/`100k` latency/resource gate + `50k` micro-gates (`passed: true`, `0` parity mismatches), `WP-70C` live RTX 3090 feature-complete matrix (`223` passed, `0` failed, `27` allowlisted ignored), `WP-70D` Inspector `0.3.0` `--full-workspace` (`86/86` Playwright tests) + 4-arg network silence, `WP-70E` clean-venv `fathomdb` + `fathomdb-cli` wheels & stripped-`PATH` `37 -> 38` upgrade proof, `WP-70F` release docs, and `WP-70G` golden Schema-38 archive (`sha256: e0a078bd...`). Independent adversarial review Round 2 returned **`APPROVE`**.
- **Slice 80 (`PUBLICATION`) — LANDED (`64e21c3c5`, `v0.8.31`):**
  - Published across crates.io (`T1–T7`, Axis-W `0.8.31`, Axis-E `0.8.0`), PyPI (`fathomdb==0.8.31` and `fathomdb-cli==0.8.31`), npm (`fathomdb@0.8.31` on `latest` + 5 platform packages), GitHub Release `v0.8.31` (12 production assets incl. Inspector `0.3.0`), and Jetson Orin Tegra Pages (run `45325322000` 7/7 pass, 11 retained artifacts). Original release run `45318429916` passed all builds, all publishers, `co-tagging-assert`, and 7/8 smokes; sole failure `post-publish-smoke (crates-sdk)` (smoke fixture `registry-sdk.rs` missing Schema-38 `properties: None`) repaired at `64e21c3c5` and verified in finalization run `45326999920` + live local PyPI/crates.io consumer proofs. Owned GPU0 runner `170` retired and Orin listener `22` stopped cleanly. Full receipts in [features/slice-80/status.md](../0.8.31/features/slice-80/status.md).

## Next action

None — `0.8.31` is complete and published. Next release planning begins with `0.8.32` upon owner direction.
