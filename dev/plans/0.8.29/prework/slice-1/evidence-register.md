# Slice 1 dated evidence and bounded research limits

Read-only source and selected primary web review November10,2026. Product
baseline6edc1155f; initial planning baseline40d82c2e7; subsequent Slice0 proposal
integration850f2c15d is documentation only. Derive live HEAD/branch before use.
No package install, audit/database refresh, lock rewrite, mutable sweep, artifact
execution, remote ref fetch or live repository-alert/PR query ran for1.
High confidence denotes directly observed source or advisory range; configuration
and exact artifact reachability remain bounded inferences until selected-build
qualification. Primary pages were opened successfully through the read-only web
tool; no unavailable endpoint was retried in this phase.

## Selected primary advisory/version evidence

| Evidence and source publication date | Direct observation / local fit | Confidence, bounded disposition and receiving owner |
| --- | --- | --- |
| [Rustls handshake GHSA-2mjx-qc3c-rqvc](https://github.com/rustls/rustls/security/advisories/GHSA-2mjx-qc3c-rqvc),2026-09-14 | Affects0.23.13–0.23.44; fixed0.23.45. Locked0.23.40 is in range. Wrong-encryption-level handshake messages may be accepted; authenticated transcript still prevents attacker handshake alteration/completion. | High range confidence; optional model-download TLS is reachable by source. Approved10 current-lock0.23.46 update addresses this issue; no claim that ordinary local queries perform TLS. |
| [Rustls0.23.46 release](https://github.com/rustls/rustls/releases/tag/v/0.23.46),2026-10-14 | Contains the linked Brotli/session-Debug/key-exchange/ECH fixes. Observed local candidate chooses this release. | High version/release confidence; selection is already approved contained scope, not an assertion that every listed exploit affects this configuration.10 verifies actual closure and artifact behavior. |
| [Brotli GHSA-h2v8-394m-p7vv](https://github.com/rustls/rustls/security/advisories/GHSA-h2v8-394m-p7vv),2026-10-14 | rustls<0.23.46 affected when Brotli certificate decompression is enabled. Peer-controlled history window may cause large pre-auth allocation. Built-in support is disabled by default. | High range/precondition confidence; inspected ureq enables rustls ring/logging/std/tls12 with default-features=false, no Brotli edge. Other-feature unification/exact artifact not executed, so applicability remains conditional.10 verifies selected feature graph. |
| [Session Debug GHSA-c8m5-c952-6cm4](https://github.com/rustls/rustls/security/advisories/GHSA-c8m5-c952-6cm4),2026-10-14 | Stable0.23.40 is below fixed0.23.46. Custom session-store Debug logging can expose secrets; normal connection/built-in-cache diagnostics remain redacted. | High advisory confidence; no custom ClientSessionStore or session-value Debug logging found in inspected FathomDB downloader. Default ureq config is used. Exact downstream/library use still needs assessment; no current leak is asserted. |
| [KX hint GHSA-w44r-93j7-qv28](https://github.com/rustls/rustls/security/advisories/GHSA-w44r-93j7-qv28),2026-10-14 | rustls<0.23.46 affected. Unauthenticated per-name hints can influence subsequent key shares; hybrid-to-classical impact depends on provider/groups, resumption state and server behavior. | High advisory confidence; cached ureq source selects ring, whose configured path does not establish hybrid MLKEM use. No demonstrated downgrade for this product path.10 records selected provider/session config and retains fixed target. |
| [ECH GHSA-vp88-798v-h67f](https://github.com/rustls/rustls/security/advisories/GHSA-vp88-798v-h67f),2026-10-14 | Advisory/release identifies0.23.44/.45 and buffered application data on ECH rejection. | High range confidence: locked0.23.40 is outside this issue's stated affected pair. Do not use it as urgency evidence for0.23.40. Fixed0.23.46 remains selected. |
| [DOMPurify GHSA-c6gr-qvv4-5v3f](https://github.com/cure53/DOMPurify/security/advisories/GHSA-c6gr-qvv4-5v3f),2026-10-12 | <=3.4.16 affected,3.4.17 fixed; locked3.4.11 in range. Requires non-default XHTML parser plus allowed animation/attribute configuration. | High range, medium-low actual-config confidence. Inspected repository wrapper supplies Puppeteer launch config and forwards extra mmdc flags; installed Mermaid sanitizer internals were not inspected.10 identifies actual config and qualifies contained fix; no default exploit is claimed. |
| [markdownlint-cli2 v0.24.0](https://github.com/DavidAnson/markdownlint-cli2/releases/tag/v0.24.0),tagged2026-10-21 | Confirms observed candidate release; page says version update, not a security mandate or complete migration assessment. | High release existence confidence; no “latest required” inference. Optional tool group remains deferred until need/compatibility evidence and9 selection. |
| [PyTorch GHSA-53q9-r3pm-6pq6](https://github.com/pytorch/pytorch/security/advisories/GHSA-53q9-r3pm-6pq6),2025-04-17 | torch<=2.5.1 affected,2.6.0 fixed; exporter2.4.1+cpu is in range. `torch.load(weights_only=True)` can execute code from crafted model data. | High range confidence; actual exporter exploitability unproven. Source loads a pinned local snapshot through AutoModel, but does not explicitly force safetensors. Before any exporter run, exporter owner must establish exact input serialization/trust and exclude unsafe pickle fallback or propose a reviewed coherent mitigation/upgrade. No new upgrade is approved by this finding. |

## Reachability and source limits

`Cargo.lock` inverse edges show rustls0.23.40←ureq2.12.1←fathomdb-embedder;
rustls-webpki0.103.13 is under rustls. Embedder manifest makes ureq optional,
default-features=false, features=[tls]; both default embedder and reranker
features enable it. `src/loader.rs::download_once` and
`src/candle_reranker.rs::download_with_retries` construct default AgentBuilder
TLS, adding timeouts/redirect bounds but no custom ClientConfig/session store.
Downloads are opt-in provider/model lifecycle, with hash/cache contracts, not
arbitrary network transport for retrieval. Artifact-specific feature selection
still needs10 verification; lock presence alone does not establish runtime use.

A read-only cached registry source for ureq2.12.1 was available under the local
Cargo cache. Its manifest requests rustls0.23.19 range with ring/logging/std/tls12
and default-features=false; its `src/rtls.rs::default_tls_config` uses ring,
TLS1.2/1.3, roots and no client auth. This is supportive source inspection,
not new downloaded artifact authentication or an executed feature graph.
FathomDB source selects no Brotli/custom-store/ECH configuration in that path;
feature unification and third-party default changes must be checked on the
actual selected build. No universal non-applicability verdict follows.

Mermaid lock shows mermaid→DOMPurify; wrapper accepts externally supplied
source and forwarded config flags and launches Chromium with the checked-in
Puppeteer file. No user diagram or sanitizer exploit was executed. Browser
rendering is developer tooling exposure, not an SDK browser/runtime dependency.

Exporter exact revision is `5c38ec7c405ec4b44b94cc5a9bb96e735b38267a` for
BAAI/bge-small-en-v1.5; current code checks local snapshot identity, then uses
AutoModel.from_pretrained(local_files_only=True). Local-only and revision pins
do not prove safe serialization when snapshot bytes are uninspected. Its stale
module guidance to borrow Memex's interpreter conflicts with owned-tool policy;
already approved10 documentation corrections must replace that instruction
before reuse. No Memex environment was read, installed into or used here.
Pending exporter safety assessment blocks exporter execution/clearance, not
unrelated current-lock Rustls/DOMPurify preparation.

## Historical and uncollected evidence dispositions

| Evidence boundary | Status at this proposal | Proposed disposition / owner / blocking extent |
| --- | --- | --- |
| `scripts/pinned-override-advisories.json` and metadata | Historical2026-08-04 minimal npm snapshot; root overrides currently empty; external Rust source explicitly unassessed. | Retain deterministic snapshot untouched. A selected exception/source revision change needs fresh human source/advisory assessment in10/140. It is never whole-tree security evidence. |
| Dependabot comments | September2 shipped-tree zero-advisory claims and standing pause are dated; all five entries still paused. | Keep pause.10 performs selected manual review; next release dependency owner receives recurring backlog.9 explicitly disposes optional coverage expansion, not unpause by inference. |
| Live alerts, open/closed PR state, remote-tracking freshness | Not queried/fetched, not certified unavailable. Local tips observed only. | No PR close/merge/action derives from them. Receiving dependency owner queries permitted live sources if an action depends on state; proposal completion does not need an invented alert result. |
| Registry latest/version yanks across all graphs | Uncollected; no mutable online sweep ran. | All unknown targets remain unknown, current choices retained/deferred with triggers.10 checks selected changed groups' actual resolver metadata/yanks before promotion; broad maintenance is a9-disposed follow-up. No latest/current/yank-clean claim. |
| Fresh licenses for full Rust/npm/PyPI graphs | Uncollected. `deny.toml` currently denies yanks and allows Apache-2.0/MIT/Unicode-3.0; scope is Cargo policy, not universal license clearance. | Preserve NEED-005; selected changed nodes require source license/policy review in10. Broader survey cannot mark a release compliant without its actual owned checks/disposition; do not silently widen license allowlist. |
| Exporter candidate hashes/releases and transitive compatibility | Local proposed version/hash lines only; official wheel identity/migration notes not fetched for these unselected groups. | Defer promotion; future exporter owner validates exact wheel provenance,28-pin compatibility and existing model SHA. PyTorch advisory assessment above is prerequisite before a run even if retaining pins. |
| Qualified forks / SQLite / ABI / MCP baselines | Source and prior records inspected; no new campaigns. | Retain. Required140 repair/source changes need fork-local qualification before pins; .30 maintenance is not .29 prerequisite. No old receipt becomes new product qualification. |
| Tools, capacity, browser/model bytes, GPU/VM readiness | Slice0 completed proposal inventories inherited observations and unknowns, not readiness. |10 owns setup/path/capacity; downstream owner owns exact artifact/corpus/venue. Specific unavailable cells stay unmeasured and block only their claims. R580/primary12.6 and publication gates remain. |

These are bounded dispositions proposed for9/receiving owners. They permit
reviewing the complete dependency proposal without fabricating a comprehensive
security/license/yank sweep. Actual applicable findings are triaged before
affected use; unknown mandatory evidence cannot be renamed a PASS.
