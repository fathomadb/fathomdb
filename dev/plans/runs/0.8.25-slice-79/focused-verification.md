---
title: Slice 79 focused verification record
status: COMPLETE
---

# Slice 79 focused verification record

This compact record preserves the commands, outcomes, and test counts observed
during implementation. Interactive command output was not separately retained;
performance raw logs and write receipts are retained beside this record. No
command below is a broad regression run.

## RED and GREEN

| Scope | RED | GREEN |
| --- | --- | --- |
| Rust runtime configuration | Missing API/types at `069f93a3` | 7/7 passed |
| Reader statement reuse | Missing cache behavior at `069f93a3` | 4/4 passed |
| Installed Python runtime control | Missing API/types at `069f93a3` | 3/3 passed |
| Installed Node runtime control | Missing API/types at `069f93a3` | 3/3 passed |
| Governed runtime-control pin | Checker ignored the new member class | Pin recurrence suite passed after `67fd017d`/`6666d8d3` |
| Populated schema-26 witness | 0/2: raw SQLite initialized first | 2/2 passed at `e2db3ffc` |

The schema-26 correction only selects the public performance mode before the
fixture opens raw SQLite. All existing upgrade 26-to-33, reopen, projection,
lifecycle, dependency, and erasure assertions remain unchanged.

## Focused outcomes

| Command or installed-artifact check | Outcome |
| --- | --- |
| `cargo check -p fathomdb-engine -p fathomdb -p fathomdb-py -p fathomdb-napi` | passed |
| `cargo test -p fathomdb-engine --test runtime_configuration -- --test-threads=1` | 7 passed |
| `cargo test -p fathomdb-engine statement_reuse_ --lib -- --test-threads=1` | 4 passed |
| `cargo test -p fathomdb-engine --test slice30_dependency_closure -- --test-threads=1` | 27 passed |
| `cargo test -p fathomdb-engine --test slice35_frozen_read -- --test-threads=1` | 5 passed |
| `cargo test -p fathomdb-engine --features test-hooks,operator --test slice75_schema26_upgrade -- --test-threads=1` | 2 passed |
| `cargo test -p fathomdb --test governed_surface -- --test-threads=1` | 3 passed |
| Fresh installed Python wheel, Slice 79 scenarios | 3 passed |
| Fresh installed Python wheel, public surface | 18 passed |
| Fresh offline Node main plus matched Linux platform packages, Slice 79 scenarios | 3 passed |
| TypeScript public surface | 13 passed |
| Changed-library and exact affected-test Clippy with `-D warnings` | passed |
| Python Ruff and Pyright on changed binding/test files | passed |
| Markdown, plan, release-state, and governed-pin focused gates | passed |

The workspace-wide `--all-targets` Clippy command was not used: an existing
operator-feature mismatch compiles `lifecycle_reliability` without `operator`.
The changed libraries and exact affected tests passed Clippy. No full
regression, hosted CI matrix, or package matrix ran.
