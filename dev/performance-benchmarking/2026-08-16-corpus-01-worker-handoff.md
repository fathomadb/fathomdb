# CORPUS-01 worker handoff

**Worker branch:** `experiments/performance-corpus-01-20260816`  
**Campaign base:** `d07c551ef0ae5de244c4f6428744a5100eb433ab`  
**Scope:** offline corpus matrix and external-only human-gold protocol.

## Delivered

- `experiments/configs/corpus-01/corpus-matrix.v1.json` records the required
  portfolio, factual-status distinction, payload/redistribution rules,
  category coverage, corpus-specific metrics, known class counts, and claim
  limits.
- `experiments/configs/corpus-01/human-gold-protocol.v2.json` binds the matrix
  and forbids raw answers, questions, evidence text, payloads, and quotes from
  repository manifests. Its v2 manifest requires source-payload and license
  hashes, source/generator revision, selected class counts, exclusions, metric,
  paired-power, and claim bindings for every selected corpus/category pair.
- `experiments/corpus_matrix.py` is a fail-closed validator for matrix-native
  eligibility, qualified human-gold manifests, approved amendments, and
  complete lifecycle portfolio coverage.
- `tests/experiments/test_corpus_matrix.py` exercises the full portfolio,
  exact matrix binding, lifecycle coverage, two-reviewer rule, and rejection of
  raw fields. `tests/experiments/test_corpus_matrix_review_remediation.py`
  adds the review-required native-vs-qualified eligibility, preflight,
  amendment, and portfolio-coverage cases.
- `2026-08-16-corpus-01-matrix-and-human-gold-contract.md` explains the
  factual boundaries, allowed claims, and review workflow for humans and
  agents.

## No live action

This lane did not acquire, download, inspect, or hash an external corpus. It
did not invoke a model or paid service, create an answer oracle, conduct a
human review, write an external manifest, append an experiment receipt, or
change any coordinator-owned file.

## Verification

Correction to the original handoff: its all-in-one commit did **not** contain a
separately committed red checkpoint, so it must not be described as one. The
review-remediation red checkpoint is committed at `572986b6`: its three tests
failed with missing qualification APIs before this implementation. The focused
suite then covered 16 cases. The approval-registry red checkpoint is separately
committed at `56fcc2ad`: a fabricated `seq-999999` amendment failed before the
registry API existed. The focused suite then covered 17 cases.

Correction to that approval-registry evidence: `56fcc2ad` initially exercised
an unexpected keyword argument, not the vulnerable three-argument API. The
replacement red checkpoint is committed at `6a52be4a`. Its direct three-
argument call initially failed because the then-fixed API required a keyword;
a read-only replay of the actual pre-registry source at `aa52220a` printed
`pre_registry_three_argument_accepted=locomo:knowledge_update` for the same
fabricated `seq-999999` amendment. The final API preserves the three-argument
call and fails closed with `CorpusMatrixError` when no coordinator registry is
supplied. It also tests exact amendment-SHA rows with mismatched corpus,
category, and approval reference. The focused suite now covers 21 cases. On
the committed state at `cb920dd2`, the focused suite passed 21 tests,
`./scripts/agent-verify.sh` exited 0, and `git diff --check` exited 0.

## Review focus

1. Verify that every license and payload assertion is traceable to the cited
   local manifest, acquire script, research note, or historical register, and
   that the `external_payload_not_verified` label is not misread as payload
   presence.
2. Confirm no generated or static artifact contains raw or verbatim
   non-redistributable material, a question, or an answer oracle.
3. Check that unsupported pairs cannot obtain eligibility except through a
   qualified v2 manifest plus a matching, approved amendment.
4. Check every selected corpus/category is bound to source-payload and license
   hashes, source/generator revision, class counts, exclusions, a permitted
   metric, paired-power artifact, and claim hash before human evidence counts.
5. Check a `seq-N` string cannot authorize an amendment by itself: the
   coordinator-supplied registry must bind the exact amendment SHA-256 and
   corpus/category pair and approval reference; omission must fail closed. No
   worker should create a registry approval or hand-edit the steward ledger.
6. Check the strict schema choices against follow-on needs before allowing an
   external manifest writer. Any protocol revision must be versioned and
   review-gated, not silently relaxed.

## Next factual prerequisites

The coordinator must select a source/split and establish its external license
copy, upstream/generator pin, payload-root hash, selected-class counts,
exclusions, metrics, paired power plan, and permitted redistribution posture.
Only then can the authorized acquisition or human-review work receive its
separate coordinator release. For an unsupported pair, the coordinator must
also record the relevant decision through `ledgerwrite` and supply the exact
content-free approved-amendment registry entry. No integration was performed by
this worker.
