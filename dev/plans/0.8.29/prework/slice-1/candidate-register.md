# Slice 1 local candidate register

Source comparison November10,2026 against dated planning40d82c2e7/product6edc1155f;
Slice0 documentation integration850f2c15d subsequently inspected.
Nine local remote-tracking refs below are observed evidence, not freshly fetched
open PRs, security alerts, qualified targets or permission to merge. Targets are
observed versions, not a claim of latest. Compare changes to current source;
apply only a reviewed coherent group, never stale full candidate ancestry.
Primary/configuration evidence and limitations are in [evidence](evidence-register.md).

## All nine full refs and proposed dispositions

Ref names share `refs/remotes/origin/dependabot/`. S/M/L are comparative effort,
not time estimates. Every row remains a proposal even where scope is approved.

| Local ref suffix / full tip | Current → candidate and edit sites | Disposition / understanding, risk, blast, effort | Receiving owner, prerequisite, qualification and reopen trigger |
| --- | --- | --- | --- |
| `cargo/rustls-0.23.46` / `96a47c5982cbddc22853aff955a668a246fd00b3` | `Cargo.lock`: rustls0.23.40→0.23.46; rustls-webpki0.103.13→0.103.16 | Include approved contained group; understood high for local graph, medium for exact-build config; risk medium; contained; S | 10 dependency owner after its independent design review/owned setup. Adapt current lock, retain MCP/E/fork/SQLite closure, selected TLS feature/download/hash/cache checks and final source gate. Reopen scope only if needed closure exceeds this group; no new generic owner permission. |
| `npm_and_yarn/dev/tools/mermaid/dompurify-3.4.17` / `97286238cf52c72cf761ac14ee8625083b0cc851` | `dev/tools/mermaid/package-lock.json`: DOMPurify3.4.11→3.4.17; one version node | Include approved contained group; understanding medium (actual renderer config unresolved); risk low-medium; contained; S | 10 tool owner under owned Node/npm/browser setup. Verify lock integrity and real safe/hostile-input sanitizer expectations under actual selected config; run existing renderer acceptance. Advisory exploitability unproven; do not couple Puppeteer/zod update. |
| `npm_and_yarn/multi-68e90142e5` / `70b93881c2bd4082d4ed95d6a0a65965d3f0db67` | `package.json`/root lock: cli2 ^0.23.3→^0.24.0; markdown-it15.0.1→15.0.3 and16 version-node changes | Conditional/defer from10 until selected; understanding medium; risk medium; coherent tool group; M | Next dependency/tool owner;9 decides if needed in .29. Primary migration/advisory review, AST-neutrality and guard regression plus scoped full-tree Markdown/link checks under pinned Node/npm. Reopen for applicable advisory, needed rule/fix or intentional tool maintenance; no inferred security urgency. |
| `npm_and_yarn/dev/tools/mermaid/multi-010aad1c49` / `d75aa9e0a355d120b8602f1de295877ce107fb0b` | Mermaid lock: Puppeteer25.3.0→25.16.0; zod3.25.76→4.6.7 and16 version-node changes | Investigate/defer; understanding medium-low; risk medium-high; coupled browser/major group; M | Next renderer owner;9 selects separately from DOMPurify. Check actual Mermaid/Puppeteer/zod compatibility and browser availability/launch/real SVG,PNG,PDF,Markdown wrapper contract. Reopen for applicable renderer/browser defect/security or supported-platform need. |
| `pip/dev/tools/onnx/certifi-2026.10.18` / `8f89fdd2ee777edc3e0a063baa96c2c6af1a5c1d` | Export requirements: certifi2026.7.22→2026.10.18 with two proposed hashes | Investigate/defer; understanding medium-low; risk medium (trust-store change); contained dependency within coupled exporter; S/M | Next exporter owner;9 selects if explicit need/applicable trust-store advisory. Verify official wheel identity/hashes and certificate-store change, install hashes/no-deps in owned CPython3.12 Linux x64 export env, reproduce current model SHA. Candidate hashes are not certified artifact provenance. |
| `pip/dev/tools/onnx/filelock-4.2.1` / `dc92d053bc0ed3130872370af2385d8b3f4157a2` | Export requirements: filelock3.32.3→4.2.1 with two hashes | Investigate/defer; understanding low; risk medium; major/API group; M | Next exporter owner;9 selects if explicit need/applicable advisory. Primary migration/dependency compatibility and concurrent/cold/warm cache/export correctness, exact wheel/hash and current model identity. Reopen for selected exporter maintenance or demonstrated lock/API defect. |
| `pip/dev/tools/onnx/setuptools-83.0.0` / `17d90b9f801404e568dd074db4dac5b359da5878` | Export requirements: setuptools78.1.0→83.0.0 with two hashes | Investigate/defer competing choice; understanding low; risk medium; build tool group; M | Next exporter/build owner; evaluate with next row as alternatives, not cumulative upgrades. Establish actual build/security need, supported no-deps wheel set and export SHA before9 selection. Reopen for applicable build advisory or reproducibility failure. |
| `pip/dev/tools/onnx/setuptools-84.1.0` / `49d9597420979b1be0064d5516ca49d30ff1ab39` | Export requirements: setuptools78.1.0→84.1.0 with two hashes | Investigate/defer competing choice; understanding low; risk medium; build tool group; M | Same owner/checks as83 candidate. Higher observed number is not selection evidence. No forced choice or live PR closure; choose one justified target or retain current after scoped review. |
| `pip/dev/tools/onnx/transformers-5.10.1` / `790a5bba7682613cad8a8425567dacec6bb7fd7f` | Export requirements: transformers4.44.2→5.10.1 with two hashes | Postpone wide migration; understanding low for target compatibility; risk high; migration; L | Future exporter/model owner;9 must explicitly select if applicable advisory/needed feature changes priority. Review torch2.4.1+cpu/tokenizers0.19.1/hub0.36.2/ONNX export APIs as one graph, model/tokenization/output identity and reproducibility. No autonomous model/golden hash replacement; incompatibility blocks promotion. |

## Current-source reconciliation details

Rustls tip's one-commit patch touches only its lock, but its whole tree is old:
workspace0.8.27/E0.6.1, old Candle0.10.2/cudarc0.19.7,
rusqlite0.40.1/libsqlite3-sys0.38.1 and no current MCP. Current .29 retains
W0.8.28/E0.8.0, fork0.11.0/cudarc-fathomdb0.19.10,
rusqlite0.41.1/libsqlite3-sys0.39.1 and MCP/rmcp3.5.1. Therefore copying the
tip lock would regress qualified predecessor work. Current-lock update only.
Other candidate touched files were compared to current source; exporter changes
are isolated version/hash lines and may still be dependency-incompatible.
DOMPurify row changes only its node; the browser candidate leaves that node at
the old3.4.11, so blindly replacing its lock later would undo the approved fix.

Root Markdown16 version-node deltas: ansi-regex6.3.0→6.4.0,
get-east-asian-width1.6.0→1.7.0, js-yaml5.4.1→5.4.3,
markdown-it15.0.1→15.0.3, its argparse3.0.2→3.0.3,
markdownlint0.41.1→0.42.0, cli2 0.23.3→0.24.0,
micromark4.0.2→4.0.3, core-commonmark2.0.3→2.0.4,
extension-gfm-table2.1.1→2.1.2, factory-space2.0.1→2.1.0,
util-types2.0.2→2.0.3, new util-edit-map1.0.0/re2js2.8.6,
smol-toml1.8.0→1.9.1 and string-width8.2.1→8.3.0.
These are package-lock locations, not16 independent approved upgrades.

Mermaid/browser16 version-node deltas: @puppeteer/browsers3.0.6→3.2.6,
ansi-regex6.2.2→6.5.0, chromium-bidi16.0.1→157.0.8090-0,
devtools-protocol0.0.1638949→0.0.1704951,
get-east-asian-width1.6.0→1.7.0, modern-tar0.7.6→0.8.6,
puppeteer/puppeteer-core25.3.0→25.16.0,
root string-width7.2.0→8.3.0 plus new nested7.2.0 under cliui/wrap-ansi,
typed-query-selector2.12.2→2.12.3, webdriver-bidi-protocol0.4.2→1.0.1,
ws8.21.0→8.22.1, yargs18.0.0→18.2.0 and zod3.25.76→4.6.7.

## Pause and evidence dispositions

All five tracked Dependabot entries remain paused by standing owner posture.
Their directories omit Mermaid/exporter and isolated survey/docs/experiment
projects; these are accounted inputs, not authorization to unpause/add entries.
Recommend explicit manual review by10 for included groups, handoff to the next
release dependency owner for retained/deferred groups, and a bounded future
coverage decision if scheduled PR coverage is desired. No automated schedule,
new role, live alert state or completed manual audit is invented. Historical
September zero-advisory comments and August snapshot cannot certify November
security. [Missing evidence](evidence-register.md) is bounded by group and
receiving task; only mandatory affected claims wait for that proof.
