---
title: FathomDB 0.8.27 Slice 120 - TypeScript SDK decomposition plan
status: APPROVED_FOR_DESIGN_REVIEW
target_release: 0.8.27
---

# Slice 120 - TypeScript SDK decomposition

## Draft reconciliation

| Change since the release draft | Disposition |
| --- | --- |
| Slices 30 and 110 now supply an immutable public-surface baseline, a complete native declaration, installed Node receipts, and a reviewed subscriber contract. Slice 110 closed its native conversion, callback, error, loader, and packaging obligations. | Consume those authorities. Do not reopen native work or update the baseline. |
| Slice 90 added runtime configuration and its frozen requested-value snapshot; Slice 110 added subscriber delivery and final native signatures. | Retain both exact behaviors while moving their TypeScript owners. |
| `index.ts` is now 4,496 lines. Existing `binding.ts`, `errors.ts`, `evidence-validation.ts`, `platform.ts`, `read.ts`, and `validation.ts` already own their respective seams. `package.json` has only the package-root entrypoint and no supported subpath export. | Move the remaining root-owned domain code into private modules; keep existing modules. Consumer fixtures exercise the root and the actual supported entrypoint set, without inventing subpaths. |
| The draft names read, write, search, graph, evidence, projection, and admin domains. The current root also owns open/configuration, embedding/reranking, lifecycle, dependency closure and tracing, mapping, and instrumentation. | Assign every current symbol to an owner in the design. Keep small wiring and lifecycle methods on `Engine`; avoid a second public facade. |
| Slice 130 owns Python decomposition, Slice 132 owns Rust SDK parity, Slice 140 owns final documentation/exception convergence, and Slice 150 owns integrated qualification. | Do not absorb those slices. Update only TypeScript source and directly affected TypeScript documentation. |

The draft is **approved with these adjustments**. No new public capability, schema,
wire shape, package subpath, or native declaration is in scope.

## Needs, requirements and acceptance

| ID | Need and requirement | Acceptance |
| --- | --- | --- |
| R27-120A | Maintainers can find each TypeScript SDK concern in a named domain without following a monolithic root file. | AC27-120A: `index.ts` is a package-root export map; `Engine` retains thin public wiring and lifecycle ownership; all other former root-owned logic has a named domain owner. No source-layout assertion becomes a product test. |
| R27-120B | Existing consumers see the same public types, exports, signatures, errors, validation precedence, wire data, and native delegation. | AC27-120B: TypeScript declaration/package/runtime rows compare exactly with a captured pre-move Slice 110-current surface; differences from the older immutable Slice 30 baseline are limited to separately named, accepted later slices. Root consumer compile and runtime fixtures, shared validation/error tests, and focused TypeScript suites pass. |
| R27-120C | The SDK remains backed by Slice 110's complete native binding. | AC27-120C: native declarations and NAPI source do not change; fresh installed consumer checks load the packaged runtime and exercise representative write/read/search/graph/evidence/admin operations. |
| R27-120D | Structural movement is reviewable and verified at the current release tip. | AC27-120D: the pre-move baseline is recorded; moved behavior is unchanged; a gpt-6-sol high code review and independent verification review the actual candidate; required source-change gates pass. |

`dev/acceptance.md` stays locked. These IDs are slice-local. The public
TypeScript interface needs no contract change because the package API is held
constant; this design records the new internal ownership.

## RED / GREEN / REFACTOR implementation

1. Capture the current compile, focused behavior, declaration/runtime export,
   and package-root entrypoint results. Add consumer compile and runtime
   characterization fixtures at the package boundary; demonstrate non-vacuity
   with a controlled export/delegation defect, then revert it. The retained
   fixture is the RED guard for a broken move.
2. Move cohesive top-level declarations, validators and mappers into private
   domain modules in reviewable batches. After each batch, compile and run the
   owning tests (GREEN). Do not rewrite behavioral assertions to accommodate
   the move.
3. Move complex `Engine` method bodies behind domain functions. Preserve public
   method signatures, overloads, call ordering and synchronous versus async
   error behavior. Keep the class as thin wiring; refactor only after GREEN.
4. Compare the emitted declaration/runtime/package rows to the exact pre-move
   capture and reconcile Slice 30's historical rows without rewriting its
   baseline. Run affected TypeScript tests and the required repository source-change
   gate once on the final source candidate, and broaden only for a concrete
   failure or blast-radius risk.
5. Request independent code review and verification; fix findings with focused
   tests. Record exact candidate SHA, gates, reviews, and limitations in status.
   Merge the reviewed branch into `release/0.8.27`, update release state/board,
   verify from Git, and remove this temporary worktree.
