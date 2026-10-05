---
title: FathomDB 0.8.27 Slice 115 independent verification
status: PASS_QUALIFIED
implementation_sha: 012e132920147396ac195f14af74444dd698f48e
---

# Slice 115 independent verification

`gpt-5.6-terra` high independently verified implementation commit
`012e132920147396ac195f14af74444dd698f48e`. Its read-only receipt
recomputation on a disposable copy matched the committed summary byte for
byte (SHA-256
`d238ba528f5af8e217e249f0d0f4a5f8addb84febf56b553c7beb85a92a8d210`).
It checked the runner bundle, corpus, protocol, lockfile and profile digests;
the twelve paths each contain seven valid samples, and the selected deeper
profiles have usable unprofiled controls. The engine source diff from the
measurement baseline is empty.

Terra's 15 focused tests passed, including the negative Gitleaks scanner
case. Ruff on the Slice 115 scripts/tests and repository Markdown lint passed.
It independently reproduced the Slice 90 root-reconciliation blob mismatch at
untouched baseline `4ca09d443d7bcc24072e39fb3c683905a9134f27`: actual
engine `lib.rs` blob `4417d3ad27122c38a1668ca746968af344418905` versus recorded
`e395a9b0e75928bcdea6528c71bba65c091050de`.

Terra did not run the Python test-hook wheel path because it writes an ignored
native module into the source package. The primary thread subsequently ran a
focused clean-checkout Python suite and verified the native wheel receipt as
recorded in [status](status.md). No full-workspace green claim follows from
these scoped checks.
