# Slice 81 erasure-availability experiment

Status: PLANNED; no POC code or measurements yet.

The [Slice 81 package](../../plans/0.8.28/features/slice-81/README.md) owns
requirements, local acceptance criteria, protocol, test plan and draft design.
Implement the standalone crate/controller here after planning review; commit
rerunnable tools and frozen manifests. Record raw output locations/digests and
concise results here after execution. Do not fabricate empty result receipts.

Core comparison: eager secure deletion versus committed incremental cleanup.
Staged replacement is conditional; production semantics and historical/temporal
backend work are outside scope. Existing [baseline tools/results](../current-history-search/followups/README.md)
remain separate. This directory is the durable home for new Slice 81 tools, not
a copy of the prior campaign.
