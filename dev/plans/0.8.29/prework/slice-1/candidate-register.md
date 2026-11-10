# Slice 1 current candidate register

Recommendations for Slice 9, researched November 10, 2026. Nine local refs
remain candidate evidence, not verified open PRs or authorization to merge.

| Ref tip / group | Locked → candidate | Proposed disposition / prerequisite |
| --- | --- | --- |
| 96a47c598 / runtime TLS | rustls .23.40 → .23.46; webpki .13 → .16 | Prioritize bounded current-lock update in 10. rustls reached through ureq2.12.1 optional model downloading in default embedder/reranker. Candidate whole lock is stale: do not merge it. |
| 97286238c / Mermaid | DOMPurify3.4.11 → 3.4.17 | Include contained tooling proposal plus renderer compatibility; current exploitability unproven. |
| 70b93881c / Markdown | cli2 .23.3 → .24.0, markdown-it15.0.1 →15.0.3 | Conditional coherent tool group after AST-neutrality/guard/scoped lint validation; 16 lock nodes. |
| d75aa9e0a / renderer | puppeteer25.3 →25.16, zod3.25.76 →4.6.7 | Investigate coupled browser/renderer major compatibility; 16 lock nodes, not a guaranteed patch. |
| 8f89fdd2e / exporter | certifi2026.7.22 →2026.10.18 | Investigate trust-store applicability and reproducible export/hashes. |
| dc92d053b / exporter | filelock3.32.3 →4.2.1 | Investigate major/tool compatibility. |
| 17d90b9f8 / 49d959742 / exporter | setuptools78.1 →83.0 /84.1 | Competing choice; defer until build need/security applicability is established. |
| 790a5bba7 / exporter | transformers4.44.2 →5.10.1 | Postpone absent explicit need/applicable vulnerability; wide torch/tokenizers/hub/model migration. |

Primary sources opened for selected security claims:

- [Rustls handshake advisory](https://github.com/rustls/rustls/security/advisories/GHSA-2mjx-qc3c-rqvc): affected .23.13–.44 includes locked .40;
  [release .23.46](https://github.com/rustls/rustls/releases/tag/v/0.23.46)
  contains security fixes. Feature/artifact reachability is model-download TLS,
  not permission to infer networking on the local query path. Brotli, session
  Debug and key-exchange cases still require configuration assessment. ECH
  advisory affecting .44/.45 does not apply to locked .40 for that issue.
- [DOMPurify advisory](https://github.com/cure53/DOMPurify/security/advisories/GHSA-c6gr-qvv4-5v3f): <=3.4.16 affected, fixed3.4.17;
  exploit requires non-default XHTML plus allowed animation/attribute settings.
  Current renderer configuration exploitability was not established.
- [Markdownlint .24.0](https://github.com/DavidAnson/markdownlint-cli2/releases/tag/v0.24.0)
  confirms candidate release, not a mandatory/latest recommendation.

The checked-in August 4 advisory snapshot governs npm overrides (none active)
and marks external Rust sources unassessed. This is selected primary-source
research, not a comprehensive advisory/license/yank/alert audit.
