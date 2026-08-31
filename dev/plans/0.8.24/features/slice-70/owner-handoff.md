---
title: 0.8.24 Slice 70 — owner handoff
status: PREPARED
target_release: 0.8.24
---

# Owner handoff

Do not act on this file without explicit owner authorization. After the final
local release gate is green, the owner may choose whether to push the release
branch, integrate to main, tag `v0.8.24`, and run the configured publication
route. Those actions can publish immutable package versions.

Before declaring release completion, query the exact registry versions and run
the configured public installed-package smokes. Do not republish a matching
immutable artifact; fail closed on uncertain registry state. The completed
Tegra public-index proof is retained in Slice 60; CPU registry publication and
post-publication smokes remain pending owner actions.
