---
title: FathomDB 0.8.27 Slice 30 - refactor navigation inventory
status: REVIEWED_BASELINE
source_sha: df8017463ecce6281a4c989a2d1bd01118e2025e
---

# Slice 30 refactor navigation inventory

This human navigation record describes the clean source commit named above.
The complete machine surface is `baseline.json` (SHA-256
`7db3d883f99940aea69c58079e64d6794be459ef9ab76561106a314e61900384`).
Line counts are advisory attention signals only. They are not movement targets,
correctness gates, or permission to split a settled boundary.

## Selected roots and top-level symbols

| Root | Lines | Top-level navigation |
| --- | ---: | --- |
| `src/rust/crates/fathomdb-engine/src/lib.rs` | 34,189 | Root `Engine`; runtime/open/write/search/close and projection worker implementation; public types/constants/functions; root re-exports from `actuation`, `data_plane_integrity`, `dependency_closure`, `dependency_trace`, `evidence`, `frozen_read`, `graph_expand`, `lifecycle`, `pagination`, and `projection_generation`; operator and test-hook sections. |
| `src/rust/crates/fathomdb-py/src/lib.rs` | 5,943 | PyO3 mirror classes and conversion helpers, native `Engine`, free functions, error mapping, and the `_fathomdb` registration block. The baseline records 121 actual class, function, alias, and exception registrations independently from the stub. |
| `src/rust/crates/fathomdb-napi/src/lib.rs` | 5,406 | NAPI data mirrors/conversions, native `Engine`, async task wrappers, free functions, and typed error conversion. The production declaration baseline contains 81 exports with `default-embedder` and no test-hook declaration. |
| `src/ts/src/index.ts` | 4,439 | Public `Engine`, request/result and wire types, validation/mapping helpers, lifecycle/search/evidence/dependency APIs, error re-exports, and the `read` namespace re-export. The re-export-resolved declaration row contains 208 entries. |
| `src/python/fathomdb/engine.py` | 2,846 | Public `Engine`; native/result conversion; graph, evidence, dependency-trace, frozen-read, validation, device, and explanation helpers. Package authority remains the 119-name literal `fathomdb.__all__`; the resolved wrapper row has 1,131 public-path signature entries; the native stub has 538 class/member/function entries. |

The Rust machine rows contain 164 facade-default, 198 facade-operator, 9,922
engine-default, 10,462 engine-operator, 10,062 engine-test-hooks, and 10,603
combined operator-plus-test-hooks entries. Cargo's complete output includes
auto/blanket impls; repeated identical impl/item pairs normalize idempotently,
while their complete impl context remains part of associated signatures.

## Direct dependencies and co-change neighbors

- Engine direct dependencies are `fathomdb-schema`, `fathomdb-query`,
  `fathomdb-embedder-api`, `fathomdb-embedder`, `rusqlite`, `sqlite-vec`,
  `serde`/`serde_json`, `jsonschema`, `sha2`, `getrandom`, and optional `libc`.
  Its co-change neighbors are the facade re-export block, the ten shipped
  semantic modules named above, schema migrations, query compilation, and
  embedder/runtime feature forwarding.
- PyO3 depends directly on the engine, schema, embedder API/runtime, PyO3, and
  JSON. Its co-change neighbors are `src/python/fathomdb/_fathomdb.pyi`, package
  `__all__`, `engine.py`, `types.py`, `errors.py`, wheel feature lists, and
  Python parity/release-surface tests.
- NAPI depends directly on the engine, schema, embedder API/runtime, NAPI,
  Tokio, and serde/JSON. Its co-change neighbors are generated
  `src/ts/index.d.ts`, `src/ts/src/binding.ts`, the platform loader, package
  metadata, native build commands, and Node release-surface tests.
- The TypeScript root imports `binding`, `errors`, `read`, `validation`, and
  `evidence-validation`. Those modules, `tsconfig.build.json`, `package.json`,
  generated declarations/runtime exports, and SDK parity tests are its
  co-change neighbors.
- The Python root wrapper imports the native extension plus `config`, `types`,
  `filter`, `errors`, and `read`. Those modules, package `__init__.py`, the
  native stub, and Python parity/release-surface tests are its co-change
  neighbors.

## Boundary map and owning checks

| Boundary | Deep owner | Focused checks during moves |
| --- | --- | --- |
| Rust root and feature visibility | Engine/facade compiler surfaces | `fathomdb` `reexports` and `governed_surface`; Slice 30 six-row comparator; affected default/operator/test-hook integration targets. |
| Storage, writer, transaction, projection runtime | Engine root plus shipped semantic modules | Existing engine unit tests and the affected integration suite; correction/erasure, dependency, frozen-read, pagination, evidence, graph, and projection-generation suites as applicable. |
| PyO3 registration and ABI declarations | `fathomdb-py` registration block | `test_native_stub_surface.py`, Python `test_surface.py`, `test_release_surface.py`, and `test_sdk_surface_parity_oracle.py`. |
| NAPI production declaration | `fathomdb-napi` production build | `npm run build:native`, NAPI declaration comparator row, and existing no-test-hook leak/release-surface coverage. |
| TypeScript public wrapper/package root | `src/ts/src/index.ts` and `package.json` | compiler declaration row, runtime export keys, `surface.test.ts`, `release-surface.test.ts`, and `sdk-surface-parity.test.ts`. |
| Python public wrapper/package root | `engine.py` and package `__all__` | package-export comparator row, native registration/stub rows, Python surface/parity checks, and affected functional tests. |

## Exceptions and navigation rules

- The five roots remain the only 0.8.27 refactor targets. Existing semantic
  modules are destinations; no new crate, SDK, public package root, engine
  field visibility, or transaction authority is implied.
- Generated Rust blanket impls make the baseline large by design. They protect
  real signature/re-export drift and are not copied expected-symbol lists.
- The production NAPI row is generated only with `default-embedder`. Debug
  NAPI/test-hook artifacts remain owned by existing leak tests and are not a
  published surface.
- Runtime native Python introspection was not used: editable installation from
  the worktree is forbidden. Registration source, the hand-maintained native
  stub, and package exports are separate required evidence, not substitutes.
  Registration aliases created with `m.add` and complete stub class bases are
  included, so exception removal or inheritance drift cannot hide behind the
  class/function-only rows.
- Package metadata currently exposes the package root through `main` and
  `types`; it has no declared public subpath map. Private TypeScript source
  modules are navigation aids, not package entrypoints.
- CUDA, Metal, migration-test-hooks, benchmark, installed-package, and
  cross-platform artifact qualification are outside this baseline capture.
  Later moves run only the route they affect, and Slice 150 owns final package
  qualification or an authorized rebaseline.
