---
title: Current MCP conformance qualification for 0.8.29
date: 2026-11-16
status: PROPOSED
desc: Fresh both-format official-suite and direct-stdio results, with supported-capability applicability and unresolved upstream protocol discrepancies distinguished.
inputs:
  - dev/interfaces/mcp.md
  - dev/design/mcp-adapter/requirements.md
  - dev/design/mcp-adapter/conformance-report.md
  - dev/experiments/mcp-conformance/README.md
---

# Current MCP conformance qualification for 0.8.29

Fresh execution on November 16 qualifies the three
applicable official server scenarios in **both formats**. Each full run retained
all 32 scenarios and raw failures:  9 successful checks, 24 failed checks, 2 warnings
and 4 informational observations. This is not a blanket JSON-RPC compliance or
Block 1 exit claim. The upstream discrepancies below remain visible and unresolved.
Selected 30/40/50 measurements and approved LLM episodes remain separate gates;
this qualification does not close those gates.

## Original suite subject and tools

- Production source:`999acc37826efa3808afc0d1a899e08bd6478f3a`, including the
  accepted Slice 90 implementation. Tracked tree:
  `3603aabdfaa7e7ae9059f93c2b541fe1c82ef18a`.
- Fresh release `fathomdb-mcp`, no optional features or test seams, rmcp 3.5.1,
  Rust 1.95.0. Its native path is
  `/mnt/data/tmp2/fathomdb-0829-continuation/final-mcp-20261116/fathomdb-mcp-providerfree`;
  SHA-256 `d3214304f5c28447143aec86d70f07098a957d3e0686bc4fab0316c25edd5650`.
  Cargo records only the empty `default` feature and no rustflags. The fixture
  was compiled against this build's production SDK closure. The earlier ef544
  executable and November 15 results remain historical evidence.
- Task-owned npm environment:Node 25.9.0, npm 11.12.1, conformance 0.1.16,
  Inspector 2.11.0, supergateway 4.2.0 and reference SDK 1.32.1. The final locked
  installation used the owned npm 11.12.1; lock and installation logs remain
  external.
- Raw root:`/mnt/data/tmp2/fathomdb-0829-continuation/final-mcp-20261116/conformance`.
  All suite `checks.json` files, bridge logs, Inspector stdout/stderr/exits,
  SDK transcripts and raw lifecycle transcripts remain there; command/build
  logs are in the task's sibling `logs` directory. No expected-failure baseline
  or result rewrite was supplied.

## Affected native refresh

The accepted typed-open admission correction was compiled separately at source
`222fa39542662f73be943c02d0c8ab14b59e84bb`, tree
`f11dea5af95a93f93586f47526da608eb3e7cf72`. Bin-only release builds retain no
`test-seams` feature or rustflags. Under the task's
`refresh-open-222fa3954` directory, the preserved subjects are:

- `fathomdb-mcp-providerfree`, SHA-256
  `11728963082068f29ff49d13731d90fdb139a515bfeb46b82b3af2f3864bc65a`.
- `fathomdb-mcp-cuda`, SHA-256
  `b3c6f256fe88f37b8287dbef29b0610ef4cd7ad029e78661eebfb344914f3eed`;
  features `default-embedder,embed-cuda` plus the empty default.

Pinned Inspector direct-stdio checks on the fresh provider-free subject passed
initialize, strict seven-tool discovery and healthy text search in both formats.
Each format also returned the exact typed `unavailable/corrupt` error with
`detail=CorruptionError` for independently corrupted default-profile name,
revision and dimension. The retained old-999 name control instead reports
`unavailable/open_failed` with `detail=StorageError`. These probes use isolated
format-37 fixture copies and retain stdout, stderr and actual exits under that
refresh directory. The unchanged 24 native source tests also pass with the fresh
provider-free server and the compatible producer-999 fixture helper.

The error-only admission correction leaves healthy protocol paths unchanged;
the original full official/SDK/lifecycle results remain scoped to their
actual producer 999 and were not retagged or replayed.

The fresh CUDA executable subsequently completed the approved 14-call stage
once, using the pinned Inspector and isolated fixture copies. In each format,
initialize, strict seven-tool discovery, store listing and a real hybrid search
succeeded; search returned one hit with no degradation. All six malformed-profile
calls returned `unavailable/corrupt`, `detail=CorruptionError`, with expected
Inspector exit 5. Raw `cuda-{format}-{case}` stdout/stderr/exits remain in the
refresh directory. The original 5882-turn and 419-turn ready stores were untouched.

Execution held the selected RTX 3090's physical lease, verified driver
580.178.04, and retained the existing resource guard, 120-second per-call and
900-second overall limits with 20-second termination grace. The stage exited 0
without a resource stop. The guarded actor and all owned MCP actors were absent;
cleanup and a fresh postcheck found no GPU compute processes, 2 MiB used, and the
same-inode physical lease available and released. These conformance probes used
no paid model; the subsequently completed approved 72-episode sequence is
reported separately in the [agent results](../../experiments/agent-remeasure/README.md#current-candidate-results-2026-11-16).

## Official-suite applicability

The suite supports HTTP only. For each format, a separate task-owned
supergateway listener on 127.0.0.1 bridged to the actual stdio binary. Its process
group was terminated after the run, including session children. Direct checks
below used no bridge. Applicability follows MR-001.2/.3 and the current
[interface](../../interfaces/mcp.md):stdio transport, exactly seven read-only
tools, tools-only capabilities and one text content block per result.

Both formats produced the same scenario-level outcomes. The table enumerates
every scenario; fixture-only failures are not counted as passes of our tools.

| Scenario | Raw result | Current classification |
| --- | --- | --- |
| server-initialize | pass | Applicable:handshake. |
| ping | pass | Applicable:lifecycle utility. |
| tools-list | pass | Applicable:actual seven tools. |
| logging-set-level | -32601 | Undeclared logging capability. |
| completion-complete | pass | Undeclared completion; empty upstream default, no qualification credit. |
| tools-call-simple-text | -32602 | Requires absent reference fixture `test_simple_text`; actual tools checked directly. |
| tools-call-image | -32602 | Requires absent fixture; no image output contract. |
| tools-call-audio | -32602 | Requires absent fixture; no audio output contract. |
| tools-call-embedded-resource | -32602 | Requires absent fixture; resources undeclared. |
| tools-call-mixed-content | -32602 | Requires absent fixture; our contract is one text block. |
| tools-call-with-logging | -32601 | Requires undeclared logging before fixture call. |
| tools-call-error | -32602 | Requires absent fixture; actual typed errors checked directly. |
| tools-call-with-progress | -32602 | Requires absent fixture; this does not qualify actual queue progress. |
| tools-call-sampling | -32602 | Requires absent fixture; no sampling request contract. |
| tools-call-elicitation | -32602 | Requires absent fixture; no elicitation contract. |
| json-schema-2020-12 | fixture absent | Requires `json_schema_2020_12_tool`; actual discovery/schema validation checked directly. |
| elicitation-sep1034-defaults | -32602 | Requires absent elicitation fixture. |
| elicitation-sep1330-enums | -32602 | Requires absent elicitation fixture. |
| server-sse-polling | 2 warnings | HTTP bridge only:no priming event or retry field. |
| server-sse-multiple-streams | pass | HTTP bridge only; no stdio-server qualification credit. |
| resources-list | pass | Undeclared resources; empty upstream default, no qualification credit. |
| resources-read-text | -32601 | Undeclared resources. |
| resources-read-binary | -32601 | Undeclared resources. |
| resources-templates-read | -32601 | Undeclared resources. |
| resources-subscribe | -32601 | Undeclared resources. |
| resources-unsubscribe | -32601 | Undeclared resources; scenario first subscribes. |
| prompts-list | pass | Undeclared prompts; empty upstream default, no qualification credit. |
| prompts-get-simple | -32601 | Undeclared prompts. |
| prompts-get-with-args | -32601 | Undeclared prompts. |
| prompts-get-embedded-resource | -32601 | Undeclared prompts. |
| prompts-get-with-image | -32601 | Undeclared prompts. |
| dns-rebinding-protection | 1 pass/1 fail | HTTP bridge accepted hostile Host/Origin with 200; server has no listener. |

Thus 3 applicable scenarios pass and 0 applicable official scenarios fail per
format; 29 scenarios are outside the declared server capability or fixture.
The full command still exits 1 in each format; raw failures were not waived.

## Direct stdio results

The real fixture was written and closed through the exact production SDK.
It contains three JSON-bodied notes with a declared filterable `group`, two
versions of another logical note, one pending note and an actual graph edge.
No embedder, paid model, GPU, fault injection or mocked database was used.

- Inspector:initialize, strict discovery and all seven tools succeeded in each
  format. Unknown store, wrong query type and absent tool each exited 5 as
  expected. Strict discovery reported 0 errors, 35 text warnings and 108 structured
  warnings; nullable single-type portability warnings remain raw, not failures.
- Reference SDK: 27 checks per format passed. Discovery keeps inputs identical
  and emits output schemas only in structured mode. Every observed success and
  typed error has one text block; text omits structuredContent, and structured
  content passes the SDK's advertised output-schema validation and equals the
  parsed text payload. Observed compact budget counts match final UTF-8 bytes.
- The SDK session calls every tool, follows every page on unchanged state with
  no gaps/duplicates, enforces a direct kind/attribute filter, and rejects absent
  source metadata. Independent existence-flag combinations return 4/5/5/6 hits.
  Fetch preserves duplicate outcomes and per-item errors, graph expansion
  returns the real neighbor, and same-session feedback returns its current
  nonrecording result. Wrong argument types, null flags, unknown attributes and
  continuation overrides produce format-correct typed errors. None-provider
  hybrid reports the exact current absent/unavailable degradation pair.
- Raw lifecycle: 18 characterization probes per format matched recorded behavior,
  including version negotiation, string IDs, EOF exits 0 before/after handshake,
  omitted arguments, cancellation/unknown-notification survival, batch rejection,
  undeclared methods and no socket in the stdio child's file descriptors.
  These 18 are characterization results, not 18 normative compliance passes.

## Unresolved protocol discrepancies and limits

- Malformed JSON lines receive no -32700 parse-error response. Current interface
  §1 explicitly documents ignored unparsable lines; that accepted launch behavior
  was observed, but differs from general JSON-RPC parse-error expectations. The
  historical report §6.2 remains accurate; this run adds no waiver or fix.
- Malformed `tools/call` envelopes (missing `name`, nonobject `arguments`) return
  -32601 instead of JSON-RPC -32602. Interface §6 reserves JSON-RPC errors for
  protocol faults but does not expressly approve this wrong code. This is an
  **unresolved upstream protocol discrepancy**, not a tool-argument-validation
  pass and not excused by upstream ownership. It occurs before adapter typed
  parsing; a mandatory general JSON-RPC error-code requirement would require a
  scoped transport/dispatch correction with RED/GREEN, not a baseline waiver.
- An unknown tools/list cursor returns the entire fixed list. The MCP pagination
  recommendation is SHOULD reject invalid cursors; this server issues no tool-list
  cursors and MR-006.7 fixes discovery across sessions/configurations. It remains
  an upstream recommendation discrepancy; actual search continuations are distinct
  and were exercised successfully. No requirement was silently narrowed.
- No current supported official scenario failed. This does not establish that
  every protocol fault conforms to JSON-RPC, or requalify queue timing, provider
  inference, full budgets, source tests, scientific measurements or LLM smoke.
  Independent review must assess the distinction above before this evidence is
  used for the final Block 1 qualification. This work changes no production
  source or release state.

Reproduction uses [the actual probes](../../experiments/mcp-conformance/README.md).
