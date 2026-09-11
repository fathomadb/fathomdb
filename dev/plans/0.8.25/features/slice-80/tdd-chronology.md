# Slice 80 TDD chronology

- Planning/design baseline: `5f0d3411`, independently approved before code.
- RED `4f8d3dd3`: missing Rust absolute oracle, missing reader dispatch witness
  and missing evidence module were observed failing.
- GREEN `7f3c94db`: AC-081a/b/c contracts, oracle, reader witness, validator,
  runner and active selector migration landed; focused 4/1/12 tests passed.
- Environment correction `ebba4211`: the first AC-081 series exposed an empty
  leaf-cgroup `cpu.max`; the runner now resolves the delegated parent. The seven
  affected raw logs remain retained as invalid evidence.
- Review RED `c236ab72`: tests captured raw-record parsing, positive durations,
  competitor recognition, complete threshold edges and bounded reader cleanup.
- Review GREEN `a93d4859`: connected parsing/campaign CLI, dirty-input rejection,
  complete process scan and a disconnect-safe five-second reader control landed;
  focused 4/1/15 tests pass.
- Final review RED `d7ce642d`: realistic Linux process rows demonstrated that
  the competitor scan missed truncated and hash-suffixed performance binaries.
- Final review GREEN `eab4c2b0`: the scan now recognizes the 15-character Linux
  `comm` form and executable basenames from argv while preserving ancestor PID
  exclusions. The focused 15-test evidence suite passes. Commit `302e6e4b`
  corrects the test fixture to the exact Linux truncation spelling.
- Reporting RED `3abfb825`: an invalid or numerically failing campaign raised
  before producing the promised structured report.
- Reporting GREEN `9cbc71bb`: structural validation still fails closed, while
  complete failed or environment-invalid campaigns reach structured summary.

No shipping search, SQL, schema, storage or runtime behavior changed in Slice 80.
