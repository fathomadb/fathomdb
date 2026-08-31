---
title: 0.8.24 Slice 60 — installed Tegra distribution design
status: REVIEWED
target_release: 0.8.24
---

# Slice 60 installed Tegra distribution design

## Design decision

The public Tegra proof is a three-source chain:

```text
retained Jetson evidence wheel.sha256 ─┐
                                        ├─ exact wheel SHA-256 ─ fresh Jetson venv
Pages PEP 503 exact-version link ──────┘                          │
                                                                    └─ lifecycle + CUDA witness
```

The public index chooses the wheel; retained candidate evidence binds it to the
Jetson-built bytes. Neither source alone is enough. The installer uses
`--index-url` (not `--extra-index-url`), `--only-binary=:all:`, `--no-cache-dir`,
and `--no-deps`; it downloads before installation, checks the independent hash,
then installs the downloaded wheel under `--no-index`. This prevents source,
editable, cache, dependency-resolution, and alternate-index substitution from
turning an index smoke into another kind of test.

## Runtime proof

The fresh venv opens a database with `use_default_embedder=True`, writes a
provenanced record, searches, closes, and exits. It uses the retained Jetson
model cache offline. `FATHOMDB_EMBED_DEVICE=cuda:0` and
`FATHOMDB_GPU_ALLOCATION_WITNESS=1` require an in-process CUDA allocation
witness, which is validated by the existing witness verifier against the local
Tegra `nvidia-smi` observation.

## Public guidance

The Python install page, compatibility page, CLI help, and classic-Tegra
generic-build warning must tell users one exact, detection-gated command for
the interim route. The docs retain the source-build wrapper as the fallback
for unsupported or future target/JetPack combinations. They explicitly say the
Pages transport is 0.8.24-only interim hosting and must be re-reviewed before a
later Tegra release.

## Architectural fit and reviewer outcome

This keeps D-80.6-3's one distribution/import identity, confines the new
trust boundary to release verification, and makes no engine, protocol, or
Windows architectural change. The independent Slice 60 review required the
separate retained digest, public-index installed proof, stale-doc correction,
and CPU-versus-Windows-scope clarification; this design incorporates all four.
