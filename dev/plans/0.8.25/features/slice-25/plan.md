---
title: 0.8.25 Slice 25 — atomic semantic actuation
status: DRAFT
depends_on: 20
---

# Slice 25 plan

## Outcome and carried obligations

Implement R25/AC25-25; Memex needs 3, 17, and 18; and A25-05. Add one typed,
model-free, idempotent batch for caller-decided records, dependencies,
facts/edges, lifecycle actions, consolidation verdicts, and metadata, with a
complete committed-consequence or whole-batch-refusal receipt.

## Verification routes

Selected: fast, heavy, all, all-feature, Windows CPU/native Rust/Python/Node,
and registry-installed batch/receipt smokes. Operator is selected if lifecycle
verbs use that feature. GPU/CUDA and live-model are N/A.

## Draft-to-ready and delivery

Specify atomicity, idempotency, policy/version, before/after state, partial
refusal, codec, parity, and Windows criteria; design transaction and receipt
semantics; review; preserve RED fault-injection/replay tests through GREEN;
review; verify selected routes; and record status. Stop on partial commit,
semantic decision-making by FathomDB, or receipt ambiguity.
