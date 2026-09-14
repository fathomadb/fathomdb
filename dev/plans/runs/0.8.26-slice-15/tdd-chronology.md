# Slice 15 prototype chronology

- `f1e608c2`: transient RED contract tests; compiler reports absent graph
  evidence types and methods.
- `79aca61b`: fixture validity correction removes edge bodies that required an
  unrelated embedder; assertions unchanged.
- `ad640796`: fixture validity correction makes the explicit seed satisfy the
  existing target-kind filter; assertions unchanged.
- `b6478d1f`: first test-hooks-only GREEN shortcut.
- `6235daab`: release-mode exploratory harness.
- `6ae37d1e`: exploratory shortcut evidence, explicitly classified as
  non-authoritative.
- `1a1e5041`: real bounded, batched full-provenance treatment.
- `a8679b96`: concurrent graph measurement harness.
- `54e96c1c`: writer-interference campaigns.
- `e77b117b`: correction that moves hydration and minting into the graph reader
  transaction, removing primary-connection serialization from the treatment.
- `7ec88e1f`: 50-result and mixed node/edge evidence arms.
- `50e471d3`: explicit corrupt hash/locator, context mismatch, foreign
  reference, post-erasure, and held-WAL refusal tests.
- `0adbb37f`: complete 10k-work, 100 KiB, process-isolated RSS, paired
  erase/excise, concurrency, and carrier measurement matrix.
- `2d0a346c`: complete inline-carrier encoder used for the option-B comparison.
- `52d1565f`: exact two-preflight, row-count, index, source-dedup, and hashing
  work counters.
- `d1f6d8a4`: durable raw samples, reproducible derivation, environment,
  surface manifest, analysis, and recommendation.
- `bf6a698e`: planned teardown of all transient runtime, test-hook, and test
  code while retaining the durable decision evidence.
- `6ecbdc32`: history-preserving restoration of the transient prototype after
  independent review found that the first decision campaign measured an N+1
  loader and ranked-payload surrogate rather than the approved treatment.
- `50a49d7d`: FIX-1 RED tests for the intrinsic carrier, exact two-statement
  treatment, source-deduplicated hashing, writer/erasure rendezvous, true 1 KiB
  fixture, and post-`ErasureIncomplete` observability.
- `7748dadb`: FIX-1 GREEN implementation of those corrected prototype and
  measurement contracts.
- `081a1eea`: records the exact 1,024-byte fixture size in every real campaign.
- `12b59f69`: retains the behavior-neutral literal ordinary graph-response
  regression independently of the transient prototype.
- `924e95c2`: replaces the invalid first campaign with corrected raw samples,
  reproducible expected-mean analysis, environment, surface count, and
  recommendation.
- `c50ccdf6`, `7f748269`, `ef204aa2`, and `e9a57498`: exact reverse-order
  teardown of the FIX-1 fixture metadata, GREEN prototype, RED tests, and
  restored prototype scaffold.

The two RED corrections repair setup preconditions and do not weaken any
oracle. The subsequent exploratory result exposed that the shortcut was not the
approved post-selection hydration design. The commit remains reachable to make
that finding auditable; its numbers are not used as decision evidence.

The first RED commit covered only the initial contract. It did not cover the
later review findings; `50a49d7d` is the tests-first record for FIX-1. The
first corrected campaign followed `081a1eea`; it was superseded by FIX-2 after
review found incomplete intrinsic material and insufficient timed writer
overlap. Its teardown remains reachable rather than rewritten.

- `d4901595`, `7d8b60c1`, `3a60767e`, and `fcb5791f`: restore the transient
  prototype and FIX-1 fixture in history-preserving order for FIX-2.
- `37a7ed99`: FIX-2 RED tests require complete intrinsic source locator/span,
  direct dependency generation, class-specific lifecycle, shipped target
  filter semantics, terminal-edge graph-disclosure semantics, and repeated
  writer overlap counted only inside the timed interval.
- `06a54027`: FIX-2 GREEN completes the intrinsic carrier and resolver,
  corrects class-aware eligibility, and adds balanced readiness/progress
  rendezvous with timed successful-operation accounting.

FIX-2's non-writer authoritative campaign follows `06a54027` and replaces the
earlier non-writer decision numbers without mixing different treatments. The
final evidence intentionally combines that retained non-writer campaign with
the later `b7a03ca2` replacement writer campaign; each subset is internally
consistent and identified in the environment record. A subsequent teardown
preserves the experimental chain in history while leaving no prototype public
method, field, codec, binding, schema, persistence path, or runtime behavior in
the final tree.

- `b7fb1878`: replaces the earlier decision evidence after the first FIX-2
  review. Its non-writer measurements remain authoritative, but its writer
  subset is superseded below because the foreground loop contained
  load-dependent progress waits.
- `9e4d9d3f`, `0ac0ccf1`, `107e536f`, `8d744d53`, `11be9218`, and `09b4bfae`:
  exact reverse-order teardown of FIX-2 and its restored transient scaffold.
- `0c2367e3`, `52ce6762`, `2e0bd4a8`, `ab9b702f`, `42f7adb3`, and `6afd362d`:
  restore the complete transient prototype in history-preserving order for the
  final writer-harness correction.
- `80161bfc`: RED contract for a fixed-duration foreground writer helper whose
  signature cannot observe background progress.
- `d47574fb`: GREEN independent one-second writer issuance. Its first run
  correctly refused the point arm because the common 250-microsecond pacing
  allowed no natural timed overlap; no sample from that run was retained.
- `b7a03ca2`: changes the common control/loaded pacing to two milliseconds,
  allowing natural sustained overlap without any load-dependent timed wait.
- `eef87183`: replaces only the invalid writer subset and derived conclusions.
  Five balanced campaigns naturally observed 807–828 background graph
  operations and 339–363 hydrated-graph-plus-point-resolution cycles; the
  throughput trigger did not fire, while undersampled writer p99 remains
  descriptive.
- `705d67d5`, `47080769`, `85e3bf67`, `cb80a0cc`, `69f4d87a`, `5762cdac`,
  `9ff4d775`, `34e6de90`, and `a20554c5`: exact reverse-order teardown of the
  final writer correction and restored transient scaffold.
