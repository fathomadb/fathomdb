# CORPUS-01 — Agent-memory gold coverage

**Status:** portfolio-qualified LLM review complete; human-gold gaps remain.

## Decision

Is the gold portfolio sufficient to support claims about mutable, temporal,
and provenance-preserving personal memory rather than LOCOMO retrieval alone?

## Draft plan

1. Use the existing 16-record review across knowledge update, supersession,
   erasure, and time-scoped validity.
2. Human-review only the insufficient or ambiguous records, including the two
   supersession cases.
3. Maintain this corpus-to-claim coverage matrix and mark each row supported,
   limited, or unsupported; do not manufacture gold.

| Claim | Qualifying corpus or fixture |
| --- | --- |
| Factoid and multi-session retrieval | LOCOMO |
| Temporal recall and time-scoped validity | LOCOMO plus TimelineQA, TimeQA, or ToT |
| Knowledge update | LongMemEval plus reviewed update cases |
| Supersession | Human-reviewed supersession cases |
| Source attribution and erasure | ELPS/TRACE lifecycle fixtures; conformance only |
| Multi-hop and global synthesis | MuSiQue and AP-News/AutoE or SummHay |

## Stop

Stop when every intended claim has a clear coverage verdict. Unsupported claims
remain blocked rather than triggering an open-ended corpus search.
