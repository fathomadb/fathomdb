---
title: FathomDB 0.8.27 Slice 140 — convergence design
status: APPROVED
target_release: 0.8.27
source_baseline: 360bbe63e65aca93cb2200f6c8e27624f4042ea8
---

# Slice 140 design

## Authority and present state

The accepted ADRs, `dev/interfaces/`, the [architecture v2](../../../../design/fathomdb-data-plane-architecture-v2.md), and the current source own behavior. The 0.8.27 master plan owns this slice's scope. Earlier versioned plans and reports remain evidence, not mutable current contracts. The production tree at `360bbe63e` has a dedicated Rust SDK, a separate lower-level Rust facade, Python and TypeScript SDKs, and one private-module engine. No new API, schema, wire, transaction, device-policy, or performance behavior is designed here.

The original five-file count was a planning signal, not a target architecture. Current first-party production files above 2,000 lines are `fathomdb-engine/src/lib.rs` (3,905), `evidence.rs` (2,999), `actuation.rs` (2,719), `data_plane_integrity.rs` (2,487), `search.rs` (2,475), `projection_registry.rs` (2,358), `open.rs` (2,085), `projection_generation.rs` (2,016), and `fathomdb-cli/src/lib.rs` (2,792). All four non-engine original outliers are now below 2,000 lines. The wider tracked-code scan also finds owned test/tool files and vendored/generated files. Recompute this list on the integrated Slice 117 source; do not transpose historical line anchors onto moved files.

## Documentation convergence

| Surface | Source of truth and edit rule |
| --- | --- |
| Crate and SDK topology | The workspace manifests and `dev/interfaces/rust-sdk.md` show `fathomdb-sdk` as the Rust peer of Python/TypeScript. `fathomdb` remains a lower-level facade; `fathomdb-engine` remains implementation. Architecture, bindings design, README, docs install/reference pages, and interface indexes must agree. |
| Engine internal ownership | Use the Slice 80/85/90 owner maps and actual modules. Keep source citations symbol-based where line numbers have moved. Label pre-move counts and paths historical rather than silently changing their measurements. |
| Device/install support | Use accepted device ADRs and installed artifact evidence. Slice 117's qualified, unpublished Tegra Node candidate is not a shipping claim. Check its closeout against README, TypeScript install, compatibility, embedder, and publication-operator docs before final edits. |
| Performance/correctness | Cite tracked Slice 135 phase reports and status. The 611-query CPU–CUDA match is limited to its x86_64 installed candidate, frozen sample, and host. Neither raw local licensed archives nor Slice 150 final release qualification follow from it. |
| Examples and references | Link checking runs from a clean release checkout. Source anchors resolve to tracked files and existing symbols. Public Rust examples use `fathomdb_sdk`; lower-level examples deliberately name `fathomdb` when exercising its distinct contract. Python and TypeScript examples import documented package paths, not build-tree internals. |

The Slice 70 carryovers are editorially distinct. The dual-runtime ADR status is changed only after checking `seq:250`/`seq:252`, shipped policy, and the user's explicit acceptance-if-consistent ruling; update decision-index row 43 together. `embedder-decision.md` is a parameter history with later amendments, not a current implementation contract; `0.8.1-slice-10-reranker-design.md` is an implemented historical memo whose opening already supersedes its original crate placement. Mark each historical and point to active authority, without ratifying every old parameter as current. The 0.8.25 Slice 40 dense-state table gains a precedence note: `failed` with absent sidecar and vec0 is `failed` even for an unenrolled edge; the unenrolled-edge corruption row applies only after earlier specific rows fail.

## Test-seam boundary

Only test mechanisms whose default-compiled presence is unnecessary are gated. The inventory is source-derived, by **current owner**, including root `Engine` methods and wrappers; `reader_pool.rs` pool witnesses; `filter.rs` conversion witnesses; `search.rs` profile/route/plan witnesses; `graph_api.rs` graph `Engine` witnesses; `graph_expand/execution.rs` helpers and carriers; `graph_expand/types.rs`; and relocated projection, vector, embed, mean, WAL, and runtime seams. The Slice 80/90 maps are input checklists, not proof that every current item is still at the named line.

Before each gate, record the declaration path, existing `cfg`, public or crate-private reachability, direct integration/unit callers, feature requirements, and whether runtime code calls the item. Distinguish three classes:

1. Already gated `test-hooks`, `slice72-test-hooks`, migration, benchmark, or `cfg(test)` mechanisms: preserve their effective reachability unless a failing test proves a defect.
2. Always-compiled `#[doc(hidden)]` public Engine/test wrappers and private witness plumbing: move to the narrowest `cfg(any(test, feature = "test-hooks"))` or a more specific existing feature. An external integration test requiring them declares the feature explicitly; `cfg(test)` alone does not apply to its library dependency. Default pinned proofs are retained through production routes or relocated to unit tests, rather than requiring `test-hooks` for their entire target.
3. Production diagnostic or configuration paths that happen to carry a `for_test` name: preserve default reachability until caller analysis proves they are strictly test-only. Do not suppress a whole module because one witness is test-only.

`execute_for_test` and the other root arbitrary-writer-SQL fixtures need special handling: `debug_assertions` currently makes some available in default dev builds. The removal objective includes **default dev dependencies** as well as release builds. The final gate must remove that default-dev route while retaining an explicit supported test route and preserving tests that exercise lifecycle, WAL, projection, and graph behavior. Isolated external-dependency compile-negative controls must name each removed public path in both default dev and release builds, paired with positive explicit-feature controls. The hidden oracle's release probe is supplementary: once a path is absent from the default rustdoc row, that probe no longer selects it and cannot alone prove removal. Method-name searches are not proof either. Document removed doc-hidden paths under the 0.8.27 `CHANGELOG.md` Removed heading and run AC-050c's removal checker.

Run the Slice 30 public comparator unchanged. The hidden oracle captures the before and after manifests, tests and release probe. Unexpected differences block the batch. For intended removals, the successor `baseline-<sha>.json` is immutable and paired with a reviewed diff record listing each changed row/path, former and new gate, direct caller disposition, and test-inventory effect. Slice 150 consumes that baseline and retires the oracle only after final qualification. Do not edit the existing baselines.

## Default proof preservation and release documentation

Keep `slice20_dense_readiness` and `slice25_registration_identity_inert` as
default targets. Slice 20's two pinned proofs use public
`open_with_choice_and_config`, a caller embedder, and governed vector
declarations. The readiness proof pauses that embedder with a condition
variable, then releases it; its raw sidecar/vec0 absence and presence checks
are unchanged. A drop guard releases the embedder before Engine teardown if
an assertion panics. Six remaining fixture cases require `test-hooks` at the
test-item level.

Slice 25 uses public no-embedder open. Its negative control runs the same
forbidden backfill through a separate real SQLite connection, preserving the
raw identity comparator. The structural writer has no production caller or
public equivalent. Move its exact named proof and raw SQL/content-id oracle
to a top-level `#[test]` function in `write.rs`; Rust compiles that function
only for unit tests. C1's evidence locator follows this default unit proof;
its obligation, category, and pinned contract hash stay unchanged. Extend the
existing evaporation mutants to the relocated function. The managed-opener
scanner excludes actual `#[test]` function spans while continuing to reject
the same unannotated opener, comment decoy, and string decoy.

The user's publication-facing ruling sets README, package READMEs, public
installation/reference/compatibility pages, navigation, and release notes to
0.8.27. Document the intended registry routes and both Tegra Pages artifacts
without transient candidate/publication-pending prose. Digest verification
requires the exact retained manifest entry and checked bytes; an absent entry
fails closed. Do not invent a digest or update retention before publication.
Internal status retains the actual qualified/unpublished state, manifest
versions, and Slice 160 publication owner. Source examples can verify API
semantics before publication; they cannot prove future registry availability.

## Large-file exception register

The register is a source-SHA-bound navigation and disposition record, not a style or human sign-off gate. Scan every tracked code file, including first-party production, tests, tools, and checked-in vendor/generated code. Exclude documentation and result data from this code-cohesion scan, not from link/source-citation checks. Record `keep` or a scoped follow-up for each first-party file, with a short reason grounded in its ownership and test shape. Record provenance for checked-in third-party/generated files rather than hand-splitting them. A completeness check may confirm that every triggering path is listed, but cannot fail merely because a file crosses a threshold. The five original outliers must each have an explained current state. No human review or approval of the register is required for Slice 140 or release closure.

## Verification boundary

- Documentation-only corrections: `scripts/agent-lint-md.sh`; `scripts/agent-lint-docs.sh` when public `docs/` changes; `scripts/check-release-state-views.sh` when state changes; clean-checkout link/source-anchor checks.
- Test-seam code: failing-first focused compile/feature tests; `cargo check`/targeted Rust tests for default, explicit `test-hooks`, and affected other feature sets; comparator/hidden/test-inventory and AC-050c checks; then the required full `scripts/agent-verify.sh` candidate gate. A public import example must use the package's actual documented install/build route.
- Material source change: independent `gpt-6-sol` high code review. The first design review uses `gpt-6.1-sol` high; later design reviews use `gpt-6-sol` high. Independent Terra verification checks the final acceptance matrix. Record any unavailable route precisely, without calling a scoped pass a full release pass.

## Source-derived library and contributor documentation

This amendment received the required subsequent `gpt-6-sol` high approval
at `2c1b12a0c`; see [design-review.md](design-review.md).

A stdlib Python generator under `scripts/` produces two tracked Markdown
pages. It resolves inputs relative to the repository root and supports
explicit `--write` and read-only `--check`. Output is deterministic,
version-neutral and free of timestamps or checkout-specific absolute paths.
Malformed or missing source inputs fail with an actionable diagnostic;
`--check` reports stale/missing pages and exits nonzero without modifying
files. Generated pages identify their source and regeneration command.

- `docs/generated/sdk-operation-map.md` reads
  `src/conformance/governed-operation-parity.json`. It includes each governed
  operation exactly once with its ID/state and the Python, TypeScript and
  Rust locator/spelling pairs. All 44 current operations are covered. It is
  explicitly an operation index: signatures, options, return shapes and
  error contracts remain in the linked maintained API reference pages.
- `docs/generated/contributor-map.md` reads root `Cargo.toml` workspace
  membership and member manifests, Python package metadata from
  `src/python/pyproject.toml`, TypeScript package metadata from
  `src/ts/package.json`, and the actual
  `scripts/agent-{build,lint,typecheck,test,verify}.sh` entry points. It
  enumerates Rust workspace packages and Python/TypeScript package names
  with their manifest paths, plus the typed verbs with source
  links. Version fields are omitted so Slice 160 version bumps do not
  contradict 0.8.27 publication-facing prose. It does not execute scripts
  or infer undocumented behavior from arbitrary shell text.

A short maintained contributor landing page explains TDD, worktree
isolation, scoped documentation checks, full source/script verification,
release-state ownership and regeneration. README and MkDocs navigation link
the landing and generated indexes. Public release documents continue to
present 0.8.27; actual prepublication facts stay in internal status. No
registry versions, retained artifacts, or publication states change here.

`scripts/release/build-pages-site.sh` invokes generator `--check` before
MkDocs. Both `docs-pages.yml` and the Jetson workflow already call this shared
builder; each complete-site artifact therefore includes the generated
pages and rejects stale output. docs-pages push filters include the
generator, parity manifest, root/member Cargo manifests, Python/TypeScript
package manifests and typed script input families. Existing retained Tegra artifact/digest gates are preserved.

Generated freshness within one checkout does not establish publication
currency. Both complete-site Pages prepare jobs must fetch current
`origin/main` with full enough history and require
`git merge-base --is-ancestor origin/main "$GITHUB_SHA"` before building or
uploading the site. The guard uses the exact dispatched candidate commit,
not an inferred current branch tip. A release branch that contains main and
is ahead passes; an old main commit or release branch behind/diverged from
current main fails. If main advances, incorporate it into the release branch
and re-dispatch the new exact candidate. A small shared helper implements
the guard with explicit fetch/ref/history diagnostics. Failed fetches or
missing ancestry fail closed.

Repeat the same fresh-main check immediately before each deploy action,
inside the existing shared Pages concurrency boundary, so a site built
before a newer main push cannot deploy from a stale queued source. Use
read-only anonymous fetches of this public repository for the deploy guard;
do not add publication credentials, checkout persistence, or contents-write
permissions. Existing deploy jobs need no contents permission. This is a
point-in-time currency check, not an atomic lock against future main pushes.
The next serialized deployment repeats the check on its own candidate.

Focused local Git-history tests prove equal/ahead accepted, behind/diverged
rejected, and missing history/fetch failures rejected. Pipeline contract
tests prove the guards run in both prepare jobs before the builder/upload
and immediately before both deploy actions, preserve exact candidate SHA
and least-privilege permissions, and retain the shared Pages concurrency.

TDD tests exercise deterministic generation and exact operation mapping,
source edits changing output, stale/missing output rejection without writes,
workspace/member/Python/TypeScript package-name/script input coverage, navigation, and the shared-builder
freshness gate. Focused Pages controls prove both callers retain the shared
builder. Run Markdown/public-doc and workflow checks, then the integrated
full gate because executable scripts change. Subsequent source review, Terra
verification and three final independent reviews cover the complete
candidate. Re-scan the large-file register without a human approval gate.

### Contributor package coverage amendment

The contributor map includes a small Python and TypeScript package section
from each package's own manifest (`project.name` in Python TOML and `name`
in TypeScript JSON). Display the language, name and manifest path; omit
version fields. The exact manifest inputs join docs-pages trigger coverage.
RED freshness tests mutate each package name independently and verify the
corresponding output changes without package-version leakage. The
contributor landing points readers to this three-language source map. This
coverage amendment received subsequent `gpt-6-sol` high approval at
`78aced66d` before implementation.

## Explicit public-documentation target

A publication-facing release branch must be able to describe its intended
release before registries/Pages publish it. Add the optional state field
`public_docs_target_release: "0.8.27"` to the active 0.8.27 canonical release
state. It must equal that state's own release; it is a documentation target,
not acceptance or a publication receipt. `published` and all package/retained
artifact metadata remain unchanged.

The public-doc checker first validates every tracked publication record as
today, then resolves the active tracked state/board through
`scripts/release-current.py`. Only that active state's explicit marker can
override the newest valid published release. Absent a marker (including
when all releases are historical), use the newest published record. Reject
malformed/foreign markers and failed/ambiguous canonical resolution;
untracked scratch states and historical markers cannot override the target.
After publication, the existing valid receipt naturally supplies that same
version without reinterpreting the marker as publication evidence.

Public docs must identify the target release and its exact intended install
route (the existing published fallback may use its recorded npm dist-tag).
A pending target must not explicitly call that version already published;
generic artifact/platform descriptions and dated historical release notes
remain valid. Keep the checks rejecting an unpublished claim for a genuinely
published version, malformed publication-complete lifecycle/receipts, missing
native platform boundaries and schema-34/fresh-database constraints. Preserve
the five native-artifact manifest assertions. Replace the obsolete mandatory
README crate-count bullet with exact workspace manifest-path/count coverage
in the generated contributor map, while rejecting false count claims if a
README states one. No guard is dropped merely because prose was redesigned.

RED fixtures construct tracked canonical state/board pairs and cover pending
0.8.27 marker, 0.8.26 published fallback/historical notes, target mismatch,
untracked override, ambiguous active state, missing/wrong install/version
guidance, false pending publication assertions and preserved receipt/platform/
schema/count mutations. GREEN changes the checker/state only after review.
Focused API doc fixes source-check Python/TypeScript default-embedder options
and Python `search` explain/view parameters. Compatibility links the safe
Tegra digest/lock-first install procedure instead of an unchecked bare URL
command; the board distinguishes historical 0.8.26 evidence from final
0.8.27 release qualification routes. Subsequent source review and parent-owned
full verification remain required. These script-only changes do not alter
Rust API/test inventory; the existing exact f327 Rust capture remains the
reviewed surface receipt with this follow-up scope stated explicitly.
