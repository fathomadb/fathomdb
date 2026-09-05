---
title: 0.8.25 Slice 45 design review FIX-1
status: COMPLETE
review_cycle: FIX-1
reviewed_commit: b21455d6
---

# Slice 45 design review FIX-1

Independent cycle 1 returned `CHANGES_REQUIRED` with four P1 and two P2
findings. Design v5 resolves them without widening the approved scope.

| Finding | Resolution | Status |
| --- | --- | --- |
| DR45-1 cursor confidentiality | Page order and continuation now use only the already-public numeric `write_cursor`; caller text remains hashed outside the payload. HMAC is sufficient because no confidential plaintext remains. AEAD and persisted cursor state are unnecessary. | RESOLVED_PENDING_REVIEW |
| DR45-2 schema/token cutover | Step 33 now specifies two indexes, six triggers, exact 14/16/18 validation branches, one checked cutover generation increment, exhaustion rollback, and the pre-step token refusal fixture. | RESOLVED_PENDING_REVIEW |
| DR45-3 revision-boundary crossing | Canonical pages now return the existing `NodeRecord`; legacy revision synthesis remains internal until Slice 50. | RESOLVED_PENDING_REVIEW |
| DR45-4 causal performance protocol | Matched-shape query, continuation-stage, and state-point cells are separate from operational list/full-walk observations. Corpus distributions, process repetitions, paired bootstrap, fresh RSS runs, stage/terminal counts, hashes, and per-cell materiality are fixed. | RESOLVED_PENDING_REVIEW |
| DR45-5 precedence/SDK placement | Cursor authentication now precedes frozen authentication, then binding comparison and snapshot validation. A reason/path table and combined-malformation parity fixtures are required. Python stays under `fathomdb.read`. | RESOLVED_PENDING_REVIEW |
| DR45-6 collection format | State reads require persisted `format_version = 1`; unsupported versions fail with a typed cross-SDK outcome. | RESOLVED_PENDING_REVIEW |

The cursor correction intentionally selects a simpler alternative to the
reviewer's proposed authenticated encryption. Keyset ordering does not require
caller text: both authoritative tables carry a persisted total write cursor,
and the response already discloses it. The cursor therefore authenticates only
digests, fixed discriminants, bounded integers, and that public coordinate.
