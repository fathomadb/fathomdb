# Slice 7 delivery design proposal

## Existing architecture and bounded delta

Retain prerequisite checks → scoped feedback → final stable-tree source-phase
full verification → affected package rehearsal → separately authorized publish
→ registry-installed lifecycle proof. This slice implements none of those
actions. The [rulings](../slice-9/decision-log.md) already approve owned setup
and narrow .29 route scope;10 reviews/implements the concrete remedies below.

### Owned linked-worktree setup

[Bootstrap](../../../../../scripts/bootstrap.sh),
[heavy bootstrap](../../../../../scripts/bootstrap-heavy.sh) and
[build](../../../../../scripts/agent-build.sh) still invoke editable Python
installation. Build also falls back to system Python and trusts a timestamp
sentinel keyed only to pyproject freshness. Those mechanisms cannot establish
the required source/features/interpreter/native identity for a linked checkout.
The full [verifier](../../../../../scripts/agent-verify.sh) already rejects
missing checkout Python/pip/pinned tools before lint; preserve that behavior.

10 should provide one supported owned noneditable wheel/native setup path and
make affected linked-worktree entry points either use it or fail early with an
actionable pointer. Never silently use editable/system/shared setup. Update
bootstrap-heavy's existing editable-specific fixture to the reviewed new human
intent during its RED step, rather than weakening the old fixture after a fix.
Resolve instructions with0/6; no manual native-library copy or borrowed venv.
Verify resolved executable/import/module paths, source SHA plus dirty delta,
locks, features, target/ABI/interpreter and wheel/native digest. Reuse installed
native bytes only when that identity matches; pyproject timestamps alone are
insufficient. Keep existing tool pins and root/TS locked npm installation.
No generic bootstrap redesign or unselected dependency upgrade is required.

### Exact .29 first-tag route

[Tag guard](../../../../../scripts/release/check-tag-cuda-route.sh) rejects every
nonempty bootstrap marker except matching `v0.8.28`. Normal routing already
checks exact tag workflow in group3 and exact tag deployment policy. The narrow
Batch2 proposal is to allow the exact `v0.8.29` bootstrap through the same
strict conditions, retain .28 behavior, and reject arbitrary future tags or
marker/tag mismatch. Keep hooks active and align release-design guidance/tests.

Before any eventual tag action, authorized actor verifies exact remote tag
absence using failed HTTP404 with valid NotFound JSON, restricted selected
`fathomdb-gpu-release` group3 permitting trusted `release.yml@refs/heads/main`,
exact .29 environment tag policy and actual nonempty valid required reviewers.
No successful API call carrying a404-looking body,403, malformed response or
network error proves absence. Bootstrap readiness means **GPU approval HELD**,
never normal route readiness. Existing policies/workflow selections are preserved.

After separately authorized tag push, verify remote peeled commit equals the
approved candidate, let authorized actor add only the exact tag workflow ref,
then run marker-free normal guard successfully before GPU approval. Missing API
privilege/policy/ref leaves the affected route blocked. Code scope approval is
not permission to mutate remote policy, tag or publish. No release workflow
eligibility/permissions/environment bypass or general future-tag exemption.

## Resources and latency discipline

Keep the existing10GiB plain-preflight floor. It is neither a build-peak budget
nor strict state/dependency eligibility.10 first activates valid state/views in
the assigned existing checkout, then uses strict dependent preflight; no invented
closure or source worktree eligibility from plain health. Check real exit/JSON;
do not mask it through a tail pipeline.

Before an expensive phase,10 records numeric free/reserved/stop bytes on every
filesystem used by target, wheel/native staging, npm/pip cache, TMPDIR, spill
logs, upload compression and post-job cache archive; also memory availability,
measured peak, concurrency/Cargo jobs and competing users. Phase thresholds must
come from actual relevant needs/measurements and approval for owned paths, not
the dated16GiB/914GiB inventory. Start with one native build/full gate at once;
do not infer safe Cargo jobs from24 CPUs. Missing numeric reservation or
insufficient headroom stops that phase. Existing standing install is preserved.

The current CI already caches pip/npm, uses separate heavy bootstrap, removes
one-off Ripgrep installation, exposes BGE warm failure as skipped live coverage,
serializes default-embedder Node files and bounds the advisory parallel report
at jobs2/debug0/incremental0 without target-cache restore/save. Retain it.
Read current per-suite timings before proposing any later timeout/scheduling
change.45/60-minute fast/heavy job limits are source configuration, not measured
current durations. No speedup or universal30-second preflight promise is made.

CI selectors distinguish verify/rust/security/native harnesses and Windows-only
controls; `ci.yml` itself deliberately selects all eight scoped heavy jobs.
Retain that conservative self-change behavior absent reviewed evidence for a
specific refinement. Manual exact-candidate qualification requests full coverage;
docs-only or trusted `[ci-lite]` automatic selection cannot waive it. No new
required-status checks, retry-to-green tests or broad rerun campaign.

## Evidence, packaging and scanning

Full/default verifier preflights then lint→typecheck→security→test; heavy skips
the first three and cannot alone establish full verification. Markdown scope
runs only its authoritative gate and states broad skips. Preserve AC-036 strict
ptrace behavior and existing AC-037 live-layer venue distinction. Final source/
test/executable phase runs required full verification once on stable owned
inputs; scoped checks while editing, then failed/invalidated suites only unless
broad changes justify another full run. No edits during a gate.

Evidence reuse requires an explicit comparison of relevant source, dirty delta,
features, locks/tools, platform/ABI, workflow/config, artifact digest and data/
model inputs. A change invalidates the affected proof; docs-only unchanged
product inputs do not require rebuilding every native artifact. A filename,
source SHA alone, runner label or prior OIDC token is insufficient. Record actual
command/log/exits and registered/ran/pass/fail/skip/excluded counts. Retain .28's
failed/assembled results; package build, installed local candidate, trusted
workflow, authentication and installed registry consumer are separate proofs.

Use W set-version/check-files and independent E drift guard; E0.8.0 is not bumped
by inference. Local rehearsal builds workspace and packages/dry-runs three leaves;
dependent sibling registry resolution still waits for real prior tiers. Existing
SDK OIDC/promotion/self dev-dependency recovery requires preservation, not repair.
Registry query uncertainty fails closed; exact-version API visibility is weaker
than Cargo index resolution, whose narrow propagation retry already exists.
No token capture or package upload belongs to7/10 proposal checks.160 retains
affected native package provenance and registry install/open/write/search/close/
exit proof after separately authorized publication.

Retain staged/current-tree scanner enforcement and their real nonzero exits.
CI current-tree job is report-only; history is dispatchable and independent
release advisory, not a publisher prerequisite. History runs current scan first,
isolated mirror, owned empty ignore, config neutralization, redacted safe tuples
and exact multiset classifier. Scanner/setup/report failure is not a credential
finding or clean scan; new comparable tuples are unclassified until corroborated.
Keep known nonsecret classification and actual nonzero results without broad
allowlist, manifest refresh, history rewrite or raw-secret retention. The .28
171-versus100 history observation and bounded71 corroborations do not establish
cross-run tuple equality or whole-history cleanliness. No rescan campaign is
required to complete this proposal.
