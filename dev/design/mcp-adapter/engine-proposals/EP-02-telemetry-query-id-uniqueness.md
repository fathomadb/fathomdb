---
title: EP-02 — Telemetry query ids unique per sink file
date: 2026-10-17
status: PROPOSED
desc: Draft engine proposal. Give each enable_telemetry session a nonce that is deterministic and unique within its sink file, so query ids never repeat across re-enables and one sink file can serve every open of a store.
---

# EP-02 — Telemetry query ids unique per sink file

Status: Draft. Priority P1. Not an accepted decision.

## Problem

- `TelemetrySink.nonce` is always set to 0, and `seq` restarts at 0 on every
  `enable_telemetry` (`fathomdb-engine/src/telemetry.rs:139-145`). Query ids are
  `q{nonce}-{seq}` (`telemetry.rs:218`), so a second enable on the same file
  repeats `q0-0`, `q0-1`, ...
- `record_feedback` accepts any id with `nonce == sink.nonce` and
  `seq < sink.seq` (`telemetry.rs:282-289`). After a re-enable, a stale id from
  an earlier session is accepted and joins to whichever new search reused it.
- The design that introduced the field meant it to be "a deterministic per-open
  value (so fixtures reproduce)" (`dev/design/0.8.8-telemetry-design.md:46-47`).
  The implementation hard-codes 0.

Adapter workaround removed:

- A fresh sink file per open epoch, `<dir>/<store>.<pid>.<epoch>.jsonl`
  (`src/rust/crates/fathomdb-mcp/src/registry.rs:699-707`).
- An adapter-side epoch check that refuses earlier-epoch feedback
  (`tools.rs:1143-1150`, MR-006.5).

A second, verified reason this matters: erasure redacts only the sink attached
at the time (`erasure.rs:1260-1265`). Every earlier per-epoch file keeps the
erased stable ids, and nothing will redact it. One file per store that is
reattached on every open would give one redaction target. That still needs a
store to be open when the erasure runs; see open questions.

## Proposed surface

Change `enable_telemetry` in place. No new verb.

- On enable, the engine scans the existing sink for its own `session` records
  and takes `nonce = max(seen) + 1`, or 0 for an empty or new file. It then
  appends `{"type":"session","schema_version":1,"nonce":N}` before arming
  capture.
- Ids stay `q{nonce}-{seq}`. They are unique within one file and deterministic
  given that file's contents, which keeps the fixture property.
- `record_feedback` keeps its check. A stale id from an earlier session now
  fails as `InvalidArgument("unknown query_id")` instead of being misattributed.
- A sink with a malformed `session` record fails the enable with `Storage`, so
  the file is never trusted silently.

Alternative for the owner: an optional caller-supplied nonce,
`enable_telemetry_with(path, nonce)`. It is simpler, but it pushes uniqueness
onto every caller, which is exactly the duty the adapter carries today.

## Invariants and consistency

- Within a sink file, `(nonce, seq)` never repeats while the file is only
  appended by the engine.
- A sink edited or truncated by hand can reuse nonces. That is acceptable, since
  the file is local and operator-owned. A test records the behaviour.
- Reopen: the nonce comes from the file, not from engine memory, so closing and
  reopening the store gives the next nonce.
- Erasure redaction already copies lines that have no `result_stable_ids`
  through verbatim (`erasure.rs:123-128`, `erasure.rs:208-221`), so `session`
  records survive it. A test pins that, because losing one would let the next
  enable reuse a nonce.

## Interaction with ADRs

- `dev/design/0.8.8-explain-and-telemetry-adr.md` §B.1 requires deterministic
  ids. This keeps them deterministic per file.
- `ADR-0.8.27-offline-physical-erasure-completion.md` discharges telemetry
  obligations only through "the correct original sink". A stable sink path per
  store makes that sink findable. No successor ADR is needed. The sink format
  gains a record type, which the telemetry design doc must list.

## Cost and performance

Enable reads the whole sink once, which is O(file size). Enable happens once per
open. Capture and feedback cost do not change.

## Test plan sketch

- Real engine, real file: enable, search twice, disable by reopening, enable on
  the same file, search. Ids are `q0-0, q0-1, q1-0`.
- Stale feedback: after re-enable, `record_feedback("q0-1", ...)` is rejected,
  and nothing is appended.
- Erasure keeps `session` records: erase a source after two sessions, then
  re-enable and assert the nonce is 2.
- Property test: for any sequence of enables and captures on one file, all
  emitted ids are distinct.

## Adapter change once it lands

Use one sink per store, `<dir>/<store>.jsonl`, re-enabled on every open. The
per-epoch file naming goes. `store_reopened` stays, because a reopened engine's
in-memory sink no longer counts the earlier session's ids, but the engine now
enforces it, not only the adapter.

## Open questions for the owner

1. File-derived nonce, or caller-supplied nonce?
2. Should the adapter's existing per-epoch files be listed for operators as
   telemetry obligations? The engine cannot redact them after an erasure.
3. Feedback ids are `write_cursor` values (`telemetry.rs:254-272`), which are
   "NOT cross-session-stable". Should `record_feedback` also accept stable ids?
   That would be a separate change.
