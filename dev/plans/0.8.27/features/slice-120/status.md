---
title: FathomDB 0.8.27 Slice 120 - TypeScript SDK decomposition status
status: COMPLETE_ON_RELEASE_BRANCH
target_release: 0.8.27
source_sha: e05bfd5330beb737b7704300e2752d2b977bb5b8
---

# Slice 120 TypeScript SDK decomposition status

Slice 120 is complete on `release/0.8.27`. The work started from
`e4bbd542ed6e8323a4a2009891609199e97b0fe9` in a temporary worktree.
The reconciled [plan](plan.md) enumerates the changes since the release draft,
defines slice-local R27-120A-D and AC27-120A-D, and assigns all root-owned
behavior. The [design](design.md) passed an initial `gpt-6.1-sol` high review
and two subsequent `gpt-6-sol` high reviews; [findings](design-review.md) were
closed before implementation.

## Result and acceptance

| Acceptance | Evidence |
| --- | --- |
| AC27-120A | `src/ts/src/index.ts` is a package-root export map. The `Engine` class retains thin public wiring and lifecycle ownership; open, read, write, projection, search, evidence, graph, embedding, instrumentation, native-call, and admin logic has explicit private owners. No unsupported package subpath was added. |
| AC27-120B | The exact pre-move capture and candidate each resolve to 207 TypeScript public declarations, with identical entries. The 64 package/runtime export entries are also identical. Eight focused TypeScript suites passed. The installed root consumer compile and runtime fixtures passed. |
| AC27-120C | NAPI/native declarations and source were untouched. A fresh isolated install of the packed main and platform tarballs loaded the supplied native addon and exercised representative write, read, search, graph, frozen-evidence error, and admin calls. |
| AC27-120D | The compile fixture was proved non-vacuous: removing `rerank` from the generated root declaration produced TS2459 before the source move. AC050c scanner regressions were also RED before their fixes and GREEN after. Independent `gpt-6-sol` high code review and Terra high verification passed on final source. The strict full repository gate passed. |

The pre-move `index.ts` SHA-256 was
`bae323a09eab9d689b2c5668342a86466552c218ab6fa7de51c9303be0336f4c`;
the pre-move `index.d.ts` SHA-256 was
`6960c0e764267110fd023be9d7fb85956b83efa878b077ccee7203ca6215c684`.
`parse_typescript_surface` over the pre-move and candidate declaration
directories returned 207/207 identical entries. The runtime export-list
SHA-256 was `c2d20eb1502e9f45bf1b5553e894aca7f179d98aef96c5c3034254f45f78e7d8`
on both sides. The older immutable Slice 30 baseline differs only in the
accepted Slice 90 readonly configuration and Slice 110 subscriber changes
listed in the plan; the package/runtime row is identical.

The installed check used Node 25.9.0 and npm 11.19.0. Its main tarball SHA-256
was `a343c5e8dd787414b9798231873390b106d2519df700debd172de1367c1ab830`;
the platform tarball was
`acd4107eae98b89449cc14661ad1710bd3b2366ca35a6417ec8778e247a90fc5`;
the supplied production native addon was
`bf87c40b1dfbca3b60c73d15b71b2026537204d87007088bb8146b45c3a64c79`.
The npm version differs from the pinned 11.12.1 in prework. The supplied addon
was already built in the release checkout, so this check does not establish a
fresh native build from the Slice 120 tree; native source was unchanged.

The focused Node suites passed 8/8 with that production addon. The full
`slice90-engine-config` suite's test-hook-only cases could not pass with a
production addon; 6/8 cases passed. Sandbox child-process restrictions made
`sdk-surface-parity` and `ffi-safety` suite launchers unsuitable as an oracle
here; direct Python parity checks and the installed consumer were used.
No claim of a fully green TypeScript suite is made from those focused runs.

## Gate and review

The final candidate `e05bfd533` passed `CARGO_PROFILE_TEST_OPT_LEVEL=3 AC013_VECTOR_DIM=384 ./scripts/agent-verify.sh` on the ptrace-capable executor: lint and typecheck passed; security recorded 0 violations, 0 blockers, and 0 downgrades; `agent-test.sh` passed 184/184 suites with 0 skipped and 0 excluded.

The first full gate exposed AC050c's direct-declaration-only TypeScript
scanner. A failing fixture preceded its repair. Independent review then found
runtime/type and literal-spoof cases; each received a failing case before a
fix. The final scanner accepts only real named re-exports from an export-only
package root with a compatible declared target kind. The complete
`test_removal_detect.sh` fixture suite and live-history path pass. The final
code review reported no actionable issue; Terra independently passed the
focused scanner tests and TypeScript no-emit compile at `e05bfd533`.

The status is local release-branch closeout only. Slice 130 owns Python SDK
decomposition; publication remains gated by later release qualification.
