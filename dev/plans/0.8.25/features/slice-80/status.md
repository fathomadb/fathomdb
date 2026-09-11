# Slice 80 status

Status: **BLOCKED — AC-072 ENVIRONMENT QUALIFICATION**

- Contract/design: complete and independently approved.
- TDD implementation: complete through `e8b63952`.
- AC-081a/b: Slice 80.m has the applicable accepted seven-cell campaign: 7/7
  qualified numeric passes, no warnings, with 172.883034 ms sequential and
  52.527637 ms concurrent medians. Earlier invalid campaigns remain history.
- AC-081c: focused real-database proof passes.
- AC-072: 6/6 numeric passes; five are swap-invalid and R2's complete competitor
  exclusion is unproved. Zero of three required cells is fully proved valid.
- The 80.m stop left the replacement AC-072 campaign unstarted after R1. The
  owner has now authorized the bounded 80.n smoke and one conditional three-cell
  campaign under [80n authorization](80n-authorization.md).
- Slice 80.n's smoke-repair path passed and R1 completed at 69/76 ms, but its
  `pswpin` increased by three. R2/R3 remain unstarted by the campaign stop rule;
  see [the stopped receipt](../../../runs/0.8.25-slice-80/80n-stopped-campaign.md).
- Slice 80.m readiness proof passed; its repaired AC-081 campaign has 7/7
  qualified numeric passes with no warnings (172.883034 ms sequential median;
  52.527637 ms concurrent median).
- Slice 80.m AC-072 R1 passed numerically (69/75 ms p50/p99) but is invalid:
  `pswpin` increased by two. The current stop rule leaves R2 and R3 unstarted.
- Protected Slice 79 write evidence: applicable.
- Broad verification: zero rounds, as planned.
- The superseding continuation ran its required smoke, then retained qualified
  C1 (70/79 ms) and C4 (70/76 ms) passes and C2/C3/C5's swap-invalid numeric
  passes. C5 broke the only possible three-cell valid-pass streak; C6 cannot
  satisfy the streak and was not run.
  The two allowed post-invalidity collector-only readiness checks both passed.
- Next slice: remains 80. Slice 85 is not unblocked.

The initial AC-081 quota defect and both collector-census defects are retained
transparently. Slice 80.m's allowance remains exhausted historical evidence.
Slice 80.n's prior allowance stopped after its first AC-072 cell. The owner
then authorized a fresh smoke and up to six continuation observations,
requiring three consecutive valid numeric passes and permitting two bounded
read-only diagnosis/readiness cycles after environment invalidity. The campaign
cannot now obtain that streak: C5 is environment-invalid and only C6 remains.
No criterion is waived; Slice 80 remains blocked and Slice 85 is not unblocked.
See [the continuation receipt](../../../runs/0.8.25-slice-80/80n-continuation-campaign.md).
