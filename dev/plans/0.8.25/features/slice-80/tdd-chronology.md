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
- Authorized collector RED `11ed3dc3` and GREEN `086311de`: AC-072 now reuses
  the corrected scanner for truncated names, hash-suffixed test binaries and
  runner processes while excluding the current campaign ancestry.
- Review RED `07b452dc` and GREEN `4e0f2619`: the AC-072 command-substitution
  shell is excluded by its `BASHPID`; a runner-level test preserves that exact
  current-versus-other-runner distinction. The focused reviewer passed.
- Receipt RED `cb5e3f54` and GREEN `8eafa888`: AC-072 records the exact source,
  collector and scanner identities when the Slice 80 source guard is enabled.
- The authorized AC-081 collection exposed the analogous uncorrected AC-081
  self-census defect. Its seven raw cells are retained as environment-invalid;
  no fix or replacement was run because the owner authorization requires stop.
- Slice 80.m RED `ac7ceaa2` and GREEN `7df7af60`: AC-081 gained the same
  PID-specific census-subshell exclusion as AC-072. RED `012e147e` and GREEN
  `edb36bdc` added both production collector-only paths, complete AC-072
  environment controls, and their valid/competitor readiness proof.
- Reviewer RED `9ecd9ead` and GREEN `e8b63952`: readiness now binds each
  collector's exact environment marker and rejects duplicate, malformed,
  missing, empty and unexpected identity fields. Seventeen focused Python
  tests, both shell census/readiness proofs, Ruff and shell syntax passed.
- Slice 80.m timing used the independently reviewed collectors. AC-081 passed
  all seven qualified fresh cells; AC-072 R1 was numerically within limits but
  environment-invalid from `pswpin +2`, so no later AC-072 cell was dispatched.

No shipping search, SQL, schema, storage or runtime behavior changed in Slice 80.
