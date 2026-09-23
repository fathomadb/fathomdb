---
title: FathomDB 0.8.27 hidden-surface oracle - design review record
status: ACTIVE
target_release: 0.8.27
---

# Hidden-surface oracle design review

## Round 1 (734dcf2f): FAIL

Independent read-only review, validated against the real engine rustdoc JSON
(format 57) and scratch fixture crates.

| ID | Sev | Finding | Chosen remediation |
| --- | --- | --- | --- |
| D-1 | P1 | Resolving type references through `paths` records definition locations, so the Slice 40 move itself would diff (897 of 2,698 entries depend on them). | Rewrite local references to the first-found public path; `private:<path>` for unreachable items; drop constant `expr`. |
| D-2 | P1 | Id lists (fields, variants, impl items) survive id stripping; a consistent renumbering changed 1,076 entries. | Replace id lists with names or normalized types; drop member lists covered by their own entries; renumbering self-test. |
| D-3 | P1 | `git archive` stamps commit time, so a shared target directory can reuse another commit's workspace crate builds. | Extract with `tar -m`; `--locked`; X-Y-X byte-identity check. |
| D-4 | P2 | Own-attribute hidden flag misses `#[doc(hidden)]` on a `pub use` or an `impl` block. | Separate `own_hidden`, `use_hidden`, `impl_hidden`. |
| D-5 | P2 | Walk underspecified: variants and trait items are `default` visibility, glob shadowing, external globs, cycles, local blanket impls, trait generic arguments. | Precise reachability table with fixture cases. |
| D-6 | P2 | Shared scratch root with Slice 30, which removes the whole root. | Own root and marker, `mkdtemp` per capture. |
| D-7 | P2 | Gates no ruled row covers: `migration-test-hooks`, `tc5-benchmark`, `debug_assertions` on `execute_for_test`, facade hidden modules. | Escalated; owner added five rows (nine total). |
| D-8 | P2 | No successor baseline after Slice 140, no Slice 150 run, no real-data injection. | Successor baselines with diff records; cadence step 4, Slices 140 and 150; injection AC. |
| D-9 | P3 | Fixtures carry host paths; fixture crate needs its own workspace; hazard cases missing. | Canonicalizing regeneration script; extended fixture crate. |
| D-10 | P3 | Schema check, exclusive create, heavy-tier labelling, `for-each-ref`, deviation note. | All adopted. |

## Round 2 (822e73a2): FAIL

Round-1 fixes confirmed empirically: the fixture move compares equal and a
full id renumbering of the real engine JSON changes 0 of 2,698 entries.

| ID | Sev | Finding | Chosen remediation |
| --- | --- | --- | --- |
| E-1 | P1 | Rustdoc always enables `debug_assertions`, so a release-profile rustdoc row cannot exist; ACH-1's facade claims were wrong. | Owner ruling `hidden-surface-release-probe`: per-site cfg predicates in signatures plus a release compile probe; ACH-1 corrected. |
| E-2 | P1 | Facade re-exports are external, so `paths` gives engine definition locations (D-1 again at the crate boundary). | External re-export signatures use the facade's own `use.source` spelling. |
| E-3 | P2 | The injection as written does not compile; a `--source-dir` manifest would look like a real capture. | Self-contained `decode_dependency_trace_root_for_test` injected at both sites; `export` subcommand; `source_modified` label; baseline-directory refusal. |
| E-4 | P3 | `private:<defining path>` still depends on location. | `private:<kind>:<name>` with a collision suffix. |
| E-5 | P3 | Capacity paths, absolute paths in row identities, "newest baseline", shared JSON output, cache growth, fixture size. | Own capacity check; relative arguments; most-recent-ancestor baseline rule; `flock` and read-after-run; stable export path and `prune`; pruned `paths`. |

## Round 3 (b34df007): FAIL on E'-1 only; text fixes applied (FIX-3)

E-1 to E-5 confirmed resolved. Empirical checks passed:

- all 94 real `CfgTrace` attributes parse;
- a release build reports unresolved debug-only items per function;
- the facade's `use.source` spellings are stable;
- the corrected injection adds exactly one entry.

The reviewer stated that with E'-1 to E'-4 fixed as below the design is
PASS-WITH-FINDINGS without a further round.

| ID | Sev | Finding | Chosen remediation (verified by the reviewer) |
| --- | --- | --- | --- |
| E'-1 | P1 | A generated consumer crate cannot build under `--locked` (new lock entry) or inside the workspace as a non-member. | Probe as `examples/hs_probe.rs` inside the exported engine and facade crates; `--locked --offline`; no manifest or lock change. |
| E'-2 | P2 | Release-only discovery infeasible; some kinds cannot be named; selection row unstated. | Curated release-only list plus a fail-closed source scan; probe form per kind (trait impls by trait bound, fields and variants by predicate only); selection from the default rows with all site predicates evaluated. |
| E'-3 | P2 | The injection target shares a gated `pub use` group with a still-gated item. | Split the group; exact edit written into the design. |
| E'-4 | P3 | Canonical predicate order in ACH-1; `export` directory collides with `<root>/src`; ancestor-module cfgs for the probe. | Canonical order quoted; `<root>/exports/<sha>-<n>` with `discard`; ancestor module predicates included in probe evaluation. |

Design status: APPROVED after FIX-3.

## Implementation finding and revision 4 (d77719c5)

During implementation, the `5f5c1798` capture differed from the `e3358800`
baseline in 25 entries. Every one was a site move: Slice 40 moved `#[cfg]`
and `#[doc(hidden)]` onto re-exports with identical effect. The owner ruled
`hidden-surface-effective-and-inventory`: sign effective values, add test
inventories, and retire the oracle at Slice 150.

## Round 4 (d77719c5): PASS-WITH-FINDINGS; revision 5

| ID | Sev | Finding | Chosen remediation |
| --- | --- | --- | --- |
| F-1 | P2 | `cargo test -- --list` output cannot attribute tests to targets, and same-named tests collapse. | Per-binary listing from `--no-run --message-format json`. |
| F-2 | P2 | Nine test targets need feature sets no row enables, and are silently skipped. | Owner ruling `feature-complete-test-coverage`: static `test-targets` row, derived `tests-req-<n>` rows, plus a permanent feature-complete gate (weights and GPU) and a coverage check. |
| F-3 | P2 | Syntactic cfg canonicalization leaves logically equal gates different. | Truth-table minimal sum-of-products canonicalization; fail above twelve atoms. |
| F-4 | P2 | Master plan lacks the inventory policy and the retirement step; retirement ordering. | Cadence edits listed; retirement is the last Slice 150 step and rewrites the plan text. |
| F-5 | P3 | The dead-code claim holds only in part. | Claim corrected with the verified residual gaps and the gate that enforces it. |
| F-6 | P3 | Private-module sites invisible; field and external-glob effective values; unknown cfg names. | Defined in the design; the probe evaluator fails closed on unknown cfg names. |

## Round 5 (9934b9dc): FAIL on G-1; revision 6

| ID | Sev | Finding | Chosen remediation |
| --- | --- | --- | --- |
| G-1 | P1 | Self-skipping tests report `ok`, 40 `#[ignore]` tests exist, and opt-in experiments need gitignored inputs, so "none skipped, short allowlist" can be neither detected nor met. | Skip contract: `--nocapture` marker matching, a per-test-id allowlist with classes and reasons, ignored-count reconciliation, stale-entry failure, the gate sets runner variables; ACH-14 reworded. A shared `FATHOMDB_REQUIRE_LIVE` helper is noted as later hardening. |
| G-2 | P2 | RH-15 was vacuous because RH-14 covered the derived set by construction. | Committed `scripts/test-feature-matrix.toml`, regenerated by one command; RH-15 fails on drift, uncovered targets, and stale allowlist entries. |
| G-3 | P2 | CUDA environment unspecified: `nvcc` off PATH, `cudart_static` not found, device order. | Gate preflight sets CUDA variables, `CUDA_DEVICE_ORDER=PCI_BUS_ID`, `CUDA_VISIBLE_DEVICES=0,1`, and verifies the 3090s. |
| G-4 | P2 | The warm-cache verb covers the embedder only. | Warm the embedder through the CLI; reranker downloads in-test, with a failed download caught by the skip contract. |
| G-5 | P3 | `--filter-platform`; file-level cfgs need their own parser with platform and profile atoms; CUDA inventory cost. | Adopted and stated. |

## Round 6 (61bb11fa): PASS-WITH-FINDINGS; revision 7 approved

No P0 or P1. Applied: `--test-threads=1` so skip markers attribute to one test
id; the feature matrix and skip allowlist listed as permanent; a
lexicographic tie-break for the minimal cfg form; a `benign-message` allowlist
class for harmless `skipping` text; one sentence tying the derived inventory
sets to the committed matrix. Design status: APPROVED.
