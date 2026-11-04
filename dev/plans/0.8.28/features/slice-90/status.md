---
title: FathomDB 0.8.28 Slice 90 — status
status: COMPLETE
---

# Slice 90 status

D28-14 implemented on `dev/0.8.28` at
`14d710093f1b3e061d4625c0a62665a6ff899372`. Production source is unchanged
from reviewed `9b6fc8b6f36c0b2e0afc383c10583319521c7bb2`; the successor
adds only the explicitly approved signature fixture delta. [Plan and design](plan.md) reconcile the draft and intervening
work, define local acceptance S90-1–5, and record Astra design approval.

- Direct body FTS accepts SearchFilter in Rust core/SDK, Python and TypeScript.
  Existing positional calls remain valid. Predicates execute in the reader
  snapshot before candidate bounds, deduplication, ranking and limit.
- Provider-free kind/source/attribute filtering works. Status and created_after
  retain G10 semantics: existing projected empty-status/projection-time metadata,
  inclusive time bound, missing metadata fails to match. No new JSON metadata
  population, inference, vector retrieval or schema change.
- Astra code review APPROVE at the production source commit above. Both P3 follow-ups are
  resolved: nonempty-filter mixed node/edge duplicate prefix coverage and the
  earlier Python signature. Independent Astra verification APPROVE.
- Observed RED then GREEN for core, SDK and language binding forwarding; commits
  record missing-API/type/keyword failures. One metadata test fixture was
  corrected to a single token because existing text compilation uses OR.

## Verification

- Independent final Slice 90 engine suite: 5/5; preceding regression checks:
  Slice 23 prefix 2/2, Slice 40 filters 7/7, Slice 80 property lifecycle 6/6
  and view paths 2/2. No ignored tests. Additional default-route compilation
  passes. Real provider-free and Noop-projected SQLite fixtures cover S90-1–4.
- Rust SDK forwarding 1/1, existing behavior 25/25 and surface 15/15 pass.
- Fresh correctly packaged Python abi3 wheel, installed outside the source tree:
  Slice 90, retrieval limits and filter-unification tests 14/14 pass without
  skips. This run uses installed wrappers/native code, not the older extension
  used solely to observe RED.
- Fresh Node addon and Node 25.9.0: Slice 90, retrieval limits and filter
  unification suites 3/3 pass without skips. A disposable wrong-expected-ID
  control fails, confirming the native assertions execute. TypeScript typecheck
  passes.
- Changed Rust crate library clippy (`-D warnings`), Python Ruff, Markdown,
  module-boundary and SDK surface parity pass. Governed surface remains
  87 spellings / 53 operations.
- Final canonical gate at `a2d942949e29e2e29e487c2bf93662682e0f3476`:
  lint, typecheck, strict security, Rust workspace, TypeScript, native receipt
  and all other suites pass. Collection: 205 ran, 204 passed, one failed,
  zero skipped/excluded suites; Python had 1,711 passed, 36 skipped and one
  failure. The sole failure was the frozen package-boundary fixture expecting
  the pre-D28-14 signature. The exact approved alias/stub deltas were added
  without changing the frozen baseline or checksum; Astra approved and all
  three affected fixture tests plus Ruff pass at `14d710093`. No full rerun:
  this is assembled verification with a narrow fixture recheck, not a single
  uninterrupted full-gate pass. Temporary-worktree Pyright lacked that
  checkout's virtual environment; canonical full-gate typecheck passed.

S90-5 is satisfied by native binding execution, validation/type/surface checks
and updated interface contracts. Slice completion does not claim the separate
0.8.28 release platform/performance/registry qualification or CI is green.
Publication still requires the owner's separate ruling. No GPU/paid benchmark
or hosted CI run was needed for this lexical-only change.
