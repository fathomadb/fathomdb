# Slice 103 Track R — owed erasure recovery receipt

- Baseline: `25d90e5334382cecef041bd3991e24f8094160d5` on `release/0.8.27`.
- Branch: `llm/slice-103-erasure-recovery`.
- Verified code HEAD: `e555363a93a6c0b87dee09f3f1454ada62ce756f`.
- Commits: `9d2dc326c` RED test and ADR; `57ac7abc7` implementation and public contracts; `0a0f55e68` at-rest proof; `e555363a9` nonvacuous WAL precondition.
- Scope: R27-103G only. This is branch-local evidence; integrated-candidate qualification belongs to the coordinator.

## Behavior

`doctor check-integrity` emits `E_ERASURE_INCOMPLETE` per owed physical closure, with structured phase, cause, blocker, sequence, WAL frame observation, redacted source, and documentation anchor. A checkpoint blocker includes parser-ready remediation argv. A telemetry redaction obligation, including the queue-present/blocker-NULL crash window, instead instructs the operator to restore the original sink and retry the originating erasure; it never advertises the offline command as a fix.

`recover --accept-data-loss --complete-erasures <db_path>` takes the canonical engine lock, performs recovery admission before opening a single read-write SQLite connection, validates physical zero, and only then checkpoints WAL with TRUNCATE and completes the closures. It refuses malformed WAL before the read-write open, refuses pending telemetry redaction without the original sink, and does not checkpoint when nothing is owed. Success is exit 64; lock or checkpoint busy is 71; validation or sink refusal is 70; no pending closures returns clean exit 0. The accepted successor ADR and CLI, Rust, and recovery contracts were updated together.

## Verification

- Initial RED command: `cargo test -p fathomdb-cli --test owed_erasure_recovery -- --nocapture`; observed exit 101, four real-database CLI tests failed because the finding and recover action were absent. The RED test and ADR are separately committed at `9d2dc326c`. No raw transcript was retained.
- Later focused RED tests exposed incorrect telemetry advice, no-pending checkpoint behavior, and queue-present/blocker-NULL advice; each observed exit 101 before the fix. The crash-window RED test and its fix share `57ac7abc7`; no separate raw RED log was retained.
- Final focused real-database CLI suite: `cargo test -p fathomdb-cli --test owed_erasure_recovery`, exit 0, 8/8 tests passed. Its success fixture requires erased-content sentinel bytes in WAL before offline completion, then proves absence from DB and WAL afterward, closure completion, survivor retention, and removal of the write fence. Malformed-WAL refusal proves DB, WAL, and SHM byte stability.
- Final full `./scripts/agent-verify.sh`, exit 0 at the clean code HEAD above: 178/182 test suites passed, 4 skipped, 0 excluded. Lint and typecheck passed. Security checks, including AC-036 and AC-037, passed with 0 violations, 0 blockers, and 0 downgrades. Exact captured output: `evidence/verify-full.log`, SHA-256 `6193e1bffdfb1b18b21084ca9860f207fd92c843b5020d52d72288ffa08a478b`.

The full gate used a checkout-local virtual environment populated with copied build dependencies and a prebuilt Python binding from the release checkout; it did not build an exact-source installed Python wheel for this track. No `pip install -e` or `maturin develop` ran from this worktree. Integrated-source installation and Windows WAL behavior remain coordinator-track checks.
