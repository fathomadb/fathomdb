---
title: Slice 135 current-source installed SDK capability exercise
status: AUDITED_FUNCTIONAL_PENDING_FINAL_ARTIFACT_RETENTION
target_release: 0.8.27
---

# Current-source Python and TypeScript capability exercise

The [manifest](manifest.json) binds this functional exercise to candidate
source `d465cd56d2e863f900ea9a9da8bc372ca6a077c5` and the retained
artifact and receipt hashes. The wheel and Linux x64 GNU npm archives still
carry package version 0.8.26; the source SHA and hashes identify this 0.8.27
candidate. These archives are retained locally here pending the end-of-phase
raw-archive review. They are not yet committed branch evidence.

The installed Python wheel (`fathomdb-0.8.26-cp310-abi3-manylinux_2_39_x86_64.whl`)
ran 48 selected cases with 48 independently checked reopened real-database
snapshots. Its [raw receipt](python-raw.json) partitions all 44 governed
operations into **41 executed, zero failed, zero supported gaps, and three
unavailable provider/model cases**. The new committed closure test asserts the
keyed `complete` proof and equality after reopen; the older absence and typed
schema-refusal route remains the negative case. The
[independent audit](python-retained-audit.json) recomputed the operation
partition, source/test hashes, installed wheel bytes and persisted-state
snapshots; it rejected false count and changed-state controls.

The installed TypeScript packages (`fathomdb-0.8.26.tgz`) and
native archive (`fathomdb-linux-x64-gnu-0.8.26.tgz`) ran 43 selected cases,
each against a real database with a reopened-state check. The
[raw receipt](ts-raw.json) reports the same **41/0/0/3** operation partition.
The existing committed-closure case and a new successful dependency-trace
case are both positive routes, paired with existing refusal cases. The
[live audit](ts-live-audit.json) and
[retained-archive audit](ts-retained-audit.json) independently accepted the
source, selected case names, adapted test bytes, npm archive members, raw
results and four negative controls. Both auditors now reject replacing the
positive closure/trace routes with absence/refusal cases. The selected
adapted tests (`adapted-tests.tgz`) are retained for that recheck.

Two [invalid TypeScript attempts](invalid-attempts.json) preceded the accepted
run: the default npm cache was read-only, then sandboxed child Node processes
returned `EPERM`. The second attempt's [raw receipt](invalid-sandbox-raw.json)
is retained and excluded from the accepted counts. The successful run used a
temporary npm cache and an executor that permitted the same isolated child
tests. These are execution-environment faults, not product failures.

This closes the three previously supported positive-path gaps in the selected
operation register. It does not prove every condition of those operations,
qualify the three provider/model cases, establish other platforms, or refresh
the final-source latency and coverage campaigns. No full workspace gate was
run for this focused functional exercise.
