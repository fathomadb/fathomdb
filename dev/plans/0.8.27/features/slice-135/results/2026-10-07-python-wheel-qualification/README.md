---
title: Slice 135 installed Python wheel functional qualification
status: INTERIM_FUNCTIONAL_RECEIPT
target_release: 0.8.27
---

# Installed Python wheel functional qualification

This is an interim **functional** result from candidate source
`b2ac8081e79a6e626d01f5f97331be331f1cc16d` on
`llm/0.8.27-slice-135`, with `Cargo.lock` SHA-256
`9e9d7b5e82184a0bddfbe96de28fa0615ef4443fc84639b271c1cff29600ccfe`.
The built wheel is named `fathomdb-0.8.26` because the candidate package version
has not yet been bumped; the filename is **not** its source identity. The wheel
SHA-256 is `6713ade54d62cc1e067fcaf1982a539c1fb8bd3d45d3db18e41155badd0c12e8`.

Command run from the candidate worktree:

```sh
./scripts/verify-release-python-wheel.sh \
  --python .venv/bin/python \
  --wheel-dir /tmp/slice135-python-wheel-candidate-b2ac808 \
  --venv-dir /tmp/slice135-python-venv-candidate-b2ac808
```

The script built the wheel, installed it in the isolated virtual environment,
checked imports from `site-packages` and a non-editable native module, and ran
the source-independent `frozen-evidence-profile-v1` against a real SQLite
database. The runner's SHA-256 is
`2d544b4a1331b9f294bcdd2e6c715645482d99abf29880fd25010b26c5985258`.
The observed import locations and editable flag are in
[install-provenance.txt](install-provenance.txt). The exact
[wheel](fathomdb-0.8.26-cp310-abi3-manylinux_2_39_x86_64.whl),
[runner](frozen-evidence-profile.py), and
[SHA256SUMS](SHA256SUMS) are retained here.

The smoke asserted source body and evidence resolution, ordinary and actuated
graph evidence, frozen context across close/reopen, persisted source
dependency, and typed non-mutating refusal of schema version 33. It produced
[24 graph evidence rows across 12 cases](slice50-graph-evidence.json), covering
ordinary and actuated edge sources. An independent re-read validated the rows
and schema refusal, then rejected an in-memory changed target revision; see
[independent-check.json](independent-check.json).

This run has no timing samples, 0.8.26 baseline pairing, vector/hybrid
exercise, concurrency, or comprehensive operation coverage. It does not
qualify S01/S02 or the Phase 1 checkpoint. The capability register marks only
the directly exercised Python operations as an interim functional smoke.
