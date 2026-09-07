---
title: Slice 60 Windows lifecycle oracle correction 17
status: GREEN
candidate: dc488bc7b210a9ebe65499615311b83af71c9266
---

# Slice 60 Windows lifecycle oracle correction 17

The exact Windows candidate produced the allowed fail-closed lifecycle result
after the source rows had committed their deletion:

```text
ErasureIncomplete {
  stage: "wal_checkpoint",
  detail: "`excise_source` deleted its rows, but
  `wal_checkpoint(TRUNCATE)` reported BUSY on all 5 attempts
  (318 frames still in the log)"
}
```

The pre-correction test used `unwrap()`, so it treated the documented
at-rest-completion refusal as though the source rows had not been erased. One
unchanged isolated Windows reproduction then passed 1/1 in 0.22 seconds, which
confirms the clean-or-BUSY shape without attributing the BUSY outcome or
holder.

Independent review classified this as a P2 test-oracle defect. The product
must continue to refuse success while WAL truncation remains BUSY. The
test-only correction therefore accepts only `Ok` or
`ErasureIncomplete { stage: "wal_checkpoint" }`, rejects every other error,
and retains both real `graph_expand` disappearance assertions after the
committed erase and excise calls. It changes no retry, timeout, checkpoint,
runtime, cleanup, public API, or product behavior.

The Windows failure at exact candidate `dc488bc7` is the RED witness. GREEN is
limited to `slice60_fix3_runtime` on Linux and Windows before the remaining
unreached Windows fixtures resume.
