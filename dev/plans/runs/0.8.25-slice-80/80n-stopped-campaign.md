# Slice 80.n AC-072 stopped campaign

Status: **STOPPED — ENVIRONMENT_INVALID**

The owner-authorized three-cell AC-072 campaign began after a qualified,
non-acceptance smoke. R1 completed its real 10,000-row/384-dimensional warm
workload using the sealed executable. The dispatcher did not launch R2 or R3.

| Cell | Numeric result | Environment | Dispatch result |
| --- | --- | --- | --- |
| R1 | PASS: p50 69 ms, p99 76 ms | INVALID: `pswpin` 370916 to 370919; `pswpout` unchanged | Stopped before R2. |
| R2 | Not started | Not observed | Prohibited by R1 stop rule. |
| R3 | Not started | Not observed | Prohibited by R1 stop rule. |

R1's raw receipt is `raw/ac072-slice80n-campaign1/R1.log`. It records one
executed non-ignored test, 1,000 positive samples, 10,000 accepted writes and
materialized vectors, no competing process, and the protected product input
hash. The manually invoked sealed validator retains distinct
`numeric_pass=true` and `environment_applicable=false` verdicts in
`R1.verdict.json`.

The initial dispatcher attempted to execute the Python validator directly and
therefore wrote an empty verdict path after R1. Commit `aa2bb4f5` changes that
invocation to `python3` and adds a controlled non-executable-validator RED/GREEN
test. This harness repair does not change R1's raw result or permit a replacement
cell. The environment-invalid R1 independently requires an owner disposition
before any further AC-072 timing.
