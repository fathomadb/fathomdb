---
title: Inspector 0.2.0 Slice 80 status
date: 2026-11-21
status: COMPLETE
---

# Slice80 — COMPLETE

Release readiness is complete. [INS-020-LOCAL-RELEASE](../release-decisions.md)
removes the unperformed five-person study;70 is closed by owner disposition,
without claiming any human session or pass. T-01–T-05, High INS07/08 and every
technical disclosure/identity/resource constraint remain.90 is next and owns the
actual local release; final `dist/inspector/0.2.0/` is still absent.

## Delivered and reviewed

Reconciled current needs/requirements/AC, single matrix, HLD, scope, project/user/
contributor/tool/package documentation and roadmap. Historical study/expert
records and70's actual earlier failures remain clearly scoped. Public0.2.0
release notes/navigation, exact environment/model prerequisites, installation,
removal, actual75 workflow/refusal guidance and future distribution methods are
current. No registry availability or human-validation claim is made.

Added the Inspector-owned local CI composition and compact source-bound candidate
verification/sealing/promotion tool. Packaging now has outer archive checksums,
explicit isolated target selection and fresh-output failure cleanup. Promotion
revalidates owned sibling staging then uses atomic Linux no-overwrite rename;
existing destinations refuse and caught interruptions clean only owned staging.
Existing shared hosted routing and root dist ignore rules were already correct
and remain unchanged. MkDocs navigation is the only shared documentation change.

Direct Sol-high design review APPROVE after source-binding and Ubuntu22 release
example clarifications. Fresh Sol-high code review APPROVE after fixing actual
tar extraction/modes, complete checkout-Python consistency and SIGTERM cleanup.
No material findings remain. Direct independent Terra verification PASS, with
actual evidence inspected under `/mnt/data/inspector80-owned/terra/`.

## Test-first changes and checks

| Behavior | Actual RED → GREEN commits |
| --- | --- |
| Candidate integrity/source seal | `a83bc39df` → `e9b6ed8d0` |
| Atomic promotion/collision/interruption | `4e0f9cbbb` → `d78d001cf` |
| Outer checksum/isolated target/build failure cleanup | `64e5d1715` → `419caa74c` |
| CI prerequisite and source exit propagation | `c02ec9581` → `d9593f029` |
| Archive executable mode/fresh extraction | `40395df5a` → `9c65fb918` |
| Real subprocess SIGTERM cleanup | `70717c812` → `7764c834b` |
| Checkout Python and hostile ambient interpreter | `e96d2631b`, `9ba8558c0` → `1e0a6aaef`, `89d89f2e3` |

Each RED actually failed its intended assertion. A premature GREEN statement in
`6ec7c2b24` followed an overstrict data-file0644 check failing under normal umask;
`9c65fb918` explicitly corrects that statement and passes the unchanged tests.
No oracle was weakened or database mocked. Documentation changes need no fake RED.

The documented default command ran once from clean source
**`7449ee0108d503cbed3b6a33e2c841072d6de0ce`** and exited0:

```sh
PATH=/tmp/inspector40-tools:/usr/local/go/bin:$PATH bash src/inspector/tools/local-ci.sh /mnt/data/inspector80-owned/candidate /mnt/data/inspector80-owned/fixtures /tmp/inspector70-jammy-target
```

- Strict full source gate: **206/206 suites PASS, skipped0/excluded0**. Lint/typecheck/security PASS; live AC-036/037 ran, security0 violations/blockers/downgrades. Inapplicable AArch64/CUDA lint and individual ignored/opt-in measurements remain distinct from executed positives.
- Tooling **12/12**, JavaScript units **10/10**, fresh-tar installed browser/native tests **81/81**, fail0/cancelled0/skipped0. Includes all native views/person/compare/Back, all8D01, seven75 UX fixes, real F06 large content/queue/cancel/fault/detach, natural30,002ms lexical deadline, cache asset matrix and actual CPU cancellation/close.
- Scoped shell/Markdown/public docs lint, strict MkDocs and existing proportional-CI routing fixture PASS. Normal commit hooks ran. No unchanged full gate was repeated.
- Independent Terra: source binding/extraction/inner checksums/help/ABI, real scratch promotion/reverification/collision refusal and release filesystem tests **6/6** PASS. Published0.8.29 review GUI **5/5**, equal-body distinct identity **1/1**, traced installed person **1/1** PASS, no skips. Native closes, private-state/lock cleanup and recorded descendants gone. Trace: **0 Internet-family attempts,3 read-only source asset opens**.

Raw local CI result: `/mnt/data/inspector80-owned/local-ci.log`; independent
logs/screenshots and trace: `terra/` in the same owned directory. Original review
store SHA remains `597b909dca5f487e909b37bcc6d2bd1f70e4b9fd582524fbd3d15bb277aa7e4e`.
Only closed disposable copies, including existing lock sentinels, were used.
Unchanged broad scale campaigns retain their earlier source/artifact scope.

## Exact candidate for90

Candidate root: **`/mnt/data/inspector80-owned/candidate`**.
It contains `candidate.json`, outer `SHA256SUMS`, the tar and directly usable
extracted bundle named `fathomdb-inspector-0.2.0-linux-x86_64`.

| Identity | Value |
| --- | --- |
| Source/build packaging revision | `7449ee0108d503cbed3b6a33e2c841072d6de0ce` |
| Archive SHA256 | `0a731d3243b2c5b2e2611578ac3a9c06b4f12ba488af558c4a9530c341dc8e0f` |
| Executable SHA256 | `4fce1f5b03bde56021721d48f990b1427e33a712c40231c9fb15266ee856d2d9` |
| Actual embedded build | `f1e9337dacdf79f77b60dd382a197eea340aa816aa3f1cc96992ab059ddbf7d1` |
| Selected unchanged Engine | `6edc1155f039f87cd0a37660cb27ae4373bbce71`, compiled schema37 |
| Configuration | Inspector0.2.0, GNUx86_64 release, cached CPU true, fixture control false, Rust1.95.0 |
| Qualified pairing | Ubuntu22/glibc2.35 userland, kernel7.0.0-43-generic/LandlockABI8, Chromium156.0.8078.4; ELF maximumGLIBC2.34 |

This is a newly packaged/tested candidate;70/75 artifacts are preserved, not
relabeled. Checksums and the unchanged embedded source/build identity do not
claim reproducible compilation or signed provenance. Later status/documentation
closure commits do not change this candidate's real source/build identity.
Stock Ubuntu22/kernel5.15 lacks required cached-CPU LandlockABI>=3. No blanket
OS/browser support, bundled model, download/repair, registry deployment or human
validation is claimed. Existing pinned BGE assets remain prerequisites.

## Runnable90 handoff and cleanup

Start clean in this release checkout. The [tool guide](../../../../../src/inspector/tools/README.md#local-release-readiness)
contains installation and promotion semantics. Execute90's approved plan; these
commands are instructions, not an80 final release:

```sh
.venv/bin/python src/inspector/tools/local-release.py verify /mnt/data/inspector80-owned/candidate --source
mkdir -p dist/inspector
.venv/bin/python src/inspector/tools/local-release.py promote /mnt/data/inspector80-owned/candidate "$PWD/dist/inspector/0.2.0"
.venv/bin/python src/inspector/tools/local-release.py verify "$PWD/dist/inspector/0.2.0" --source
```

90 then checks every final path is ignored/no payload tracked, independently
extracts the final tar and exercises actual installed person/all-view/compare/
Back/native close using the existing harness. Its final record distinguishes
candidate source from later docs SHA. Any existing destination refuses without
overwrite; inspect/verify it explicitly. No registry credential, tag or upload.

The single full source gate's feature unification was followed by a locked
offline explicit cached-CPU build of development binaries/examples (PASS),
while the isolated Ubuntu22 production pair and candidate remain qualified.
All owned runtime processes have exited. Source/build/runtime ownership returns
to the parent. Keep candidate, closed fixture baselines and Terra evidence for90.
No new worktree/branch was created; the existing release checkout/target symlink
is retained. Primary checkout and Engine/SDK/MCP/schema/ranker/shared pins and
versions are unchanged. No main merge, tag, push, remote upload or publication.
