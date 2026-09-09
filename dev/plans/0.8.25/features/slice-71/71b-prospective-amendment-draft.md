---
title: Slice 71B — measurement clarification
status: APPROVED_AND_APPLIED
date: 2026-09-09
---

# Slice 71B measurement clarification

The performance standards do not change.

For the prospective Phase 5 recovery run:

- 1–1,000-row results use the already-approved median and conjunctive
  relative/absolute regression rule;
- Scale-02 10k acknowledgement and total must each have at most 25% spread;
- AC-013 10k total must have at most 25% spread; and
- AC-013 acknowledgement remains reported but diagnostic because asynchronous
  projection moves work between acknowledgement and drain without changing
  ingest-to-drained completion time.

This scopes measurement validity to the actual completion metrics. It does not
raise a latency boundary, discard an observation, or permit fast
acknowledgement to hide slow completion. Earlier invalid runs remain retained
and are not reused. The repository owner directed execution without a separate
permission gate on 2026-09-09.
