# GLOBAL-01 witness execution note

**Status:** resumable compact-encoding correction; no held-out questions
executed.

## Evidence

- A/A passed over 16 preserved answers and 128 metric judgments.
- Tie rate was 1.0 and maximum side preference was 0.
- A/A spend was $0.060695.
- The first control map for the witness produced three invalid responses.
- Each response used 6,035 prompt tokens and exactly 300 completion tokens.
- All three response hashes were
  `1cf4b12338cd300338cbf812f51fda3b929aab1ae713ae0682ade8b2f4473c47`.
- The checkpointed campaign spend after the stop was $0.0881576.
- No held-out question was executed.

The identical responses at the exact output ceiling identify deterministic
schema truncation, not an attribution or retrieval failure.

## Correction

Semantic revision `v2-bounded-map` instructs control maps to return at most two
claims and treatment maps to return at most four claims. Each claim is limited
to 30 words and the minimum sufficient supplied citations. The 300-token
control and 600-token treatment ceilings are unchanged.

Revision-specific invalid-cell paths preserve the original failures and allow
an idempotent resume. Inputs, split, models, scorer, acceptance boundaries, and
the $12 hard cap are unchanged. Resume the same artifact root; proceed to the
held-out comparison only if the three-question witness validates.

The bounded revision completed the first five control batches. The sixth batch
then produced three more responses at the exact 300-token ceiling, exhausting
its revision-specific retries. Campaign spend at the second stop was
$0.16363388; no complete answer or held-out work existed.

The successful bounded maps contained two claims of 13–28 words and one source
each. The remaining avoidable output cost was repetition of the canonical
source UUID and 64-byte content hash inside every generated claim. Semantic
revision `v3-compact-source-refs` gives each supplied source a short local
reference such as `S0`. The generator emits those references, and the runner
deterministically restores the registered canonical ID and hash before
persistence. Claim limits, evidence, source attribution, model/token budgets,
and decision boundaries remain unchanged.

## Artifacts

- External checkpoint:
  `data/performance-benchmarking/global-01/runs/global-01-lazy-coverage-20260829-a/checkpoint.json`
- [Measurement contract](2026-08-29-global-01-lazy-coverage-contract.md)
- [Authorized preflight receipt](../../experiments/runs/global-01-lazy-preflight-20260829T1922Z-aa159044/record.json)
