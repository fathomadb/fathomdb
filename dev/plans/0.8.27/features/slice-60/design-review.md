---
title: FathomDB 0.8.27 Slice 60 - independent design review
status: PASS
reviewed_on: 2026-09-25
---

# Slice 60 independent design review

Cycle 1 used Fable at medium reasoning. Cycles 2-4 used Opus 5.5 at high
reasoning. Every reviewer was read-only. Each checked the drafts against:

- live source and tests;
- the release plan, the test approach, and the slice execution contract;
- the Slice 50 precedent; and
- `AGENTS.md`.

## Findings and resolution

| Cycle | Finding | Resolution |
| ---: | --- | --- |
| 1 | Consolidation `supersede`/`merge` consumes the in-memory cursor inside its transaction (`lib.rs:8785`), so a universal cursor probe would force a false defect stop. | Plan item 8. The consolidation case pins a preceding `invalidate` verdict and omits the cursor probe. |
| 1 | The background projection worker could mutate state between snapshots. | `drain` must return `Ok` before both snapshots. |
| 1 | The seam map was wrong: `record_lifecycle` does not call `commit_batch`, and `temporal` mentions `validate_write` only in comments. It also missed the projection-worker, test-hook, `#[cfg(test)]` counter, root-test, and `ProviderSession` field seams. | Replaced with a grep-derived seam table. |
| 1 | Two mutants would not compile. The ingest after-side-effect injection was silently dropped. Test-seam cfg gating was unstated. | Mutants replaced. Cross-request ingest non-atomicity recorded as existing contract (plan item 7). Profiles paragraph added. |
| 2 | `tests/slice35_virtual_mutation_manifest.rs` and the slice35 audit scrape moved functions from `lib.rs`. | Reviewed path-only amendment in plan step 3: `concat!` of `include_str!`s and re-keyed audit entries, with needles and counts unchanged. |
| 2 | Seam table incomplete. `enforce_provenance_retention` misattributed to Slice 70. `maintain_before_writer` can legitimately commit before validation. The translation and enrolment boundaries were unaddressed. The worker-wake claim was unverifiable. | Seam rows added. Retention moves with its only caller. Closure precondition added. Translation declared infallible. `enrolment_raise` case added. The unverifiable clause dropped. |
| 3 | The `enrolment_raise` mutant was vacuous, because the TEMP trigger fires on any early insert on the writer connection. The cursor probe could not drop TEMP triggers from another connection. | The mutant ignores the `register_vector_kind` result inside apply, and the trigger is `WHEN`-scoped to a sentinel kind. TEMP triggers are dropped through `execute_for_test`, and the probe uses an unmatched kind. |
| 3 | NITs: retention contradiction, `schema_id` equal to the collection, audit comprehension, rustdoc-link claim, and the duplicate `WritePlan` row. | All corrected. |
| 4 | No issue remained. | PASS. |

## Final verdict

The plan reconciles the draft with Slices 40 and 50, the existing actuation
module, and the live consumers. It names exact inventory and seams, and adds
falsifiable local requirements. It fills the full-state rollback gap at every
write boundary with non-vacuous mutants, and leaves projection, read, and open
domains to later slices.

**VERDICT: PASS. Implementation may begin.**
