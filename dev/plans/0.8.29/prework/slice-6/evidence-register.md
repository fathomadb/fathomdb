# Slice 6 evidence and limits

PhaseA source observations finalized in PhaseB, November10,2026. Product source `6edc1155f039f87cd0a37660cb27ae4373bbce71`;
planning baseline `850f2c15df1ebf3262f0346e340721f9ff2422ad`. Reads are not runtime
example tests or current release qualification. Paths below name exact repository
sources; sections/symbols identify claims without fragile code-line anchors.

## Repository evidence

| ID | Claim / location and fresh source comparison | Maturity, counterevidence and receiving rows |
| --- | --- | --- |
| E6-1 | README “Release0.8.28”/SDK install table; docs/index.md release banner; docs/install/typescript.md requirements and retained Tegra route; schema/lib.rs SCHEMA_VERSION=37; interfaces/wire.md current-only37. | Published `.28` instructions remain applicable. `.29` planning does not justify version promotion. No package install, remote download or example execution. D6-1. |
| E6-2 | dev/needs.md, requirements.md and test-plan.md “Current applicability” say current format36. Accepted ADR-0.8.28-transactional-visibility-coalescing.md, schema const and maintained wire contract say37. | Concrete maintained-banner mismatch; preserve stable IDs, original titles, historical migration fixtures and thresholds. Owner seq-368 forbids `.29` migration. D6-6. |
| E6-3 | dev/design/mcp-adapter/architecture.md explicitly scopes SDK facts to `.27`; design.md and dev/interfaces/mcp.md preambles say SDK `.27` as-is. Interface config/hints already records queue/hints evolution. | Preserve evaluation baseline; annotate current integration rather than globally declaring proposed documents implemented. MCP crate exists in current workspace; accepted hints/queue decisions and built source do not prove every MR. D6-2. |
| E6-4 | dev/interfaces/mcp.md common envelope/search; MCP edge.rs::to_result attaches structured content for success/errors and tool output schemas. tools.rs::SearchArgs has no search view flags; search rejects text filters before invocation. Maintained Rust/Python/TS filtered body-FTS sections and engine search source support filters/ReadView axes. | Current adapter restriction is real; interface's assertion “engine rejects them on every search path” is stale. Text-default in agent-access/compact-text-block.md §7 is owner-approved October27 but source lacks the new mode;60 owns behavior/contracts. D6-3. |
| E6-5 | query README “all sub-three-character” fallback versus compiler docs and complete short_stopwords_are_still_dropped, one_and_two_char_content_tokens_are_kept and short_token_only_query_takes_the_content_path tests in query/src/lib.rs. Python README opening “no network call” versus its own Embeddings section, accepted ADR-0.7.1-default-embedder-weight-fetch and embedder candle_bge loader/open_default_embedder source. | Query short content survives; all-stopword/symbol fallback remains. Python default no-embedder path is distinct from explicit cold-weight fetch. Test bodies/source read, not run. Package README inputs are published via Cargo/pyproject. D6-8/9. |
| E6-6 | dev/DOC-INDEX.md `.28` plan/state/board rows say active with work remaining; release-notes row says pending format36. Actual `.28` state/status record published completion and format37. Root0.8.23-release-todo.md historical banner still calls `.25` active. docs/index.md Concepts summary says “five-verb surface”; accepted governed-surface ADR and REQ-053/AC-074 supersede the cap. | Update live navigation/applicability only. Historical todo body, failed runs, boards and release bindings remain. No `.29` active claim before10 state activation. D6-7/10/12. |
| E6-7 | AGENTS setup, bootstrap.sh and agent-build.sh advertise/invoke editable install; agent-verify.sh full preflight demands checkout-owned .venv. Verified memory agent-worktree-stale-base-trap.md forbids worktree editable/maturin develop. Contributor guide requires isolated writers and relevant scoped/full checks. | Reviewed Slice0 proposal routes owned noneditable/source-features-bound setup to10. No env prepared and no executable script change here; later actual script fixes require TDD. D6-4. |
| E6-8 | generated contributor/SDK maps explicitly name generator and tracked inputs. generate-public-docs.py reads Cargo workspace/member manifests, Python/TS manifests, parity manifest and typed-script hashes; SOURCE_URL is release/0.8.28. docs-pages/build-pages-site owns site checking. v2 current profile says11 members and44 governed operations. | Fresh parser count:12 workspace members including MCP;53 operation entries, all live, in governed-operation-parity.json. interfaces/rust-sdk.md independently says53/53 live Rust endpoints. These are distinct from allowlist member counts and SDK-only Rust overloads. Generator --check exits0/current; remote linked URL existence not checked. D6-11/13. |
| E6-9 | dev/design/release.md publisher and post-publish sections; `.28` Slice95 status; fresh T6-1 messages. | SDK registration repair already complete; run42750034084 was authentication only with35 build/publication jobs skipped. Assembled `.28` qualification retains original failures. No fresh remote/settings check or repeated repair. D6-5. |
| E6-10 | `.27` Slice90 status final table at1398c821dd26b7945bb2f6fbfa02b68cd4daa8af and final-review-verification.md; root recovery plan and Slice85 receipt/status remain earlier chronology. | Later90 explicitly reconciles recovery294af94b5:21GPU runs/349passed/8ignored/0failed;85 binding remains7a2f9bf9. Original disposable85 baseline payloads absent; historical verifier compared authoritative recorded hashes, not absent payload bytes. This owner read repository records only, not external raw evidence or gates. Optional history pointer; no new qualification condition. D6-14. |

Relative entry points: [MCP interface](../../../../interfaces/mcp.md),
[wire](../../../../interfaces/wire.md), [Rust SDK contract](../../../../interfaces/rust-sdk.md),
[v2](../../../../design/fathomdb-data-plane-architecture-v2.md),
[format37 ADR](../../../../adr/ADR-0.8.28-transactional-visibility-coalescing.md),
[generator](../../../../../scripts/generate-public-docs.py),
[release design](../../../../design/release.md),
[.27 final status](../../../0.8.27/features/slice-90/status.md),
[.27 final verification](../../../0.8.27/features/slice-90/final-review-verification.md).

## Fresh bounded chat-message reads

The inherited evidence-register already records two actual turn reads per chat.
The older planning-evidence extraction reports different larger read windows;
that extraction is inherited, not this owner's transcript coverage. This owner
freshly called read_thread for two latest turns per named chat, with tool outputs
excluded, then inspected actual user/agent message items. No transcript dump,
command history, credential or raw remote payload is saved in these documents.
Both returned hasMore=true; all older turns remain outside this fresh review.

| ID / chat | Exact turns and dates | Actual messages and evidence limit |
| --- | --- | --- |
| T6-1 “Complete Slice 90”,01a1a80a-e240-7e82-9ced-d701d105a3be |01a1c37f-9494-7791-979c-c55f32da6f05,2026-11-10 03:28:00–03:38:13 UTC;01a1c373-be13-7a21-a016-2916b1077ed5,03:15:04–03:26:31 UTC. Both November9 America/Chicago. | Final message msg_05cb32226949d66c016af291a445e087d2a8c961fcb5c67c3a reports saved publisher, OIDC-only success and35 skipped jobs; preceding final msg_05cb32226949d66c016af28ee59a4c87d2abb43aad1634218a requests settings sign-in. Historical completion, consistent with current release design; no settings/run/API recheck now. |
| T6-2 “Find unfinished 0.8.28 work”,01a1a8aa-844e-7ed0-84bf-5f6669d85986 |01a1a9b5-f88f-76b3-bfcc-8928aa8c175b,2026-11-05 03:17:17–03:19:12 UTC;01a1a8ad-a818-7b33-9845-db8c5898f131,November4 22:28:35–22:30:09 UTC. Both November4 America/Chicago. | Final msg_0e392171a7eb814b016aebf5aea30087d2868d6cb0fee8f698 reports49GiB reclaimed/120GiB free then. Final msg_0c408d68d846f701016aebb1e200b487d2953bd5cc4b755af7 audits fdb9fa932/unmerged `.28`/format36/old120 close. Superseded by6edc1155f integration/format37/current160 allocation, not by speculative transcript reinterpretation. Disk relief is historical and grants no current cleanup. |

## Predecessors, fit and gaps

Slice0 at850f2c15d is certified proposal output, not setup. PhaseB reread actual
root2 plan/acceptance/dispositions/research. Parent reports its independent final
Astra PASS and passed scoped checks, with completion commit being prepared;
root2 phase text still predates that report. Retention, selected in-place10
corrections, optional history annotations and final `.27` recovery disposition
agree with E6's independently inspected sources. Do not infer completed product
qualification, cleanup or raw-payload revalidation. Slice1 is imported with
fresh final review running, not certified yet. Slices3–5 earlier local mappings
remain draft cross-checks. No new source/test behavior or evidence scope was
introduced after Slice6's parent-reported design PASS.

Relevant verified memory files read: verify-design-against-code-not-just-architecture.md,
onnx-gpu-not-supported.md, acceptance-md-locked-no-feature-acs.md,
0.8.27-slice132-rust-sdk-lessons.md, release-publish-gotchas.md,
agent-access-block1-design-state.md, long-record-embedding-findings.md and
agent-worktree-stale-base-trap.md, under the canonical FathomDB memory directory.
Historical AC-073 ceiling and old B2-8 waiting text are superseded by actual
canonical082d/owner rulings; preserve their invariants, not stale counts/status.

Unknowns: uninspected older turns; all example runtime outcomes; full public/API/
internal doc audit; current runner/auth/registry settings; external raw recovery
payloads; all remote generated links; installed `.29` behavior. Missing authority
blocks the affected asserted correction, not unrelated source-based proposals.
