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

## Slice 80.m independent audit

Verdict: **evidence integrity PASS; Slice 80 acceptance NOT PASS**.

The reviewer matched both readiness logs, seven AC-081 raw logs and AC-072 R1
to their identities and SHA-256 values in `measurements.json`. Readiness used
source `ad422346`, product-input hash `95e15e…`, the sealed AC-081 executable,
the corrected AC-072 runner and scanner. Every AC-081 log has one 1,600/1,600/
eight-reader marker, a passing exit and complete qualified start/end controls;
the median is 172.883034 ms sequential and 52.527637 ms concurrent. AC-072 R1
has the required 10k/384d/1,000-query marker and passes 69/75 ms p50/p99, but
its `pswpin` changed 370407→370409. Its environment is therefore invalid.

Only AC-072 R1 exists; R2 and R3 are unstarted. File ordering is readiness,
AC-081 R1–R7, then AC-072 R1, consistent with immediate qualification and the
stop rule. The invalid timed R1 exhausts the allowance: no replacement,
closure, release-state advance, or Slice 85 start is authorized. No product or
build change invalidates the retained Slice 79 write receipts.
