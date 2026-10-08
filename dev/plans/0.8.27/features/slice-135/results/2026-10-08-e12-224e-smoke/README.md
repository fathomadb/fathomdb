---
title: Slice 135 E01–E12 engine functional smoke at source 224e44c59
status: AUDITED_FEASIBILITY_NOT_PAIRED_TIMING
target_release: 0.8.27
---

# E01–E12 engine smoke at source 224e44c59

A clean detached checkout at `224e44c593c13d86ece648adabe445723db04070`
built the release-mode engine workload binary recorded in [build
provenance](build-provenance.json). Its SHA-256 is
`9809af9e7687711d536fc4cc621185f2107854b4604e9d7031915152a29d358e`.
The pinned local CPU model, 32-row corpus and unchanged twelve Slice 115
engine paths were used. All twelve paths completed 100 valid observations
each. The [independent audit](audit.json) checked source, binary, model,
expected ordered outputs, persisted state, samples and resources against
the [raw receipt](raw-archive/raw.json). The [negative
controls](negative-controls.json) rejected changed source identity and an
altered ordered text result.

The copied [raw archive](raw-archive/) is local and untracked pending
end-of-phase retention. Its 17 regular-file `SHA256SUMS` manifest has
SHA-256 `48170fa2e4678f22616ef83d851dfce60c76a7fca25cc6b4448e5fe794fa537b`;
model asset symlinks are bound separately by their pinned hashes. The raw
observation SHA-256 is
`2352d6ba0b0e90229b5bff1f2fa1aaae1f016b68927729769622f45ff77b5d5b`.
This 100-sample smoke does not support query p99 or a paired latency claim.
