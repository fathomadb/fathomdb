---
title: FathomDB 0.8.26 Slice 60 — TDD chronology
status: COMPLETE
target_release: 0.8.26
---

# Slice 60 TDD chronology

Slice 60 changes maintained documentation owners, not product behavior. Its
test-first oracle is therefore a reproducible set of source-comparison
presence/absence probes rather than a manufactured runtime test.

## RED — stale owners at `b770de01`

Before changing the owners, focused `rg -q` probes produced these failures:

```text
RED retrieval-current-profile
RED retrieval-cross-encoder
RED recovery-current-profile
RED recovery-full-inventory
RED engine-current-profile
RED engine-projection-gate
RED recovery-no-stream-fiction
RED engine-global-logical-id
```

Direct witnesses included:

```text
dev/design/retrieval.md:45: `rerank` is deferred
dev/design/engine.md:144: per kind
dev/design/recovery.md:74: progress stream plus terminal summary
```

The accepted successor ADR was then owner-authorized, and the reviewed planning
bundle was committed as `3f23ca59` before implementation.

## GREEN — reconciled current owners

The same claim families were rerun after the rewrite, with positive probes for
the current profile and inverse probes for stale claims:

```bash
check_present() { rg -q "$2" "$3"; }
check_absent() { ! rg -q "$2" "$3"; }

check_present retrieval-current-profile '^target_release: 0\.8\.26$' dev/design/retrieval.md
check_present retrieval-cross-encoder 'Cross-encoder reranking|cross-encoder reranking' dev/design/retrieval.md
check_present recovery-current-profile '^target_release: 0\.8\.26$' dev/design/recovery.md
check_present recovery-full-inventory 'orphan-provenance' dev/design/recovery.md
check_present engine-current-profile '^target_release: 0\.8\.26$' dev/design/engine.md
check_present engine-projection-gate 'commit_gate' dev/design/engine.md
check_present engine-shared-vector 'vector_default' dev/design/engine.md
check_absent retrieval-no-deferred-rerank '`rerank` is deferred' dev/design/retrieval.md
check_absent recovery-no-stream-fiction 'progress stream plus terminal summary' dev/design/recovery.md
check_absent engine-no-kind-identity 'per kind\.' dev/design/engine.md
check_absent engine-no-restore 'restore_logical_id' dev/design/engine.md
check_absent engine-no-single-batch-cursor \
  'One write cursor `c_w` is allocated for the committed batch as a whole' \
  dev/design/engine.md
```

```text
GREEN present retrieval-current-profile
GREEN present retrieval-cross-encoder
GREEN present recovery-current-profile
GREEN present recovery-full-inventory
GREEN present engine-current-profile
GREEN present engine-projection-gate
GREEN present engine-shared-vector
GREEN absent retrieval-no-deferred-rerank
GREEN absent recovery-no-stream-fiction
GREEN absent engine-no-kind-identity
GREEN absent engine-no-restore
GREEN absent engine-no-single-batch-cursor
```

The implementation-side guard also remained GREEN:

```console
git diff --name-only b770de01 -- src/rust src/python src/ts
```

It emitted no paths. The only non-document implementation file changed is the
mandatory monotone status-budget ratchet in `scripts/lint-design-status.sh`,
lowered from 46 to 43 because three maintained owners moved from legacy
free-form `locked` status to governed `ACTIVE` status.

## Focused validation

The first Markdown run intentionally exposed the ratchet reduction:

```text
FAIL lint-design-status: legacy count is now 43, ceiling is 46 — lower
`LEGACY_BUDGET` to 43 in scripts/lint-design-status.sh in this same change.
```

After that required one-line guard adjustment, `agent-lint-md.sh` passed.
Design lifecycle, its regression fixture, release-state views, and product-code
diff checks also passed before independent content review.

## Independent-review RED

The first independent implementation review returned CHANGES REQUESTED. Its
focused source comparison found:

- fresh bootstrap reports schema construction steps, while only a current
  reopen reports an empty `migration_steps` list;
- active source/tests still link six headings removed by the initial rewrite;
- JSON-object wording needed to be scoped to `--json`; and
- CLI recovery opens through the public fail-closed path before invoking its
  action, making the malformed-WAL recovery hint unreachable.

The first three documentation defects are corrected in the follow-up diff. The
last is a product reachability defect and remains a plan stop gate pending
explicit scope authorization; no runtime test or implementation has been
altered to conceal it.
