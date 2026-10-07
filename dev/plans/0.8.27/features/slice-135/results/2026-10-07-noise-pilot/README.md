# Slice 135 baseline-only noise pilot — 2026-10-07 UTC

**Status:** five valid baseline blocks retained; comparison protocol **not
frozen**. This is a baseline variance observation, not a 0.8.26 versus 0.8.27
performance result. The upward text-query trend needs a quiet-host repeat or
better idle/thermal control before setting final comparison counts and
uncertainty.

## Identity and method

- Baseline: `v0.8.26`, peeled source
  `f99e002f0d2e4002f3694c9f8d4986b56089edaa`; source-tree SHA-256
  `d25750313a18352c8f81c34fef8c160b7d6b884c4156c143ea12afc11d7fc133`;
  source `Cargo.lock` SHA-256
  `ffebda90ad6bae977a63cdb7d4c8edbce88632c2d353d5251247410040a16022`.
- Measured binary SHA-256
  `c6d2133640535ce49846e328ccceac91a96997306377bd83a998d99825574db1`;
  runner bundle SHA-256
  `1c86910bfddba6d77b15937435f2e2da3180c7e1abf91a63253bc45adeb89397`.
  The [build provenance](build-provenance/) retains the resolved lock,
  compiler output, runner bundle and qualification attempt. The 10 MB binary
  remains in `/tmp/slice135-pilot-final-build-smoke/`, outside Git; its hash
  and exact source/build inputs are retained here.
- Five serial unprofiled blocks used the same verified binary, 32 fixed rows,
  one warm-up per cell, then 100 text searches and 100 fresh-database close
  calls per block. Search output and close/reopen state were checked outside
  the timed calls. `text` measures engine call to materialized result;
  `close_fresh` measures `Engine::close` call to return. Reopen verification is
  outside the close timer. This is an engine-only CPU/no-embedder pilot.
- Run from the integrated Slice 135 branch using `scripts/slice135_pilot.py`
  with `--checkout` set to the isolated 0.8.26 checkout, `--source-sha` set to
  the commit above, `--samples 100`, `--reuse-binary-from` set to the verified
  build directory and `--binary-sha256` set to the binary hash above. The exact
  per-block command and source/protocol bindings are in each block directory.

## Blocks and independently checked values

Every [block](block-01/) through [block 05](block-05/) has `raw.json`,
`summary.json`, `attempt.json`, the protocol, source identity, command and
stdout/stderr. All five attempts say `VALID_BLOCK_RECEIPT`; each cell has 100
valid observations, zero invalid attempts and semantic checks matching the
protocol. Environment invalidators were empty: performance governor stable,
no observed cargo/rustc or named heavy job, no swap activity, and 31.9 GB or
more free at the block boundaries.

| Block | Text p50 / p95 (ns) | Close p50 / p95 (ns) | Raw SHA-256 prefix |
| --- | ---: | ---: | --- |
| [01](block-01/) | 173087 / 519342 | 5300300 / 5710726 | `acc68e55` |
| [02](block-02/) | 175752 / 520645 | 5259093 / 6013729 | `c8688577` |
| [03](block-03/) | 174349 / 520043 | 5182377 / 5668837 | `2f81fdb4` |
| [04](block-04/) | 176294 / 560129 | 5280172 / 5876500 | `91b531e4` |
| [05](block-05/) | 186803 / 598552 | 5254334 / 5777052 | `89ef339e` |

An independent standard-library calculation read each raw file, verified its
SHA-256 against the summary, required 100 valid attempts per cell and no
invalidators, sorted the nanosecond observations, and selected nearest-rank
`ceil(q × n) - 1` for p50 and p95. It matched all ten validator summaries.
Across blocks, text p50 ranged 173087–186803 ns (range 7.8% of the median)
and text p95 ranged 519342–598552 ns (15.2%). Close p50 ranged
5182377–5300300 ns (2.24%) and close p95 ranged 5668837–6013729 ns
(5.97%). Slow valid close maxima of 13.86 ms in block 02 and 14.29 ms in
block 05 remain in the raw samples. No p99 is reported from 100 observations.

## Invalid attempts and limits

The [adapter invalid-attempt register](adapter-invalid-attempts/) retains an
early baseline qualification refusal caused by incorrectly requiring a
`third_party` directory in 0.8.26, and the deliberately wrong binary-hash
reuse refusal. Neither produced a valid timing block. Both issues were
resolved or correctly rejected before the five blocks above.

The text p95 increases in the final two blocks despite the recorded boundary
conditions remaining stable. The runner does not sample temperature, effective
CPU frequency or every possible background process throughout a block. These
five blocks therefore do **not** justify a final uncertainty rule or a paired
performance claim. Repeat the baseline-only pilot on a quiet host with
recorded idle stabilization, then freeze sample counts, workload cells and
the uncertainty method before candidate timing. The later paired campaign
must include a whole-system mixed sequence and installed-SDK cells; this
two-cell engine pilot does not substitute for them.
