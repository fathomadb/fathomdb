---
title: FathomDB 0.8.26 Slice 65 — completion execution record
status: COMPLETE
candidate_sha: 8ffb34867b2622c3c17c16a95c8a85a37909722a
---

# Slice 65 completion execution record

The executable qualification template was frozen in `plan.md` before
implementation at commits `35328d2b` and `080a98cf`. Execution substituted
only these observed coordinates:

- candidate: `8ffb34867b2622c3c17c16a95c8a85a37909722a`;
- Rust: `rustc 1.95.0 (59807616e 2026-04-14)`;
- Python: `Python 3.12.3`;
- Node: `v25.9.0`;
- evidence root:
  `/tmp/fathomdb-slice65-8ffb34867b2622c3c17c16a95c8a85a37909722a`;
- exact-SHA CI run: `35393069930`.

## Execution outcome

The focused matrix, strict security, canonical 118-suite gate, fresh wheel,
exact npm main tarball and matched native package, installed Python/Node
profiles, source-built CLI integrity checks, tracked-tree Gitleaks scan, and
generated-evidence Gitleaks scan all passed. The release branch was pushed at
the candidate SHA; GitHub Actions completed successfully and emitted the
five-platform native matrix plus distinct installed-wheel Windows WAL receipt.

`candidate-manifest.json` then assembled and validated with Slice 65 schema,
the unchanged Slice 50 base schema, equal candidate SHAs, all six command
outcomes, all three Slice 65 qualification outcomes, graph evidence, both
scans, and both external receipt classes in `passed` state.

No tag, merge, release, registry publication, or npm dist-tag change was
performed.
