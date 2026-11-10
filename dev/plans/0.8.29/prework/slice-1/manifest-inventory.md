# Slice 1 tracked manifest accounting proposal

Source inspected November10,2026 initially at dated planning baseline
`40d82c2e75d1d70fea61f327bf5b2ffcb8a1f479`; product baseline
`6edc1155f039f87cd0a37660cb27ae4373bbce71`. This is a read-only Git/stdlib
inventory and a **PROPOSED** data policy. It is not output from an installed
survey, a resolved dependency audit or a classification already applied in code.

## Reproduction and reconciliation

Input is `git ls-files -z` from the surveyed checkout, never a filesystem walk
or `git ls-tree main`. Match `MANIFEST_TABLE` exact basenames in
`scripts/sbom-survey/sbom_survey/discovery.py` plus `requirements*.txt`, then
apply the longest matching prefix in `scripts/sbom-survey/tiers.toml`.
Independent stdlib inspection reproduces **119 recognized files**:40 classified
(32 tiered,8 excluded) and79 untiered (6 dev experiments,33 historical .27 locks,
4 experiment-driver files,36 script fixtures). Proposed119-file totals: **8 dev-tooling**, **11 eval-only**, **44 exclude: fixture**, **33 exclude: historical-evidence**, **23 shipped**. These sum to119.

`fathomdb-tc5-benchmark` currently inherits shipped from the broad crate rule;
its manifest is publish=false and its purpose is measurement, so propose an
exact manifest override to eval-only (its dependencies can still also occur in
the shipped workspace lock). MCP is also publish=false but is a distributable
product binary, so retain its shipped tier. Python dev extras and npm dev edges
inside shipped manifests must retain edge kind; manifest tier alone is not a
claim that every locked node ships in every artifact.

The supplemental set adds14 paths: exporter1, historical manifest snapshots9,
historical probe lock1 and renamed lock fixtures3. Thus133 tracked dependency
input/evidence paths are explicitly accounted for here. Only exporter is a
proposed new executable discovery input, making120 recognized after the bounded
fix. Snapshot/renamed fixture paths stay explicit supplemental exclusions; no
catch-all directory or arbitrary `*.txt`/`*.lock` discovery is proposed.

## All current recognized paths

Every row is PROPOSED, including retain rows. Owner descriptions name receiving
work, not a new organizational role or approval.10 implements reviewed data and
survey behavior; experiment/SDK/tool owners assess future changes in their scope.

| ID | Tracked path | Current rule result | PROPOSED tier/exclusion | Receiving owner and rationale |
| --- | --- | --- | --- | --- |
| M001 | `Cargo.lock` | shipped | shipped | 10 dependency owner; affected SDK/MCP owner qualifies; product/workspace or native platform metadata; optional/dev edges remain labelled. |
| M002 | `Cargo.toml` | shipped | shipped | 10 dependency owner; affected SDK/MCP owner qualifies; product/workspace or native platform metadata; optional/dev edges remain labelled. |
| M003 | `dev/experiments/current-history-search/Cargo.lock` | UNTIERED | eval-only | experiment owner;10 classifies; unpublished isolated experiment with real dependencies; assess before reuse. |
| M004 | `dev/experiments/current-history-search/Cargo.toml` | UNTIERED | eval-only | experiment owner;10 classifies; unpublished isolated experiment with real dependencies; assess before reuse. |
| M005 | `dev/experiments/current-history-search/followups/Cargo.lock` | UNTIERED | eval-only | experiment owner;10 classifies; unpublished isolated experiment with real dependencies; assess before reuse. |
| M006 | `dev/experiments/current-history-search/followups/Cargo.toml` | UNTIERED | eval-only | experiment owner;10 classifies; unpublished isolated experiment with real dependencies; assess before reuse. |
| M007 | `dev/experiments/erasure-availability/Cargo.lock` | UNTIERED | eval-only | experiment owner;10 classifies; unpublished isolated experiment with real dependencies; assess before reuse. |
| M008 | `dev/experiments/erasure-availability/Cargo.toml` | UNTIERED | eval-only | experiment owner;10 classifies; unpublished isolated experiment with real dependencies; assess before reuse. |
| M009 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-e01-e12-feasibility/baseline/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M010 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-e01-e12-feasibility/candidate/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M011 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-e12-adapter-pilot/shared/baseline-build/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M012 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-e12-adapter-pilot/shared/candidate-build/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M013 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-first-results/attribution/build/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M014 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-first-results/candidate-build/build/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M015 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-first-results/invalid-attempts/profile-disk-pressure/build/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M016 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-first-results/paired/pair-01/baseline/build/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M017 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-first-results/paired/pair-01/candidate/build/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M018 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-first-results/paired/pair-02/baseline/build/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M019 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-first-results/paired/pair-02/candidate/build/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M020 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-first-results/paired/pair-03/baseline/build/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M021 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-first-results/paired/pair-03/candidate/build/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M022 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-first-results/paired/pair-04/baseline/build/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M023 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-first-results/paired/pair-04/candidate/build/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M024 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-first-results/paired/pair-05/baseline/build/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M025 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-first-results/paired/pair-05/candidate/build/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M026 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-first-results/profile/build/build/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M027 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-mixed-noise-pilot/block-01/build/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M028 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-mixed-noise-pilot/block-02/build/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M029 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-mixed-noise-pilot/block-03/build/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M030 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-mixed-noise-pilot/block-04/build/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M031 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-mixed-noise-pilot/block-05/build/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M032 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-mixed-noise-pilot/build-provenance/build/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M033 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-mixed-noise-pilot/invalid-attempts/gnu-time-blank-line/build/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M034 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-mixed-noise-pilot/invalid-attempts/host-swap-1-page/build/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M035 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-mixed-noise-pilot/invalid-attempts/host-swap-3-pages/build/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M036 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-noise-pilot/build-provenance/build/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M037 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-rust-sdk-capability/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M038 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-rust-sdk-s02-current-refresh/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M039 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-rust-sdk-s02-feasibility/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M040 | `dev/plans/0.8.27/features/slice-135/results/2026-10-08-crash-permission-224e/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M041 | `dev/plans/0.8.27/features/slice-135/results/2026-10-08-rust-sdk-capability-224e/Cargo.lock` | UNTIERED | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph. |
| M042 | `dev/release/fixtures/cargo-skew/Cargo.toml` | exclude: fixture | exclude: fixture | 10 release-tool owner; deliberate version-skew fixtures; preserve bytes. |
| M043 | `dev/release/fixtures/cargo-skew/mock-skew-api/Cargo.toml` | exclude: fixture | exclude: fixture | 10 release-tool owner; deliberate version-skew fixtures; preserve bytes. |
| M044 | `dev/release/fixtures/cargo-skew/mock-skew-consumer-a/Cargo.toml` | exclude: fixture | exclude: fixture | 10 release-tool owner; deliberate version-skew fixtures; preserve bytes. |
| M045 | `dev/release/fixtures/cargo-skew/mock-skew-consumer-b/Cargo.toml` | exclude: fixture | exclude: fixture | 10 release-tool owner; deliberate version-skew fixtures; preserve bytes. |
| M046 | `dev/release/fixtures/pip-skew/api-v1/setup.py` | exclude: fixture | exclude: fixture | 10 release-tool owner; deliberate version-skew fixtures; preserve bytes. |
| M047 | `dev/release/fixtures/pip-skew/api-v2/setup.py` | exclude: fixture | exclude: fixture | 10 release-tool owner; deliberate version-skew fixtures; preserve bytes. |
| M048 | `dev/release/fixtures/pip-skew/probe-a/setup.py` | exclude: fixture | exclude: fixture | 10 release-tool owner; deliberate version-skew fixtures; preserve bytes. |
| M049 | `dev/release/fixtures/pip-skew/probe-b/setup.py` | exclude: fixture | exclude: fixture | 10 release-tool owner; deliberate version-skew fixtures; preserve bytes. |
| M050 | `dev/tools/mermaid/package-lock.json` | dev-tooling | dev-tooling | 10 preparation owner; tool maintainer qualifies; tracked contributor/build/docs tooling, not runtime exposure. |
| M051 | `dev/tools/mermaid/package.json` | dev-tooling | dev-tooling | 10 preparation owner; tool maintainer qualifies; tracked contributor/build/docs tooling, not runtime exposure. |
| M052 | `dev/tools/module-boundary-gate/Cargo.lock` | dev-tooling | dev-tooling | 10 preparation owner; tool maintainer qualifies; tracked contributor/build/docs tooling, not runtime exposure. |
| M053 | `dev/tools/module-boundary-gate/Cargo.toml` | dev-tooling | dev-tooling | 10 preparation owner; tool maintainer qualifies; tracked contributor/build/docs tooling, not runtime exposure. |
| M054 | `experiments/comp_drivers/surrealdb/Cargo.lock` | UNTIERED | eval-only | competitor benchmark owner;10 classifies; unpublished competitor driver; does not enter product SBOM tier. |
| M055 | `experiments/comp_drivers/surrealdb/Cargo.toml` | UNTIERED | eval-only | competitor benchmark owner;10 classifies; unpublished competitor driver; does not enter product SBOM tier. |
| M056 | `experiments/sdk_drivers/rust/Cargo.lock` | UNTIERED | eval-only | SDK benchmark owner;10 classifies; unpublished measurement consumer, with own lock and kernel patch. |
| M057 | `experiments/sdk_drivers/rust/Cargo.toml` | UNTIERED | eval-only | SDK benchmark owner;10 classifies; unpublished measurement consumer, with own lock and kernel patch. |
| M058 | `package-lock.json` | dev-tooling | dev-tooling | 10 preparation owner; tool maintainer qualifies; tracked contributor/build/docs tooling, not runtime exposure. |
| M059 | `package.json` | dev-tooling | dev-tooling | 10 preparation owner; tool maintainer qualifies; tracked contributor/build/docs tooling, not runtime exposure. |
| M060 | `scripts/sbom-survey/pyproject.toml` | dev-tooling | dev-tooling | 10 preparation owner; tool maintainer qualifies; tracked contributor/build/docs tooling, not runtime exposure. |
| M061 | `scripts/tests/fixtures/cudarc-lock-set/added-dependency/Cargo.lock` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M062 | `scripts/tests/fixtures/cudarc-lock-set/build-only-change/Cargo.lock` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M063 | `scripts/tests/fixtures/cudarc-lock-set/changed-libloading/Cargo.lock` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M064 | `scripts/tests/fixtures/cudarc-lock-set/changed-root-features/Cargo.lock` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M065 | `scripts/tests/fixtures/cudarc-lock-set/matching/Cargo.lock` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M066 | `scripts/tests/fixtures/cudarc-lock-set/removed-dependency/Cargo.lock` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M067 | `scripts/tests/fixtures/cudarc-lock-set/unrelated-lock-change/Cargo.lock` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M068 | `scripts/tests/fixtures/cudarc-lock-set/windows-only-change/Cargo.lock` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M069 | `scripts/tests/fixtures/cudarc-lock-set/wrong-set-sha256/Cargo.lock` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M070 | `scripts/tests/fixtures/single-cudarc/tree/concrete-feature/Cargo.lock` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M071 | `scripts/tests/fixtures/single-cudarc/tree/concrete-feature/Cargo.toml` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M072 | `scripts/tests/fixtures/single-cudarc/tree/concrete-feature/crates/fathomdb-embedder/Cargo.toml` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M073 | `scripts/tests/fixtures/single-cudarc/tree/concrete-feature/vendor/cudarc-fathomdb-0.19.10/Cargo.toml` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M074 | `scripts/tests/fixtures/single-cudarc/tree/flash-attn-only/Cargo.lock` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M075 | `scripts/tests/fixtures/single-cudarc/tree/flash-attn-only/Cargo.toml` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M076 | `scripts/tests/fixtures/single-cudarc/tree/flash-attn-only/crates/fathomdb-embedder/Cargo.toml` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M077 | `scripts/tests/fixtures/single-cudarc/tree/flash-attn-only/vendor/cudarc-0.19.4/Cargo.toml` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M078 | `scripts/tests/fixtures/single-cudarc/tree/flash-attn-only/vendor/cudarc-fathomdb-0.19.10/Cargo.toml` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M079 | `scripts/tests/fixtures/single-cudarc/tree/no-cudarc-fathomdb/Cargo.lock` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M080 | `scripts/tests/fixtures/single-cudarc/tree/no-cudarc-fathomdb/Cargo.toml` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M081 | `scripts/tests/fixtures/single-cudarc/tree/no-cudarc-fathomdb/crates/fathomdb-embedder/Cargo.toml` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M082 | `scripts/tests/fixtures/single-cudarc/tree/no-cudarc-fathomdb/vendor/cudarc-fathomdb-0.19.10/Cargo.toml` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M083 | `scripts/tests/fixtures/single-cudarc/tree/ok/Cargo.lock` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M084 | `scripts/tests/fixtures/single-cudarc/tree/ok/Cargo.toml` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M085 | `scripts/tests/fixtures/single-cudarc/tree/ok/crates/fathomdb-embedder/Cargo.toml` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M086 | `scripts/tests/fixtures/single-cudarc/tree/ok/vendor/cudarc-fathomdb-0.19.10/Cargo.toml` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M087 | `scripts/tests/fixtures/single-cudarc/tree/path-cudarc-fathomdb/Cargo.lock` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M088 | `scripts/tests/fixtures/single-cudarc/tree/path-cudarc-fathomdb/Cargo.toml` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M089 | `scripts/tests/fixtures/single-cudarc/tree/path-cudarc-fathomdb/crates/fathomdb-embedder/Cargo.toml` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M090 | `scripts/tests/fixtures/single-cudarc/tree/path-cudarc-fathomdb/path/cudarc-fathomdb/Cargo.toml` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M091 | `scripts/tests/fixtures/single-cudarc/tree/upstream-cudarc/Cargo.lock` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M092 | `scripts/tests/fixtures/single-cudarc/tree/upstream-cudarc/Cargo.toml` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M093 | `scripts/tests/fixtures/single-cudarc/tree/upstream-cudarc/crates/fathomdb-embedder/Cargo.toml` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M094 | `scripts/tests/fixtures/single-cudarc/tree/upstream-cudarc/vendor/cudarc-0.19.4/Cargo.toml` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M095 | `scripts/tests/fixtures/single-cudarc/tree/upstream-cudarc/vendor/cudarc-fathomdb-0.19.10/Cargo.toml` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M096 | `scripts/tests/fixtures/test-targets/crate/Cargo.toml` | UNTIERED | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes. |
| M097 | `src/python/pyproject.toml` | shipped | shipped | 10 dependency owner; affected SDK/MCP owner qualifies; product/workspace or native platform metadata; optional/dev edges remain labelled. |
| M098 | `src/python/uv.lock` | shipped | shipped | 10 dependency owner; affected SDK/MCP owner qualifies; product/workspace or native platform metadata; optional/dev edges remain labelled. |
| M099 | `src/rust/crates/fathomdb-cli/Cargo.toml` | shipped | shipped | 10 dependency owner; affected SDK/MCP owner qualifies; product/workspace or native platform metadata; optional/dev edges remain labelled. |
| M100 | `src/rust/crates/fathomdb-embedder-api/Cargo.toml` | shipped | shipped | 10 dependency owner; affected SDK/MCP owner qualifies; product/workspace or native platform metadata; optional/dev edges remain labelled. |
| M101 | `src/rust/crates/fathomdb-embedder/Cargo.toml` | shipped | shipped | 10 dependency owner; affected SDK/MCP owner qualifies; product/workspace or native platform metadata; optional/dev edges remain labelled. |
| M102 | `src/rust/crates/fathomdb-engine/Cargo.toml` | shipped | shipped | 10 dependency owner; affected SDK/MCP owner qualifies; product/workspace or native platform metadata; optional/dev edges remain labelled. |
| M103 | `src/rust/crates/fathomdb-mcp/Cargo.toml` | shipped | shipped | 10 dependency owner; affected SDK/MCP owner qualifies; product/workspace or native platform metadata; optional/dev edges remain labelled. |
| M104 | `src/rust/crates/fathomdb-napi/Cargo.toml` | shipped | shipped | 10 dependency owner; affected SDK/MCP owner qualifies; product/workspace or native platform metadata; optional/dev edges remain labelled. |
| M105 | `src/rust/crates/fathomdb-py/Cargo.toml` | shipped | shipped | 10 dependency owner; affected SDK/MCP owner qualifies; product/workspace or native platform metadata; optional/dev edges remain labelled. |
| M106 | `src/rust/crates/fathomdb-query/Cargo.toml` | shipped | shipped | 10 dependency owner; affected SDK/MCP owner qualifies; product/workspace or native platform metadata; optional/dev edges remain labelled. |
| M107 | `src/rust/crates/fathomdb-schema/Cargo.toml` | shipped | shipped | 10 dependency owner; affected SDK/MCP owner qualifies; product/workspace or native platform metadata; optional/dev edges remain labelled. |
| M108 | `src/rust/crates/fathomdb-sdk/Cargo.toml` | shipped | shipped | 10 dependency owner; affected SDK/MCP owner qualifies; product/workspace or native platform metadata; optional/dev edges remain labelled. |
| M109 | `src/rust/crates/fathomdb-tc5-benchmark/Cargo.toml` | shipped | eval-only | benchmark owner;10 classifies; publish=false benchmark member; longest-prefix exact override proposed. |
| M110 | `src/rust/crates/fathomdb/Cargo.toml` | shipped | shipped | 10 dependency owner; affected SDK/MCP owner qualifies; product/workspace or native platform metadata; optional/dev edges remain labelled. |
| M111 | `src/ts/npm-tegra/linux-arm64-gnu-tegra/package.json` | shipped | shipped | 10 dependency owner; affected SDK/MCP owner qualifies; product/workspace or native platform metadata; optional/dev edges remain labelled. |
| M112 | `src/ts/npm/darwin-arm64/package.json` | shipped | shipped | 10 dependency owner; affected SDK/MCP owner qualifies; product/workspace or native platform metadata; optional/dev edges remain labelled. |
| M113 | `src/ts/npm/darwin-x64/package.json` | shipped | shipped | 10 dependency owner; affected SDK/MCP owner qualifies; product/workspace or native platform metadata; optional/dev edges remain labelled. |
| M114 | `src/ts/npm/linux-arm64-gnu/package.json` | shipped | shipped | 10 dependency owner; affected SDK/MCP owner qualifies; product/workspace or native platform metadata; optional/dev edges remain labelled. |
| M115 | `src/ts/npm/linux-x64-gnu/package.json` | shipped | shipped | 10 dependency owner; affected SDK/MCP owner qualifies; product/workspace or native platform metadata; optional/dev edges remain labelled. |
| M116 | `src/ts/npm/win32-x64-msvc/package.json` | shipped | shipped | 10 dependency owner; affected SDK/MCP owner qualifies; product/workspace or native platform metadata; optional/dev edges remain labelled. |
| M117 | `src/ts/package-lock.json` | shipped | shipped | 10 dependency owner; affected SDK/MCP owner qualifies; product/workspace or native platform metadata; optional/dev edges remain labelled. |
| M118 | `src/ts/package.json` | shipped | shipped | 10 dependency owner; affected SDK/MCP owner qualifies; product/workspace or native platform metadata; optional/dev edges remain labelled. |
| M119 | `tools/docs/requirements.txt` | dev-tooling | dev-tooling | 10 preparation owner; tool maintainer qualifies; tracked contributor/build/docs tooling, not runtime exposure. |

## Supplemental tracked input/evidence paths

| ID | Tracked path silently outside current discovery | PROPOSED treatment | Owner, rationale and discovery action |
| --- | --- | --- | --- |
| X01 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-e01-e12-feasibility/baseline/Cargo.toml.snapshot` | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph; supplemental exclusion only; do not broaden executable discovery. |
| X02 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-e01-e12-feasibility/candidate/Cargo.toml.snapshot` | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph; supplemental exclusion only; do not broaden executable discovery. |
| X03 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-e12-adapter-pilot/shared/baseline-build/Cargo.toml.snapshot` | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph; supplemental exclusion only; do not broaden executable discovery. |
| X04 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-e12-adapter-pilot/shared/candidate-build/Cargo.toml.snapshot` | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph; supplemental exclusion only; do not broaden executable discovery. |
| X05 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-rust-sdk-capability/Cargo.toml.snapshot` | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph; supplemental exclusion only; do not broaden executable discovery. |
| X06 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-rust-sdk-s02-current-refresh/Cargo.toml.snapshot` | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph; supplemental exclusion only; do not broaden executable discovery. |
| X07 | `dev/plans/0.8.27/features/slice-135/results/2026-10-07-rust-sdk-s02-feasibility/Cargo.toml.snapshot` | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph; supplemental exclusion only; do not broaden executable discovery. |
| X08 | `dev/plans/0.8.27/features/slice-135/results/2026-10-08-crash-permission-224e/Cargo.toml.snapshot` | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph; supplemental exclusion only; do not broaden executable discovery. |
| X09 | `dev/plans/0.8.27/features/slice-135/results/2026-10-08-rust-sdk-capability-224e/Cargo.toml.snapshot` | exclude: historical-evidence | historical .27 evidence custodian;10 classifies; frozen build provenance, including invalid attempts; never update as current graph; supplemental exclusion only; do not broaden executable discovery. |
| X10 | `dev/tools/onnx/export-requirements.txt` | dev-tooling | 10 preparation owner; tool maintainer qualifies; tracked contributor/build/docs tooling, not runtime exposure; add bounded discovery + dispatch + hash-aware parser in10. |
| X11 | `experiments/configs/release-0825-slice71b-probe.Cargo.lock` | exclude: historical-evidence | historical .25 evidence custodian;10 inventories; bound by slice71b attribution manifest and lock SHA; preserve bytes; supplemental exclusion only; do not broaden executable discovery. |
| X12 | `scripts/tests/fixtures/single-cudarc/lock/ok.lock` | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes; supplemental exclusion only; do not broaden executable discovery. |
| X13 | `scripts/tests/fixtures/single-cudarc/lock/ug-cudarc.lock` | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes; supplemental exclusion only; do not broaden executable discovery. |
| X14 | `scripts/tests/fixtures/single-cudarc/lock/upstream-cudarc.lock` | exclude: fixture | 10 delivery-tool owner; intentional guard inputs, including fake/vendor packages; preserve bytes; supplemental exclusion only; do not broaden executable discovery. |

## Exact proposed tier-data changes for10

Retain existing root/source/tool/docs/release-fixture rules. Add data prefixes
`dev/experiments/`→eval-only, `experiments/comp_drivers/`→eval-only and
`experiments/sdk_drivers/`→eval-only; add the exact benchmark manifest→eval-only.
Add `scripts/tests/fixtures/` exclusion with reason fixture, and
`dev/plans/0.8.27/features/slice-135/results/` exclusion with reason
historical-evidence and a note that these are immutable provenance inputs.
These prefixes cover precisely the listed current recognized paths. New files
under an assigned prefix inherit its documented purpose; adding a different
purpose requires a narrower rule and review. No repository-wide default exists.
The14 supplemental rows remain in the maintained inventory even where discovery
intentionally excludes their names. Only exporter receives parser support.

## Completeness check proposed for10

Re-enumerate tracked names, compare sorted path sets to this table, ensure no
row is duplicated and no recognized path is unclassified. Require120 recognized
paths after exporter support, with explicit additions if the source changes;
counts are a dated check, not a forever hardcoded product invariant. Classify
before parse/registry lookup; exclusion must avoid fake/historical components.
Review a tracked-name scan for suffix/prefix manifests against this supplemental
set, and keep evidence snapshots byte-identical. Exact path-set comparison,
not a count alone, is the completeness criterion. These checks were designed
here; no survey/test/product execution is claimed.
