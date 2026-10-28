---
title: What matters most in an agent datastore — a critical review of the three query-interface notes, and FathomDB's next steps
date: 2026-10-17
status: research note (non-canonical; see dev/notes/README.md)
purpose: Critically review the three 2026-10-16 notes on agent query interfaces, name the five properties that matter most in an agent database, the three largest FathomDB 0.8.27 shortfalls, and the next three technical steps.
inputs:
  - dev/notes/llm-agent-query-interfaces.md
  - dev/notes/agent-tools-regrade-of-query-interfaces.md
  - dev/notes/infino-hypotheses-vs-agent-query-interfaces.md
  - dev/design/fathomdb-graph-vs-mem0-zep-and-longmemeval-diagnosis.md (Cause C)
  - src/rust/crates/fathomdb-engine/src/search_types.rs (`SearchResult`)
  - src/python/fathomdb/_sdk_search.py (`search` signature)
  - src/rust/crates/fathomdb-engine/tests/f9_importance_ranking.rs, pr_g12_recency.rs
---

# What matters most in an agent datastore

> "§4.x" refers to `dev/notes/llm-agent-query-interfaces.md`, "N-n" to
> `dev/notes/agent-tools-regrade-of-query-interfaces.md`, and "H-n" to
> `dev/notes/infino-hypotheses-vs-agent-query-interfaces.md`. Written from the
> agent's side of the tool boundary. Candidates only; nothing here commissions
> work.

## 1. Vantage point

The author has no episodic recall of training. What it can report is how it
works as an agent, and one working session is a fair sample: a 59 KB hook
output was spilled to a file with a 2 KB preview and a path; the agent's memory
is a set of Markdown files whose loader warns that entries "reflect what was
true when written" and must be verified before use; and `AGENTS.md` records
that it once cited ten memory files that never existed.

The lesson is that the most expensive agent failure is not a miss. It is a
confident retrieval of something stale or wrong, which the agent then acts on.
A miss costs a reformulation; a wrong hit costs the task.

## 2. The five properties that matter most

1. **Trustworthy truth over time.** Every record carries provenance, the trust
   class of its origin, a validity interval, and supersession state, and the
   read path honours them by default. Retrieval output is untrusted input to
   the model; the store must say how untrusted.
2. **Precision at small k on real workloads.** An agent reads the top three to
   five hits and moves on. Interface shaping cannot rescue a wrong top three.
   Exact-token constraints composed with semantic match (§4.5) are part of this
   property, not a separate one.
3. **Honest result semantics.** Complete versus truncated, absent versus not yet
   indexed, full versus degraded. Never degrade silently. This is the "when to
   stop" signal (§4.3), and it is what separates a database from a search box.
4. **Budget-shaped, legible, cache-stable output.** Handles and snippets first,
   hydrate on demand, a hard cap with a continuation pointer, short ids that are
   stable within a snapshot, deterministic ordering and bytes (§4.2, §4.15).
5. **A disciplined write path.** Deduplication, contradiction and supersession
   handling, origin recorded at write time, cheap correction and forgetting.
   Memory quality is decided at write time far more than at read time.

Local, embedded, single-file, crash-safe, zero-ops are the entry ticket, not
differentiators. FathomDB already holds them.

## 3. Critical review of the three notes

### Strengths

- Well sourced; honest that vendor claims were reported, not measured.
- The diagnosis is right: context is the scarce resource, and an agent's hard
  problem is knowing when to stop.
- The regrade usefully corrects the first note against shipping tools, and the
  Infino note correctly separates "let the agent ask once" from "help the agent
  know when to stop."

### Weaknesses

- **Interface-centric; retrieval quality is absent.** None of the notes asks
  whether the right record comes back. The repo's own evidence says this is
  the binding constraint: `fathomdb-graph-vs-mem0-zep-and-longmemeval-diagnosis.md`
  Cause C records dense-plus-FTS fusion trailing plain BM25 on LongMemEval
  (−9.6 pp factoid, −6 pp temporal, −6.4 pp knowledge-update). That
  measurement is 0.8.1-era and later releases (BM25F at 0.8.14, F9 at 0.8.16,
  0.8.28 Slice 40 ranking defects) may have moved it, but the notes do not
  check. A catalogue or a token budget cannot fix a wrong top-k.
- **The write side is nearly missing.** It appears only as N3 and N5.
  Deduplication, contradiction handling, consolidation policy and correction
  are half of the agent-memory problem and get no §4 property.
- **A factual error in the regrade.** It describes an "absent importance
  field." `canonical_nodes.importance` and an importance/confidence reweight
  shipped at 0.8.16 (`fathomdb-engine/tests/f9_importance_ranking.rs`), off by
  default; the recency reweight is likewise present and off by default
  (`pr_g12_recency.rs`). Neither appears on the Python `search` signature. The
  gap is defaults and exposure, not mechanism.
- **Convergence treated as validation.** "Every tool ships MMR, decay and hard
  caps" is evidence of a design fashion, not of efficacy. The regrade states
  that nothing was run; its "heightened" and "table stakes" verdicts should be
  read as hypotheses for measurement.
- **The §4.7 handle revision conflicts with §4.6.** A 0-indexed per-response
  scheme produces ids that do not round-trip across calls, which defeats one
  snapshot per plan. The right target is short, legible ids that are stable
  within a frozen context, with the opaque form available in a detailed mode.
- **The §4.16 escape hatch misses that FathomDB already is SQLite.** Any
  `sqlite3` client can read the file today (H7 makes the point for Infino). The
  real question is not whether to offer an escape hatch but whether raw-table
  reads bypass semantics the governed verbs enforce: validity views,
  supersession, and source erasure. That deserves a documented position.
- **Smaller issues.** The regrade's tally (6 hold, 4 heightened, 5 revised,
  1 downgraded) does not match its own lists, which give 6, 5 and 6, with §4.7
  counted under both "hold" and "revise". The FathomDB mapping is graded
  against 0.8.27 on `main` and does not consider the open 0.8.28 slices.

## 4. The three largest FathomDB 0.8.27 shortfalls

1. **Ranking is not yet shown to beat plain BM25 on real agent workloads.**
   The last recorded real-corpus diagnosis has fusion behind BM25 on most
   question classes, and the time-aware reweights that agent memory relies on
   are off and not exposed. This outweighs every interface gap. It needs a
   current re-measurement before it is treated as still true.
2. **No agent-facing boundary, and output not shaped for one.** No MCP server
   or tool adapter is tracked in the repo. `search` takes a row `limit`, returns
   full bodies, offers no snippet, no `max_tokens`, no truncation with
   continuation, and uses opaque id values. Every agent integration must build
   its own wrapper, and that wrapper decides most of the agent experience.
3. **No "when to stop" or "how far to trust" signals.** `SearchResult`
   (`search_types.rs`) carries `projection_cursor`, `soft_fallback`, `results`
   and the opt-in `Explanation`; it has no match count, saturation flag,
   calibrated no-match verdict, or negative explain, and scores carry no
   maximum-possible reference. Records carry `source_id` but no trust class,
   so agent-written and untrusted-source content are indistinguishable on the
   read path, which is a prompt-injection channel for any memory that is fed
   back into a prompt.

## 5. The next three technical steps

1. **A read-first MCP adapter with a budgeted response envelope, over the
   governed verbs.** `search`, `get`, `expand`, `changes_since`; read-only by
   default; one frozen read context per session. Handles plus snippets by
   default, `max_tokens`, concise and detailed modes, truncation with a
   continuation handle or MCP resource link, `structuredContent` with an
   `outputSchema`, deterministic order and bytes. Typed errors that name the fix,
   with did-you-mean on kind, projection and id in the error path only. No
   schema change. Closes shortfall 2.
2. **Stop signals on `search`.** A bounded match count and saturation flag;
   per-arm contribution, including which arms returned nothing; readiness of
   each projection the query touched; an explicit partial or degraded label
   whenever an arm is skipped; top-gap and maximum-possible score so an explicit
   "no good match" verdict is possible. Negative explain ("why did X not
   appear") follows as a second step. All read-side shaping over data the engine
   already holds. Closes most of shortfall 3.
3. **Trust and time as first-class, gated by an agent-loop evaluation.** Record
   an origin class (`owner`, `agent`, `untrusted`, `system`) beside `source_id`
   at write time, filterable in `Filter` and surfaced on every hit. In the same
   step, build an evaluation that measures what agents pay rather than recall@k
   alone: tokens to a correct answer, stop correctness (did the agent stop at
   the right point), and stale-fact rate. Use it to decide whether the shipped
   recency and importance reweights become defaults. Closes the trust half of
   shortfall 3 and gives shortfall 1 a measurement instead of a claim.
