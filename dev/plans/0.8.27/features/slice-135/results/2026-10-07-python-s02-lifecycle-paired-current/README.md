---
title: Slice 135 installed Python S02-L integrated candidate comparison
---

# Installed Python S02-L paired lifecycle comparison

The [frozen subset protocol](../../s02-python-lifecycle-comparison-protocol.json)
was recorded after the [baseline-only pilot](../2026-10-07-python-s02-lifecycle-baseline-pilot/README.md)
and before candidate timing. The campaign ran five alternating pairs, one
warmup plus 20 measured fresh installed-Python processes per block, against
the exact 0.8.26 wheel and the integrated 0.8.27 candidate wheel. The candidate
source is `cdf253cd223a82e954591db532397a3d78a2027a` and the wheel SHA-256
is `c33987023754fee2d85f887bfbbd17c68889f287603d3df06844d06a240a60a0`.
That commit descends from `ef4bb42da` and includes both off-ladder landings.
Product source bytes are unchanged between that commit and this receipt's
branch head.

All ten producer blocks passed. The [independent audit](independent-audit.json)
reopened every raw JSON sample, reparsed the GNU Time child counters, checked
artifact/runner hashes, semantic and reopen assertions, protocol identity,
block order and idle intervals, and recomputed all percentiles and deltas.
The audit returned byte-identical output after the campaign was copied into
this result directory. A [negative control](negative-control.json) changed a
candidate reopen result and updated its enclosing receipt hashes; the
independent semantic check rejected it.

| Installed Python close() | 0.8.26 baseline | Integrated 0.8.27 candidate | Change |
| --- | ---: | ---: | ---: |
| p50, 100 cycles | 5.256 ms | 12.678 ms | +141.2% |
| p95, 100 cycles | 6.238 ms | 14.674 ms | +135.3% |
| Median opened-to-closed-idle PSS release | -33 KiB | 45,719 KiB | Intended release at close |
| Median opened-to-closed-idle RSS release | -152 KiB | 45,600 KiB | Diagnostic |
| Median opened-to-closed-idle private release | -32 KiB | 45,720 KiB | Diagnostic |

The five paired close-p50 deltas are +141.7%, +132.9%, +153.1%, +133.9% and
+135.9%. Their median is +135.9% and range is +132.9% to +153.1%. Pair 02
has a six-page host swap-counter warning, but zero measured child swaps;
excluding that entire pair leaves a +133.9% to +153.1% p50-delta range.
The close-latency increase is a **measured lifecycle regression** at this
boundary, alongside the intended roughly 44.6 MiB PSS release. It is not yet
a verdict on whole-system latency or release acceptability. The close path
needs a system-level impact and ownership/deadlock disposition. This cell
does not support p99 or a significance/equivalence claim. PSS, RSS and private
changes describe process memory, not model ownership by themselves.

The original baseline pilot and this paired campaign use fresh real SQLite
databases and the same CPU default embedder. All setup and memory sampling
are outside the `Engine.close()` timer. The candidate wheel has a 0.8.26
package-version string; source and artifact hashes, not that string, identify
the candidate. This subset does not close the broader Phase 1 protocol,
contention, Rust/TypeScript lifecycle paths or the correct-results phase.
