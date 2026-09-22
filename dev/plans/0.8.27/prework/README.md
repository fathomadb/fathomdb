---
title: FathomDB 0.8.27 prework index
status: COMPLETE
target_release: 0.8.27
---

# FathomDB 0.8.27 prework index

Prework Slices 0-7 gather and reconcile evidence. Slice 8 is deliberately
reserved. Slice 9 consolidates the proposals, records direct user-authorized
scope decisions, and prepares independently reviewed Slice 10 inputs. No
prework slice changes product code, dependencies, environments, CI, accepted
public contracts, schema, tags, registries, or publication state.

| Slice | Subject | Durable record |
| ---: | --- | --- |
| 0 | Environment and project infrastructure | [`slice-0.md`](slice-0.md) |
| 1 | Dependencies and pins | [`slice-1.md`](slice-1.md) |
| 2 | Repository and documentation cruft | [`slice-2.md`](slice-2.md) |
| 3 | Draft contracts and architecture allocation | [`slice-3.md`](slice-3.md) |
| 4 | Architecture/code alignment | [`slice-4.md`](slice-4.md) |
| 5 | Verification adequacy | [`slice-5.md`](slice-5.md) |
| 6 | Stale-documentation evidence | [`slice-6.md`](slice-6.md) |
| 7 | Build and delivery failure evidence | [`slice-7.md`](slice-7.md) |
| 8 | Reserved, no inferred authority | [`slice-8.md`](slice-8.md) |
| 9 | Proposal review and decisions | [`slice-9.md`](slice-9.md) |

Shared records:

- [`0.8.27-refactor-test-approach.md`](0.8.27-refactor-test-approach.md)
  defines stable behavioral oracles for the later correction/refactor ladder.
- [`proposal-register.md`](proposal-register.md) records every prework
  disposition and delivery placement.
- [`prework-design-review.md`](prework-design-review.md) and
  [`prework-verification.md`](prework-verification.md) record independent
  read-only PASS verdicts.

The proportional workflow is
[`../slice-execution-contract.md`](../slice-execution-contract.md).
