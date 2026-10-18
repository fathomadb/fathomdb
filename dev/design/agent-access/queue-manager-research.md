---
title: Queue manager for agent traffic — research
date: 2026-10-18
status: PROPOSED
desc: Names the workload class of agent traffic into FathomDB (fork-join micro-bursts from a few closed-loop clients with long think time), compares seven admission and queueing exemplars against it, sizes the cap and queue with burst arithmetic and Little's law, reviews what MCP and Claude Code do with parallel calls and busy errors, and recommends a bounded FIFO concurrency limiter that absorbs bursts silently and sheds on predicted wait, with the changes this implies for queue-admission.md.
inputs:
  - dev/design/agent-access/queue-admission.md
  - dev/design/agent-access/README.md
  - src/rust/crates/fathomdb-mcp/src/edge.rs
  - https://kubernetes.io/docs/concepts/cluster-administration/flow-control/
  - https://www.rfc-editor.org/rfc/rfc8289.html
  - https://www.rfc-editor.org/rfc/rfc8290.html
  - https://nginx.org/en/docs/http/ngx_http_limit_req_module.html
  - https://www.envoyproxy.io/docs/envoy/latest/configuration/http/http_filters/adaptive_concurrency_filter
  - https://github.com/Netflix/concurrency-limits
  - https://docs.rs/tower/0.5.3/tower/buffer/struct.Buffer.html
  - https://www.pgbouncer.org/config.html
  - https://www.cs.princeton.edu/courses/archive/fall15/cos518/papers/seda.pdf
  - https://static.usenix.org/events/usits03/tech/full_papers/welsh/welsh_html/index.html
  - https://modelcontextprotocol.io/specification/2025-06-18/server/tools
  - https://modelcontextprotocol.io/specification/2025-06-18/basic/lifecycle
  - https://platform.claude.com/docs/en/agents-and-tools/tool-use/parallel-tool-use
  - https://platform.claude.com/docs/en/agents-and-tools/tool-use/handle-tool-calls
  - https://code.claude.com/docs/en/sub-agents
  - https://code.claude.com/docs/en/env-vars
  - https://code.claude.com/docs/en/mcp
  - https://sre.google/sre-book/handling-overload/
---

# Queue manager for agent traffic — research

Research for work item 1 of [README.md](README.md). It answers one
question: what kind of queue manager suits agent traffic into FathomDB? It
does not edit [queue-admission.md](queue-admission.md), which stays the
design of record. § 8 lists the changes this research proposes for it.

Every web claim cites its source. Claims marked **unverified** were not
confirmed against a primary document or source.

## 1. Summary

- **Workload class.** Fork-join micro-bursts from a few closed-loop
  clients with long think time. Each agent turn issues *k* parallel calls
  at the same instant, waits for all of them, then thinks for seconds. The
  mean load is tiny (§ 4.2: about 0.1 calls in flight). The peak is *N* × *k*
  calls at one instant. The ratio of peak to mean is several hundred.
- **What the literature says.** A queue that absorbs a burst and drains
  within one interval is a "good queue" (CoDel, RFC 8289). The right tool is
  a buffer in front of a concurrency limit, with shedding based on time
  spent waiting, not on queue length. Rate limiters, adaptive concurrency
  and per-flow fair queuing all target sustained overload, which is not
  the incident.
- **Recommendation.** A **bounded FIFO concurrency limiter that absorbs a
  burst silently and sheds on predicted wait.** In detail: one fair
  semaphore per store (as queue-admission.md § 6.1 already has), a queue
  deep enough for the largest client fan-out (proposed 32, not
  `2 × admission_cap` = 8), and an admission test at arrival that refuses at
  once only when the predicted wait would exceed the remaining deadline.
  `busy` becomes a rare event for sustained overload or stuck work. It is
  never the normal answer to a burst.
- **Why silently.** A turn resumes only when every parallel call has
  returned (§ 6.1). A refusal therefore costs a whole extra LLM turn, which
  takes seconds and tokens, to save a wait of milliseconds. A burst of 36
  hybrid calls at cap 4 drains in about 95 ms (§ 4.1).
- **Fairness unit: the store.** Claude Code subagents share the parent's
  MCP connection (§ 6.2, verified), so six subagents are one MCP session.
  The server has no agent identity to queue by. FIFO across the store
  bounds how long a lone call waits behind another agent's burst to a few
  service times. Per-flow fair queuing is deferred; a per-session limit on
  queued calls covers embedded hosts with several sessions.
- **The cap of 4 is not the constraint; the zero-length queue is.** With a
  queue, cap 4 against cap 7 changes burst latency by milliseconds (§ 4.1).
  The cap still matters for head-of-line blocking and for the GPU embedder,
  and the sweep in queue-admission.md § 7 still sets it.

## 2. The workload

From README § Why and the owner's brief for this research:

- Six Claude subagents each fired about 5–6 tool calls in parallel within
  one turn. The calls in each burst have identical timestamps.
- At cap 4 with immediate refusal, the calls past the fourth in each burst
  were refused: 29 of about 125 calls.
- Each burst cleared within about 330 ms. Between bursts there are seconds
  of LLM think time.
- Measured service times on the RTX 3090 host, small stores (MB):

  | Tool | p50 | p99 |
  | --- | --- | --- |
  | `search` text | 1.0 ms | 1.8 ms |
  | `search` hybrid | 6.9 ms | 10.5 ms |
  | `fetch` | 0.5 ms | — |
  | `expand` | 0.4 ms | — |

Consistency check: 125 calls in bursts of 5–6 is about 23 bursts. If no two
agents' bursts overlapped, each burst would lose 1–2 calls, about 23–46 in
all. The observed 29 sits at the low end, so overlap across agents was
limited. The simulation should confirm this from the trace (§ 9).

**A discrepancy to explain.** Burst arithmetic (§ 4.1) says a burst of 6 at
cap 4 drains in two service times, 4–21 ms. The observed 330 ms is 15–80
times longer. Possible causes: service time under concurrency is much
longer than the solo measurement (GPU embedding contention, H7 in
queue-admission.md), the 330 ms includes client-side scheduling, or the
figure spans several overlapping bursts. Which one it is matters for
sizing, and it is the first input asked of the simulation (§ 9).

### 2.1 Naming the class

| Property | This workload | Name in the literature |
| --- | --- | --- |
| Many requests at one instant, then nothing | Yes | Micro-burst; impulse arrival |
| One client issues *k* calls and needs all *k* back | Yes | Fork-join (scatter-gather) |
| A client sends again only after its previous calls return and it has thought | Yes | Closed system with think time (interactive model) |
| Few clients | 1–6 sessions, often 1 | Small population |
| Mean utilisation | About 2 % of 4 slots | Light load |

The closest established term is **fork-join micro-bursts in a closed
system with think time**. Two consequences follow from the textbook
models:

- **A closed system cannot be overloaded by its own clients in steady
  state.** Each client has at most *k* calls outstanding, so total demand is
  bounded by *N* × *k*. A queue of that depth can never overflow from these
  clients. Shedding is only needed for open-loop clients, misbehaving
  clients, or service that has stalled.
- **Fork-join latency is the maximum of the *k* call latencies.** "The Tail
  at Scale" (Dean and Barroso, CACM 2013,
  <https://pdos.csail.mit.edu/6.824/papers/tail-dean.pdf>) shows that the
  tail of each part dominates a fan-out. One refused call in a burst of 6
  delays the whole turn as much as six refused calls would.

## 3. Exemplars

Each exemplar is described by the problem it solves, its queue discipline,
fairness unit, limit signal and shedding signal, then judged against § 2.

### 3.1 Kubernetes API Priority and Fairness (APF)

Source: <https://kubernetes.io/docs/concepts/cluster-administration/flow-control/>.

- **Problem.** One API server shared by controllers, nodes and users,
  where one heavy client must not starve others. Its stated aim includes
  bursts: APF "introduces a limited amount of queuing, so that no requests
  are rejected in cases of very brief bursts."
- **Structure.** The server's total concurrency is divided among priority
  levels. Within a level, "each request is assigned to a *flow*, identified
  by the name of the matching FlowSchema plus a *flow distinguisher* —
  which is either the requesting user, the target resource's namespace, or
  nothing". Flows are spread over queues by shuffle sharding, which
  "insulate[s] low-intensity flows from high-intensity flows", and served by
  fair queuing. "The maximum number of queued requests possible from a
  single flow is `handSize * queueLengthLimit`."
- **Seats.** A request normally takes one seat; a large list request is
  estimated to "take a number of seats that is proportional to that
  estimated number" of objects.
- **Limit / shed signals.** A fixed configured concurrency per level, lent
  and borrowed between levels. A full queue or an expired queuing time
  limit rejects with HTTP 429. Whether a `Retry-After` header is sent is
  **unverified**: the page sections read do not mention it.
- **Fit.** The structure is right for a multi-tenant server with a
  persistent heavy flow. Here, the burst clears in milliseconds, and in the
  stdio case there is one flow (§ 6.2). Shuffle sharding over one flow
  degenerates to one FIFO. **Adopt later**: the per-flow bound on queued
  requests (`handSize × queueLengthLimit`) is the model for a per-session
  queued-call limit in embedded mode, and seats are the model if hybrid
  calls ever need to count for more than text calls.

### 3.2 CoDel and fq_codel

Sources: RFC 8289 (<https://www.rfc-editor.org/rfc/rfc8289.html>),
RFC 8290 (<https://www.rfc-editor.org/rfc/rfc8290.html>).

- **Problem.** Router buffers that fill and stay full ("bufferbloat"),
  adding delay without adding throughput.
- **Key idea.** It is "critical to understand the difference between the
  necessary, useful 'good' queue and the counterproductive 'bad' queue". A
  good queue absorbs a burst and drains; a bad queue persists.
- **Signal.** Sojourn time, not length: "Tracking the packet sojourn times
  in the buffer observes the actual delay experienced by each packet." CoDel
  drops only after the minimum sojourn time has stayed above TARGET
  (5 ms) for an INTERVAL (100 ms), "ensuring that a burst of packets that
  fits in the pipe will not be dropped." The drop rate then rises as
  `INTERVAL / sqrt(count)`.
- **fq_codel.** Hashes flows into 1 024 queues by default, serves them by
  deficit round robin, gives priority to sparse ("new") flows, and runs
  CoDel per queue. When the global limit (10 240 packets) is reached, it
  drops from the head of the queue holding the most bytes.
- **Fit.** The principle fits exactly: our bursts are good queue and must
  not be shed. **Adopt**: shed on time, not on depth; keep depth limits as a
  backstop only. **Reject**: dropping is cheap for TCP, which retransmits in
  one RTT; for an agent, a refusal costs a whole LLM turn, so the "drop to
  signal the sender" mechanism does not transfer. fq_codel's sparse-flow
  priority would favour a lone call over a burst, which is attractive, but
  it needs flow identity we do not have (§ 6.2).

### 3.3 nginx `limit_req`

Source: <https://nginx.org/en/docs/http/ngx_http_limit_req_module.html>.

- **Problem.** Limit the request *rate* per key (for example per client
  address), by "the 'leaky bucket' method".
- **Discipline.** Excess requests "are delayed until their number exceeds
  the maximum burst size in which case the request is terminated with an
  error"; the default burst is zero. `nodelay` serves the burst at once
  instead of pacing it; `delay` (1.15.7) sets the point above which excess
  requests are delayed. Rejected requests get `limit_req_status`, default
  503.
- **Fairness unit / signals.** The configured key; a fixed rate; a full
  burst allowance.
- **Fit: misfit.** Rate is not our scarce resource: 12 calls per second
  (§ 4.2) is nothing to eight readers. Pacing a burst to a rate adds delay
  that the readers do not need; `burst` with `nodelay` is, in effect, a
  queue in front of no limit. The lesson worth keeping is that nginx's
  default burst of zero turns every burst into errors, which is exactly our
  defect.

### 3.4 Envoy adaptive concurrency and Netflix `concurrency-limits`

Sources:
<https://www.envoyproxy.io/docs/envoy/latest/configuration/http/http_filters/adaptive_concurrency_filter>,
<https://github.com/Netflix/concurrency-limits>. queue-admission.md § 5.3
covers the gradient formula.

- **Problem.** Find the concurrency limit of a backend whose capacity is
  unknown and changes, from latency alone. Netflix frames it with Little's
  law: "`Limit = Average RPS * Average Latency`".
- **Signals.** Envoy measures minRTT by "pinning the concurrency limit to
  the configured `min_concurrency`" every interval (example: 60 s, 50
  requests) and aggregates samples at a percentile (example: 90). Over the
  limit, requests are refused; the documentation warns of "a noticeable
  increase in request 503s during the minRTT measurement window."
- **Fit: misfit.** The method needs a steady stream of samples to track a
  moving limit. Our traffic is idle most of the time, and our ceiling is
  known: eight readers and five embed workers. The minRTT window would
  refuse calls on purpose, and our refusals are what we are trying to
  remove. The Little's-law framing is kept for sizing (§ 4.2).

### 3.5 tower `Buffer` and `ConcurrencyLimit`

Source: <https://docs.rs/tower/0.5.3/tower/buffer/struct.Buffer.html>;
`ConcurrencyLimit` and `LoadShed` are in queue-admission.md § 5.1.

- **Problem.** Share one service between many callers (`Buffer`) and bound
  the work in flight (`ConcurrencyLimit`).
- **Discipline.** `Buffer` is a bounded channel to a worker task; "bound
  gives the maximal number of requests that can be queued for the service
  before backpressure is applied to callers." Its documentation advises
  setting "`bound` to be at least the maximum number of concurrent
  requests", because otherwise "all the slots in the buffer may be held up
  by futures that have just called poll_ready". `ConcurrencyLimit` is a FIFO
  semaphore.
- **Fit.** The composition `Buffer(bound ≥ max callers)` → `ConcurrencyLimit`
  is our design. The `Buffer` advice is the same sizing rule this research
  reaches in § 4: size the queue to the maximum number of concurrent
  callers, not to a multiple of the limit.

### 3.6 PgBouncer

Source: <https://www.pgbouncer.org/config.html>; details in
queue-admission.md § 5.2.

- **Problem.** Many clients, few database connections.
- **Discipline.** FIFO per pool; waiters bounded by `max_client_conn`
  (default 100) against `default_pool_size` (default 20), a ratio of 5:1;
  wait bounded by `query_wait_timeout` (default 120 s).
- **Fit.** Close to our problem. The defaults point the same way as § 4:
  the waiting room is several times the pool, and the wait bound, not the
  depth, is the main shed signal.

### 3.7 SEDA

Sources: Welsh, Culler and Brewer, SOSP 2001
(<https://www.cs.princeton.edu/courses/archive/fall15/cos518/papers/seda.pdf>);
Welsh and Culler, USITS 2003
(<https://static.usenix.org/events/usits03/tech/full_papers/welsh/welsh_html/index.html>).

- **Problem.** Keep an Internet service "well-conditioned" under load by
  splitting it into stages joined by event queues.
- **Discipline.** Each stage has a finite queue: "an enqueue operation may
  fail if the queue wishes to reject new entries, say, because it has
  reached a threshold." A thread pool controller adds threads when queue
  length passes a threshold; a batching controller searches "for the
  smallest batching factor that sustains high throughput".
- **Shed signal.** In the 2001 paper, a controller watches average response
  time and "exponentially reduces the stage's queue threshold" above 5 s.
  The 90th-percentile controller is the 2003 paper: it adjusts "the rate at
  which new tokens are generated in a token bucket traffic shaper" by
  additive increase and multiplicative decrease, run after `nreq` samples
  or a `timeout`, so it can still adjust under light traffic.
- **Fit.** We already have SEDA's shape: adapter queue → embed queue (H7) →
  reader queues (H10). **Adopt** its rule that every stage queue is bounded
  and rejects explicitly. Today H7 degrades silently and H10 blocks without
  a bound, which EP-03 and EP-18 address. **Note** the batching controller:
  a burst of hybrid calls could share one GPU embedding batch (§ 5,
  "coalescing"). **Reject** the adaptive controllers, for the reason in
  § 3.4.

### 3.8 MCP and LLM-agent clients

No source found describes admission control for MCP tool calls. What the
protocol and clients do:

- **The MCP specification has no concurrency field.** The draft schema's
  `ServerCapabilities` has `experimental`, `logging`, `completions`,
  `prompts`, `resources`, `tools` and `extensions`; no field anywhere
  mentions concurrency, rate or retry
  (<https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/main/schema/draft/schema.ts>,
  read 2026-10-17). A server cannot advertise its limit.
- **Servers must rate-limit.** Under Security Considerations, "Servers
  **MUST** … Rate limit tool invocations"
  (<https://modelcontextprotocol.io/specification/2025-06-18/server/tools>).
  The spec does not say how.
- **Errors.** A busy refusal is a tool execution error (`isError: true`),
  not a protocol error (same page, § Error Handling).
- **Timeouts.** Senders "SHOULD establish timeouts for all sent requests"
  and "MAY choose to reset the timeout clock when receiving a progress
  notification"
  (<https://modelcontextprotocol.io/specification/2025-06-18/basic/lifecycle>).
  Claude Code does not: its per-server `timeout` "is a hard wall-clock limit
  per tool call, and progress notifications from the server don't extend
  it" (<https://code.claude.com/docs/en/mcp>).
- **Tool annotations.** `readOnlyHint` and `idempotentHint` exist and are
  "hints". `fathomdb-mcp` marks every tool except `feedback` read-only and
  idempotent (`edge.rs:137-227`). Codex CLI is reported to run tools with
  `readOnlyHint: true` concurrently and others serially; **unverified**
  (third-party blog,
  <https://codex.danielvaughan.com/2026/05/27/codex-cli-mcp-readonlyhint-concurrent-tool-execution-annotations-parallelism/>).

## 4. Sizing

### 4.1 Burst arithmetic

With *c* permits, a burst of *B* calls arriving at once, FIFO, and a
service time *S* that does not change with concurrency, the last call
finishes after `ceil(B / c) × S`. Using p99 service times:

| Burst *B* | Cap *c* | Rounds | Text (1.8 ms) | Hybrid (10.5 ms) |
| --- | --- | --- | --- | --- |
| 6 (one agent) | 4 | 2 | 3.6 ms | 21 ms |
| 6 | 7 | 1 | 1.8 ms | 10.5 ms |
| 36 (six agents at once) | 4 | 9 | 16 ms | 95 ms |
| 36 | 7 | 6 | 11 ms | 63 ms |

Every row is under 100 ms, against a call deadline of 10 s and an LLM
think time of seconds. The queue depth needed for no refusals is
`B − c`: 32 at cap 4 for the worst case of six agents bursting together.

Two caveats the simulation must test:

- **Service time under concurrency.** Hybrid calls embed on the GPU
  through five provider workers (H7). Above five concurrent hybrid calls the
  embedder queues, and per-call service time is likely to grow. The solo
  10.5 ms is a lower bound. The 330 ms observed (§ 2) suggests the real
  figure under a burst is larger.
- **Head-of-line blocking.** Round-robin reader dispatch (queue-admission.md
  § 4) can put two calls of one burst behind the same slow read while other
  readers idle. The arithmetic assumes work-conserving dispatch.

### 4.2 Little's law

Steady state, with *N* agents, *k* calls per turn, think time *Z* and burst
response *R*:

- Throughput `λ = N × k / (Z + R)`. With *N* = 6, *k* = 6, *Z* = 3 s: about
  12 calls per second.
- Mean calls in service `L = λ × S`. At hybrid p50 6.9 ms: about 0.08.

Eight readers at a mean of 0.08 calls in service are about 1 % busy. No
cap from 1 to 7 is a throughput constraint at this load. The cap and the
queue exist only for the peak. Little's law confirms that this is a burst
problem, not a capacity problem.

### 4.3 Is the cap of 4 the real constraint?

No. The zero-length queue is. Without a queue:

- Cap 4 refuses `B − 4` calls of every burst.
- Cap 7 removes refusals for one agent's burst of 5–6, but refuses again
  whenever two agents' bursts overlap. The incident had six agents.
- No cap below 8 covers 36 calls.

With a queue of depth 32 or more, cap 4 and cap 7 differ by at most about
30 ms on the worst burst (§ 4.1). The cap then matters for two other
reasons, both already in queue-admission.md: a larger cap lets more reads
pile up behind a slow read (§ 4 there), and above five concurrent hybrid
calls the embedder, not the readers, is the bottleneck (§ 7.1 there). Leave
the cap to the sweep.

### 4.4 Fairness: can one agent's burst starve another's call?

- **Under FIFO.** A lone call that arrives just after a burst of *B* waits
  `ceil((B + 1) / c)` service times: for *B* = 6 and *c* = 4, two service
  times (4–21 ms). That is delay, not starvation. Starvation needs a client
  that keeps the queue full, which a closed-loop agent cannot do: it waits
  for its own calls before sending more.
- **Which unit.** The candidates are MCP session, agent and store.
  - *Agent*: not visible. Requests carry no agent identity.
  - *MCP session*: in stdio, Claude Code subagents share the parent's
    session (§ 6.2), so fairness per session cannot separate them. In
    embedded mode, sessions are real and separate, and a host could open
    one that loops without think time.
  - *Store*: the unit of the shared resource. FIFO per store is the
    fairness rule.
- **Recommendation.** FIFO per store, plus a per-session cap on *queued*
  calls in embedded mode (APF's per-flow bound, § 3.1). Per-flow fair
  queuing (APF shuffle sharding, fq_codel DRR) is deferred until a trace
  shows one session holding the queue for more than one burst.

## 5. Alternatives ranked

| Rank | Queue manager | Verdict | Why |
| --- | --- | --- | --- |
| 1 | **Bounded FIFO concurrency limit; burst-sized queue; shed on predicted wait** | **Recommended** | Absorbs every burst a closed-loop client can send; refuses only when the wait cannot fit the deadline; one fair semaphore, already designed (tower `Buffer` + `ConcurrencyLimit`, PgBouncer, CoDel's principle) |
| 2 | Rank 1 plus per-session fair queuing (DRR or shuffle sharding) | Defer | Helps only with several real sessions and a persistent heavy one (embedded hosts). Gives nothing in stdio, where all subagents are one session |
| 3 | Plain larger cap (4 → 7), no queue | Reject as a fix; keep as a sweep variable | Removes refusals for one agent's burst of 5–6, not for overlapping bursts; worsens head-of-line pile-up |
| 4 | Request coalescing | Optional, later | Identical concurrent calls (repeated `list_stores` or `orient`, README § Why) could share one execution ("single-flight"). Batching a burst's hybrid query embeddings into one GPU batch is the SEDA batching idea, engine-side. Neither removes the need for rank 1 |
| 5 | Adaptive concurrency (Envoy gradient, Netflix Vegas, SEDA 2003) | Reject | Needs steady samples; our traffic is idle between bursts and our ceiling is known; the minRTT window refuses calls by design |
| 6 | Token bucket with burst (nginx `limit_req`) | Reject | Limits rate, which is not scarce; pacing adds delay the readers do not need; with `nodelay` it is a queue without a concurrency bound |

## 6. Agent-side behaviour

### 6.1 How a parallel burst is consumed

From the Claude API documentation
(<https://platform.claude.com/docs/en/agents-and-tools/tool-use/parallel-tool-use>):

- "By default, Claude may call multiple tools in a single response."
- "The API doesn't prescribe an execution order: you can run the calls
  concurrently … sequentially in the order they appear, or in any
  combination". So serialising a burst is explicitly allowed.
- "Return one `tool_result` for each `tool_use` block, all together in the
  next user message." The model sees nothing until the slowest call of the
  burst returns. This is the fork-join property of § 2.1, verified.

Consequence: a refused call is answered with the others, and the model can
only retry it in its next turn, after a full model round trip (seconds,
plus input tokens for the whole context). The time a server-side wait
costs (tens of ms) is two to three orders of magnitude less.

### 6.2 Who shares a connection

From <https://code.claude.com/docs/en/sub-agents>:

- "Subagents inherit the built-in tools and MCP tools available in the
  main conversation". For a server named in the subagent's `mcpServers`
  list, "String references share the parent session's connection." Inline
  definitions are "connected when the subagent starts … and disconnected
  when it finishes."
- Subagents run concurrently; the default limit is 20 running subagents.
- `CLAUDE_CODE_MAX_TOOL_USE_CONCURRENCY`: "Maximum number of read-only
  tools and subagents that can execute in parallel (default: 10)"
  (<https://code.claude.com/docs/en/env-vars>). Whether this is per agent
  or per session is **unverified**. If it is per session, one Claude Code
  session never has more than 10 calls in flight to our server, and a queue
  of `10 − c` absorbs it entirely. The incident's 30 refusals in bursts of
  5–6 do not settle it.

This settles the open point in queue-admission.md § 6.1 ("If Claude Code
subagents share the parent's MCP connection (likely, not verified here)"):
for a server configured in the parent session, they do.

### 6.3 Do agents retry, and does `retry_after_ms` change anything?

- **Guidance, not evidence.** Anthropic's documentation advises
  instructive errors, for example "Rate limit exceeded. Retry after 60
  seconds.", to give "Claude the context it needs to recover or adapt"
  (<https://platform.claude.com/docs/en/agents-and-tools/tool-use/handle-tool-calls>).
  The only measured behaviour it states is for invalid calls: "Claude will
  retry 2-3 times with corrections before apologizing to the user."
- **No study found.** No source was found that measures whether an LLM
  agent honours a numeric retry delay in a tool result. Practitioner
  material on agent retries treats rate limits as something the *harness*
  retries automatically (for example LangChain Deep Agents,
  <https://docs.langchain.com/oss/python/deepagents/fault-tolerance>), not
  the model.
- **An agent cannot wait 50 ms.** A model's next action comes one turn
  later. Its effective retry delay is its own turn latency, seconds, which
  is longer than any `retry_after_ms` the formulas in queue-admission.md
  produce for sub-10 ms work. The incident fits this: agents retried in
  their next turn and met the next burst (queue-admission.md § 3).
- **What `retry_after_ms` is still for.** (a) Telling the model that the
  failure is transient, so it retries instead of answering without the
  data; (b) SDK and harness clients that can sleep; (c) a value large
  enough to matter when work is stuck or the store is switching. The SRE
  book's client rules are the model for (b): "a *per-request retry budget*
  of up to three attempts", and a per-client retry ratio below 10 %
  (<https://sre.google/sre-book/handling-overload/>).

### 6.4 Should the server serialise a burst silently?

Yes. All the work is sub-10 ms, the client waits for the whole burst
anyway, the API documentation allows serial execution, and a refusal costs
a turn. Silent absorption is what the FIFO wait in queue-admission.md
§ 6.1 does; this research says the queue must be deep enough that it
always does it for a closed-loop client. Busy remains for three cases: the
predicted wait does not fit the deadline (sustained overload or a slow
store), the backstop depth is reached (a client without think time), or
every permit is held by abandoned work (`stuck`).

Progress notifications for queued calls (queue-admission.md § 6.1) add
little: waits are milliseconds, and Claude Code does not extend its
timeout on progress (§ 3.8). They are harmless for long waits on a stuck
store.

## 7. The recommended queue manager

**Type:** a bounded FIFO concurrency limiter per store, with a queue sized
to absorb the largest client fan-out, and admission based on predicted
wait (CoDel's sojourn principle applied at arrival).

1. **Permits.** One fair `tokio::sync::Semaphore` per store with
   `admission_cap` permits, as queue-admission.md § 6.1. Unchanged.
2. **Arrival test, replacing depth as the main shed rule.** On arrival,
   with *q* queued and `c_eff` effective permits, compute the predicted wait
   `w = ceil((q + 1) / c_eff) × s`, where *s* is a high percentile of recent
   service times for the tool (placeholder: the EWMA, until the simulation
   decides). Refuse at once with reason `deadline` if
   `w > deadline − now − need`. Otherwise queue. This gives the fast refusal
   that queue-admission.md wanted from a small `queue_limit`, without
   refusing bursts that would have drained in milliseconds.
3. **Backstop depth.** `queue_limit` default 32 (the incident's worst case,
   § 4.1), reason `queue_full`. It is a guard against clients without think
   time, not the normal shed rule.
4. **Wait bound.** Unchanged: `timeout_at(deadline − need)` and the late
   permit check (queue-admission.md § 6.1 steps 3–5).
5. **Fairness.** FIFO per store. In embedded mode, a per-session limit on
   queued calls (for example half of `queue_limit`).
6. **`retry_after_ms`.** Unchanged formulas; documented as advisory for
   LLM agents (§ 6.3).

## 8. Changes proposed for queue-admission.md

Not made here. For the owner and the design's reviewer:

| Section | Change |
| --- | --- |
| § 1 Summary | Name the workload class (§ 2.1) and state the goal: absorb bursts silently; `busy` is for sustained overload and stuck work |
| § 5 Exemplars | Add CoDel (good versus bad queue; sojourn signal) and APF (per-flow queued bound, seats) as adopted ideas; nginx `limit_req` as rejected; tower `Buffer` sizing advice next to § 5.1 |
| § 6.1 Queue depth limit | Replace `queue_limit = 2 × admission_cap` (8 at cap 4) with a backstop sized to client fan-out, proposed 32; add the predicted-wait test at arrival (§ 7 step 2) as the main shed rule; drop the claim that a deeper queue mostly converts quick refusals into late ones, since the arrival test keeps refusals quick |
| § 6.1 Fairness | Replace "likely, not verified here" with the verified sharing rule (§ 6.2): subagents using a server named in the parent share its connection. Keep FIFO per store; per-session queued limit for embedded mode |
| § 6.1 `retry_after_ms` | Add that LLM agents cannot wait less than one turn; the value serves SDK clients and stuck stores, and its presence tells the model the error is transient |
| § 6.1 Progress | Note that Claude Code does not extend its timeout on progress; keep notifications for long waits only |
| § 7.1 | Add: a larger cap without a queue still refuses overlapping bursts (§ 4.3) |
| § 7.2 Arms | Add a **burst arm**: *N* ∈ {1, 2, 6} clients, each sending *k* ∈ {1, 4, 6, 10} calls at one instant, think time drawn from the incident trace; measure refusals and per-burst completion (the maximum of *k*). The closed loop with no think time in § 7.2 is a different workload class and does not reproduce the incident |
| § 7.3 Queue limit rule | Rewrite for the backstop: keep 32 unless the burst arm shows refusals at *N* = 6, *k* = 6 |
| § 10 Open questions | Add the questions in § 10 below |

## 9. Inputs needed from the simulation

1. **The incident trace.** Per call: arrival time, session, inferred agent
   (by burst membership), tool and mode, outcome. From it: the distribution
   of burst size *k*, of think time *Z*, and of overlap between agents'
   bursts.
2. **The 330 ms.** Which of the causes in § 2 explains a burst clearing in
   330 ms when arithmetic predicts 4–21 ms.
3. **Service time against concurrency**, per tool, for 1–8 concurrent
   calls, text and hybrid separately; hybrid above five concurrent calls in
   particular (embedder queue).
4. **For each cap in {4, 7} and queue depth in {0, 8, 16, 32, 64}:**
   refusal rate, p50 and p99 per-burst completion time, maximum queue
   depth, and the worst wait of a lone call behind another agent's burst.
5. **Arrival test against fixed depth.** Refusals and their latency under
   the predicted-wait rule (§ 7 step 2) compared with depth-only shedding,
   at sustained overload (a client with no think time).
6. **Percentile for *s*** in the predicted wait: EWMA mean against p90 or
   p99 of the last *n* samples, judged by false refusals (refused but would
   have finished) and late admissions (admitted but abandoned).
7. **Head-of-line.** The same arms with round-robin dispatch and with a
   work-conserving shared queue (EP-18 part A), with one slow read injected.
8. **Client ceiling.** Whether `CLAUDE_CODE_MAX_TOOL_USE_CONCURRENCY` caps
   in-flight calls per session or per agent, if the trace can show it (the
   largest number of calls in flight at once from one session).

## 10. Open questions for the owner

1. **Absorb, not refuse.** Accept that the goal is no refusals for any
   burst a closed-loop client can send, with `busy` reserved for sustained
   overload, a backstop depth and stuck work?
2. **Queue depth.** Accept a backstop of 32 (or the simulation's figure)
   instead of `2 × admission_cap`, with the predicted-wait test at arrival
   as the main shed rule?
3. **Fairness.** Accept FIFO per store with a per-session queued limit in
   embedded mode, and no per-flow fair queuing until a trace shows the
   need?
4. **Coalescing.** Is single-flight for identical concurrent calls, or
   batching a burst's query embeddings on the GPU, worth an engine
   proposal now, or after the queue lands?
5. **Advertising the limit.** MCP has no field for it. Should the server
   mention its concurrency in `instructions` or in `orient`? This research
   leans no: with bursts absorbed, agents do not need to know.
