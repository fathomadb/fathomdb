---
title: Slices 90, 100 and 110 prospective binding-closure review
status: PLANNED
target_release: 0.8.27
reviewed_candidate: 63091b6249e5805ea71b44de59f0ac6703b786a9
---

# Prospective binding-closure review

Entry was clean `63091b6249e5805ea71b44de59f0ac6703b786a9`.
Initial verdict: **FAIL — binding plans lack complete ownership and exit
evidence, and generic callback/thread claims obscure current code behavior**.
The author reviewed and edited the prospective records. This is not independent
approval of the edited candidate, commissioning, or a runtime test receipt.

## Evidence inspected

- Master plan, release-state/board, Slice 90 design and Slice 85 recommendation
  and runtime-closure review; Slice 30 comparator design and actual Python
  registration source reader in `dev/tools/surface_comparator.py`.
- PyO3 Cargo/features, native `lib.rs` classes/exception mappings/conversions,
  `call_engine`, Engine open/control methods, pause carriers, initializer and
  registrations; Python config/open, package imports, pyproject, native stub,
  FFI/surface tests and wheel verification script.
- NAPI Cargo/features, native `lib.rs` typed errors, sync/async execution,
  objects, JSON/numeric translation, Engine methods and hooks; TS native
  binding/platform loader, package metadata, production build wrapper,
  generated-declaration contract, FFI/release tests and platform artifact recipe.
- Accepted async/TS API/embedder ADRs, bindings design, Python/TypeScript
  interfaces, package install contracts and source-scraper dependencies.

## Findings and prospective resolutions

| ID | Severity | Finding | Resolution |
| --- | --- | --- | --- |
| BC-1 | P1 | Slice 90 does not define a consumable exact binding handoff, allowing config forwarding to be repeated or deferred during decomposition. | Add native signatures, validation order, effective five-knob evidence, surface deltas and installed-artifact receipts as a mandatory Slice 90 exit package. 100/110 consume it without reimplementing config. |
| BC-2 | P1 | Slice 100 has no complete native item/registration disposition, requirements or zero-open-item exit. | Dedicated design specifies source-derived named inventories, final owners, one initializer/Engine identity, R27-100A–E and exact exit before 110. |
| BC-3 | P1 | Slice 110 has no complete NAPI registration/async/conversion/package inventory or native closure test. | Dedicated design specifies native and generated/runtime joins, owner map, R27-110A–F and zero native obligations before 120. |
| BC-4 | P1 | Installed artifact claims could be satisfied with source-only registrations, stale declarations or a directly loaded development binary. | Require independent source/stub/declaration/runtime/package oracles, fresh wheels and thin-main/platform tarball pair installations, import/binary provenance and exact feature/toolchain receipts. |
| BC-5 | P1 | Both native subscriber methods currently discard their arguments; NAPI uses Tokio spawn_blocking while accepted authority names a different handoff mechanism. Generic callback/responsiveness testing could falsely claim contract compliance. | Add named pre-move contract-disposition gates. Existing accepted permission, an accepted and implemented successor, or a separately tested fix-to-contract must close each within 100/110; no proposal-only closure or deferral to SDK slices. |
| BC-6 | P2 | A mechanical module split could break PyO3 macro constraints or Python class identity. The crate lacks multiple-pymethods, and some pyclasses omit an explicit module. | Preserve one Engine pymethods block, existing macro attributes and actual runtime identities; no feature addition for aesthetics. Keep explicit initializer registrations at root so the current comparator stays complete. |
| BC-7 | P2 | The plans omit concrete GIL/event-loop, ownership, close/failure, numeric/string/buffer and panic/precedence witnesses. | Add signature-derived characterization and real native tests, separately specifying Python GIL and NAPI sync/Promise behavior; no invented buffer/callback API or wrapper-mock substitute. |
| BC-8 | P2 | No review-sized order, source-scraper retarget proof, candidate-bound platform matrix or independent exit gates are specified. | Add ordered sub-batches, established size thresholds, RED mutants for scanners, supported-route manifests, code review and independent read-only verification. |
| BC-9 | P2 | Slice 120 could inherit unfinished native declaration/loader/async obligations. | Explicitly require complete native substrate and package receipts at 110 exit; 120 retains only its TypeScript SDK decomposition scope. |

These resolutions change planning requirements only. They do not assert that
the discovered subscriber/executor discrepancies have been implemented or
accepted. Existing historical review records and the verbatim Slice 85
recommendation remain unchanged. Public baselines stay immutable; only named
reviewed contract deltas can be accounted for separately.

## Ladder decision and verdict

**No Slice 111.** Contract correction, mechanical native movement, declaration
generation and installed-package qualification can be sequential reviewed
batches within Slice 110. No independent technical prerequisite requires a
new ladder position. Size or a desire to mark partial work complete is not a
reason to split the slice. A required unavailable build route blocks exit;
it does not silently move the proof to 120 or 150.

Remediation verdict: **the enumerated planning findings are addressed;
independent review of the edited designs is still required**. Slices 90,
100 and 110 remain PLANNED/uncommissioned. Exact symbol/registration and
supported-route manifests are approved at their actual entry candidates,
after prior slices land; they cannot silently change the final owner policy
or excuse a missing obligation.

## Documentation validation

The full Markdown lint wrapper passed, as did separate plan-status lint,
release-state generated views, plan anchors, JSON parsing and whitespace
validation. The optional-wording scan found only explicit prohibitions on
deferral/remainder buckets, not open placement decisions. The prior verbatim
Slice 85 recommendation has no diff. No production/runtime verification claim
is made by these documentation checks.
