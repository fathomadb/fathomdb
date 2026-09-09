---
title: Slice 71B — attribution review record
status: PASS_WITH_CAMPAIGN_STOP
date: 2026-09-08
---

# Slice 71B attribution review record

## Protocol and harness review

Independent read-only review passed the immutable protocol and harness at
`76e42b90`. The reviewer confirmed exact matrix order, dependency and runtime
identity, direct total timing, treatment signatures, canonical stop receipts,
active-cell failure binding, manifest hashes, runner digest, valid schemas,
and 34 passing focused tests. No timed or broad verification was run.

## Evidence reviews

Two independent read-only reviewers passed the retained campaign evidence.
They confirmed:

- the exact 16-cell sealed prefix and immediate `spread_invalid` stop;
- AC-013 `generation_only` acknowledgement spread of 118.430% against 25%;
- total-time spread of 5.776% for that arm;
- valid environments for all completed cells;
- matching source, manifest, runner, probe, lockfile, executable, runtime,
  environment, raw-log, cell, and failure-disposition identities and hashes;
- valid treatment signatures and timing arithmetic; and
- no admissible causal attribution, correction selection, retry, outlier
  deletion, or threshold adjustment.

The final retention audit also confirmed that the copied 9,532,344-byte
executable is byte-identical to the receipt-bound temporary executable, with
SHA-256
`92604c16a0ad376ceeb291f9445222b2a8e3db307f99ac6cd7619c7f6713160f`.
After one factual documentation correction distinguishing 24 online CPUs from
the 12.0 load limit, its final verdict was PASS.

The reviewers inspected retained artifacts and did not launch duplicate
verification or measurement campaigns. The prospective amendment remains a
draft awaiting owner approval; this review does not approve that new protocol.
