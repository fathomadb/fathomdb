# Slice 1 sweep design and pin posture

Discover current-checkout tracked manifests → classify shipped/tooling/excluded
→ inspect lock/source/feature reachability → assess primary advisory applicability
→ select owned coherent change group → qualify. Candidate branch names and
historical zero-advisory snapshots are not current security evidence.

Current `scripts/sbom-survey` discovery uses invoking checkout `git ls-files`,
contrary to stale “main” wording. No branch-baseline workaround is needed.
Current recognition/tiering leaves 79 paths untiered (6 dev experiments, 33
historical .27 locks, 4 experiment drivers, 36 script fixtures), and silently
misses `dev/tools/onnx/export-requirements.txt` because its basename does not
start with requirements. Propose explicit classifications and discovery/parser
support or supplemental inventory in 10; preserve fixtures/history.

Retain qualified rusqlite 0.41.1/libsqlite3-sys 0.39.1/sqlite-vec 0.1.9; rmcp
exact 3.5.1; Candle-fathomdb 0.11.0/cudarc-fathomdb 0.19.10 and kernels Git
182ceab0; ort/ort-sys rc.10; PyO3 .29/abi3-py310; napi2 family; reviewed Ruff,
Pyright, actionlint, ShellCheck, Gitleaks, action SHAs, MkDocs and exporter hashes.
These are reviewed baselines, not assertions that newer versions are unavailable.
Unknown dunce pin rationale stays investigate. Root fork patch does not propagate
to registry Rust consumers. Owner seq-367 limits .29/140 to required repairs before product pins;
broader rebase/marker/multiarch maintenance belongs to .30.

All five Dependabot entries remain paused. Choose manual review ownership and
whether Mermaid/exporter need additional paused entries; unpausing is a separate
decision. Never import a stale candidate's full lockfile or regenerate model/
golden hashes autonomously. Coupled majors need explicit scope, not patch labels.
