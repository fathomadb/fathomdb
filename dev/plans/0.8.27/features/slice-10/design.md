---
title: FathomDB 0.8.27 Slice 10 - repository preparation design
status: COMPLETE
target_release: 0.8.27
---

# Slice 10 repository preparation design

The slice changes repository truth and development/release instrumentation, not
product behavior.

- Public truth derives from the published 0.8.26 state/receipts and active
  0.8.27 plan/state. Historical release notes remain historical.
- Dependency remediation is resolver-minimal. Protected Rust/native stacks and
  unrelated root packages do not move.
- Action runtime identity remains its full SHA; comments are descriptive and
  cannot authorize a version change.
- Benign-digest exceptions are data with one owner. Each entry binds an exact
  path plus expected key/value shape. The validator rejects broadened paths,
  duplicate authorities, arbitrary digests, and credential-shaped mutations.
- Comparator prerequisites are exactly versioned and their generated paths are
  owned. The slice may select a tool and nightly but may not yet
  capture/approve the immutable surface baseline; Slice 30 owns that
  product-facing decision.
- The selected comparator inputs are `cargo-public-api 0.52.0` and
  `nightly-2026-04-24`, matching the experiment's selected candidates and the
  local version probes. Slice 10 records and probes them; Slice 30 must
  requalify every required feature row before capturing a baseline. No global
  installation or baseline capture belongs to Slice 10.
- Slice 30's generated tool, build, and evidence material is confined to the
  checkout-owned, ignored `.cache/0.8.27-slice30/` tree. Experiment scratch is
  separately confined outside the repository to the owned
  `/tmp/fathomdb-0.8.27-slice30/` root, preserving the experiment's isolation
  invariant. The heavy route requires at least 100 decimal GB free on the
  filesystems carrying both roots, based on the recorded 85.7 GB durable-target
  peak plus margin. Its 16 GiB container scratch ceiling remains a separate
  resource limit. The ordinary repository preflight keeps its 10 GB default.
- The performance-gauntlet tokenizer exception metadata has one reviewed data
  owner. Its derived Gitleaks policy remains machine-checked against that
  owner; the checker must reject duplicate registrations, broadened paths,
  wrong key/value shapes, arbitrary digests, and credential-shaped additions.
  Other reviewed exception families remain unchanged.
- Public truth uses the 0.8.26 publication receipt and schema-34 state. Current
  banners, exact-version install examples, API landing pages,
  compatibility/platform facts, and navigation move to 0.8.26. Channel-based
  commands such as `npm install fathomdb@next` retain their registry contract.
  Historical 0.8.25 feature headings and release-note bodies remain unchanged.
- Public-doc, markdown, design-owner, state/view, workflow, security, and
  preflight checks form the focused verification boundary. Package/platform
  qualification remains Slice 150.
