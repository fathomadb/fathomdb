---
title: FathomDB 0.8.27 Slice 114 — engine configuration audit plan
status: PLANNED
target_release: 0.8.27
planning_baseline: 1a6cd4938
---

# Slice 114 — engine configuration audit

This is a reviewed planning draft, not a start authorization. Slice 110 must
close before execution. Reconcile the inventory against its final merge SHA;
the current baseline is `release/0.8.27` at `1a6cd4938`.

## Changes since the September 28 draft

| Completed or allocated work | Decision for Slice 114 |
| --- | --- |
| Slice 90 implemented the accepted engine-owned runtime topology and the five-field `EngineConfig`. The accepted default is two scheduler workers and five embed workers; its matrix, named gates, installed bindings and D27 candidate receipt are complete. | Audit the actual consumers and documentation for drift. Do not reopen the accepted defaults or supply missing Slice 90 proof. |
| Slice 90 split the engine root into semantic owners. `runtime_configuration.rs` resolves open settings; `connection_runtime.rs`, `embed_dispatch.rs`, `projection_runtime.rs`, `reader_pool.rs`, `telemetry.rs`, and `provenance.rs` hold relevant consumers and constants. | Inventory by owner and call site, not by the former root line map. Include process-wide SQLite mode separately from per-engine settings. |
| Slice 100 decomposed PyO3 and Slice 110 decomposed NAPI, added the accepted TypeScript subscriber, and qualified current binding behavior except for an intermittent Jetson forced-CUDA row. | Check Python/TypeScript forwarding, error and public guidance against the current engine contract. Do not claim Slice 110 is closed or absorb its GPU defect into this audit. |
| Slice 103 changed SQLite/WAL path handling and added CLI-only owed-erasure recovery. | Review any configuration or failure guidance touched by the SQLite change; preserve the CLI-only recovery boundary. |
| Slice 120 owns TypeScript SDK decomposition, Slice 132 owns Rust SDK parity, and Slice 140 owns engine test seam extraction. | Record findings for those owners. Do not restructure SDKs, expand public capability, or move test seams here. |
| Slice 115 follows this audit and profiles the engine; Slice 135 owns the full published 0.8.26 comparison. | Pass an exact setting/constant disposition and any measurement questions to Slice 115 without making speculative tuning changes. |

The September 28 scope is **approved with explicit classification**: inspect
every production engine configuration input, operational limit and module-level
`const`/`static`, plus every compiler-reported or deliberately suppressed
unused production binding. Include SQL strings and schema/version constants in
the census, then classify them as non-settings rather than giving them an
artificial default or range. Identify test-only declarations as exclusions by
source location. Local variables need individual disposition when unused or
deliberately suppressed; auditing every ordinary local binding would add no
configuration or dead-code evidence. A value becomes a user setting only
through a reviewed public contract change.

## Requirements and acceptance

| ID | Requirement | Falsifiable acceptance |
| --- | --- | --- |
| R27-114A | The production configuration, constant and limit inventory has no unexplained owner or consumer. | AC27-114A: a source-linked census covers every production engine module-level `const`/`static`, `EngineConfig`, process-wide SQLite mode, engine-consumed environment variables, public setters, and operational capacity/timeout/retention/search/reader/WAL limits across default and relevant feature builds. Every declaration has an owner, location, reference or unused status, and setting/internal/schema/SQL/test-only classification; each setting and operational limit names its consumer. Source search and compiler warning checks reconcile omissions. |
| R27-114B | Public settings match the effective runtime and all supported language contracts. | AC27-114B: for each public setting the inventory records exact spelling per language, default, unit, accepted range or grammar, omission/zero meaning, precedence, open-time or live mutability, consuming effect and observable invalid/fallback outcome. It is checked against the accepted ADR, Rust/Python/TypeScript interfaces, `dev/design/engine.md`, `dev/design/scheduler.md`, `dev/design/embedder.md` and `docs/reference/config.md`. |
| R27-114C | Dead or misleading production values receive a justified disposition. | AC27-114C: each unused or deliberately suppressed production constant, static or local binding found by the census and warning audit is retained with a concrete invariant, removed with a focused behavior witness, or assigned to a named follow-up. Contradictory comments and docs are corrected in this slice; no stale claim is left as a placeholder. |
| R27-114D | The audit does not silently change runtime behavior or public surface. | AC27-114D: accepted defaults, ranges, effective precedence, runtime exports and public API remain unchanged unless a separately reviewed successor ADR/interface change and RED/GREEN tests explicitly authorize a correction. Existing Slice 90 runtime/configuration and binding fixtures pass for any code change, along with the required repository gate. |

These IDs are release-local; `dev/acceptance.md` remains locked. A finding
that the shipped behavior contradicts an accepted contract is a defect to
resolve with a focused failing test and reviewed fix. A proposed improvement
or new tuning range becomes a follow-up unless it is necessary to correct
such a defect and has appropriate behavior and performance evidence.

## Design and execution method

The inventory is a trace from input to effective value to consumer to
observable outcome. This structure distinguishes a documented knob from an
internal safety constant, and makes stale documentation detectable without
turning every literal into a product option. Start from `EngineConfig` and
`runtime_configuration.rs`, then trace process mode, env reads, setters and
operational limits through the current owner modules. Census module-level
constants and statics, then search source and public docs in both directions;
compile relevant features to surface dead bindings. Preserve the accepted
owner boundaries and document **why** each
non-obvious default or internal limit is retained where evidence exists. If
evidence is insufficient, say so and do not invent an accepted range.

1. At entry, capture the post-Slice-110 SHA and reconcile changed code,
   interfaces, ADRs and allocated work with this draft. Freeze the inventory
   rules and baseline public/configuration surfaces before edits.
2. Build and review the source-linked inventory. Classify each mismatch as
   stale prose, unused code, accepted-contract defect, proposed behavior
   change, or downstream owner handoff.
3. For any code or behavioral correction, add a focused failing test first,
   retain the RED witness, implement GREEN, then run owner tests and checks in
   the blast radius. Pure prose corrections need scoped markdown validators.
4. Review the final diff independently, verify affected public interfaces and
   the full source gate if source changed, and write a candidate-bound status
   with every inventory row disposed. Handoff measurement questions to 115.

No profiling campaign, broad tuning sweep, new configuration framework, or
automatic source rewriter is required by this audit.
