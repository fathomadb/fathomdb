---
title: Inspector 0.2.0 Slice 90 — local release design
status: APPROVED
desc: Promote exact verified bytes into a versioned ignored local distribution
---

# Slice90 local release design

[Plan](plan.md) and the [owner ruling](../release-decisions.md) govern. Consume
completed80's tested method and actual candidate, never an assumed future script.

```text
80 qualified candidate + metadata/checksums
  → verify exact source/build/bytes
  → owned dist staging + integrity checks
  → dist/inspector/0.2.0/ final local bundle/archive
  → fresh-extract/native smoke + independent Terra
  → tracked local release notes/status (binaries remain ignored)
```

Fresh direct Sol-high review approved application of the completed80 design on
2026-11-21. No architecture, requirement, product or script addition is needed.
The tested protocol refuses every existing destination, including identical ones;
manual verification is separate from promotion. [Status](status.md) records actual
release execution and final installed verification.

Use same-filesystem staging and an explicit final destination contract from80.
Preserve unrelated/existing releases; refuse every existing destination. Verify an
existing release separately without overwriting it. No uncontrolled path removal or output outside owned local dist.
The final metadata states local distribution and the candidate's real tested source
and build, distinct from later documentation commits. Checksums detect changed bytes;
they do not imply a registry signature, remote publication or new source qualification.

The tar and extracted binary must match the tested candidate and declare0.2.0,
Linuxx64, the qualified glibc/kernel/Chromium pairing, unchanged Engine/schema,
CPU enabled and fixture controls absent. Product operation requires existing
Chromium/model prerequisites; developer test runtimes are not bundled. Verify final
paths/checksums/gitignore and actual application launch/use/close, with only owned
processes and disposable fixture copies. Preserve original benchmarks and datasets.

No new code is expected. If release behavior needs correction, use actual RED/GREEN,
Sol-high design/code review and scoped validation before continuing. Terra verifies
final artifacts and documented installation independently. Report actual local success
only after those final-destination checks; registry/tag/push/main merge are excluded.
