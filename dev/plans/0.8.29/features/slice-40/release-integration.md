# Proposed Slice 40 release requirement and plan mapping

Status: bounded proposal for main-session integration; no release master,
canonical requirements/acceptance, release JSON, generated region or ledger is
edited. Main session owns any eventual durable state update.

## Existing release requirement

Retain master **N29-S / R29-REL-S**: exact retrieval holds up at scale. Slice40
contributes only measured eligibility/batch cost reduction that preserves exact
snapshot/pre-cap behavior. [R40-1–10](requirements.md) and [A40-1–5](acceptance.md)
are local refinements. Slice30 supplies current baseline; Slice50 retains its
independent identity census/ADR and exact-search decision. 40 is preferred
before50, not its semantic gate.

Suggested non-generated master prose: “Slice40 remains conditional on actual30
read-mostly and write-interleaved evidence and a reviewed concrete snapshot or
batch-invariance design. Existing caching/hoists are baseline. Each opportunity
may be selected, not selected or deferred with evidence; no measurements exist
yet. Missing inputs cannot be replaced by an invented prototype or schema.”

## Unchanged acceptance and targets

- Preserve accepted AC-072/076 binding 10k latency tiers and their existing sample/statistic rules; larger tracked tiers do not become new gates. Do not reinstate superseded unconditional 100k legacy budgets.
- Preserve current accepted AC-075 TC5 reconciliation and exact frozen bridge inputs, including recall floor 0.90/accepted CI rule and unchanged candidate/rerank configuration. Canonical historical EU7 prose is not authority to substitute a different pack; accepted.28 qualification/reconciliation governs current receipt selection.
- Preserve all other applicable accepted release gates, including read-performance/projection correctness contracts; Slice40 invents no gate waiver, page-cache confirming run or new performance target.
- AC-083 remains owner-authorized **tracked, not gated**: 100k LongMemEval text p50≤80 ms and p99≤300 ms. This package neither mints its canonical entry nor claims it measured/passed.
- No new canonical AC numbers or changes to locked `dev/acceptance.md`/`dev/requirements.md` are proposed. Concrete public API/interface/ADR changes, if unexpectedly required, return for review rather than enter through local optimization requirements.

## Present readiness

Slice10 complete; integrated20 Linux subset is not whole20 exit. A3/A5 TDD
exception and WindowsA6 remain pending. 30 still lacks its full20 gate, exact
TC5 bridge pack on disconnected UUID `C8D47C11D47C03CA`, and all actual.29
measurements. No new live Memex database access/copy/write is part of40.
Slice60 source work is elsewhere;70 reason-enum proposal and80 stored-span
aggregate proposal are not settled source contracts for40 to absorb.

This package can receive a **bounded design review**. It cannot justify source
entry, treatment selection, runtime acceptance, release readiness or publication.
After actual30, rebase the source inventory/proofs on integrated predecessors,
review the exact proposed treatment and prospective tolerances, or record a
supported not-selected/deferred disposition. Do not mark40 complete today.
