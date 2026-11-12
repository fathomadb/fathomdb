---
title: Slice 50 documentation checkpoint and pending choices
status: PROPOSED
target_release: 0.8.29
slice: 50
---

# Slice 50 documentation checkpoint and pending choices

Baseline3fa16620ac30cbe3ed66f5bfe542f2f7a62b8982; branch llm/0.8.29-s50-data.
Owned Markdown/proposed identity ADR only. Earlier bounded Phase A, feedback
presentation and v1/v2 design/citation PASS remain historical. The initial final
PASS was superseded by the P2 cursor-injectivity proposal finding. Exact-v4
DESIGN PASS covers patch
`0f7ac8312786f8db9e116e175ffdbf1af7aa0b64cc9f861430a272f5b09ffd67`.
New different-reviewer FINAL DOC/OUTPUT PASS covers corrected 13-file parent
candidate SHA256
`7a9be30c9974f36fa3ccd6bb1bef7347d1ee1c045616963c44b2b4a0387add49`,
with no remaining P1/P2. Parent generation/Markdown/state/diff checks passed on
that subject; [review log](review-log.md) attributes exact receipts. V5 is a
record-only update, not a separately reviewed or qualified product candidate.
No tests, native execution, source edits, runtime census or new semantic ruling.

## Qualified inputs and limits

| Input | Actual accepted boundary | Remaining limit |
| --- | --- | --- |
|10 |104dcbcbe92a1eca730d146b99ab17d444df6217; closure d7d928f8e49d7af6a792385534f4e991c13232b3 | Preparation only, no dependency waiver. |
|20 |Linux subset657edeb76caaf9c8d2aab433766bec9362e73217 | A3/A5 policy and Windows A6 open. |
|60 |Formats/direct-filter/budget5271c1bf022f0ea22f95f7433e226feb2315043e | Whole60/F11 held; search still rejects relaxed flags. |
|70 |COMPLETE e875f36042f89c1d84756f78135db0e8926c1ed4 | Implemented reason/hint/native parity must survive50. |
|30 P |Runtime2fd772558e46d0b1645dce2ffea730d0bdd3305c; final docs0d8be5bb75516955ebf99b94913b9b54921bb3a4 |16cells/20000timed successes; assembled qualification, original failures/skips preserved. Zero vectors; not G/D/M/F or TC5 fidelity. |
|100 |Current documentation integrated3fa16620a | No answer to its coverage exception is inferred. |

G/D candidate49be773da5c2d7637160c278a4c9a7cc9e5f4c20 is isolated,
unqualified and quiescent below its16GiB heavy floor; it is not an input to this
checkout. Exact frozen TC5 pack is absent on the separate cold volume;
/mnt/data is mounted. Runtime Memex census scope is approved, but the exact
consistent snapshot and safe read-only access method are unresolved. Source
callsite inspection grants no database/snapshot access method.

## Minimal pending owner packet

These are concrete choices for parent presentation after design review, not
questions issued to the human by this owner. All recommendations remain
UNSELECTED and the identity ADR stays PROPOSED/HITL-required.

| Choice | Recommended disposition | Concrete consequence / alternative |
| --- | --- | --- |
| Retrieval unit | Snapshot-local canonical(Node/Edge,cursor), explicit ownerless(NodesFts,rowid); same record credited once per arm at first original position. | Equal-body distinct records/sources/revisions remain distinct; keep l:/h:/p: write-time identity. Alternative is explicit body-unit or logical-entity presentation, with the corresponding loss/representative policy acknowledged. |
| Same-record provenance | Existing representative-first vector→text→graph; retain first winning graph path separately. | Vector/text source stays record-owned; graph-only source/confidence stays winning-edge-owned except entity seed's selected node source. Do not unilaterally switch GraphArm to canonical source or union/duplicate paths. |
| Ambiguous MCP feedback | Keep public refs; visible targets are SafeCursor only when their ref occurs once in the visible page and their cursor occurs once across the complete captured query result, including hidden cursor aliases; otherwise Ambiguous. Refuse named ambiguity atomically after visible-membership/not_recorded guards. | Same cursor never proves same record; conservatively refuse repeated visible refs/captured cursors. Hidden-only ref strings remain nonmembers. Safe labels survive unrelated unmentioned ambiguity and off-page same-ref/different-cursor records. No new engine/public carrier or telemetry schema; cursor-only events remain limited, and feedback_ref alone is insufficient. F11 stays held for actual ruling/reviewed addendum. |
| SDK helper→pack composition | Preserve prefixed-ID derivation and upfront duplicate-ID refusal, explicitly accepting the newly exposed success→refusal for h: twins. | Historical l: duplicate refusal already exists. Caller-owned unique ContextItem labels can represent both with a caller map. If automatic mapping of every hit list must pack, return a concrete changed helper/pack contract for review; no relabeling/ID ABI is presumed. |
| Doc-hidden fusion helpers | Proposed public-export retirement and crate-private keyed carrier, expressly covered in the future accepted ADR/interface change. | SearchHit lacks ownerless locator/class. Move manual formula tests without weakening their oracle; real Engine search supplies semantic RED. If retirement is rejected, review an explicit public carrier/signature; no body-keyed shim. |

The first four are product semantics; the last is a proposed public-surface
disposition needed to make that design implementable. A design PASS settles none
of them. Runtime census must report frequencies or an explicit human disposition
of the missing evidence; source-only callsites establish no prevalence/benefit.

## Entry and downstream holds

50 hard30 remains OPEN; no dependency amendment or source GO is selected here.
40-before50 is attribution preference, not semantic prerequisite. Before source:
actual dependency/decision disposition, accepted successor, fresh design PASS,
strict entry/resource checks and owned setup remain. Strict per-behavior runtime
RED commits/body evidence then separate GREEN with unchanged oracle apply; source,
test, setup and runtime execution are outside this checkpoint.

Full80 needs settled identity/ref/feedback/packing/vector-coverage consequences;
90 needs full80 and actual pre90 Block1 exit after integrated20–70.160 remains
downstream. No source gate, F11,40/80/90/Block1, AC or coverage exception is waived.

Light inspection requires MemAvailable>=8GiB, data free>=250GiB and root>=32GiB;
heavy/native entry remains excluded and its16GiB floor unchanged. Parent actually passed generation, agent-lint-md.sh, state-view and diff checks
on corrected candidate7a9be30c, as attributed in the review log. Owner ran only
lightweight paths/blob/hash/link/diff checks and no declared Markdown gate; no
local mutable tooling was installed or borrowed. Parent scoped passes do not
qualify runtime/source or the full workspace.
