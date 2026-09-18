---
title: FathomDB 0.8.26 Slice 60 — independent design review
status: PASS
reviewed_on: 2026-09-17
---

# Slice 60 independent design review

## Round 1

Changes were requested for two P1 and two P2 findings:

- the proposed CLI successor overstated `recompute-mean` as canonical-source
  reconstruction and ignored shared pre-writer maintenance;
- a merely reviewed ADR could not supersede accepted authority;
- inventory witnesses were categories rather than resolvable paths/symbols;
  and
- the RED/GREEN and verification commands were not executable from the plan.

## Resolution

The repository owner explicitly authorized the narrow successor. The ADR now
binds only the command-owned mean/vector transaction, separates shared
pre-writer maintenance, preserves governed SDK erasure, and keeps `recover` as
the CLI data-loss-authorized operator-recovery root. The predecessor backlink
and decision index record acceptance.

The inventory is pinned to planning baseline `b770de01`, retains its bounded
claim clusters, and names concrete authority, implementation, and test
witnesses. The plan names the exact focused/full commands and product-source
diff guard.

## Rereview verdict

PASS. No P1 or P2 design finding remains. The owner profiles, review ordering,
and separation from Slice 65 are appropriately scoped. `git diff --check`, the
design-lifecycle checker, Markdown lint, and the baseline-relative product-code
diff passed during review.

## Authorized recovery addendum review

The post-implementation-review recovery expansion received a separate
independent design review. The first pass requested four P2 clusters:

- distinguish the authorized product exception from the original empty-product-
  diff rule and from default SDK surface;
- specify the public operator-feature Rust/CLI contracts and exact exit classes;
- pin the canonical lock, main-file validation, WAL classification, and
  SQLite-owned checkpoint sequence without raw sidecar mutation; and
- name executable RED/GREEN targets, including the existing Busy test and
  default-feature absence proof.

Rereview requested two residual P2 corrections: the discard field must be true
only when malformed classification and `Done` coincide, and default-SDK absence
needed an explicit compile-fail doctest command. Both are now explicit in the
plan/design.

Final addendum verdict: **PASS**, with no unresolved P1/P2 finding. The design
preserves fail-closed public open, uses no raw WAL/SHM mutation, keeps
malformed-header safe export fail-closed, and limits the product exception to
one operator-feature WAL recovery function plus CLI dispatch and tests.

## Implementation-alignment rereview

The first implementation-alignment pass found one P1: effective schema
validation used the recovery read/write connection, whose close could
checkpoint a healthy but noncurrent WAL while returning refusal. It also found
P2 gaps in schema-cookie fixtures and wording that did not distinguish the
standalone main file from SQLite's effective main-plus-WAL view.

The correction validates healthy/absent WAL state through `mode=ro` plus
`query_only` before opening any read/write connection. Malformed WAL still
requires schema 34 in the standalone main. Deterministic
`SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE` fixtures now cover pending-current success,
effective-noncurrent byte-preserving refusal, and malformed-WAL plus
noncurrent-main byte-preserving refusal. The plan, design, and CLI contract use
the same distinction.

Final implementation-alignment verdict: **PASS**. No P1/P2 finding remains;
the focused engine recovery suite passed 13/13.

## Adversarial hardening rereview

Code review found that a non-Fathom SQLite database stamped with
`user_version = 34` could reach malformed-WAL discard, that the final recovery
open retained create permission, and that read-only effective-view validation
could change transient SHM bytes on refusal. The correction reuses current
open-time dependency, frozen-read, and dependency-closure schema invariants,
restores the preflight SHM snapshot on refusal, and opens the final recovery
connection with `mode=rw` and no create flag.

The first rereview requested two P2 corrections: do not promise byte
preservation for a `Busy` checkpoint that may partially checkpoint, and bind
healthy-WAL effective-invariant refusal with its own negative fixture. The
acceptance criterion now scopes byte preservation to preflight refusals. A
schema-33 main plus healthy schema-34 WAL with a removed required frozen-read
trigger returns `SchemaInconsistent` while preserving exact database, WAL, and
SHM bytes.

Final adversarial-hardening verdict: **PASS**. No P1/P2 finding remains; the
independent reviewer reran the focused engine recovery suite at 15/15 and
confirmed plan, design, interfaces, implementation, and tests agree.
