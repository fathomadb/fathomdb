# Slice 103 Windows erasure trial protocol

Frozen before new measurements on 2026-10-03. This is Track W's trial
protocol, not a change to the Slice 103 plan or a production remedy.

## Provenance and controls

- Source baseline: `8fb6fdac9d2f1655a62355e0ca8af51d42dd1fb7` on
  `llm/slice-103-windows`; release branch is `release/0.8.27`.
- Historical control: install the unchanged registry `fathomdb==0.8.26`
  Windows wheel in an isolated environment; record wheel SHA256, package
  metadata, installed extension path/hash, interpreter, platform, and the
  exact Memex `w12_fathomdb_erase_repro.py` hash. Do not edit that script.
- Candidate: build a Windows wheel from an exact source archive of this
  baseline, hash the archive and wheel, install in a fresh environment,
  and verify installed import paths and extension hash outside the source
  tree. The wheel may still declare package version 0.8.26 because the
  unreleased 0.8.27 release branch has not yet had its mechanical version
  bump; identify the candidate by source SHA and artifact hash, not version
  alone. Keep source, build target, and environments disposable and isolated.
- Linux controls use the same historical wheel/version and an exact-baseline
  locally built and installed Linux wheel, with import and hash checks.

## Fixed trial sequence

1. Historical Windows: run unchanged Memex reproducer in order `noread 40`,
   `read 40`, `page 20`. Run its Linux `read 40` and `page 40` controls.
2. Candidate Windows: three fresh `noread 40`, `read 40`, `page 20` batches,
   preserving every trial and process exit. Candidate Linux: at least
   `read 40`, `page 40` controls. Fresh database per trial, one engine,
   50 anonymous `Note` rows from `src-a`, one from `src-b`, optional fully
   materialized read or canonical page, then public `erase_source(src-a)`.
   No external SQLite handle is open during erase. No raw checkpoint before
   erase, including diagnostic checkpoints.
3. In separate focused candidate runs, use the public Python lifecycle
   `transition(..., deleted)` then `purge(logical_id)` on a governed node;
   also invoke operator `excise_source` by its actual CLI contract. Repeat
   each shape on Windows and Linux. These are distinct from source erase.
4. For each candidate trial, capture public verb, binding, typed error
   class/stage/detail, rows before/after, WAL bytes before/after, and success
   report including zero-count idempotent completion. On an incomplete
   erase, capture the row/retained-source state and exact write-fence error,
   retry the identical verb on the same engine at bounded delays, then close,
   reopen, retry identically, and verify independent-reopen absence and a
   successful post-completion write. Record all attempt results. Sampling of
   actual checkpoint BUSY/frame counts and managed connection state must use
   the existing operation hook or a clearly identified test-only build; its
   timing-sensitive runs remain separate from ordinary installed-wheel runs.

## Decision rules

- A current candidate WAL refusal establishes persistence of the reported
  symptom; a typed five-attempt refusal can still be correct fail-closed
  behavior. Production change requires a demonstrated contract violation or
  sticky same-engine BUSY plus controlled attribution.
- Zero candidate refusals after the specified sample means only “not
  reproduced within this sample”; historical nonreproduction makes the
  comparison inconclusive. A clean batch does not establish absence.
- Keep the retained historical 11 first failures as the supported count;
  the separate draft's 12th has no preserved raw trial and is unverified.
  Existing 1,154 log occurrences are not independent trials.
- A physical close/reopen clearing BUSY is evidence of a connection-lifetime
  mechanism but does not identify the holder. Keep engine-owned statement,
  read-mark, checkpoint-lock, and Windows `-shm` hypotheses separate until
  a controlled discriminator establishes one. Do not infer a holder from
  an idle role collector or an open file handle alone.
