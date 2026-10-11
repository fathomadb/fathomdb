---
title: Slice 135 installed Python S02 verification
status: FULL_GATE_RED_UNRELATED_SUITES
target_release: 0.8.27
---

# Verification of the S02 harness and receipt

After commit `37ce48d26` added the S02 runner, negative controls and
functional receipt, `./scripts/agent-verify.sh` ran on that clean branch.
Product source and `Cargo.lock` still matched `b2ac8081e79a6e626d01f5f97331be331f1cc16d`.
The [full gate log](full-gate.log) reports **186 registered, 186 run, 184
passed, 2 failed, 0 skipped** in 1,067 seconds. Lint, typecheck and security
completed before the test leg. The Rust, TypeScript and executable native
binding suites passed. The Slice 135 harness (including the three S02
negative-control tests) and TypeScript S01 contract suite also passed.

The two failed suites reproduced earlier checkout failures:

- `test-steward-orient`: its real-checkout briefing was 4,353 bytes, over a
  4,096-byte assertion, while another assertion in that same suite accepts
  the 5,120-byte cap. The [exact suite log](full-gate-briefing-failure.log)
  records its output. This is a suite name, not a role used for Slice 135.
- `test-python`: 1,582 passed, 30 skipped, 3 failed. One Slice 130 package
  declaration-count assertion expects 1,131 and observed 1,132; two isolated
  `verify_embed_db` children could not import `eval`. The
  [exact suite log](full-gate-python-failure.log) retains the traces. The
  import cases had previously passed with `PYTHONPATH=src/python`; this gate
  ran the unchanged repository command and did not mask the failures.

The full gate is **red**. Its passing focused harness and independent
[S02 audit](audit.json) support only the bounded functional feasibility
claim in the [result note](README.md). Neither a scoped pass nor the two
known failed suites establishes a release-wide green verdict.
