# Slice 110 hosted native package qualification

The nonpublishing [CI run](https://github.com/fathomadb/fathomdb/actions/runs/37220605280)
was dispatched on `release/0.8.27` with exact `candidate_sha`
`366e1bc3d4e4df2a8bb8cd9268ccfdae0e02b08a`. Each row below completed
`native-artifact-runtime-validation` successfully. That job builds the native
artifact and TypeScript package, packs the thin main and matching platform
package, installs the pair in a fresh external consumer without registry
resolution, runs the installed runtime smoke, checks binary provenance, and
uploads a candidate-bound receipt. Node was 25.9.0 and Rust was 1.95.0.

| Target | Job | Native SHA-256 | Retained receipt |
| --- | --- | --- | --- |
| Linux arm64 GNU | [success](https://github.com/fathomadb/fathomdb/actions/runs/37220605280/job/111490030945) | `a33f0b0b26df365c45ef1aedf494e3c4836edf35f3609eb30ef3d16680c8966e` | [JSON](native-artifact-receipt-linux-arm64-gnu.json) |
| macOS arm64 | [success](https://github.com/fathomadb/fathomdb/actions/runs/37220605280/job/111490030934) | `3679d3d4b4b299748d9e2270fb8395dd7db04d2daf26317298ece4371cd51ac8` | [JSON](native-artifact-receipt-darwin-arm64.json) |
| macOS x64 | [success](https://github.com/fathomadb/fathomdb/actions/runs/37220605280/job/111490030994) | `d156a09c2db0976c57fc109482978b1459924538a3419899e0a715f6b19d9e83` | [JSON](native-artifact-receipt-darwin-x64.json) |

This is a pass for these three installed-package rows, not a green conclusion
for the whole CI workflow. The unrelated `verify` job
[failed at its tool preflight](https://github.com/fathomadb/fathomdb/actions/runs/37220605280/job/111490030750)
before testing because shellcheck 0.11.0, actionlint 1.7.12, Ruff 0.15.17,
Pyright 1.1.410, and repository-root js-yaml were missing or had the wrong
version. The separate local full `agent-verify` passed 182/182 suites on the
same product code. The self-hosted Windows CI row was still queued when these
receipts were captured; Slice 110 already has an independently qualified
exact-source Windows installed package from the local VM.
