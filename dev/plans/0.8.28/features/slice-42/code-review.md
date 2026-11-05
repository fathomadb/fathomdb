---
title: FathomDB 0.8.28 Slice 42 — code review
status: R-CODE CONCERN (one medium finding), fixed in 562477d70 and 1ea86b182
target_release: 0.8.28
observed_on: 2026-10-31
---

# Slice 42 code review

## codex gpt-6-astra, reasoning high, read-only, reviewed 59079d5c5..16550b4ba

The verdict body follows as the reviewer wrote it. One change: the finding's link
pointed at the reviewer's scratch copy and is given here as a repository path.

CONCERN

- **medium — `scripts/d27-runtime-runner.py:126`**  
  The candidate runner still generates `rusqlite = "0.40"` alongside path dependencies on the updated engine/schema requiring 0.41. Their incompatible `libsqlite3-sys` versions both declare `links = "sqlite3"`, preventing Cargo resolution for current candidates.  
  **Fix:** derive the requirement from the selected checkout’s manifest, preserving historical 0.40 runs; cover both historical and current manifest generation.

**What passed on inspection**

- **Windows guard:** `rusqlite::version_number()` calls the linked library’s `sqlite3_libversion_number()`. The 3.53.3 threshold matches [upstream’s Windows VFS source](https://raw.githubusercontent.com/sqlite/sqlite/version-3.53.3/src/os_win.c). Original and canonical paths are checked before database/lock creation; connection and URI helpers apply the same simplification. No production bypass for unsimplifiable local drive paths was found.
- **System SQLite:** selecting linked mode through `LIBSQLITE3_SYS_USE_PKG_CONFIG` still uses the actual runtime version. `SQLITE3_LIB_DIR` controls library selection within linked mode; alone, it does not override the enabled bundled feature.
- **API migration:** the `trace_v2`/`busy_handler` edits preserve assertions. All changed trace calls handle `Result`; their `.expect()` calls are confined to tests/test hooks.
- **Override checker:** no newly admitted bypass found. SQLite patches lose both root and SDK-driver exemptions; existing Candle/fork enforcement remains.
- **Vendor retirement:** manifests, lockfiles, lint exclusions, whitespace exemptions and Gitleaks configuration are consistent. Remaining vendor references are historical records, refusal fixtures or optional frozen provenance inputs.
- **Unit A:** component-aware directory matching excludes unrelated tests while retaining database/sidecar descriptors, including deleted-file targets. The equality assertion remains intact; non-Linux `None` behavior matches the explicit ruling.
- **RED commits:** `a64a1c6b0`, `3dfe21d61` and `d1078aa76` support their stated failures: old runtime version, obsolete patch admission/requirement, and missing predicate symbols respectively. Unit A uses the approved existing failing baseline.
- **Records:** the ADR addendum, decision-index update, interfaces and CHANGELOG describe the registry-cache transition and consumer compatibility change.

**Reviewer process notes**

Reviewed `59079d5c5..16550b4ba` read-only, excluding deleted vendor contents. The override checker, Gitleaks configuration checker and diff whitespace check passed.

Rust/Windows tests and RED commits were inspected, not rerun. Recorded verification remains qualified: no uninterrupted full-gate pass, with environmental/pre-existing failures and deferred platform legs documented. The late dependency-contract update is a follow-up correction, not an independently failing RED commit.

## Disposition

| Finding | Disposition |
| --- | --- |
| medium — the D27 runner hard-codes `rusqlite` 0.40 | Fixed. The script's SHA-256 is not frozen: the D27 verifier rebuilds the runner bundle from git at each receipt's source sha, and `check-runtime-checkpoints.py` still passes. RED `562477d70` (the generated manifest follows the selected checkout: 0.40 for a historical source, 0.41 for a current one, and this checkout's engine requirement); GREEN `1ea86b182` (`rusqlite_requirement` reads the selected engine manifest; one audit-test fixture gained an engine manifest) |
| note — the dependency-contract update is a follow-up correction | Agreed and recorded in status.md: `246367c61` corrects a contract missed in B-RED-1; its failure evidence was produced against the base export, not at its own parent |

## Follow-up round (16550b4ba..fdbb74e10)

Reviewer: codex gpt-6-astra, reasoning high, read-only, on an export of
`fdbb74e10`. Verdict: CONCERN, one finding.

- **medium — `scripts/check-runtime-checkpoints.py:646-654`.** A symlink
  placed at `ac073-bundle/target`, `release` or `deps` passes the
  directory and leaf checks when the binary hash matches; the new test
  covers only the bundle directory itself. Codex states the gap predates
  this delta and is not a hash bypass.

Passed on inspection: the root relocation (redirecting the root does not
bypass the retained receipt, binary or raw-evidence hashes); the D27
dependency selection (0.40 historical, 0.41 current; workspace
inheritance and a missing manifest raise rather than fall back); the PyPI
parser (matches `cgi.FieldStorage` for `name`, `version` and missing
fields, multipart and urlencoded); the RED evidence; no assertion
weakened.

| Finding | Disposition |
| --- | --- |
| medium — nested symlinks inside the AC-073 bundle | Not fixed in this slice. Pre-existing, not introduced by the relocation fix, and the hashes still bind the evidence. Owner policy for measurement tooling (2026-10-15) is to test measurement correctness and skip tamper guards. |
