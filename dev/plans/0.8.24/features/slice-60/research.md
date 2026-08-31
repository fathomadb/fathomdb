---
title: 0.8.24 Slice 60 — research and evidence sources
status: COMPLETE
target_release: 0.8.24
---

# Slice 60 research

## Facts used by the implementation

- The authorized first-party simple-index base is
  `https://fathomadb.github.io/fathomdb/tegra/simple/`.
- Slice 30 run `32878233246` built/deployed
  `fathomdb-0.8.24+tegra-cp310-abi3-linux_aarch64.whl` from candidate
  `2431f8729afb247518804e90b9ca324592c95456`.
- That run's retained Jetson evidence artifact is still available. The static
  Pages artifact is expired, so its artifact archive cannot be the immutable
  source; the live index and retained Jetson wheel evidence are checked
  separately.
- The package has no mandatory Python dependencies. A no-dependency, exact
  wheel install can therefore prove the selected wheel itself without allowing
  a resolver to substitute another distribution.
- Existing workflow evidence shows the target proof needs the locally retained
  Hugging Face cache in offline mode and the existing GPU witness verifier.

## Bounded deferred research

Before a later Tegra release, re-evaluate durable multi-version hosting and
whether Pages should host wheels or only a signed index to retained release
assets. This 0.8.24 implementation does not elevate the interim Pages route to
a permanent distribution policy.
