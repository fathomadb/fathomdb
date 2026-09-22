---
title: FathomDB 0.8.27 prework Slice 1 - dependency and pinning sweep
status: COMPLETE
target_release: 0.8.27
observed_on: 2026-09-21
---

# Slice 1 - dependency and pinning sweep

## Plan and delta reconciliation

Outcome: record current advisory, manifest, lock, source-pin, toolchain, and
Action evidence without changing dependencies, locks, installed environments,
or workflows. The draft is adjusted for failed live npm/GitHub evidence: those
routes fail closed instead of being represented as current.

## Requirements and acceptance

| ID | Requirement | Acceptance signal |
| --- | --- | --- |
| PW27-1A | Every shipped manifest class and protected pin has a disposition and compatibility rationale. | Register below covers Rust, Python, root/TS/Mermaid npm, tools, and Actions, with unavailable evidence explicit. |
| PW27-1B | Security-driven work is narrow and testable. | Accepted updates name dependency path, driver, blast radius, and focused proof; coupled native stacks are not split. |
| PW27-1C | No dependency or environment mutation occurs in prework. | Git diff contains planning records only. |

## Dependency and pin register

Update in Slice 10 if independently approved:

- Root lock contains `smol-toml 1.7.0`; `GHSA-7w5x-hrqm-74c2` affects versions
  through 1.7.0 and is patched in 1.7.1. Exposure is development/config
  tooling, not shipped runtime. Update narrowly to >=1.7.1 and prove markdown
  AST neutrality, lint, and tooling self-tests.
- Two `upload-artifact` SHA comments in `.github/workflows/ci.yml` identify the
  v7.0.1 pin as v4.6.2. Correct comments only; keep the SHA/runtime unchanged.

Investigate or postpone:

- `paste 1.0.15` has unmaintained advisory `RUSTSEC-2024-0436` and no patched
  version. It is transitive through coupled GEMM/tokenizer stacks; postpone to
  a deliberate Candle/native-stack migration.
- Ruff 0.15.17 versus observed 0.16.8, Pyright 1.1.410 versus 1.1.414,
  `@types/node` 26.1.0 versus 26.3.0, and Mermaid CLI 11.16.0 versus 11.17.0
  have no 0.8.27 product driver. Compare exact diagnostics/render fixtures
  before any later patch update. DOMPurify 3.4.11 is beyond the previously
  relevant <=3.4.10 range.

Retain:

- TypeScript 6.0.3; a 7.x move is a separate major migration.
- `@napi-rs/cli` 2.18.4 and the Rust NAPI 2 stack/artifact topology.
- Candle fork rev `cf02edbc...`, ORT `=2.0.0-rc.10`, sqlite-vec `=0.1.9`,
  rusqlite 0.40, PyO3 0.29, Rust 1.95, abi3-py310, and full-SHA Actions absent
  a dedicated compatibility/provider/ABI design.
- checkout v7.0.0, upload-artifact v7.0.1, and the intentional mix of
  download-artifact v8.0.1 and retained v4.3.0 routes. A newer tag alone is not
  a release driver.

Evidence limitations:

- root, TypeScript, and Mermaid npm audits failed DNS lookup of
  `registry.npmjs.org` and are not green;
- GitHub CLI authentication is invalid, so Dependabot/security alerts are
  unavailable; all five configured Dependabot ecosystems also set
  `open-pull-requests-limit: 0`, making no-open-PR evidence non-probative;
- Mermaid tooling is outside current Dependabot coverage;
- offline Rust audit reported 1,239 advisory records, 440 packages, no known
  vulnerability, and the unmaintained `paste` warning, but advisory freshness
  could not be established; and
- the complete feature graph for `paste` could not be regenerated because the
  sandboxed Cargo route attempted a read-only cache write.

## Implementation, review, verification, and status

Implementation is this read-only register. TDD and code review are not
applicable. `cargo metadata --locked --offline --no-deps` passed. Live routes
that could not produce evidence are recorded as unavailable. Independent
package design review and closeout verification passed.

Status is `COMPLETE`. No manifest, lock, workflow, cache, or environment
changed. Next: Slice 2 cruft review.
