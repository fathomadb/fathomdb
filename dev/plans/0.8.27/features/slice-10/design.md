---
title: FathomDB 0.8.27 Slice 10 - repository preparation design
status: ACTIVE
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
- Comparator prerequisites are checkout-owned and versioned. The slice may
  select a tool and nightly but may not yet capture/approve the immutable
  surface baseline; Slice 30 owns that product-facing decision.
- The selected comparator inputs are `cargo-public-api 0.52.0` and
  `nightly-2026-04-24`, matching the qualified refactor-pilot evidence. Slice
  10 records and probes them; it neither installs a global toolchain nor
  captures a public-surface baseline.
- Slice 30's generated tool, build, and scratch material is confined to the
  checkout-owned, ignored `.cache/0.8.27-slice30/` tree. The heavy pilot route
  requires at least 20 decimal GB free on that filesystem: the retained 16 GiB
  runner scratch limit plus the 2 GB admission floor and operational margin.
  The ordinary repository preflight remains at its existing 10 GB default.
- The performance-tokenizer exception metadata has one reviewed data owner.
  Derived Gitleaks policy remains machine-checked against that owner; the
  checker must reject duplicate registrations, broadened paths, wrong
  key/value shapes, arbitrary digests, and credential-shaped additions.
- Public truth uses the 0.8.26 publication receipt and schema-34 state. Current
  banners, install commands, API landing pages, compatibility/platform facts,
  and navigation move to 0.8.26. Historical 0.8.25 feature headings and release
  note bodies remain unchanged.
- Public-doc, markdown, design-owner, state/view, workflow, security, and
  preflight checks form the focused verification boundary. Package/platform
  qualification remains Slice 150.
