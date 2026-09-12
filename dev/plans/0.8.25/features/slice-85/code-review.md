---
title: Slice 85 implementation code review
status: PASS
---

# Slice 85 implementation code review

The independent reviewer returned **PASS** after focused correction rounds.
The final review confirmed:

- Node 25.9 is the sole 0.8.25 Node contract across package metadata, docs,
  local/CI/release pins, runtime-floor routes and active test oracles;
- the TypeScript runtime route compiles the complete test configuration before
  running its emitted test;
- Slice 80 reuse identity, candidate/artifact/evidence paths and hashes, and
  added/overridden command routes are guarded against accidental drift; and
- installed Python and Node runtime-configuration checks use fresh processes;
- Windows runner status tolerates PowerShell CRLF without masking command
  failures, and quarantined evidence/artifact paths cannot close a row; and
- the Slice 72 CE accepted non-pass is limited to that row and bound to the
  exact [CUDA Engine p95 exception](ce-engine-p95-exception.md) path and digest.

Focused evidence at review time was 25/25 Slice 85 validator tests, the exact
Python workflow assertion, both registered shell contract guards, the Node 25
TypeScript runtime selector, actionlint, Ruff and a clean `git diff --check`.
The validator intentionally remains a basic completion/correctness guard; it
does not add signing or adversarial anti-forgery infrastructure.

The final correction review at `137cf86e` returned **PASS**. It confirmed that
venv executable symlinks cannot collapse candidate identity to the ambient
interpreter, `sys.prefix` must name the configured non-base venv, and retries
clear the environment and force-reinstall the exact wheel. The focused result
was 39 tests and 36 subtests passing, with a clean diff check.

The final validator review at `e1dee526` also returned **PASS**. The AC-075
receipt check now requires finite ordered recall/CI values, a valid SUT-result
digest and `CI-high >= 0.90`, while preserving exact fixture, ground-truth,
route and candidate-artifact bindings. Mutation tests cover malformed digest,
low CI and unordered CI. The reviewer confirmed that candidate `2e52602c`
remains valid because the subsequent commits change only closeout tests,
validation and documentation, not product or artifact bytes.

Hosted execution later exposed macOS's logical `/var` versus physical
`/private/var` spelling in the installed-package isolation guard. A focused
test failed first, then passed after the smoke helper canonicalized its
temporary root with `pwd -P`. Independent review returned **PASS**: the change
aligns both sides of the containment check without weakening isolation or
cleanup. This verification-only correction advances the exact release
candidate to `d1bff6f7`; no product or native-artifact bytes changed.

Hosted execution then exposed three CI-fixture isolation defects: preflight
assumed a local `main`, release-gate tests inherited the dispatch event, and
TC-5 unit tests inherited the author's model cache. Focused RED/GREEN fixes
fall back to `origin/main`, neutralize the event for ordinary tag cases, and
create the model directory in each test fixture. The reviewer first required
permanent remote-only preflight coverage; the added arm deletes local `main`,
retains `origin/main`, and asserts the exact resolved SHA. Final re-review is
**PASS** at `36b7b86d`; all 21 preflight assertions, 22 combined Python tests,
the release-gate shell suite, shell syntax and diff checks pass. These
test/tooling-only changes do not alter product or native-artifact bytes.

The exact-candidate hosted Windows attribution job then exposed one stale
feature-wheel inventory expectation. Runtime probe connections are
`cfg(test)`-only, so a release wheel built with `test-hooks` correctly creates
zero of them. The private observer, installed-wheel oracle and structural
recurrence guard now agree on `probes:0` while still requiring all twelve
managed connections and `complete=1`. Independent review returned **PASS** at
`4e78afd1`; the structural guard passed 249/249 and the exact Rust observer
test passed 1/1. The change affects private verification hooks only.

The next exact-candidate Windows run exposed the same stale count in the
installed binding inventory method and its hosted assertion. The release
`test-hooks` feature does not enable Rust's `cfg(test)`, so both correctly
expect zero runtime-probe connections while continuing to require all twelve
managed connections. RED was hosted job `103561334054`; GREEN included the
307/307 structural and mutation suite plus `cargo check -p fathomdb-engine
--features test-hooks`. The first review found two false-pass-prone recurrence
assertions; after scoping them to the exact workflow marker and public method
and adding a mutation for each site, independent re-review returned **PASS**.
Candidate `18fefc67` is the final code candidate.

Final closeout evidence review also returned **PASS** after correcting three
stale living-document references. It verified the 31-row manifest and hashes,
release-state views, exact hosted/Windows/Jetson identities, final TC-5 metrics,
bounded AC-012 rerun, Node 25-only scope, CE disposition link and
non-publication boundary. Markdown lint and `git diff --check` are clean.

The closeout pre-commit hook then correctly failed on the copied CE overlay's
tokenizer artifact digest because the exact Slice 85 path was not in the
machine-checked Gitleaks exception set. A path-specific, lowercase-64-hex-only
exception and contract assertion went RED/GREEN at 24/24. Independent review
returned **PASS**: the AND-composed rule cannot mask a credential-shaped value
even at the allowed path and does not broaden any other exception.
