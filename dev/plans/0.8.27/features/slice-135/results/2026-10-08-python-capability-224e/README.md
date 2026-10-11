---
title: Slice 135 exact-source installed Python capability exercise
status: AUDITED_SELECTED_OPERATION_EXERCISE
target_release: 0.8.27
---

# Installed Python capability exercise at source 224e44c59

The exact candidate wheel SHA-256
`ee8b402f76377956034f900ef69f9e3c0296d79ea30d6728dc1283040266f85a`
was installed in an isolated Python environment and exercised against the
44 live governed operations at source
`224e44c593c13d86ece648adabe445723db04070`. The [raw operation and
case receipt](raw.json) records **41 executed, zero failed, zero supported
gaps and three unavailable provider/model cases**. It includes 48 selected
real-database cases with negative calls and reopened-state assertions.

The [independent audit](independent-audit.json) recomputed the operation
partition, exact wheel/native bytes, fixture and runner identities, and all
48 retained reopened SQLite snapshots. It passed and rejected changed
persisted counts and a false operation count. Raw SHA-256 is
`b30a0667aeb0a43eef653226a167459054501809775f3132b7951ed400488364`;
audit SHA-256 is
`1996b63a34bd5b513a52061f31e0b60ff0474892cfba61b0cbe1ae0f98a49874`.

The first invocation failed before case execution because the isolated wheel
environment lacked the runner-required `pytest`; it wrote no receipt. After
installing pytest into that same environment, the exercise and audit passed.
Selected calls do not cover every error, filter, concurrency or platform
condition. The three provider/model cases remain unavailable, and no
baseline Rust SDK peer or other-platform Python wheel is implied.
