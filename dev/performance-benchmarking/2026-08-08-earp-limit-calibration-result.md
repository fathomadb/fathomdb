# EARP limit-calibration result

## Decision question and claim class

Does increasing `Engine.search_text_only` from its default limit of 10 to 50
preserve the visible top-10 prefix, and does the EARP paired-comparison
instrument detect a violation? This is a diagnostic calibration and regression
witness, not a retrieval-quality improvement claim.

## Systems and exact configurations

Both arms were FathomDB 0.8.22 development code, FTS-only, without the default
embedder. The sole changed knob was `scenario.query.limit`: omitted/default 10
for the control and 50 for the treatment. The shared resolved configuration
SHA-256 was
`b743776d53d3479ee69a597c73bad10409ab3d4ad4db760ec966c5de8ace709e`.

The pre-fix run used code commit `cb4ca4b0`. The predeclaration, finding, and
self-contained mechanism brief were recorded in commits `cb4ca4b0`,
`1b5769bb`, and `c28e2257`. The post-fix rerun followed PR #209's fixed
internal text-candidate bound at `f1ccf269` and was recorded by commit
`767a24b9` on `feat/earp-eval-platform-20260806`.

## Corpus, gold, and protocol

The paired runs used the frozen 10,506-document corpus with SHA-256
`fe973fcd49fbbda083158f69fe720f17858ab8528e171fa2188eec84131c7d4e`
and `ir-c-reused-v2` gold with 4,597 queries and SHA-256
`4caabddf7ce55f417e639e3c169fe2035b09c231f36d2f39d293a596373de2bb`.
The mixed-license corpus remains local; source licenses include MIT,
Apache-2.0, BSD-3-Clause, CC-BY-4.0, research-use, and
undeclared/upstream-chain material.

The predeclared metric was treatment-minus-control strict evidence recall at
K=10. Pairing used immutable query IDs, 1,000 paired-bootstrap resamples, seed
`0x0E88B007574A9001`, and minimum N of 1,000. The 125 negative queries were
typed metric-inapplicable, leaving 4,472 paired observations. No better-than
decision rule was declared.

## Result and uncertainty

Before the fix, 11 of 4,472 paired queries differed and every difference was
treatment-worse. The effect was -0.00246 with paired-bootstrap interval
[-0.0038, -0.0011]. A live probe identified rank fusion over caller-limit-
truncated input lists as the mechanism: changing the requested limit could
change which documents appeared in the top ten.

After the fixed internal candidate bound landed, all 11 witness queries were
prefix-stable. The same resolved configuration then produced effect 0.0,
interval [0.0, 0.0], and zero differing queries among 4,472 pairs. These
intervals describe paired query variation within each execution. There was one
corpus-scale execution per code state, so they do not quantify repeated-run or
environmental variability.

## Artifact availability

The local primary checkout retained the pre- and post-fix run directories:

- `experiments/runs/earp-comparison-limit-calibration-20260808T0841Z-b743776d/`
- `experiments/runs/earp-comparison-limit-calibration-20260808T1624Z-b743776d/`

The pre-fix `earp.result.v1.json` SHA-256 is
`9c46ce3f711739472ddbabcc1cd49d4a377e789bcc6997c1c1c605515edc9a4e`;
the post-fix result SHA-256 is
`a0d4b43585eff52d03919712424b23d29e2dc8bd81dc37c3e55c3a78d06e4e9b`.
Their per-query sidecar SHA-256 values are, respectively,
`73b448e2908ab7fcec22f6d60af651885b5705f8401bfc9f511e627f88f56629`
and `f0226b6885073e4c3e22f7a0edbd1e7e8de1a2e208d51fc7afb96c3b0759d1aa`.
These gitignored artifacts are not copied into this register.

## Nonclaims

This result does not show that `limit=50` improves retrieval, establish a
general recall level, or provide a repeated-run parity or performance verdict.
It shows that the predeclared calibration exposed a specific prefix-instability
mechanism and that the same instrument did not observe that mechanism after
the targeted fix.
