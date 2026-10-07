---
title: Slice 135 installed 0.8.26 Python wheel baseline and paired functional smoke
status: INTERIM_FUNCTIONAL_RECEIPT
target_release: 0.8.27
---

# Installed 0.8.26 Python wheel baseline

This is an interim installed-artifact functional comparison. The baseline is
the clean, detached `v0.8.26` checkout at peeled source
`f99e002f0d2e4002f3694c9f8d4986b56089edaa`; its `Cargo.lock` SHA-256 is
`ffebda90ad6bae977a63cdb7d4c8edbce88632c2d353d5251247410040a16022`.
The baseline wheel SHA-256 is
`7c11803d9629550be60a76e1549c4c0bb42d23978d9fc901cf5df3c463611282`.
The candidate comparison uses [the retained candidate wheel](../2026-10-07-python-wheel-qualification/README.md)
built from source `b2ac8081e79a6e626d01f5f97331be331f1cc16d`.
At this observation, Slice 135 branch HEAD `722faba943b409e00b0a17d31131abfe020d7439`
had no product-source or `Cargo.lock` changes since that candidate source.

Command run from the baseline checkout:

```sh
./scripts/verify-release-python-wheel.sh \
  --python /home/coreyt/projects/fathomdb-worktrees/release-0.8.27-slice-135/.venv/bin/python \
  --wheel-dir /tmp/slice135-python-wheel-baseline-f99e002 \
  --venv-dir /tmp/slice135-python-venv-baseline-f99e002
```

The build, isolated non-editable wheel import, real-database smoke and graph
matrix validator passed. The runner SHA-256 is
`2d544b4a1331b9f294bcdd2e6c715645482d99abf29880fd25010b26c5985258`,
identical to the candidate runner. Retained files are the
[wheel](fathomdb-0.8.26-cp310-abi3-manylinux_2_39_x86_64.whl),
[runner](frozen-evidence-profile.py),
[provenance](install-provenance.txt),
[graph evidence matrix](slice50-graph-evidence.json), and
[hash manifest](SHA256SUMS).

The [paired functional check](paired-functional.json) independently read both
matrices. All 24 rows agree on 15 semantic fields, and schema-33 refusal and
nonmutation agree. Two opaque, database-bound evidence-reference fields
(`target_ref`, `terminal_ref`) differ; both have the expected `fdbgev1.`
format. Changing one candidate target revision in memory made the semantic
comparison fail, confirming that this check can detect a concrete mismatch.

This is **not** a timing comparison, an S01/S02 run, or evidence that all
supported operations are exercised. The matrices were generated from separate
fresh databases, so their opaque references cannot be byte-compared. No
Phase 1 checkpoint or correctness verdict follows from this narrow smoke.
