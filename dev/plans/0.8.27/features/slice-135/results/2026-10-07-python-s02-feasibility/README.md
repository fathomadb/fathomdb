---
title: Slice 135 installed Python S02 whole-sequence feasibility
status: INTERIM_FUNCTIONAL_FEASIBILITY
target_release: 0.8.27
---

# Installed Python S02 whole-sequence feasibility — 2026-10-07

One fresh-database sequence passed through each **installed wheel**, baseline
0.8.26 and the post-Slice-132 0.8.27 candidate. The
[independent audit](audit.json) checked both materialized observations and
rejected a retained-edge mutation. This is a functional S02 feasibility result,
not a frozen or sampled S02 latency comparison and not the full Phase 1
functional exercise gate.

The [verification record](verification.md) retains the full workspace gate:
184 of 186 suites passed, including the new S02 harness, with two previously
observed unrelated suites still failing.

## Source and run identity

| Boundary | Exact source | Wheel SHA-256 | Installed native SHA-256 |
| --- | --- | --- | --- |
| Baseline | `f99e002f0d2e4002f3694c9f8d4986b56089edaa` | `7c11803d9629550be60a76e1549c4c0bb42d23978d9fc901cf5df3c463611282` | `269fafc1c696244d05f2eabea19b23cfba6de79f5041b1123fe10589c46c51dc` |
| Candidate | `b2ac8081e79a6e626d01f5f97331be331f1cc16d` | `6713ade54d62cc1e067fcaf1982a539c1fb8bd3d45d3db18e41155badd0c12e8` | `4a3da4cda21841942de2fc010b81062f60da3c802401cbdb7184a0eee93a3f65` |

The wheel/source build links are in the [baseline installed-wheel receipt](../2026-10-07-python-wheel-baseline/README.md)
and [candidate installed-wheel receipt](../2026-10-07-python-wheel-qualification/README.md).
This run verified installed files against those wheel bytes. The retained
[runner](runner.py) SHA-256 is
`75424eb9b939530e4ac039d0e1e4ac5d112ea0fa084cc406688f68c10fac550d`;
the retained [S01 corpus helper](s01-helper.py) SHA-256 is
`91574e04c212c27c57cad23aa0b8ddb16b091966d2109b38cca4f9d50734c17b`.
The [focused contract tests](runner-contract-tests.py) passed 3/3, including
missing vector, retained edge, evidence and erasure negative controls.
`Cargo.lock` is the same SHA-256 for both source checkouts:
`9e9d7b5e82184a0bddfbe96de28fa0615ef4443fc84639b271c1cff29600ccfe`.

The successful attempts ran serially, baseline then candidate, through
`scripts/slice135_python_s02.py` with `PYTHONDONTWRITEBYTECODE=1` and a
180-second external timeout. Each used its isolated wheel-installed Python
environment, its matching `--wheel`, `--wheel-sha256`, `--source-sha` and a
unique `--output`. The precise argument values above and the raw output's
artifact paths bind the commands. From the worktree root, they were:

```sh
PYTHONDONTWRITEBYTECODE=1 timeout 180s \
  /tmp/slice135-python-venv-baseline-f99e002/bin/python \
  scripts/slice135_python_s02.py \
  --wheel dev/plans/0.8.27/features/slice-135/results/2026-10-07-python-wheel-baseline/fathomdb-0.8.26-cp310-abi3-manylinux_2_39_x86_64.whl \
  --wheel-sha256 7c11803d9629550be60a76e1549c4c0bb42d23978d9fc901cf5df3c463611282 \
  --source-sha f99e002f0d2e4002f3694c9f8d4986b56089edaa \
  --output /tmp/slice135-s02-functional-v2-2026-10-07/baseline.json

PYTHONDONTWRITEBYTECODE=1 timeout 180s \
  /tmp/slice135-python-venv-candidate-b2ac808/bin/python \
  scripts/slice135_python_s02.py \
  --wheel dev/plans/0.8.27/features/slice-135/results/2026-10-07-python-wheel-qualification/fathomdb-0.8.26-cp310-abi3-manylinux_2_39_x86_64.whl \
  --wheel-sha256 6713ade54d62cc1e067fcaf1982a539c1fb8bd3d45d3db18e41155badd0c12e8 \
  --source-sha b2ac8081e79a6e626d01f5f97331be331f1cc16d \
  --output /tmp/slice135-s02-functional-v2-2026-10-07/candidate.json
```

Those commands ran with GNU Time around each Python process; its full reports
are retained beside the JSON. The auditor can be rerun from this directory with:

```sh
python3 audit.py \
  ../2026-10-07-python-wheel-baseline/fathomdb-0.8.26-cp310-abi3-manylinux_2_39_x86_64.whl \
  ../2026-10-07-python-wheel-qualification/fathomdb-0.8.26-cp310-abi3-manylinux_2_39_x86_64.whl
```

## Product behavior exercised

On a fresh real SQLite database, the sequence opened with the default
BGE-small embedder, wrote 32 ordinary documents plus a canonical source, two
derived nodes and one provenance-bearing edge, configured a searchable vector
projection, drained it and checked `ready`. It checked both anchor bodies,
materialized a text hit, a vector-bearing result and a hybrid API result,
froze read context, resolved search evidence and both graph target/edge
evidence references, erased the graph source, retried erasure, closed and
reopened. It then checked retained corpus state and projected readiness.

Both [baseline](baseline.json) and [candidate](candidate.json) returned the
same checked observations: text hit `A`; a vector branch; hybrid result
containing `A` and a separate lexical query finding `A`; exact canonical
source bytes for claim evidence; graph target `s02-claim` via edge `s02-edge`;
erasure reports of 3 nodes and 1 edge, then zero on retry. After erasure and
after reopen, SDK reads found the erased records absent and anchor `A`
retained. Independent SQLite reads counted **zero graph-source canonical
nodes and edges** and **32 corpus-source nodes** at both points. The selected
hybrid result displayed only `vector` branches; the lexical control establishes
eligibility, not a text contribution to that fused result. This fixture uses
`EvidenceSearchRequestV1(limit=1)`; broader evidence limits remain untested.

## Timing and limits

The raw JSON retains per-call stages and a whole wall time that **includes
validation reads and checks**. Baseline whole wall was 5,331.350 ms and
candidate 5,295.178 ms in these single successful attempts. Reopen with the
default model accounted for about 4.4 seconds in each. GNU Time reported
5.58/5.53 seconds process wall time, 449,108/316,348 KiB peak RSS, zero
child swap and zero process failures for baseline/candidate. These values are
diagnostic only: there was no baseline noise pilot, alternating paired blocks,
frozen whole-sequence timing boundary, concurrency condition, or p50/p95/p99
sample. No regression or improvement can be inferred from this pair.

Earlier exploratory candidate attempts failed in the harness: the first
requested evidence for a wider unprovenanced mixed result set and received
`EvidenceError: evidence_incomplete at /provenance`; the second omitted the
required `depth` argument in the post-erasure graph check. Their full raw
stderr was not saved, so they are excluded from this receipt and cannot be
counted as product failures or performance samples. The first is a narrower
evidence-contract question for the broader operation map. The successful
pair's [stdout](baseline.stdout), [stderr](baseline.stderr),
[resource report](baseline-resource.txt) and corresponding candidate files
are retained; the output streams are empty. The audit compares raw values,
source and wheel hashes and native bytes, but it does not independently
replay the deleted temporary databases or prove complete erasure of all
projection/FTS artifacts. A full S02 receipt needs those state checks and the
declared Phase 1 sampling protocol.
