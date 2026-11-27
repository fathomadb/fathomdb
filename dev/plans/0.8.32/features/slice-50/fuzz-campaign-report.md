# Slice 50 (`WP-50C`) Coverage-Guided Fuzzing Campaign Report (`AC32-FUZZ-01`–`03`)

**Date:** 2026-11-26
**Release / Slice:** `0.8.32` / Slice 50 (`WP-50C`)
**Authoritative Contracts:** `dev/plans/0.8.32/features/slice-50/plan.md` (`AC32-FUZZ-01`–`03`), `dev/adr/ADR-0.8.32-durability-sync-mode-and-power-loss-oracle.md` (`#101`, §4)

## 1. Toolchain, Isolation & Environment

- **Root workspace toolchain (`rust-toolchain.toml`):** pinned unchanged at `1.95.0` (`x86_64-unknown-linux-gnu`).
- **Fuzzing toolchain:** `nightly-2026-08-01-x86_64-unknown-linux-gnu` with `llvm-tools` (`llvm-profdata`, `llvm-cov`), `cargo-fuzz 0.13.2`, `libfuzzer-sys 0.4.13`, `arbitrary 1.4.2` (resolved `1.5.0` offline), GCC `13.3.0` (`g++-13` + `libubsan`).
- **Offline build verification:** Every build and campaign run executed with `CARGO_NET_OFFLINE=true`.
- **Workspace & SBOM isolation (`AC32-FUZZ-01`):**
  - Root `Cargo.toml` excludes `fuzz/` via `exclude = ["fuzz"]`.
  - Standalone `fuzz/Cargo.toml` declares `[workspace]`, `publish = false`, `[package.metadata] cargo-fuzz = true`, and **zero** `[patch]`, `[replace]`, or `git =` overrides.
  - `scripts/sbom-survey/tiers.toml` classifies `prefix = "fuzz/"` as `tier = "dev-tooling"`.
  - `.gitignore` excludes `fuzz/target/`, `fuzz/artifacts/`, `fuzz/coverage/`, `fuzz/Cargo.lock`, `*.profraw`, and `*.profdata`, while tracking `fuzz/Cargo.toml`, `fuzz/fuzz_targets/*.rs`, and 23 small curated adversarial seeds under `fuzz/corpus/<target>/`.
  - Verified deterministically by `ac32_fuzz_01_shipping_crates_have_zero_fuzz_dependencies` (`cargo tree --locked --offline` across all 11 shipping crates contains zero `libfuzzer-sys` or `arbitrary` dependencies) and `scripts/check-sdk-surface-parity.py`.

## 2. AddressSanitizer (`ASan`) Campaign (`>= 60 s` per Target)

Invocation (offline, `max_len = 8192`, `max_total_time = 60` per target, seeded from `fuzz/corpus/<target>/`):

```bash
CARGO_NET_OFFLINE=true cargo +nightly-2026-08-01 fuzz run --sanitizer address <target> \
  /tmp/fathomdb-fuzz-campaign/corpus/<target> fuzz/corpus/<target> -- \
  -max_total_time=60 -max_len=8192 -print_final_stats=1
```

Targets exercising `vec0` (`sqlite-vec = "=0.1.9"`) ran with `ASAN_OPTIONS=detect_leaks=0:detect_odr_violation=0` and `-detect_leaks=0` so AddressSanitizer memory-safety checks remained 100% active without halting on the upstream `sqlite-vec 0.1.9` `vec0_init` leak documented in §5. `fuzz_mcp_jsonrpc` ran with **both** `ASan` and `LSan` (`-detect_leaks=1`) enabled.

| Target | Duration | Executed Units | Avg Exec/s | Edge `cov` | Feature `ft` | Final Corpus | Peak RSS | Findings |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `fuzz_query_filter` | 61 s | 126,359 | 2,071 | 9,663 | 18,790 | 1,126 / 42 KB | 700 MB | 0 panics / 0 ASan |
| `fuzz_property_predicates` | 61 s | 62,339 | 1,021 | 9,045 | 17,014 | 656 / 13 KB | 685 MB | 0 panics / 0 ASan |
| `fuzz_scope_policy` | 61 s | 2,023 | 33 | 7,733 | 10,819 | 242 / 5.7 KB | 570 MB | 0 panics / 0 ASan |
| `fuzz_auth_tokens` | 61 s | 88,095 | 1,444 | 8,804 | 14,842 | 711 / 25 KB | 805 MB | 0 panics / 0 ASan |
| `fuzz_wire_codecs` | 61 s | 347,762 | 5,701 | 11,082 | 21,612 | 2,176 / 308 KB | 792 MB | 0 panics / 0 ASan |
| `fuzz_mcp_jsonrpc` | 61 s | 620,079 | 10,165 | 3,931 | 9,429 | 1,673 / 115 KB | 496 MB | 0 panics / 0 ASan / 0 LSan |
| `fuzz_export_and_open_recovery` | 61 s | 2,266 | 37 | 8,059 | 12,574 | 194 / 6.7 KB | 650 MB | 0 panics / 0 ASan |
| **Total (`ASan`)** | **427 s** | **1,248,923** | — | — | — | **6,778 units** | — | **0 FathomDB defects** |

## 3. UndefinedBehaviorSanitizer (`UBSan`) & Rust UB-Checks Sweep

`rustc` on `nightly-2026-08-01` does not provide `-Zsanitizer=undefined` (`error: incorrect value 'undefined' for unstable option 'sanitizer'`). Instead, Rust runtime UB checks were enabled via `RUSTFLAGS="-Coverflow-checks=on -Zub-checks=on -Clink-arg=-lubsan"` (`-Cdebug-assertions` also enabled by `cargo-fuzz`), paired with GCC `13.3.0` C/C++ UndefinedBehaviorSanitizer (`CFLAGS="-fsanitize=undefined -fno-sanitize-recover=all"`, `CXXFLAGS="-fsanitize=undefined -fno-sanitize-recover=all"`) instrumenting bundled `sqlite3.c`, `sqlite-vec.c`, and the `libFuzzer` C++ runtime, with `UBSAN_OPTIONS="print_stacktrace=1:halt_on_error=1"`.

All 7 targets built and replayed their full `ASan`-discovered corpora plus 15 s of additional mutation per target:

| Target | Duration | Executed Units | Avg Exec/s | Edge `cov` | Feature `ft` | Findings |
| --- | --- | --- | --- | --- | --- | --- |
| `fuzz_query_filter` | 16 s | 57,040 | 3,565 | 6,174 | 12,164 | 0 UBSan / 0 Rust UB-check |
| `fuzz_property_predicates` | 16 s | 35,796 | 2,237 | 5,849 | 11,713 | 0 UBSan / 0 Rust UB-check |
| `fuzz_scope_policy` | 16 s | 1,167 | 72 | 4,726 | 6,552 | 0 UBSan / 0 Rust UB-check |
| `fuzz_auth_tokens` | 16 s | 51,411 | 3,213 | 5,416 | 8,863 | 0 UBSan / 0 Rust UB-check |
| `fuzz_wire_codecs` | 16 s | 235,065 | 14,691 | 7,923 | 15,601 | 0 UBSan / 0 Rust UB-check |
| `fuzz_mcp_jsonrpc` | 16 s | 554,744 | 34,671 | 2,874 | 6,685 | 0 UBSan / 0 Rust UB-check |
| `fuzz_export_and_open_recovery` | 16 s | 1,254 | 78 | 4,984 | 7,960 | 0 UBSan / 0 Rust UB-check |
| **Total (`UBSan` + `-Zub-checks=on`)** | **112 s** | **936,477** | — | — | — | **0 defects** |

**Combined campaign executions (`ASan` + `UBSan`):** **2,185,400** executions across all 7 targets.

## 4. `llvm-cov` Coverage Summary (`nightly-2026-08-01` `llvm-tools`)

Coverage profiles were generated by replaying each target's campaign corpus + curated seeds with `cargo +nightly-2026-08-01 fuzz coverage <target>`, merged via `llvm-profdata merge -sparse`, and reported with `llvm-cov report --ignore-filename-regex='\.cargo/registry|/rustc/|fuzz/fuzz_targets'`.

### Per-Target Workspace Coverage

| Target | Regions Covered | Functions Executed | Lines Covered |
| --- | --- | --- | --- |
| `fuzz_query_filter` | 13.15% (10,528 / 80,062) | 15.22% (705 / 4,631) | 14.18% (7,425 / 52,373) |
| `fuzz_property_predicates` | 12.63% (10,109 / 80,062) | 15.42% (714 / 4,631) | 13.69% (7,172 / 52,373) |
| `fuzz_scope_policy` | 8.59% (7,829 / 91,163) | 9.94% (536 / 5,394) | 9.13% (5,431 / 59,507) |
| `fuzz_auth_tokens` | 11.70% (9,369 / 80,062) | 13.82% (640 / 4,631) | 12.35% (6,469 / 52,373) |
| `fuzz_wire_codecs` | 11.17% (8,940 / 80,062) | 13.54% (627 / 4,631) | 12.11% (6,343 / 52,373) |
| `fuzz_mcp_jsonrpc` | 1.73% (1,579 / 91,163) | 1.93% (104 / 5,394) | 1.77% (1,055 / 59,507) |
| `fuzz_export_and_open_recovery` | 7.52% (6,023 / 80,062) | 9.35% (433 / 4,631) | 8.08% (4,230 / 52,373) |
| **Merged 7-Target Union** | **21.44%** (19,547 / 91,163) | **23.21%** (1,252 / 5,394) | **23.18%** (13,794 / 59,507) |

### Merged Coverage on Directly Targeted Modules

| Source Module | Region Cover | Function Cover | Line Cover |
| --- | --- | --- | --- |
| `fathomdb-query/src/lib.rs` | 86.72% | 86.36% | 82.93% |
| `fathomdb-mcp/src/names.rs` | 95.83% | 100.00% | 93.33% |
| `fathomdb-engine/src/frozen_read.rs` | 69.49% | 67.46% | 72.32% |
| `fathomdb-engine/src/scope.rs` | 67.57% | 70.00% | 69.82% |
| `fathomdb-mcp/src/config.rs` | 68.57% | 59.57% | 66.32% |
| `fathomdb-engine/src/properties.rs` | 61.16% | 66.67% | 67.07% |
| `fathomdb-engine/src/open.rs` | 62.35% | 37.06% | 54.29% |
| `fathomdb-engine/src/write_validation.rs` | 48.40% | 57.89% | 52.03% |
| `fathomdb-mcp/src/actuate.rs` | 43.02% | 52.54% | 47.98% |
| `fathomdb-engine/src/actuation.rs` | 37.94% | 47.66% | 40.70% |
| `fathomdb-engine/src/filter.rs` | 33.70% | 35.38% | 38.89% |
| `fathomdb-engine/src/logical_export.rs` | 31.59% | 20.00% | 37.83% |
| `fathomdb-engine/src/pagination.rs` | 33.77% | 30.43% | 33.33% |
| `fathomdb-engine/src/evidence/codec.rs` | 23.94% | 20.51% | 24.64% |
| `fathomdb-engine/src/context_pack.rs` | 19.61% | 28.57% | 18.72% |
| `fathomdb-engine/src/evidence/graph.rs` | 6.35% | 10.29% | 5.80% |

## 5. Findings, Triage & Resolutions

1. **Zero FathomDB crashes, panics, memory-safety errors, or UB defects uncovered:** Across `2,185,400` total executions (`ASan` + `UBSan` + `-Zub-checks=on`), all FathomDB Rust parsers, validators, SQL lowering paths, MAC/digest auth codecs (including `0x53 0x39` trailing-tag parsing), `fathomdb.logical-export/v1` archive decoders/restorers, `fathomdb-mcp` config/tool parsers, and `Engine::open` corruption recovery paths failed closed cleanly with typed errors.
2. **Upstream third-party C leak observed under LeakSanitizer (`sqlite-vec = "=0.1.9"`):** During initial `ASan` smoke runs, `LSan` flagged a deterministic 280-byte leak (7 allocations × 40 bytes per `CREATE VIRTUAL TABLE` / `xConnect` of `vec0`) symbolized via `addr2line` to `vec0_init` at `sqlite-vec-0.1.9/sqlite-vec.c:5045` (`sqlite3_mprintf` inside `sqlite3VtabCallCreate` / `sqlite3VtabCallConnect`). Zero allocations leaked in FathomDB Rust code. This is a pinned upstream `sqlite-vec` C extension allocation behavior, recorded honestly here for future `sqlite-vec` upgrade tracking; `ASan` memory-safety checks ran at 100% on all 7 targets with `-detect_leaks=0` on `vec0`-opening targets and full `-detect_leaks=1` on `fuzz_mcp_jsonrpc`.
3. **Fuzz harness defect caught and fixed during `fuzz_mcp_jsonrpc` run:** At 156,369 runs (`17 s`), `fuzz_mcp_jsonrpc` panicked on a harness assertion when `op_name == "put_latest_state"` cleared an earlier inserted `forbidden_key` before asserting refusal. Fixed the harness to preserve `forbidden_key` on `put_latest_state`, pinned the case in `fuzz_regression_mcp_tool_args_anti_spoof_and_forbidden_tools`, and re-ran `fuzz_mcp_jsonrpc` for a full `61 s` (`620,079` runs, `ASan + LSan`) with zero findings.

## 6. Committed Stable Deterministic Regressions (`AC32-FUZZ-03`)

All deterministic regressions execute under stable `1.95.0` `cargo test` with **zero** `libfuzzer-sys` or `arbitrary` linkage:

- **`src/rust/crates/fathomdb-engine/tests/slice50_fuzz_regressions.rs`** (`required-features = ["operator", "test-hooks"]`):
  - `ac32_fuzz_01_shipping_crates_have_zero_fuzz_dependencies`
  - `fuzz_regression_query_filter_fts5_injection_and_stopword_bounds`
  - `fuzz_regression_property_predicates_numeric_and_projection_edges`
  - `fuzz_regression_scope_policy_grammar_and_sql_lowering`
  - `fuzz_regression_auth_tokens_malformed_and_forged_s9_trailers`
  - `fuzz_regression_wire_codecs_archives_writes_and_actuation`
  - `fuzz_regression_export_restore_and_corrupt_open_recovery`
- **`src/rust/crates/fathomdb-mcp/tests/slice50_fuzz_regressions.rs`** (`test-seams` self dev-dep):
  - `fuzz_regression_mcp_config_malformed_and_sync_mode_conflicts`
  - `fuzz_regression_mcp_tool_args_anti_spoof_and_forbidden_tools`
- **`src/rust/crates/fathomdb-mcp/src/seams.rs`** (`#[cfg(test)]`):
  - `parse_tool_arguments_enforces_strict_schema_and_forbidden_tools`
