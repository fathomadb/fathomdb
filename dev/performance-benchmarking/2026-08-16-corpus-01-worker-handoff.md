# CORPUS-01 worker handoff

**Worker branch:** `experiments/performance-corpus-01-20260816`  
**Campaign base:** `d07c551ef0ae5de244c4f6428744a5100eb433ab`  
**Scope:** offline corpus matrix and external-only human-gold protocol.

## Delivered

- `experiments/configs/corpus-01/corpus-matrix.v1.json` records the required
  portfolio, factual-status distinction, payload/redistribution rules,
  category coverage, corpus-specific metrics, known class counts, and claim
  limits.
- `experiments/configs/corpus-01/human-gold-protocol.v1.json` binds the matrix
  and forbids raw answers, questions, evidence text, payloads, and quotes from
  repository manifests.
- `experiments/corpus_matrix.py` is a fail-closed validator for the two
  committed contracts and a future external content-free human-gold manifest.
- `tests/experiments/test_corpus_matrix.py` was written first. It exercises the
  full portfolio, exact matrix binding, lifecycle coverage, two-reviewer rule,
  and rejection of raw fields.
- `2026-08-16-corpus-01-matrix-and-human-gold-contract.md` explains the
  factual boundaries, allowed claims, and review workflow for humans and
  agents.

## No live action

This lane did not acquire, download, inspect, or hash an external corpus. It
did not invoke a model or paid service, create an answer oracle, conduct a
human review, write an external manifest, append an experiment receipt, or
change any coordinator-owned file.

## Verification

The red checkpoint was `PYTHONPATH=. python3 -m pytest
tests/experiments/test_corpus_matrix.py -q` before `experiments.corpus_matrix`
existed: collection failed with `ImportError`. The green focused result is ten
tests passing. `./scripts/agent-lint-md.sh`, `git diff --check`, and the full
`./scripts/agent-verify.sh` pass in this worktree.

## Review focus

1. Verify that every license and payload assertion is traceable to the cited
   local manifest, acquire script, research note, or historical register, and
   that the `external_payload_not_verified` label is not misread as payload
   presence.
2. Confirm no generated or static artifact contains raw or verbatim
   non-redistributable material, a question, or an answer oracle.
3. Check that the protocol does not convert the authorized human-gold work
   into a license-free claim: actual source/split, hash, class count, paired
   power, and claim still need factual preflight.
4. Check the strict schema choices against follow-on needs before allowing an
   external manifest writer. Any protocol revision must be versioned and
   review-gated, not silently relaxed.

## Next factual prerequisites

The coordinator must select a source/split and establish its external license
copy, upstream/generator pin, payload-root hash, selected-class counts,
exclusions, metrics, paired power plan, and permitted redistribution posture.
Only then can the authorized acquisition or human-review work receive its
separate coordinator release. No integration was performed by this worker.
