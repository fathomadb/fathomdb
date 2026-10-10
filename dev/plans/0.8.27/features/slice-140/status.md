---
title: FathomDB 0.8.27 Slice 140 — implementation status
status: IMPLEMENTED_PENDING_VERIFICATION
target_release: 0.8.27
planning_baseline: 360bbe63e65aca93cb2200f6c8e27624f4042ea8
---

# Slice 140 implementation status — 2026-10-10

The [plan](plan.md) is reviewed and the [design](design.md) is approved,
including the generated-documentation amendment. Rust implementation and final hidden capture are complete at `f327c28ca`;
the documentation-target follow-up below also passes focused verification.
Independent final reviews and the integrated full gate remain open.
The historical planning reviews used `360bbe63e`; implementation includes
Slice 117's completed handoff at `1476d83e0`. The release ladder remains
`140 → 150 → 160`. Slice 117 artifacts are qualified and unpublished, with
Slice 160 owning publication. Slice 140 is not closed by this worktree
record, and publication-facing documentation is not a publication receipt.

## Reviewed decisions and corrections

- Reconciled the draft with Slices 40–135, the new Rust SDK, the Slice 117
  handoff, and the unapproved Slice 137 proposal. No SDK capability, provider
  policy, or final-release qualification work was added to Slice 140.
- The first independent `gpt-6.1-sol` high design review returned
  **APPROVE WITH FIXES**, two P2s. The [review record](design-review.md)
  identifies the missing CLI size trigger and hidden release-probe blind spot.
  Both were corrected. A second `gpt-6-sol` high review returned **APPROVE**
  with no remaining material finding.
- The owner ruled the dual-runtime ADR accepted if consistent. Its policy,
  CLI outcome matrix, result types, and CUDA error classifier were checked
  against HITL `seq:250`/`seq:252` and current source. The ADR and index row 43
  now say accepted, limited to their x86_64 artifact scope.
- The two `UNREVIEWED` design memos now carry explicit historical banners
  and `COMPLETE` metadata. The 0.8.25 Slice 40 classifier table now gives the
  `failed`/absent-physical row precedence over unenrolled-edge corruption.
- The [large-file register](large-file-review.md) enumerates 39 tracked code
  triggers at the planning baseline, explains the five original outliers, and
  records bounded dispositions. A later owner ruling removed human review of
  this register as a slice or release gate. AC27-140C still requires the
  final-candidate re-scan and any new triggers to be recorded.

## Planning verification

An independent `gpt-5.6-terra` high read-only verification returned **PASS**
for the planning-only diff. It checked the 39-file count, nine production-file
sizes, three-SDK taxonomy, seam scope, Slice 40 classifier order, dual-runtime
ADR against policy/source and HITL `seq:250`/`seq:252`, historical labels,
and the then-current `117 → 140 → 150` ladder. It confirmed that formal Slice 140 release
state remains `PLANNED` with Slice 135 as its recorded dependency; the reviewed
Slice 117 handoff is an execution precondition, not a silently rewritten
release-state dependency.

`./scripts/agent-lint-md.sh`, `./scripts/check-release-state-views.sh`, JSON
parsing of the release state, and `git diff --check` passed for this document
and metadata-only planning change. Public `docs/` and executable/source files
did not change, so this is not a public-doc or full-workspace gate claim.

## Work still required to close Slice 140

Source implementation, integration with the completed Slice 117 handoff,
focused controls, contract/document convergence and final-source large-file
coverage are complete. Terra verification, three independent final reviews, runnable
prepared-package examples and integrated full gate must be recorded before
acceptance. The parent owns release-state closeout and worktree cleanup.

## Large-file register policy update — 2026-10-10

The owner directed that the register remain in the release record without a
human review requirement for slice or release closure. The plan, design,
master plan, prework test approach, release board, and release-state ruling
now reflect that instruction. The register's recorded dispositions stand as
the baseline inventory; final-candidate coverage remains an ordinary Slice
140 check. No size threshold or human sign-off is a release gate.
An independent `gpt-6-sol` high review accepted the policy change with one
wording fix in the prework test approach; that fix was applied and the scoped
Markdown and release-state checks passed.

## Implementation progress — 2026-10-10

The integrated candidate at `18ef03da9` includes the qualified unpublished
Slice 117 handoff (`1476d83e0`). RED `2ea164f09`, GREEN `7556f1913`, and the
narrow ownership-mutation correction `7d91b5a38` gate the inventoried fixtures;
see the [test-seam change record](test-seam-change.md) for exact controls and
focused receipts. The [independent code review](code-review.md) returned PASS
on that candidate. Current architecture, interfaces, Rust install routes,
and local-only evidence citations have been corrected, and the integrated
large-file scan still enumerates 39 triggers.

Public capture completed 13 rows against `18ef03da9`. The immutable Slice 30
baseline remains unchanged; its comparison reports inherited public changes
from earlier slices. The [immediate public comparison](public-surface-comparison.md)
against the completed Slice 117 source isolates this slice to 31 intended
reader-guard-derived removals in each default/operator engine row, with no SDK
changes. Independent follow-up review accepted that delta and its documented
contract correction. The [hidden successor](../hidden-surface/baseline-01f4063f-diff.md)
completed 37 rows versus 35 before; 295 targets remain covered, both captures
have no test-build failure, and the 1,924-entry aggregate test/ignore-status
union is exactly preserved. Independent `gpt-6-sol` high hidden-diff review returned APPROVED with no
P0/P1/P2. The review is static and does not replace execution receipts. The scoped Markdown and public-document checks passed; targeted AC-050c also
passed with 64 removals documented.

The implementation worktree's full verification attempt fails at preflight
because no checkout-owned `.venv/bin/python` is installed. The prepared
release checkout must run the unchanged full gate after landing the reviewed
candidate. Independent Terra implementation verification, runnable package
examples, final acceptance, and release-state closeout remain open. No tag
or publication has occurred.

## Full-gate follow-up and publication-facing documents

The first prepared-release full gate at `16ede4130` reported stale historical
root/WAL/projection/Windows owner assertions and two pinned default targets
gated behind `test-hooks`. It later terminated with exit 143; it is not a
passing full-gate receipt. Follow-up restores default proof coverage through
production routes and relocates the structural writer proof to its unit
owner, preserving raw assertions, contract obligation/category/hash, and
anti-evaporation controls. The [design amendment](design.md#default-proof-preservation-and-release-documentation)
received a subsequent `gpt-6-sol` high **APPROVE**, no P0/P1/P2; the
read-only reviewer did not run tests. See [design-review.md](design-review.md).
The parent owns final source
review, Terra verification, integrated full-gate rerun, and release closeout.

The user explicitly ruled that the README and public/package docs are
publication-facing 0.8.27 documents. Their version labels, registry routes,
Jetson Pages instructions, reference/compatibility/operations pages, release
notes, and navigation now describe that release without transient candidate
notices. This editorial intent does not claim registry or Pages publication.
Current workspace/Python/TypeScript manifests remain 0.8.26 until Slice 160;
the retained manifest still contains only the verified prior 0.8.26 Tegra
wheel. 0.8.27 registry installs and live Tegra digest/download verification
cannot be claimed before publication. The documented Node verifier requires
an exact retained entry and matching SHA-256, fails closed when absent, and
uses a scoped shell failure guard before installation. The exact Pages URL
is recorded using `--package-lock-only --ignore-scripts`; the lockfile SHA-512
must match verified bytes before `npm ci` may install. No unknown digest is
invented and retention is unchanged. A local HTTP tarball fixture on release
Node 25.9.0 confirmed lock-only neither extracts packages nor runs lifecycle
scripts, preserves the exact URL, and records matching integrity; `npm ci`
ran the fixture lifecycle only after that comparison. This proves npm route
semantics, not the unpublished 0.8.27 artifact bytes.

Focused follow-up evidence: default Slice 20 proofs 2/2; default Slice 25
proofs and real raw-SQL negative control 7/7; relocated structural unit 1/1;
explicit-hook Slice 20 8/8 and Slice 25 7/7; C1 mutation suite, governed pin,
transcript hygiene, root reconciliation 8/8, 295-target coverage, focused
engine clippy, and CLI version-help test 1/1 pass. Historical WAL owner 5/5
and projection owner 4/4 were committed separately. The actual public
`read_projection_status` remains ungated; its fixture-only private adapter
uses the explicit unit/test-hooks route. The Windows scanner's new focused
control proves actual `#[test]` functions excluded and unannotated/comment/
string-decoy openers retained; the complete Windows suite passes **343/343**.
Its external-module escape mutant removes both the module and built-in test
function gates, then requires the original production-opener diagnostic.

The SDK README's complete program compiled and ran in an external source-path
application: one stored note and one text hit. This is source-package API
proof, not a future 0.8.27 crates.io installation receipt. Root Python and
Node package-route executions and strict site build remain assigned to the
prepared release checkout. A refreshed hidden successor capture accounts
for default-target restoration and the integration-to-unit proof path move.

## Generated-documentation decisions

After source/docs follow-up `bc555c7ee`, the user added source-derived
library/contributor documentation published by CI to Pages and three
independent final reviews. The initial amendment review found a misplaced
master paragraph and missing publication currency checks. Both were
corrected at `2c1b12a0c`, which received the required follow-up approval.
The implementation and focused results follow.

## Generated documentation implemented — focused verification

RED tests were committed at `c351edc04`; the Python runtime diagnostic
regression was committed at `4c10d9dfa` before its fix. The subsequent
`gpt-6-sol` high design review approved `2c1b12a0c`, with no open P0/P1.
The implementation creates a deterministic 44-operation index and an
11-package/five-entry-point contributor map, plus the maintained contributor
landing. Script fingerprints are invisible HTML comments; displayed tables
remain concise. The generator requires Python 3.11 or later and reports an
actionable configured-interpreter diagnostic when `tomllib` is unavailable.

The shared complete-site builder checks generated freshness; both workflows
keep that builder and freshly verify current-main ancestry before prepare
and deploy. Deployment uses anonymous public Git fetches and gains no
contents token or publication privilege. Old queued candidates fail closed.
The stale Python-preference caveats were replaced with the measured shared
three-SDK operation/error surface.

Focused receipts: generator **8/8**, real Git-history currency **5/5**,
combined Pages **20/20**, Tegra publisher **43/43**, and Jetson workflow
**90/90** pass; Ruff, Bash syntax, actionlint, scoped Markdown/public-doc
checks, and strict MkDocs build pass. All three new pages exist in the built
site. These are local source/pipeline proofs, not a live Pages publication.
The final source code review passed at `f327c28ca`, with no P0/P1/P2.
Final-source large-file coverage remains 39 paths. The hidden/test successor is approved. Terra, three independent final
reviews and the integrated full gate remain required for closeout.

Generated-doc source-link review found that `blob/main` can lag the Pages
publication candidate. A focused RED assertion now requires the existing
`release/0.8.27` source branch for every generated source link; the generator
and both maps use that branch. Package-version content remains neutral.
The completed `b3416577f` hidden capture is diagnostic only; the final
successor binds `f327c28ca` after source-link and package-root corrections.

The `5f7c39c8` source-link code-review follow-up passed, with no remaining
P0/P1/P2 on that candidate. A later completeness check found that the
contributor map also needs Python/TypeScript package roots. Plan/design now
include their source-derived names and manifest paths, version-neutral
output, source triggers and RED freshness controls. That bounded amendment
received `gpt-6-sol` high approval at `78aced66d`, no P0/P1/P2. RED package
coverage tests preceded the implementation, which adds the two package
manifest inputs and their CI triggers. Final capture binds the complete
reviewed source; `b3416577f` remains diagnostic evidence only.

## Final-source capture and register

The [final successor record](../hidden-surface/baseline-f327c28c-diff.md)
binds source `f327c28caa5f20c3342127fbc5fde3c6ea189c7b`. The unchanged
collector completed 37 rows with no test-build failures. Metadata and every
Rust API/release-probe row match the approved `01f4063f` successor. All 295
targets remain covered; the two pinned integration targets are default
again. The 1,924-entry test/ignore union preserves every proof after the
explicit integration-to-unit structural-proof relocation. Seven non-hook
rows regain ten default proofs, and eight hook rows keep their counts.
The earlier `b3416577f` diagnostic has equal comparable rows, but only final
`f327c28ca` is the approved successor. Independent `gpt-6-sol` high exact
artifact review returned **APPROVE**, no P0/P1/P2, after recomputing all 16
changed rows, hashes, API/probe equality, 295 targets, restored routes, test
union/path relocation and 321 unchanged ignored occurrences. The review was
read-only and did not rerun the collector or tests.

A fresh scan of every tracked code file at that final source confirms the
39 paths and physical counts in the [register](large-file-review.md), with
no new trigger and no human approval requirement. Source/CI/scripts are
unchanged after capture; subsequent commits carry internal receipt records.
The scoped Markdown check passes. This worktree has no full-workspace pass:
prepared examples, independent Terra/final reviews, integrated full gate,
release-state acceptance and cleanup remain parent-owned.

## Integrated public-doc truth finding

The integrated full gate at `1a462d036` failed lint in the public-doc checker
which still enforces current/published 0.8.26 and obsolete README/npm prose.
This is not a full pass. The new [design amendment](design.md#explicit-public-documentation-target)
plans an explicit active-state documentation target with separate truthful
publication receipts and preserved native/schema/false-publication controls.
Subsequent `gpt-6-sol` high design review approved the amendment with a P2
selected-target schema clarification, now implemented with a RED mutation.
The parent owns the final gate and reviews after this correction.

## Public-doc target correction — focused GREEN

RED `0d21213f9` rejects the old checker's forced 0.8.26-current prose. The
fixture correction `c3e08da9f` exercises the actual recorded historical npm
tag (`latest`) while retaining the old-checker RED proof; `b42009d7c` adds
RED independent published-schema and unchanged-post-publication controls.
The active 0.8.27 state now declares its documentation target, with acceptance
and `published` unchanged. The checker resolves the canonical active
state/board, validates both published and selected-target schema 34, ignores
untracked/historical overrides, rejects false pending publication claims and
retains receipt/native-platform/schema/migration guards. Historical prose
about another version cannot misattribute its unpublished status. A valid
0.8.27 publication receipt accepts these same forward docs without requiring
a later prose rewrite. Workspace path/count coverage follows the generated
contributor map; optional README count claims still must match Cargo.

All **32/32** focused public-doc controls pass, including the RED source-link
masking case `57cb5e80f`: a source URL cannot substitute for a visible
release label. Actual checkout public-doc
truth, canonical release resolution, generated release-state views, Ruff,
Bash syntax and generated-doc freshness pass. Scoped Markdown/public-doc
checks and strict MkDocs pass. Source-checked API corrections document the
Python/TypeScript default-embedder open options and Python search explain/view
arguments. Landing pages retain explicit platform boundaries. Compatibility
now links the digest/lock-first Tegra procedure without an unchecked tarball
install; the board separates prior live 0.8.26 evidence from 0.8.27 final
qualification/publication. Historical 0.8.26 notes are unchanged.

This follow-up changes the documentation checker, state metadata, test
fixtures and prose only; Rust source/manifests and test inventory remain
byte-identical to reviewed capture `f327c28ca`. That Rust surface receipt
remains scoped to its exact source. No new full-workspace pass is claimed.
The parent owns final source reviews, Terra recheck, integrated full gate,
package examples, acceptance and worktree cleanup.

Current engineering indexes and lifecycle metadata now point at 0.8.27. The
architecture preserves the true ten-member 0.8.26 historical snapshot and
adds the current eleven-member workspace/Rust SDK peer ownership explicitly.
Its existing approval/activation history remains intact. The parent obtains
a subsequent `gpt-6-sol` high design review of that documentation correction
before closeout.

The initial follow-up implementation is `9ff7326b7`; its source scan bound
the register to `9ff7326b7b431ebcee489a4d0052db279ad0ff00`, with all
39 paths/counts unchanged and no new trigger. Subsequent register receipts
change internal documentation only. Source/code and architecture addendum
reviews remain parent-owned before integration and full verification.

## Documentation-target review corrections — focused GREEN

RED `575d18a96` demonstrates the exact-version suffix bug. The unchanged
new controls pass after bounding npm version/tag tokens and extending the
explicit publication-claim detector: **38/38** tests pass, including two
suffix mutations and three PyPI/Pages/shipped-wheel assertions. Actual
checkout public-doc truth, Ruff, Bash syntax, design lifecycle and scoped
public-doc lint pass. The Tegra install guide now describes the exact
Pages distribution route and fail-closed retained-manifest verification.

The architecture addendum follows the accepted SDK ADR precisely: root
`rerank` belongs to the signed 44-operation map, `embed_batch_cls` remains
outside it, and option translations resolve existing binding differences
through the TypeScript contract. This addresses the design review's P2.
Independent review confirmation, the integrated full gate, Terra and three
final reviews remain parent-owned. Rust source and its f327 surface receipt
are unchanged; the register will be rebound to this exact checker follow-up.
