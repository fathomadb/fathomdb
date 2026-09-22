---
title: FathomDB 0.8.27 Slice 10 - repository preparation design
status: PROPOSED
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
- Public-doc, markdown, design-owner, state/view, workflow, security, and
  preflight checks form the focused verification boundary. Package/platform
  qualification remains Slice 150.
