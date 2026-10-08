---
title: Slice 135 installed Python functional smoke at source 224e44c59
status: FUNCTIONAL_SMOKE_NOT_TIMING_COMPARISON
target_release: 0.8.27
---

# Installed Python S01/S02 smoke at source 224e44c59

A clean detached checkout at `224e44c593c13d86ece648adabe445723db04070`
produced a non-editable wheel with SHA-256
`ee8b402f76377956034f900ef69f9e3c0296d79ea30d6728dc1283040266f85a`.
Its installed native module has SHA-256
`1f213a700cf0c96b9997700d6e44ed7019b26fb91df1259bc6514fefb4a8d013`.
The checkout was clean and passed `scripts/preflight.sh --worktree` before the
build. The wheel retains the pre-release `0.8.26` package version; source and
wheel hashes identify these candidate bytes.

The [32-row S01 raw result](s01-32.json) and [256-row result](s01-256.json)
each contain session-first, warmup and ten warm installed-wheel calls for text,
vector and hybrid search. All 72 materialized calls passed seeded-ID and
branch checks. The [S02 raw result](s02.json) contains one fresh-database
open, 36 writes, projection readiness, text/vector/hybrid retrieval,
graph/evidence, erasure, close and reopened-state sequence. Its 18 stages and
reopened counts passed. The [independent check](audit.json) recomputed hashes,
artifact identity, result shape, stage and state assertions from the raw
records; deliberately changed S01 text IDs at both sizes and a retained
graph edge after S02 reopen were all rejected.

The wheel remains at `/tmp/slice135-current-wheel-224e44/` and the isolated
installed environment at `/tmp/slice135-current-venv-224e44/` pending final
artifact retention. S01 has only ten warm samples per cell and S02 is one
validation-inclusive sequence. These runs establish functional feasibility
for an exact-source paired protocol, not comparative latency or p99.
