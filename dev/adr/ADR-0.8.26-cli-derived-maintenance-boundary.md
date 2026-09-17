---
title: ADR-0.8.26-cli-derived-maintenance-boundary
date: 2026-09-17
target_release: 0.8.26
desc: Preserve recover for data-loss-authorized work while grandfathering one atomic derived-vector maintenance doctor command
blast_radius: requirements REQ-036; interfaces/cli.md; design/recovery.md; decision index
status: accepted by explicit repository-owner authorization on 2026-09-17
supersedes_in_part: ADR-0.6.0-cli-scope.md
---

# ADR-0.8.26 — CLI derived-maintenance boundary

## Context

The accepted 0.6.0 CLI-scope ADR classifies every `doctor` command as
bit-preserving/read-only and every non-bit-preserving operation under
`recover --accept-data-loss`. The shipped `doctor recompute-mean` command is an
older counterexample: it atomically recomputes the stored mean and requantizes
derived vector rows. It preserves canonical records and provenance but is not a
read-only or bit-preserving database operation.

Slice 60 found the contradiction while reconciling current owner documents. A
documentation rewrite cannot silently override the accepted ADR, while moving
or renaming the shipped command would be an unrelated public/runtime change.

## Decision

Preserve the two operator roots and narrow their current boundary as follows:

- `recover --accept-data-loss` remains the exclusive CLI operator-recovery root
  for actions that may discard canonical or operational data, truncate
  recovery state, perform data-loss-authorized recovery/rebuild workflows, or
  otherwise require explicit acceptance of data loss. Governed SDK `purge` and
  `erase_source` remain separate non-CLI erasure routes.
- `doctor` remains the diagnostic, inspection, export, and environment root.
  It may also retain exactly one already-shipped mutable exception:
  `doctor recompute-mean`.
- `recompute-mean` is classified as atomic, non-lossy derived-vector
  maintenance. Its command-owned transaction derives the mean from retained
  uncentered `vector_default.embedding` values, updates
  `_fathomdb_embedder_profiles.mean_vec`, and recreates/requantizes
  `vector_default` rows while preserving their governed metadata. A failure
  rolls that transaction back.
- Normal engine admission and shared pre-writer dependency-closure maintenance
  still run before the command-owned transaction. Their existing owners govern
  any operational-state effect; this ADR grants no new doctor authority over
  dependency, lifecycle, or operational state.
- This exception is closed, not extensible. Any additional doctor command that
  mutates the product database requires a successor decision. It cannot cite
  this ADR as general authorization.

Database-free diagnostics, immutable inspection, engine-backed readout,
artifact export, and cache warming retain their existing effect classes. The
SDK boundary is unchanged: neither `doctor` nor `recover` becomes an SDK root.

## Consequences

REQ-036 and the recovery design must stop calling every doctor action
read-only/bit-preserving. They must name the single derived-maintenance
exception and retain the stronger data-loss boundary around `recover`.

No command, flag, output, exit code, schema, binding, or runtime behavior
changes. The decision records the already-shipped boundary and prevents that
exception from becoming an accidental general-purpose mutation root.

## Supersession

This ADR supersedes only the doctor-wide “bit-preserving / read-only” clause in
`ADR-0.6.0-cli-scope.md`. Its two-root operator shape, absence of application
query/write verbs, mandatory `--accept-data-loss` recovery acknowledgement,
and concrete interface ownership remain in force.

## Authority

The repository owner explicitly authorized this narrow successor on
2026-09-17 after Slice 60's independent design review exposed the accepted-ADR
conflict. The authorization preserves shipped behavior and closes the
documentation authority gap; it does not authorize another mutable doctor
command.
