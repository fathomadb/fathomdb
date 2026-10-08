---
title: Slice 135 Phase 2 installed-Python deterministic query pilot
status: AUDITED_PAIRED_CONTRACT_PILOT
target_release: 0.8.27
---

# Phase 2 installed-Python X1 query pilot — 2026-10-08

The [frozen pilot protocol](../../phase2-python-x1-pilot-protocol.json), SHA-256
`6698d77a8a113b0cb2da3b40144867e1f76b062946464e6a330b66af6c302855`,
was written before the paired raw runs. It binds the existing authored
[search fixture](../../../../../../../src/python/tests/functional_search_fixture.json)
and its separately asserted retrieval order, exact baseline and candidate
wheel/native bytes, and the [unscored runner](../../../../../../../scripts/slice135_phase2_contract_runner.py).
The [recorded commands](run-commands.json) used isolated installed-wheel
environments and separate fresh SQLite databases with default embedding off.

The independent [file and database audit](audit.json) recomputed each
version's result against the fixture, not against the other version. Both
0.8.26 source `f99e002f0d2e4002f3694c9f8d4986b56089edaa` and exact
candidate product source `224e44c593c13d86ece648adabe445723db04070`
matched all three authored query body sets, explicit logical IDs, kinds,
text-branch and source identity before close and after fresh reopen. The
authored `retrieval` result order matched exactly. Each retained database
passed SQLite integrity and contained exactly the four expected active
canonical rows. The [baseline](raw-baseline/raw.json) and
[candidate](raw-candidate/raw.json) raw receipts retain individual hits and
environment identities; their corresponding SQLite files are retained
beside them. The [SHA-256 manifest](SHA256SUMS) binds the receipt files.

Five [negative controls](negative-controls.json) changed copies of the
candidate raw archive and were rejected: wrong hit ID, wrong retrieval order,
missing query, changed wheel identity, and altered persisted body. The
original raw files remained unchanged. The auditor's focused tests also
cover duplicate hits, source provenance, and tampered wheel and oracle files.

The unchanged full `./scripts/agent-verify.sh` gate passed after rerunning in
a ptrace-capable executor. The first sandboxed attempt reached AC-036 and
reported only its `PTRACE_TRACEME` environmental blocker. The successful
[verification output](verification.stdout) records AC-036 and live AC-037
passes, zero security violations, blockers or downgrades, and 186/186 test
suites passing. The gate used a temporary Git exclude for untracked Phase 1
raw outputs; it did not remove or alter them.

This is one narrow deterministic contract cell on a four-row FTS-only
fixture. It does not qualify typed-error versus empty-success, filters,
pagination, graph/evidence, erasure, vector fidelity, retrieval relevance,
memory usefulness, answer quality, other SDKs, or other platforms. Those
remain in the [Phase 2 plan](../../plan.md#phase-2-qualification-and-execution-order).
The full Phase 2 gold protocol and six-dimension result are still open.
