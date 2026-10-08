---
title: Slice 135 current-source E01–E12 smoke
status: AUDITED_FEASIBILITY_NOT_PAIRED_TIMING
target_release: 0.8.27
---

# E01–E12 current-source smoke

The [source-bound protocol](protocol.json),
[build provenance](build-provenance.json) and [independent audit](audit.json)
identify candidate `8c2455b6ccf1d06d5bf87fc4278aecc631014da0`, the exact
release-mode workload binary, 32-row corpus, pinned local CPU model and
unchanged E01–E12 checks. All twelve engine paths passed 100 observations
each. The [raw observations](raw.json) and [resource report](run.stderr.log)
are retained locally. The [negative controls](negative-controls.json) show
that altered source identity and an altered ordered text result are rejected.

The smoke established feasibility before the
[current-source paired subset](../../e12-current-comparison-protocol.json)
was frozen. Its 100 query observations do not support p99 or a 0.8.26
comparison; the paired campaign uses the larger frozen sample counts. The
binary and raw observations remain local pending end-of-phase retention and
are not yet committed branch evidence.
