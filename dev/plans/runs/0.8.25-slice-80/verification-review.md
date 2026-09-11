# Slice 80 independent evidence review

Verdict: **evidence integrity PASS; release acceptance NOT PASS**.

The reviewer independently reproduced the registered thresholds, candidate and
binary hashes, seven AC-081 numeric results, medians, warnings, six AC-072
numeric results and swap deltas. AC-081c's real-database witness is adequate,
and Slice 79's six write receipts remain applicable because Slice 80 has no
release-build product-path change.

The final AC-081 logs predate the competitor-scanner correction at `eab4c2b0`.
The old scan missed Linux-truncated and hash-suffixed performance binaries;
empty recorded competitor lists therefore do not prove the required control.
The same limitation affects the inherited AC-072 collector. Five AC-072 cells
are independently invalid from swap activity; R2's otherwise-clean controls
cannot retrospectively prove competitor exclusion.

Accordingly, AC-081a/b have seven numeric passes with no warnings but unproved
environment applicability. AC-072 has six numeric passes but zero fully proved
environment-valid cells against three required. The authorized original and
replacement series are exhausted. Slice 80 remains open and Slice 85 remains
blocked; no additional performance or broad run was launched by the reviewer.

The separate code review passes through `9cbc71bb`, including structured
`FAIL`/`ENVIRONMENT_INVALID` reporting and bounded reader-test cleanup.

## Owner-authorized final allowance audit

The seven authorized AC-081 raw logs exactly match their retained hashes and
measurements. Their sequential median is 172.733800 ms and concurrent median
is 62.389734 ms; every cell passes numerically with no warning. Each start and
end snapshot, however, contains only the runner's own command-substitution
shell as a competing AC-081 runner. They are therefore correctly invalid, not
relabeled valid.

The source, binary, product-input, AC-081 runner, AC-072 collector and scanner
hashes match the pre-timing seal. The read-only environment diagnosis predates
the cells. No AC-072 authorized-final directory or raw log exists: stopping
after the invalid AC-081 campaign follows the owner's campaign-level stop rule.
Prior receipts remain unchanged and applicable as recorded. Slice 80 remains
in progress and Slice 85 remains blocked. No broad or further timing run was
performed.
