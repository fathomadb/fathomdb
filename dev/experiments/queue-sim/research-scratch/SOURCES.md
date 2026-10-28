# Source log

Access date for all entries: 2026-10-17. Legend: P=primary, S=secondary; V=verified (content read this session), U=unverified.

## Cited (46)

| # | Title | URL | Type | Claim supported |
|---|---|---|---|---|
| 1 | TraceLab project page | https://tracelab.cs.washington.edu/ | P,V | session/step/tool-call counts; Claude tool latency p50 115 ms, p90 10.1 s; human wait p50 |
| 2 | TraceLab paper (arXiv 2606.30560) | https://arxiv.org/pdf/2606.30560 | P,V | per-step LLM latency p50 4.9 s/p90 20.2 s/p99 1.3 min; tool calls per step avg 1.2, p50 1, p90 2, p99 4; decode 46.8 tok/s |
| 3 | Human-Less LLM Serving (arXiv 2606.20577) | https://arxiv.org/pdf/2606.20577 | P,V | closed-loop agent IAT near zero vs 2-5 s human |
| 4 | Artificial Analysis, Claude Sonnet 4.6 providers | https://artificialanalysis.ai/models/claude-sonnet-4-6/providers | S,V (via search snippet only) | ~44 tok/s, TTFT ~1.4 s |
| 5 | Anthropic docs: Parallel tool use | https://platform.claude.com/docs/en/agents-and-tools/tool-use/parallel-tool-use | P,V | multiple tool_use per response, no cap, parallel prompt, reissue on next turn |
| 6 | Anthropic docs: Handle tool calls | https://platform.claude.com/docs/en/agents-and-tools/tool-use/handle-tool-calls | P,V | retry 2-3 times on invalid input |
| 7 | OpenAI function calling guide | https://developers.openai.com/api/docs/guides/function-calling | P,V | multiple calls per turn, parallel_tool_calls, no cap |
| 8 | Anthropic: multi-agent research system | https://www.anthropic.com/engineering/multi-agent-research-system | P,V | 3-5 subagents, 3+ parallel tools, 3-10 tool calls |
| 9 | Claude Code docs: Subagents | https://code.claude.com/docs/en/sub-agents | P,V | 20 concurrent subagents default, no retry on limit, depth 3 |
| 10 | Claude Code docs: MCP | https://code.claude.com/docs/en/mcp | P,V | MCP_TOOL_TIMEOUT ~28 h default, idle 30 min stdio, retry behaviour |
| 11 | MCP TypeScript SDK protocol.ts | https://github.com/modelcontextprotocol/typescript-sdk/blob/main/packages/core-internal/src/shared/protocol.ts | P,V (via gh search) | DEFAULT_REQUEST_TIMEOUT_MSEC = 60_000 |
| 12 | MCP Python SDK session.py | https://github.com/modelcontextprotocol/python-sdk/blob/main/src/mcp/client/session.py | P,V | read_timeout_seconds default None |
| 13 | MCP spec lifecycle 2025-06-18 | https://modelcontextprotocol.io/specification/2025-06-18/basic/lifecycle | P,V | timeout/cancellation SHOULDs |
| 14 | SQLite WAL | https://sqlite.org/wal.html | P,V | readers/writers, checkpoint starvation, 1000 pages, occasional slow COMMIT, SQLITE_BUSY cases |
| 15 | DB Pro: SQLite Is All You Need | https://www.dbpro.app/blog/sqlite-is-all-you-need | S,V | WAL read p99 4.4/5.2 ms, worst 17/30 ms |
| 16 | dev.to sqlite-vec benchmark | https://dev.to/hypernexus/sqlite-vector-search-the-dependency-free-ai-memory-stack-that-outperforms-pinecone-5d27 | S,U (snippet only) | sqlite-vec p50/p99 |
| 17 | Filtered Vector Search lakehouse (arXiv 2608.05441) | https://arxiv.org/pdf/2608.05441 | P,U (snippet only) | cold/warm bimodal scan latency |
| 18 | AWS: Timeouts, retries, backoff with jitter | https://builder.aws.com/content/3EumjoZascWd1oZiEgL8ORlv3qE/timeouts-retries-and-backoff-with-jitter | P,V | p99.9 timeout rule, padding, 243x amplification, token bucket |
| 19 | AWS: Using load shedding | https://builder.aws.com/content/3Eun1EEyX6p2e3VYNyRLSJzLuMV/using-load-shedding-to-avoid-overload | P,V | bounded queue age, LIFO, spillover, timeout hints |
| 20 | SRE book: Addressing cascading failures | https://sre.google/sre-book/addressing-cascading-failures/ | P,V | queue <=50% of pool, deadline propagation, bimodal latency, 64x retries |
| 21 | SRE book: Handling overload | https://sre.google/sre-book/handling-overload/ | P,V | K=2, 10% retry budget, 3 attempts, one layer |
| 22 | The Tail at Scale (PDF) | https://www.barroso.org/publications/TheTailAtScale.pdf | P,V | 95th-pct hedging, 1800->74 ms, 63%, queueing variability, Mitzenmacher |
| 23 | RFC 8289 CoDel | https://www.rfc-editor.org/rfc/rfc8289.html | P,V | TARGET 5 ms, INTERVAL 100 ms, min sojourn, sqrt law |
| 24 | Colyer: Fail at Scale summary | https://blog.acolyer.org/2015/11/19/fail-at-scale-controlling-queue-delay/ | S,V | FB CoDel M/N rule, adaptive LIFO, quotes |
| 25 | Maurer, Fail at Scale (ACM Queue) | https://queue.acm.org/detail.cfm?id=2839461 | P,U (403) | underlying source for #24 |
| 26 | Netflix Gradient2Limit.java | https://github.com/Netflix/concurrency-limits/blob/master/concurrency-limits-core/src/main/java/com/netflix/concurrency/limits/limit/Gradient2Limit.java | P,V | min-RTT bias, EWMA lag, formula |
| 27 | Netflix concurrency-limits README | https://github.com/Netflix/concurrency-limits | P,V | Vegas alpha/beta, Gradient2 drift statement, Little's law |
| 28 | Envoy adaptive concurrency filter | https://www.envoyproxy.io/docs/envoy/latest/configuration/http/http_filters/adaptive_concurrency_filter | P,V | minRTT window, jitter, buffer, p90 example, sqrt headroom |
| 29 | Bronson et al., Metastable Failures (HotOS 21) | https://sigops.org/s/conferences/hotos/2021/papers/hotos21-s11-bronson.pdf | P,V | definition, retry example 560 QPS |
| 30 | Huang et al., Metastable Failures in the Wild (OSDI 22) | https://www.usenix.org/system/files/osdi22-huang-lexiang.pdf | P,V | retries sustain >50% of incidents |
| 31 | Mitzenmacher, Power of Two Choices (TPDS 2001) | https://www.eecs.harvard.edu/~michaelm/postscripts/tpds2001.pdf | P,V | d=2 exponential improvement |
| 32 | Harchol-Balter TAGS abstract | https://www.cs.cmu.edu/~harchol/Papers/abstract-tags.html | P,V | heavy tails, orders-of-magnitude differences |
| 33 | Joe Duffy: anticonvoy locks | https://joeduffyblog.com/2006/12/14/anticonvoy-locks-in-windows-server-2003-sp1-and-windows-vista/ | P,V | fair handoff convoys |
| 34 | Sue Loh: Lock convoys | https://learn.microsoft.com/en-us/archive/blogs/sloh/lock-convoys-and-how-to-recognize-them | P,V | lock convoy definition |
| 35 | Rust 1.62 release | https://blog.rust-lang.org/2022/06/30/Rust-1.62.0/ | P,V | futex Mutex |
| 36 | std futex.rs | https://github.com/rust-lang/rust/blob/master/library/std/src/sys/sync/mutex/futex.rs | P,V | wake-one, no handoff |
| 37 | std Mutex docs | https://doc.rust-lang.org/std/sync/struct.Mutex.html | P,V | no fairness statement |
| 38 | parking_lot Mutex | https://docs.rs/parking_lot/latest/parking_lot/type.Mutex.html | P,V | eventual fairness 0.5 ms |
| 39 | tokio Semaphore | https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html | P,V | FIFO fairness |
| 40 | tokio Mutex | https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html | P,V | guaranteed FIFO |
| 41 | Rust 1.67 release | https://blog.rust-lang.org/2023/01/26/Rust-1.67.0/ | P,V | crossbeam mpsc |
| 42 | std sync_channel | https://doc.rust-lang.org/std/sync/mpsc/fn.sync_channel.html | P,V | blocking, rendezvous |
| 43 | tokio Builder | https://docs.rs/tokio/latest/tokio/runtime/struct.Builder.html | P,V | 512 default, 10 s keep-alive |
| 44 | tokio spawn_blocking | https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html | P,V | queue past limit |
| 45 | rmcp service.rs | https://github.com/modelcontextprotocol/rust-sdk/blob/main/crates/rmcp/src/service.rs | P,V | timeout options |
| 46 | SQLite threadsafe / config / malloc / compile / busy_timeout / busy_handler / rusqlite Connection | https://sqlite.org/threadsafe.html ; https://sqlite.org/c3ref/c_config_covering_index_scan.html ; https://sqlite.org/malloc.html ; https://sqlite.org/compile.html ; https://sqlite.org/c3ref/busy_timeout.html ; https://sqlite.org/c3ref/busy_handler.html ; https://docs.rs/rusqlite/latest/rusqlite/struct.Connection.html | P,V | threading modes, memstatus mutex, defaults, busy handler, 5000 ms |

## Visited, not cited (20)

| URL | Why not cited |
|---|---|
| https://docs.claude.com/en/docs/claude-code/settings (and redirect https://code.claude.com/docs/en/settings) | Settings page; no MCP timeout or concurrency env vars found by grep |
| https://docs.claude.com/en/docs/agents-and-tools/tool-use/implement-tool-use (redirect to define-tools) | Served tool-definition page, not parallel-use content |
| https://research.google/pubs/the-tail-at-scale/ | Abstract only; full PDF used instead |
| https://cacm.acm.org/research/the-tail-at-scale/ | 403 |
| https://queue.acm.org/detail.cfm?id=2209336 (CoDel ACM Queue) | 403; RFC 8289 used instead |
| https://pollere.net/Pdfdocs/CoDelACMQ.pdf | 404 |
| https://engineering.fb.com/2015/12/02/production-engineering/fail-at-scale/ ; https://research.facebook.com/publications/fail-at-scale/ | 404 / generic listing page |
| https://www.envoyproxy.io/docs/envoy/latest/intro/arch_overview/http/http_filters/adaptive_concurrency_filter | 404; configuration page used |
| https://netflixtechblog.medium.com/performance-under-load-3e6fa9a60581 ; https://netflixtechblog.com/performance-under-load-3e6fa9a60581 | 403; library source used |
| https://www.cs.cmu.edu/~harchol/Papers/tois.pdf ; https://www.cs.cmu.edu/~harchol/Papers/ ; https://open.bu.edu/items/f8a31de4-aa7a-461d-a472-c48090281c79 | 404 / index only / 403; TAGS abstract used |
| https://grpc.io/blog/deadlines/ | Read; says default deadline is "a very large number" and "no single answer"; omitted for length |
| https://sqlite.org/lockingv3.html | Did not contain the busy-handler deadlock text; busy_handler page used |
| https://fast.inference.net/guides/why-is-claude-code-slow | 404; its "74.9% single-tool turns" figure (search snippet) is therefore unverified and not used |
| https://agentsroom.dev/blog/measure-claude-code-tokens-per-second | HTTP 413; not used |
| https://amitkoth.com/what-is-a-turn-claude-code/ | Anecdotal ("15-20 tool calls" per turn); not used |
| https://syfi.cs.washington.edu/blog/2026-06-25-tracelab/ | Blog duplicate of #1/#2 |
| https://github.com/edheltzel/Recall/pull/225 ; https://arxiv.org/pdf/2608.24060 | Synthetic/unrelated latency figures; not used |
| https://docs.rs/tower-acc ; https://vikas-kumar.medium.com/... | Third-party re-implementations/write-ups of Netflix limiter |
| https://devblogs.microsoft.com/oldnewthing/20170705-00/?p=96535 ; Wikipedia Lock convoy | Secondary; Duffy and Loh used instead |
| https://platform.openai.com/docs/guides/function-calling | Redirect only |
