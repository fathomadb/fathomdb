---
title: Inspector 0.2.1 planning index
date: 2026-11-21
product: FathomDB Inspector
target_release: 0.2.1
status: ACTIVE
desc: Adopted scope, ruled decisions, and four-slice release plan for FathomDB 0.8.30 support
---

# Inspector 0.2.1

Inspector 0.2.1 upgrades the locally released [Inspector 0.2.0](../0.2.0/README.md)
product onto delivered FathomDB 0.8.30 (`release/0.8.30`, `72fe93634`,
byte-identical Rust/Cargo source to `v0.8.30` `e3d31f5cd`) and drops Ubuntu
22.04 / Docker tooling in favor of native Ubuntu 24.04 (`glibc 2.39`) delivery.

## Planning documents

| Path | Purpose | Status |
| --- | --- | --- |
| [scope.md](scope.md) | Adopted scope, ruled owner decisions (D1–D4), out-of-scope boundaries, and release acceptance criteria | ACTIVE |
| [plan.md](plan.md) | Four-slice delivery plan (Slices 10, 20, 30, 40), exit gates, AC traceability, and risk mitigations | ACTIVE |
| [0.2.0 full assessment](../../reviews/assessment-0.2.0.md) | Source- and evidence-grounded assessment of released Inspector 0.2.0 | ACTIVE |
| [0.8.30 compatibility assessment](../../reviews/compatibility-0.8.30-assessment.md) | Source-grounded Engine delta and integration input review | proposed |

## Slice ladder

| Slice | Goal | Status |
| --- | --- | --- |
| Slice 10 | 0.8.30 base integration (`inspector/dev/0.2.1` on `release/0.8.30`), selective workspace wiring, `Cargo.lock` hygiene, `.venv` setup | [COMPLETE](slice-10/status.md) |
| Slice 20 | 0.8.30 retrieval ownership & copy updates, degradations/`Closing`/FTS/ownerless tests, provenance semantics, deferred open & Candle 0.13 confinement | [COMPLETE](slice-20/status.md) |
| Slice 30 | Native Ubuntu 24.04 tooling (remove Docker/Ubuntu 22.04/`openbox`/`xdotool`), `0.2.1` packaging, dual published `0.8.29`/`0.8.30` qualification, local release to `dist/inspector/0.2.1/` | [COMPLETE](slice-30/status.md) |
| Slice 40 | Documentation reconciliation (`docs/inspector/`, `src/inspector/{README,tools/README}.md`, `dev/inspector/`) and docs-only release closure | NEXT |
