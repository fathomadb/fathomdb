# Slice30 G/D executable test and TDD proposal

Status: DESIGN ONLY at349e9fa07. No listed test was authored/executed yet.
AGENTS strict per-behavior runtime RED→committed RED→separate GREEN applies.
Existing passing product tests are controls, not new coverage labelled RED.
No missing import/feature/module/compile or fixture-construction failure is RED.

## Entry and scaffolding

The first observer RED has a fixed diagnostic selector, not a requirement applied
to normal P. After GO, add only a compile-ready engine example
examples/slice30_gd_diagnostic.rs and empty non-default slice30-measurement marker
in engine Cargo.toml (plus its explicit example/required-feature declaration).
No observer type/method/implementation exists yet. The wrapper invokes existing349
Engine open/write/direct-text/close APIs on2literal rows and emits real successful
API samples. Missing observer.jsonl is read as0records, not a fixture exception.

The unchanged external runtime test builds exactly:

```bash
cargo build --locked --release -p fathomdb-engine \
  --example slice30_gd_diagnostic --features slice30-measurement
```

It then invokes that exact binary with the same cell.json/outdir argv at RED and
GREEN: family P1, n2, adapters engine, warmup0, samples1; corpus alphaonly/betaonly,
query alphaonly. Assert existing real sample ID l:s30doc-0 first, then require
one QueryComplete record: **observed0 versus expected1**. Commit this runtime RED
before the first GREEN adds only bounded per-engine completion transport and
wires the same diagnostic wrapper. No test/artifact selector/feature/argument/
configuration switch, no missing-method compile failure and no intentional
product defect. Normal P/GN remain separate untraced artifacts with no observer
output. The new feature marker is compile scaffolding, not an engine API.
Record the marker/example/interface-scaffold additions and exact source/artifact
identities in that RED commit; the eventual hidden API/interface note lands with
its GREEN. Test bytes remain unchanged. Subsequent units reuse the introduced
observer backbone; no big-bang implementation of every event.

| Unit/order | Independent literal runtime oracle and expected RED | Separate minimal GREEN |
| --- | --- | --- |
| TGD-1 call collection | Fixed compile-ready diagnostic command above succeeds through real349 APIs; one required QueryComplete is missing0vs1. IDs/ownership are part of this initial collection oracle, not future passing-on-arrival coverage. | Bounded per-engine collector/call transport/completion only; normal P stays untraced. |
| TGD-1b snapshot stamp | Existing completion lacks same-reader generation/effective time. Tiny literal fixed-view DB compares with independent literal SQL while quiescent; missing/null stamp genuinely fails. | Same-reader snapshot/time stamping only; no adjacent-state substitution. |
| TGD-2 dependency-presence span | Ordinary node fixture versus governed canonical+derived dependency fixture from slice20_source_dependencies.rs APIs. Expect called/false versus called/true, exact query correlation; missing span fails. | Instrument this existing predicate only; preserve outputs. |
| TGD-3 fallback span/guard | Literal tiny normalized fixture provider, registered vector work; vectorless direct text skips, no-dependency vector call returnsfalse without dependency SQL, invalidated governed source invokes fallbacktrue. Expected cases are fixture intent, not helper-computed expected predicates. | Actual fallback call/SQL-branch observation only. |
| TGD-4 all-nodes span/guard | Drained active no-dependency hybrid executes once; direct text/filter/dependency guards skip; future/expired node changes literal eligibility at fixed valid_as_of. Missing/wrong called/result/null fails. | Instrument original guarded call without moving/eagerly evaluating it. |
| TGD-5 vector block | Known2-vector real fixture, hybrid enters actual vector block; direct text marksnot_run. New block record absent at RED. Literal result-ID set is independent. | Vector block span only. |
| TGD-6 text block | Alpha/beta FTSliterals, hybrid/direct text enter actual text block with correct call identity; new record absent at RED. | Text block span only. |
| TGD-7 drops/error report | Existing backbone remains bounded, but new missing dropped-record/error-disposition output must fail a small-capacity literal oracle. Errors use an ordinary invalid filter, not SQLite/hardware fault injection. Engine isolation is an initial-unit/reuse control and is not separately claimed RED if already correct. | Explicit drops and typed error/unavailable-snapshot reporting in separate failing units; preserve original error semantics. |
| TGD-8 G mean populations | Manual tiny pinned-BGE GPU fixture: normal core/SDK hybrid returns literal two-ID set at limit2, direct text exact alpha ID, embed mean dimension384 finite/nonzero; new G route/sample output initially absent, while real API calls/model engagement succeed. No learned-vector golden or exact floating byte oracle. | Add one mode at a time with separate observed failures/GREEN, reusing P text observation. Distinguish Engine::embed from standalone CLS. |
| TGD-9 closed templates/dense completeness | Real tiny test-provider store: own tool must declare proper vector projection, drain, verify per-kind cursor coverage/spans and close. Independent literal SQL expects2natural records/2vectors and no pending work; new template command initially omits required preparation/count output. | Template creation/closed-copy command; no live DB copy, no terminal-only proof. |
| TGD-10 G write/control schedule | Tiny real-store10marker fixture; read-only control has0scheduled writes, b10/s10 for20natural calls has2governed calls/20rows/20marker probes, current literal revisions and separate drain/proof times. Mode resets are outside timed calls and recorded. New schedule output fails actual counts/order. | One schedule behavior at a time; no new product batch API or ancillary-commit claim. |
| TGD-11 worker intervals | Real2-worker tiny projection fixture. Bounded test-only handshake can make lifecycle observation deterministic; expect actual worker IDs/batch starts/finishes and row counts, not active_jobs treated as workers. New observer output missing at RED. | Worker batch lifecycle observation only; no timed workload delays. |
| TGD-12 held/drain adapter | Existing scheduler freeze after empty configuration, literal3rows: new D output must report pending3/vector0 before release and pending0/vector3 after actual drain. Missing phase/count output fails while product hooks remain correct. | D held adapter records real state/times; do not retest20 A5 as new product coverage. |
| TGD-13 real-overlap attribution | Real controlled tiny fixture in test only proves query interval intersects actual worker AND governed writer intervals with positive pending work; assert exact pending/worker/query interval relation in new report. Missing correlation fails. Campaign uses ordinary generator/provider with no artificial sleeps/holds. | New overlap attribution, with explicit incomplete outcome when a real campaign observes no overlap. |
| TGD-14 feasibility/count maths | Literal disjoint retained-P+both-checkout+new-setup byte sum/time table: preregistered1.5×projection declines300k when either900s or16GiB would be exceeded; explicit NOT_RUN_RESOURCE rather than smaller substitute. Existing metrics distribution/bootstrap reused. | Lower-scale eligibility/report logic only. No new release gate. |
| TGD-15 paired observer grouping | Actual tiny children/cells with explicit normal/feature-off/feature-on identities and known query IDs. Require pair completeness, preserved errors/unpaired rows and hand-calculated raw-call differences. New grouping absence fails. | Pairing/report adaptation only; no percentile subtraction. |

If any proposed missing behavior already works through a suitable existing tool,
reuse it and remove that proposed new unit; do not manufacture RED. If strict
policy leaves a genuinely required already-correct new product proof, stop only
that addition and obtain the actual owner disposition. No20 exception is assumed.

Projection/count framing additions require a bounded property invariant alongside
literal tests before the fix: generated small write cohorts preserve exact total
rows, batch order and completed-record prefix. Reuse the accepted P UTF8 prefix
property/decoder unchanged; no passing-on-arrival library mirror or broad fuzzing.
No tamper/digest-drift/path-leak/receipt-hardening tests or regenerated goldens.

## Maintained selection and qualification

Proposed source files are engine measurement_for_test.rs and tests under the
engine integration-test inventory, new SDK examples/slice30_gd_measure.rs for normal G, engine
examples/slice30_gd_diagnostic.rs for core diagnostics/D, and
G/D tool tests/controller/cells beside the existing indexed P tools. Every new
suite/example assertion must be selected explicitly by maintained runner/target
inventory. Normal `cargo test` example compilation alone does not select tests.

Engine observer tests use test-hooks,slice30-measurement so deterministic fake
providers remain contained. Normal G compile/native controls use embed-cuda only;
G diagnostic controls add slice30-measurement without test-hooks. D fixture
controls use test-hooks; diagnostic D additionally enables slice30-measurement.
No SDK public test accessor or default feature forwarding. Stage compile-ready
fit first; preserve setup/compiler failures as such, never runtime RED.

Tiny real-BGE G tests require approved exclusive3090 and actual engagement, are
manual/AGENT_LONG controlled and explicitly registered; ordinary full verification
records their manual exclusion honestly. Never run a full model suite on CPU.
Runtime source GO includes their per-mode real runtime RED/GREEN and own release
CLI proof before timings. Unit deterministic providers do not claim BGE fidelity.

All rendezvous use bounded readiness and worker waits, idempotent explicit
release and release-on-drop before thread joins, per verified memory
0.8.25-slice55-trace-integrity-lessons.md. No unbounded Barrier pair while holding
WAL. Add a separate genuine tool-guard RED for early-return cleanup before its
GREEN; do not let a held-fixture count GREEN make later new cleanup tests pass
on arrival. Stop only owned child handles/groups and always wait for them.

Run scoped lint/type/affected tests throughout. Commit/regenerate final inventories,
freeze source, run required full agent-verify once, retain inner skips/ignores and
failed scopes, resume only affected failures. One final fresh independent Astra
review evaluates actual code/TDD/native/source/measurement subjects; parent owns
integration and release-state changes. G/D cannot discharge M/F/20/Block1.
