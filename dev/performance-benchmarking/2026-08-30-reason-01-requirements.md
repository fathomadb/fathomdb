# REASON-01 treatment requirements and acceptance criteria

## Requirements

- **R1 — Explicit selection.** A caller must request intent `relationship` or
  the exact profile `protected_multiquery_v1`. Missing authority and every
  other registered intent remain on A0. No query classifier may select the
  profile.
- **R2 — Caller-side only.** The profile composes public FathomDB reads. It does
  not change canonical storage, schema, projections, Engine defaults, or SDK
  behavior.
- **R3 — Frozen recipe.** Retrieve the original-query A0 FTS top 10; generate at
  most three deterministic, model-free query forms; run each through hybrid
  retrieval with alpha 1.0, pool and rerank depth 20, graph disabled, and result
  limit 20; then round-robin unique shadow hits after the protected A0 prefix to
  a maximum of 20 context items.
- **R4 — One read view.** Every constituent read receives the caller's same
  validity/read view, projection cursor, and metadata filter by object identity.
  The profile must not organize stale, out-of-window, or filtered records.
- **R5 — Exact runtime.** The registered experiment requires the pinned
  FathomDB build, default embedder, CUDA embedding and reranking, and successful
  setup/doctor attestation. It fails closed rather than substituting CPU,
  disabling reranking, or enabling graph. The registry and receipt pin the Git
  commit, Python package and native-module hashes, CLI hash, embedder and
  reranker identities and cache hashes, CUDA UUID and driver, host identity,
  corpus hash, adapter hash, and identity-map hash.
- **R6 — Attribution.** A logical identity may resolve to only one body and
  source in one execution. Identity is `SearchHit.id.value`; canonical
  attribution is the `(source_id, body_sha256)` tuple. Conflicting attribution
  fails the request.
- **R7 — Safe trace.** The trace records the selected profile, caller authority,
  counts, timings, branch activity, query hashes, and selected identity hashes.
  It contains no raw query, body, answer, corpus payload, or secret.
  Query and identity hashes use distinct domain prefixes.
- **R8 — Strict registry.** Unknown keys, profiles, intents, versions, or recipe
  drift are rejected. The registry labels the profile experimental and binds its
  evidence scope and source hashes.
- **R9 — Reproducible tests.** Tests use a deterministic real-contract fake only
  for caller composition. Data-generating checks use fresh FathomDB databases
  prepared through the shared setup/doctor path.
- **R10 — Honest evidence.** The 282 LOCOMO questions and prior 24 LongMemEval-S
  cases are development evidence only. Acceptance uses all 109 eligible
  LongMemEval-S multi-session cases not present in the prior 24-case run. It
  records the ordered IDs, source hash, exclusion hash, selection-code hash,
  and configuration hash before execution; no outcome-based selection is
  permitted.
- **R11 — No hidden spend.** Retrieval and preflight are zero-spend. Answer and
  judge calls use the approved Airlock route only after a model/cost contract is
  accepted.
- **R12 — Rejected offshoot.** `deep_compact_v1` is not registered, tested as a
  treatment, or promoted by this work.

## Implementation acceptance criteria

- **AC1.** Registry validation accepts only the exact
  `protected_multiquery_v1` experimental relationship entry and rejects drift.
- **AC2.** The resolver implements this complete truth table before any Engine
  call: absent intent and override resolves to A0; known intents `exact`,
  `semantic`, `timeline`, `global`, and `fast` with no override resolve to A0;
  `relationship` with no override resolves to protected; an exact protected
  override with absent or relationship intent resolves to protected; an exact
  protected override with another known intent conflicts; unknown intents,
  unknown overrides, and every other combination fail.
- **AC3.** A contract fixture observes one FTS-10 call and at most three exact
  hybrid calls with the registered parameters and identical read-view,
  projection-cursor, and metadata-filter identity.
- **AC4.** Property tests show that the merge always retains the complete unique
  A0 prefix, emits no duplicate identity, never exceeds 20 items, and is
  deterministic for arbitrary candidate sequences.
- **AC5.** Missing CUDA/reranker attestation, invalid limits, metadata-filter
  mismatch, or attribution conflict fails closed with zero or bounded calls as
  appropriate.
- **AC6.** Serialized safe traces contain no fixture query/body strings and
  expose only hashes for queries and selected identities.
- **AC7.** After the runtime import and attestation are repaired, a fresh
  implementation using fresh databases reproduces all 282 pinned protected
  multi-query selected-ID rows and their aggregate retrieval metrics exactly;
  latency is remeasured rather than required to be byte-equal.
- **AC8.** The zero-spend preflight freezes exactly all 109 eligible
  LongMemEval-S multi-session cases with zero overlap against the 24 prior case
  IDs, complete gold-session labels, ordered IDs and source/exclusion/code/config
  hashes, one fresh database per case, and verified GPU/doctor policy.
- **AC9.** An importable skeleton produces targeted behavioral RED failures for
  registry/resolution, recipe calls, merge properties, refusal behavior, safe
  trace, equivalence drift, and held-out overlap. The same focused suite is
  GREEN afterward; lint, typecheck, test, and the ptrace-capable repository
  verification gate pass.

## Held-out eligibility criteria

These criteria are frozen now but evaluated only after separate spend
authorization. Retrieval quality is measured once per case. Answer generation
and judging each run once per arm at temperature zero. A failure receives zero
for its case metric, ties contribute a paired delta of zero, and incomplete
arms invalidate the run. Cold latency is the first populated reopen and steady
latency uses three repetitions per case.

- primary supporting-evidence recovery is fractional gold-session recall; its
  paired treatment-minus-A0 one-sided 95% percentile-bootstrap lower bound is
  at least zero using 10,000 case-level resamples and seed 20260830;
- answer correctness uses the same paired bootstrap and boundary;
- any-gold and all-gold session recovery are reported as secondary metrics;
- groundedness and attribution point estimates do not regress versus A0;
- citation-contract validity is 100%;
- every returned item remains attributable to a canonical source;
- context size is at most 20;
- cold retrieval p95 is at most 100 ms and steady retrieval p95 is at most
  75 ms on the registered host; and
- the run is complete within its approved model/cost cap.

Passing these criteria makes the profile eligible for native HippoRAG-2. It
does not make it the general default.
