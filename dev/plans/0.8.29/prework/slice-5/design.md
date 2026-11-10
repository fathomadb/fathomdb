# Slice 5 adequacy review design

Read relevant complete test bodies and in-source registration before claiming
an oracle/gap. Need→requirement→AC links alone are insufficient: ask how the user
outcome could fail while the mapped test passes. Retain strong real-store and
post-state controls; distinguish absent test, weak oracle, selector issue,
historical evidence and undecided contract.

Apply quality-roadmap owned-boundary faults, independent expected-state/reference
comparisons, Unicode/boundary properties and resource post-state selectively.
Shared production predicates limit apparent reference independence. Do not add
general SQLite fuzzing, all-allocation sweeps or a new numerical release gate.
Current environment is unprepared, so this is source analysis only.
