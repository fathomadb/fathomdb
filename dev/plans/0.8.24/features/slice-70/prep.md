---
title: 0.8.24 Slice 70 — integration preparation
status: COMPLETE
target_release: 0.8.24
---

# Slice 70 preparation

The local release branch contains closed Slices 10, 20, 30, and 60. Slice 60
adds the final Jetson Pages installed-package evidence: candidate
`2431f8729afb247518804e90b9ca324592c95456`, deployment `32878233246`, and
wheel SHA-256 `652ad6926b17c9580365b012ec9cb925fa1aabc6fe83047874c718dc5c5e5897`.

The stale public-doc truth guard was the only known release-wide local failure:
it selected 0.8.21 despite the released 0.8.23 record. Slice 70 owns its
smallest correction and the prepared changelog. Windows CUDA/WAL remains
deferred to 0.8.26 by `seq-258`; inherited Windows CPU artifact lanes are
preservation only, not new Windows work.
