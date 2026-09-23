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
