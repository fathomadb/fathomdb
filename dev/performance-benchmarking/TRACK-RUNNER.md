# Track Runner — PROGRAM execution control

This is the operating control for every item in the
[performance benchmarking and experiments program](PROGRAM.md) and its
[goals](PROGRAM-GOALS.md). It applies for the duration of this campaign. It
does not replace `experiments.record.v1`, the append-only experiment index, or
the individual [track plans](tracks/README.md).

Track Runner deliberately uses a small Codex-native shape instead of the
release-oriented Claude implementer/Codex reviewer ladder. Codex workers are
peers with isolated, bounded responsibilities; the coordinator integrates
their reviewed results. The only release-ladder primitives retained here are
worktree isolation, explicit commits, and verification.

## Roles and authority

| Role | Owns | May not do |
| --- | --- | --- |
| Coordinator | PROGRAM state, [live Track Runner status](TRACK-RUNNER-STATUS.md), dependency release, integration into the dedicated performance branch, and cross-track review | Write concurrently in a worker worktree or treat a worker's claim as accepted evidence |
| Track worker | One plan, one isolated worktree, its tests, implementation, and a commit | Change another track's plan, shared integration files, or run external/costly work without authorization |
| Independent reviewer | Read-only review of one worker diff against its plan | Edit the worker checkout or waive missing evidence |
| Final reviewer | One review of the integrated set for shared contract and comparability drift | Repeat individual reviews or reinterpret incomplete work as a result |

The coordinator is the sole writer for `PROGRAM.md`, `TRACK-RUNNER.md`,
`TRACK-RUNNER-STATUS.md`, shared experiment helpers, shared configuration
conventions, and integration commits. Each worker owns only the paths named in
its brief. No two writers share a checkout.

## Starting a track

Before any preparation or implementation, the coordinator and worker run:

```bash
./scripts/track-runner.sh check
./scripts/track-runner.sh status
./scripts/track-runner.sh brief <TRACK-ID>
```

`check` is enforced by the normal Markdown/agent verification path. `status`
reads the live coordinator board and `brief` reads the plan; neither
authorizes a benchmark run. The worker then records a compact handoff packet
containing:

1. Track ID, plan path, dedicated-worktree base SHA, and owned paths.
2. Explicit dependencies and the named contract it consumes or produces.
3. Failing-test evidence before implementation, targeted test evidence after,
   and the required `agent-verify` result before handoff.
4. Any receipt/configuration effect, external artifact root, and blockers.
5. A clear `ready`, `blocked`, or `not-ready` outcome—not a narrative claim of
   completion.

For new PROGRAM configurations, include `program_track: <TRACK-ID>` in the
typed resolved configuration. It stays under `record.config.resolved`; the
common `experiments.record.v1` and index-row schemas remain unchanged. Existing
historical configurations and receipts are evidence, not retroactive inputs to
this requirement.

## Parallelism and dependency gates

Start with `TRACE-01` as the contract canary. It owns the safe projection-trace
sidecar and the ELPS-to-projection lifecycle mapping. Its acceptance and
integration are prerequisites for lifecycle-dependent work, especially
`PARENT-01`, `GRAPH-01`, and extracted-memory claims.

After that canary is integrated, run at most two writer lanes at once:

```text
TRACE-01
    |
    +-- LOCOMO-01 -----------------> PARENT-01 -> ANSWER-01 -> MEMORY-01
    |
    +-- SCALE-01 -> SCALE-02

CORPUS-01 may prepare independently; LATENT-01 and GRAPH-01 are commissioned
only from a named failure diagnosis. TEMPORAL-01 and EXTRACT-01 wait for
qualified gold, TRACE-01 lifecycle coverage, and the selected baseline.
GLOBAL-01 is complete and limited; REASON-01 remains parked.
```

`LOCOMO-01` may prepare its FTS/CPU runner after its own provenance conditions
are met, but `PARENT-01` begins only from the integrated trace contract. A
worker discovers a changed shared contract or a failed canary by stopping and
returning a `blocked` packet; it does not invent a local replacement.

The complete controlled track set is `SAFETY-01`, `TRACE-01`, `LOCOMO-01`,
`PARENT-01`, `SCALE-01`, `CORPUS-01`, `ANSWER-01`, `MEMORY-01`, `SCALE-02`,
`TEMPORAL-01`, `EXTRACT-01`, `LATENT-01`, `GRAPH-01`, `GLOBAL-01`,
`REASON-01`, and `SEARCH-01`. Their individual status and decision questions
remain authoritative in PROGRAM and their plans; Track Runner governs how any
non-historical work on them moves.

## Review and integration gates

For each lane, the order is fixed:

1. Worker tests and verifies its isolated commit.
2. A different, read-only Codex reviewer checks the diff against the plan,
   receipt/configuration contract, and stated evidence.
3. The coordinator integrates only an accepted commit, verifies it from Git,
   and updates the [live status board](TRACK-RUNNER-STATUS.md) plus
   PROGRAM/plan state where the status actually changed.
4. Once the planned lane set is integrated, one final reviewer checks shared
   provenance, configuration, receipt semantics, metric definitions, and
   CPU/GPU timing boundaries across the set.

An individual review is never replaced by the final review. The final review is
not a second implementation review.

## Progress tracking

`TRACK-RUNNER-STATUS.md` is the only live coordination board. It shows active
lanes, integration base, per-track runner state, reviewed commit/receipt links,
blockers, and the next coordinator action. The coordinator updates it at every
lane commission, handoff, review verdict, integration, block/resume, and final
cross-lane review. Workers return their handoff packet but never edit this
shared board. `PROGRAM.md` remains the portfolio plan; `experiments/index.jsonl`
and receipts remain execution evidence.

## Experiment and approval boundaries

Planning, local code changes, tests, and static preflight are in scope once a
track is authorized. Corpus acquisition, GPU/model execution, paid services,
external writes, and publication remain separate user-authorization gates.
Every completed or blocked execution writes the existing safe receipt and
append-only index row; raw payloads, predictions, databases, and logs stay in
the external artifact root.

Do not use release-state JSON, release boards, Claude-only agent roles, or
release prompts to manage this campaign. Update the PROGRAM board and track
plan only when durable state has changed; leave historical result notes
historical.
