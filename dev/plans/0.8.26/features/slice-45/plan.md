---
title: FathomDB 0.8.26 Slice 45 — architecture documentation convergence
status: APPROVED_FOR_IMPLEMENTATION
---

# Slice 45 plan — architecture documentation convergence

## Purpose and placement

Per placement ruling `seq-288`, make the maintained architecture documentation
current, coherent, navigable, and correct after Slice 40 fixes the as-built
product shape and before Slice 50 asserts release readiness. This is
documentation convergence, not permission to change product behavior or
rewrite historical decisions.

## Reconciliation since the draft

The draft entered the release branch at `75f2bff1` and was amended when Slice 8
closed at `7a568279`. The following changes and allocations were reviewed at
the Slice 40 head `7a34a668` before approving implementation:

1. **Release preparation changed the authority layer.** Slice 9 created the
   0.8.26 release state/board and corrected current release and CLI inventory
   truth. Those records now identify Slice 45 as the current unit.
2. **Frozen explanation is implemented, not planned.** Slice 10 routes both
   frozen explanation paths through finalized non-empty correlation identity,
   preserves explanation-off behavior, and adds maintained cross-SDK/public
   guidance plus an installed-wheel witness.
3. **Graph evidence shape is decided and implemented.** Slice 15 selected
   D26-01 option A (`seq-290`); Slice 20 added the opt-in positional V1 sidecar,
   frozen-only opaque resolver, exact target/terminal-edge identities,
   nondisclosure precedence, two-statement bounded hydration, cross-binding
   parity, and `ADR-0.8.26-exact-graph-artifact-evidence.md`.
4. **Operator integrity is qualified, not greenfield.** Slice 30 retained the
   CLI-only `doctor data-plane-integrity` boundary, made the complete process
   immutable and alias-safe, fixed exact-version artifact selection, and kept
   doctor/recovery absent from governed Python and TypeScript surfaces.
5. **Actuation and the database boundary are final.** Slices 35 and 40 retain
   one five-operation V1 grammar with `put_derived_edge`, complete-prospective-
   state endpoint validation, canonical edge storage, compact receipt/replay
   integrity, schema 34, and refusal of any nonempty noncurrent database before
   product mutation. There is no V2 router, migration, or historical replay.
6. **The maintained architecture entry points drifted.** `dev/architecture.md`
   still labels a locked 0.6.0 snapshot authoritative; the active data-plane
   architecture still advertises only its 0.8.25 v2.1 profile; and the dev,
   design, and interface indexes direct readers to stale release ownership.
7. **Allocated draft items are now resolvable.** Slice 3's architecture CRUD
   rows for frozen explanation, artifact evidence, graph identity, operator
   integrity, actuation/receipt, and the database boundary have all reached
   as-built state. Slice 4's reader/writer, evidence, operator, endpoint, V1,
   fresh-database, and policy-boundary invariants remain valid. Slice 5's
   focused proof allocations are satisfied by the owning slice records; the
   integrated package/platform matrix remains Slice 50 work.
8. **Deferred material stays deferred.** The 0.8.23 trade-off note remains a
   bounded historical input, broad `dev/design/` lifecycle reconciliation stays
   in Slice 46, and Slice 50 retains Windows inventory, registry polling,
   conditional generated-evidence registration, and integrated verification.

The concrete inventory and claim witnesses are in
[`inventory.md`](inventory.md). The verdict is **adjust and approve**: converge
the current architecture hierarchy and add one narrow recurrence guard, while
rejecting a broad design-tree rewrite, product changes, file moves, and release
verification work as overbuild.

## Slice-local need, requirements, and acceptance

- **N26-45:** A maintainer needs one discoverable current architecture that
  distinguishes as-built 0.8.26 boundaries from historical plans and snapshots.
- **R26-45A — authority:** The dev-doc entry points name exactly one active
  data-plane architecture; the locked 0.6.0 architecture is retained in place
  as superseded history with a successor pointer.
- **R26-45B — as-built profile:** The active architecture records the current
  workspace, engine/module ownership, read/write/concurrency paths, schema-34
  fresh-open boundary, frozen explanation/evidence, CLI operator boundary, and
  atomic derived-edge contract without copying detailed interface schemas.
- **R26-45C — decision fidelity:** The architecture agrees with accepted ADRs,
  ruled D26 decisions, maintained interfaces, implementation, and focused tests;
  later graph-shape resolution is linked without rewriting the original ruling.
- **R26-45D — lifecycle guard:** A focused validator fails when the historical
  entry lacks an explicit supersession banner, the successor is missing or
  non-active, more than one versioned data-plane architecture is active, or the
  two navigation indexes omit it. Release/profile equality is a Slice 45
  closeout assertion, not a permanently valid repository invariant.
- **R26-45E — scope:** Preserve unique rationale and stable paths; make no
  product, public API, dependency, schema, package, publication, or broad
  technical-design change.

Acceptance criteria:

- **AC26-45A:** `dev/README.md` and `dev/design/README.md` point to the same
  active architecture; `dev/architecture.md` is explicitly superseded and its
  successor resolves.
- **AC26-45B:** The active profile is reconciled to 0.8.26. Every as-built claim
  in the Slice 45 matrix has independent ADR/interface plus implementation/test
  witnesses; every normative ownership claim has independent accepted authority
  plus design-review evidence.
- **AC26-45C:** The architecture describes ten Rust workspace members, schema
  34, one primary mutex-serialized caller-mutation connection, serialized
  projection-worker commits, pooled readers, frozen authenticated evidence,
  CLI-only integrity inspection, one changed-in-place five-operation V1
  actuation grammar, and fresh-database refusal consistently with source and
  interfaces.
- **AC26-45D:** The new guard is demonstrated RED against the pre-convergence
  documents, then GREEN; its focused negative fixtures cover every R26-45D
  failure mode, prove both local and Markdown-only CI invocation, and it runs
  from both paths. The closeout separately uses `scripts/release-current.py` to
  prove that the converged profile matches live release 0.8.26.
- **AC26-45E:** Independent design review and implementation review pass, the
  independent verifier confirms semantic and mechanical truth, focused doc/
  guard checks and `./scripts/agent-verify.sh` pass, and Slice 46 is the only
  next dependency.

## Approved implementation workflow

1. Complete the inventory and code-grounded claim matrix; resolve only
   documentation drift and stop on a product/contract contradiction.
2. Obtain independent read-only review of this plan, the design, inventory,
   authority model, and proposed validator. Resolve findings before RED.
3. **RED:** add the focused architecture-authority regression suite and commit
   the observed failure against the pre-convergence entry points.
4. **GREEN:** implement the smallest validator, wire it into
   `scripts/agent-lint-md.sh` and the Markdown-only CI job, add a source-contract
   assertion for both call sites, mark the 0.6.0 snapshot superseded, reconcile
   the active architecture as v2.2/0.8.26, update navigation and DOC-INDEX rows,
   and add only non-substantive ADR relationship notes.
5. Refactor the validator only for clear diagnostics and fixture isolation.
   Obtain independent read-only code review of the actual diff and resolve any
   finding through an additional RED/GREEN pair when behavior changes.
6. Use a separate independent read-only agent to verify requirements, source
   claims, links, focused guard tests, Markdown/docs gates, Git chronology, and
   `./scripts/agent-verify.sh`. Broad package/platform regressions remain Slice
   50 because this slice changes no product artifact.
7. Write `status.md`, advance only the single-writer release state/board to
   Slice 46, commit closeout, and retain the existing release worktree. No new
   branch or worktree is planned.

## Stop gates

Stop and route the issue to the owning feature or HITL on an architecture/code
contradiction, uncertain authority, proposed decision change without a
successor ADR, destructive historical move, public-contract change, or scope
large enough to require product implementation.
