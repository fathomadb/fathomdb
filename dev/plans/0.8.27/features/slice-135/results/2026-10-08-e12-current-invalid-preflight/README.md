---
title: Slice 135 rejected E01–E12 baseline binary attempt
status: INVALID_PRETIMING_ATTEMPT
target_release: 0.8.27
---

# Rejected E01–E12 baseline binary attempt

The first attempt under
[the initial current-source freeze](../../e12-current-comparison-protocol.json)
stopped before any timed workload block. Its [schedule](schedule.json) contains
one invalid entry. The [adapter stderr](query-pair-1-1-baseline-adapter.stderr.log)
reports `reused binary workload_sha256 mismatch`: the selected older baseline
binary had workload hash `c4f1115b`, while the frozen workload hash is
`97aa8554`. No candidate timing ran and no comparison is claimable from this
attempt. The [replacement freeze](../../e12-current-comparison-protocol-v2.json)
names the previously audited compatible baseline smoke binary with hash
`401d2375c66c0cd72c81ba8767f777cf79c9ce609c52eb4ceccf2405b4bcb251`.
