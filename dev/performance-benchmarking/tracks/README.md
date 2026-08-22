# Performance experiment track plans

This directory contains one short draft plan for every row in
[the program overview](../PROGRAM.md). Each plan answers five questions: status,
decision, inputs, run, and stop condition. Parameter lists belong in run
configuration; evidence belongs in receipts. A plan is not permission to run.

Every plan is visible through [Track Runner](../TRACK-RUNNER.md). It defines the
coordinator/worker/reviewer handoff, the two-writer WIP limit after the trace
canary, and the external-execution approval boundary. Read the plan through
`./scripts/track-runner.sh brief <TRACK-ID>` before starting its preparation.
Read [the live status board](../TRACK-RUNNER-STATUS.md) first; only the
coordinator updates it after a worker handoff or review gate.

## Keep plans small

1. Keep only the decision, minimum inputs, smallest useful run, decision rule,
   and stop condition in the plan.
2. Put hashes, model settings, prompts, matrix cells, and host details in the
   run configuration or receipt.
3. If one plan tries to answer two independent decisions, split the track. If
   detail does not change the decision, delete it from the plan.
