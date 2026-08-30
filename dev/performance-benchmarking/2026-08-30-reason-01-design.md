# REASON-01 protected relationship profile design

## Decision

Implement `protected_multiquery_v1` as an experiment-owned caller-side profile.
Do not modify Engine or shipped SDK surfaces. The profile is selected only by
explicit relationship intent or exact override and always protects the A0
prefix.

## Components

### Registry

`experiments/configs/reason-01/profile-registry.v1.json` contains one strict
entry with:

- profile ID, version, `experimental` status, and `relationship` intent;
- FTS prefix, query-count, hybrid, rerank, graph, and context limits;
- required embed/rerank devices and doctor policy; and
- hashes and claim boundary for the preserved development evidence.

`experiments/reason_01_profile.py` loads and validates this entry. It does not
reuse the stale 0.8.11 prototype router registry, whose multi-hop tuple is
explicitly provisional and describes a different experiment.

### Resolution

`resolve_profile(intent=None, profile_override=None)` returns either A0 or the
registered profile. The known intent enum is `exact`, `semantic`, `timeline`,
`relationship`, `global`, and `fast`. Relationship or the exact override selects
protected; the override may accompany only absent or relationship intent. Other
known intents select A0 only without an override. Unknown values and conflicts
fail. There is no content-based classifier.

### Retrieval

`execute_profile` performs:

1. FTS-only original-query search at limit 10.
2. Up to three deterministic query forms: the original, a stop-word-reduced
   form, and a proper-name plus bounded-content form.
3. Hybrid search for every unique form at alpha 1.0, pool/rerank depth 20,
   graph disabled, explain enabled, and limit 20.
4. Stable round-robin merge of unique hybrid identities after the complete A0
   prefix until 20 items are selected.

Every call receives the same read view by object identity, and the executor
requires every returned result to report the same projection cursor. The public
FTS-only primitive has no metadata-filter parameter, so the profile rejects any
non-null filter before its first Engine call. Hit identity is
`SearchHit.id.value`; the executor verifies one `(source_id, body_sha256)`
attribution tuple per identity across all branches.

### Trace

The returned runtime result contains selected hits plus a trace. The trace's
safe projection hashes queries and selected identities with distinct
`reason01:query:` and `reason01:identity:` domains and records only counts,
branch flags, caller authority, and timings. Receipt writers may serialize only
the safe projection.

### Equivalence and held-out preflight

`experiments/reason_01_equivalence.py` binds the registry to the pinned external
282-question artifact and rejects any selected-ID or aggregate-quality drift.
It uses fresh databases and the shared FathomDB test setup. Before retrieval it
attests the Git commit; CLI, Python package, and native-module hashes; embedder
and reranker identities and caches; CUDA UUID and driver; host; corpus, adapter,
and identity-map hashes. Any unavailable or mismatched item fails closed.

`experiments/reason_01_preflight.py` selects every LongMemEval-S row whose
question type is multi-session, excludes the exact 24 prior case IDs, and
requires the resulting ordered cohort to contain 109 cases. Before answer
execution it validates gold-session labels; freezes ordered IDs and source,
exclusion, selection-code, and configuration hashes; and checks the exact
GPU/setup policy. It writes only a safe preflight receipt and external
content-free manifest.

## TDD sequence

1. Add an importable skeleton plus registry/resolution, recipe-call, property,
   refusal, safe-trace, equivalence, and preflight-overlap tests.
2. Run each contract family and preserve its targeted behavioral RED result.
3. Implement only enough behavior and configuration to make those tests GREEN.
4. Run the live 282-question equivalence check and zero-spend held-out preflight.
5. Run the full repository verification gate with ptrace enabled.

## Failure behavior

Configuration, authority, attestation, attribution, overlap, and evidence drift
are typed failures. No failure falls back to a different profile or writes a
partial success receipt. Held-out outcomes are never used to alter the recipe.
