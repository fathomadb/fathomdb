---
title: FathomDB 0.8.27 Slice 150 — retention and large-artifact survey
status: VERIFIED
target_release: 0.8.27
---

# Retention and large-artifact survey — 2026-10-10

The Slice 90 evidence was moved from its historical
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/`
path to the durable
`/home/coreyt/projects/fathomdb/data/0.8.26/qualification-evidence/slice-90/`
directory. Its `README.md` records the relocation and exact minimum
checkpoint set. The checkpoint tool maps historical paths to this directory.
An initial survey of only the former path was incomplete; the independent
verifier found the relocated set and verified it before this record was
finalized.

## Slice 90 checkpoint set

The 12 files below total **81,701,383 bytes (77.92 MiB)**. Paths are relative
to the durable Slice 90 directory. The set must be retained with its layout and
bytes for checkpoint revalidation; it is distinct from the whole 16,648,121,887
byte Slice 90 tree and its other audit/qualification records.

| Bundle | Member | Bytes |
| --- | --- | ---: |
| `d27-entry-post-reboot` | `receipt.json` | 16,890 |
| `d27-entry-post-reboot` | `runner.bundle` | 59,913 |
| `d27-entry-post-reboot` | `corpus.jsonl` | 5,540,000 |
| `d27-entry-post-reboot` | `raw-output.jsonl` | 6,685,641 |
| `d27-entry-post-reboot` | `target/release/deps/d27_runtime_workload-864291735fc5bf93` | 10,602,256 |
| `d27-v2-candidate-2e94aaf4f-retry` | `receipt.json` | 18,371 |
| `d27-v2-candidate-2e94aaf4f-retry` | `runner.bundle` | 71,600 |
| `d27-v2-candidate-2e94aaf4f-retry` | `corpus.jsonl` | 5,540,000 |
| `d27-v2-candidate-2e94aaf4f-retry` | `raw-output.jsonl` | 11,327,111 |
| `d27-v2-candidate-2e94aaf4f-retry` | `target/release/deps/d27_runtime_workload-477d6d10431be3c9` | 10,636,920 |
| `ac073-2e94aaf4f` | `eu7.json` | 2,257 |
| `ac073-2e94aaf4f` | `target/release/deps/eu7_real_corpus_ac-7330e27ac1709bed` | 31,200,424 |

Both D27 receipt hashes match
`dev/plans/0.8.27/features/slice-90/runtime-performance-qualification.md`:
`74c5ee0b43188dd2f2af138eb84dd67aa281034017693e095c82327c6dadc129`
and `52a20f29cb3bbb19a0ec159dace4a3cfbd74307c2dd3234cf59dbbfc820ad9b1`.
The two receipts' runner, corpus, raw output and executable digests match
their files. The AC-073 executable matches the digest in tracked
`ac073-execution.json`. `python3 scripts/check-runtime-checkpoints.py` returned
`1 structured checkpoint(s) valid` on 2026-10-10.

The final evidence manifest SHA-256 is
`42e4b5f0c7d1c1769859efb0b1b8b6758d23d869445a0eeab41e72ce355d567b`;
all 36 listed files match its byte sizes and hashes. The stage-2 manifest
SHA-256 is
`a675ddc41a1b2eef44b95006efb234f35aed413c8cdb130e78d5779fda2c7255`.
The tracked structured checkpoint also binds performance, code-review and
verification receipt documents whose current SHA-256 values match release
state: `5c5da3e80d494ad49a436a3738ba7fc9adbccf7e17672fe2a67465dfbdfe3407`,
`f0ca6ab553474e4bfeb53e90f2e6ffae6b9be99fecb04ece23015965e5e7e820`,
and `55fde74dc34e7bc091952aac84f62c7ab42c9a8dfc68728a961640535eb3acbb`.
These tracked documents are separate from the 12 host-local files.

## Objects above 100 decimal GB

The independent read-only survey ran from 2026-10-11 01:49:57 to 01:50:04 UTC.
For each root below, it ran
`ionice -c3 nice -n 15 du -x -P -b -a --threshold=100000000001 -- ROOT`.
The threshold means strictly greater than 100,000,000,000 apparent bytes;
`-P` avoids following symlinks and `-x` keeps each scan on its filesystem.
Every command exited zero with empty standard output and error. A separate
`findmnt -rn -o TARGET` enumeration identified 16 sandbox bind mounts at
`.git`, `.agents`, `.codex` and `.aws` below the four roots. At 2026-10-11
02:24:38 UTC, the independent verifier ran the same `du` command over each
mount root; all 16 exited zero with empty standard output and error.

| Scope root | Result |
| --- | --- |
| `/home/coreyt/projects/fathomdb` | Zero qualifying objects. |
| `/home/coreyt/projects/fathomdb-worktrees` | Zero qualifying objects. |
| `/home/coreyt/projects/memex` | Zero qualifying objects. |
| `/home/coreyt/projects/memex-worktrees` | Zero qualifying objects. |

These roots cover the 0.8.27 release checkout, its host-local evidence, the
relocated Slice 90 evidence, other FathomDB/Memex checkouts and associated data.
Inaccessible service-private `/tmp` subtrees were outside this survey. No
qualifying object needs a retention or cleanup decision. The 12-file checkpoint
set and the wider Slice 90 audit tree are retained.

The completed Slice 140 checkout held a rebuildable 35.9 GiB Cargo target
cache. After confirming no active build, `cargo clean` removed that ignored
cache to satisfy the public comparator's space preflight. It did not touch
source, receipts or the relocated Slice 90 evidence. Older Slice 135 scratch
was left intact.
