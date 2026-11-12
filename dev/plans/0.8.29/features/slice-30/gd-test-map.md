# Slice30 G/D executable test and TDD proposal

Current execution: reviewed design PASS and parent source GO at5968884b.
Bounded source TDD is underway; see [actual execution](execution-gd.md).
Historical design-stage statements below describe the349/b83 review subject,
not current runtime authorization or completed qualification. No G/D natural
campaign or whole-slice acceptance is claimed.

Status: DESIGN ONLY at349e9fa07. No listed test was authored/executed yet.
AGENTS strict per-behavior runtime RED→committed RED→separate GREEN applies.
Existing passing product tests are controls, not new coverage labelled RED.
No missing import/feature/module/compile or fixture-construction failure is RED.

## Entry and scaffolding

The first observer RED has a fixed diagnostic selector, not a requirement applied
to normal P. After GO, add only a compile-ready engine example
examples/slice30_gd_diagnostic.rs and empty non-default slice30-measurement marker
in engine Cargo.toml (plus its explicit example declaration; do not require slice30-measurement for
the shared DN/DO wrapper, and cfg-gate observer API references).
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
| TGD-1b snapshot stamp | Stage BOTH quiescent and bounded post-pin/concurrent-commit cases below before any stamp GREEN. Existing completion has no stamp; missing metadata fails. Wrong adjacent post-commit generation also fails the unchanged race oracle. | Read generation on the exact reader transaction after the existing post-validation hook, retaining its literal old-row/effective-time result. |
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
| TGD-13 real-overlap attribution | Stage the strict common-intersection/pending-witness literal cases below and real controlled tiny fixture together before overlap GREEN. Missing correlation or broad pairwise classification fails. Campaign uses ordinary provider/work with no artificial sleeps/holds. | Common max-start/min-end intersection plus wholly contained positive pending witness; absent actual overlap remains incomplete. |
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

## Initial snapshot-stamp race oracle and source fit

TGD-1b must stage this test before its first stamp GREEN, alongside the
quiescent literal case. Do not add an already-passing race test after implementing
the stamp. The existing product snapshot behaviour is a reuse control; the new
required observer's exact stamp is the behavior under test.

Use test-hooks plus slice30-measurement in the contained test artifact. Existing
arm_frozen_after_validation_hook_for_test accepts an Engine-owned one-shot
callback. In search.rs read_search_in_tx it fires AFTER begin_attributed_reader_tx
and frozen_read::validate_snapshot; binding_for_snapshot has already performed
real reads on that SQLite transaction, so it is pinned. The callback shape and
pinned-read reuse are present in slice35_after_validation_races.rs. Do not copy
that historical test's unbounded Barrier pair.

Write literal alpha(oldonly, valid_from90) and beta(valid_from200), mint a
ReadContext with valid_as_of100, and independently read old generation g0 with
literal read-only SQLite SQL. Arm the existing post-validation hook with bounded
mpsc ready/release channels: ready send succeeds before waiting, release wait
<=15s, disconnection cancels. A local release-on-drop sender always releases
before scoped join on success/assertion/error; reader-side wait is bounded. No
new engine pause, product knob or fault venue is necessary.

Start search_frozen(oldonly). After ready proves that SAME reader transaction
has validated/pinned g0, commit a normal governed alpha update to newonly on the
writer connection. A fresh mode=ro (not immutable) connection must see newonly
and generation g1>g0. While paused there must be no emitted SnapshotStamp yet.
Release/resume; place the new diagnostic stamp read after the existing hook and
use the live reader tx, not Engine writer/atomic/public adjacent state. Require
observer generation g0 (not g1), effective_at100 and literal returned alpha body
oldonly, while the independent adjacent reader sees g1/newonly. Thus a wrong
post-commit adjacent lookup is rejected despite the already-correct row result.
No exact guessed generation increment or regenerated snapshot/golden is used.

## Triple-overlap and pending-witness literal oracles

Use one shared monotonic clock origin for every interval in an attempt. Let
L=max(query_start,writer_start,worker_start), U=min(query_end,writer_end,worker_end).
Require L<U. Pending witness [p_start,p_end] records the full invocation interval
of existing Engine::read_embedding_readiness on the independent100ms-cadence
tool observer thread (same in all D controls), immediate timestamp before call
and immediately after its successful return, plus actual pending_count. Require
L<p_start<=p_end<U AND pending_count>0. All SQL reads producing that result lie
inside the bounded invocation interval; positivity is an existential pending
witness inside the common interval, not a constant pending count throughout it.
Failed/unavailable/cached-adjacent or out-of-interval witnesses do not qualify.

Stage all literals before TGD-13 GREEN: q[0,10],w[1,2],j[8,9] fails (disjoint
pairwise intersections); q[0,10],w[0,5],j[5,9] fails (boundary contact);
q[0,10],w[2,8],j[3,7] with positive p[1,2] fails (outside common interval), and
p[2,4] fails (straddles boundary). The last triple with p[4,5],count0 fails;
p[4,5],count1 succeeds. Values are literal clock units, not implementation-
generated expected overlaps. The real tiny fixture independently checks the
same rule against retained writer/worker/query/readiness records. No actual
qualifying witness yields INCOMPLETE_OVERLAP, never a false overlap report.

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

## Direct-text per-mode implementation fit (review before source entry)

Reuse only the accepted d6756348aa579222f8722cb6dd73fbd0da1ad1a2 core-hybrid
source fixture, not a campaign template. Exact producer proof root:
/mnt/data/tmp2/fathomdb-0829-continuation/slice30-gd/proofs/g-mean/1794488732288403793.
Its identity records actual ac48ec2e9 plus the emit-only GREEN delta and binary
a19cf266d4b0c76799f9be00a5490578af03c2bf8a8b678ee8a9e30ddb96a5dc;
producer exits0 after Engine::close. The local read-only verifier in
GAdapter::prepare_hybrid is dropped on return before the query loop and close.
All producer owners are released; no binding lifecycle qualification is inferred.

Source out/store.sqlite is696320B, SHA256
f24e6f6b4f87d661fcdcda763819a03db66be8b7ebe5bac6cd9330b21cbbbdf4.
Read-only filesystem inventory found WAL absent. Before EACH test, require this
same hash and WAL absent/empty, then copy only the closed main DB into a fresh
writable owned output directory; verify the copied initial hash BEFORE opening.
Never open/mutate the source fixture or copy live SQLite state. Independent
read-only checks on the COPY require2active doc rows,2physical vectors,pending0,
alpha/beta bodies "alphaonly evidence"/"betaonly evidence" and raw FTS alphaonly
matches1; release that verifier before API timing. Core
Engine::search_text_only_filtered_with_snippets and SDK Engine::search_text_only
use no filter/default live view/limit10 and return l:alpha/returned_hits1. Raw
match counts stay distinct; the original source hash remains unchanged.

Keep normal GN embed-cuda artifact, Default core open / SDK
use_default_embedder:true, forced approved UUID/cuda:0 and actual OpenReport
identity. Text-only APIs request no query embedding; no positive GPU utilization
requirement or query-GPU-work claim applies. Model open may use CUDA; this is
not a claim of zero process GPU activity. Reserve/recheck the same GPU and
preserve required device/memory accounting and the8GiB cap. ONLY positive query
utilization evidence is inapplicable: failed/unparseable required identity or
memory accounting keeps the existing refusal/stop behavior, is NOT_RED and
unqualified, and never counts as8GiB-cap compliance. Healthy no-inference text
calls need no positive utilization. Mean/hybrid GPU oracles remain unchanged.

Manual text TDD fixtures alone use fixed text_fixture_handshake=true at RED
and GREEN: after Default open/copied-count verifier release and BEFORE API/loop
clocks, stdout READY <own_pid> <selected_uuid> followed by stdin ACK. Readiness
and ACK each have<=15s deadlines, including required accounting work. Controller
validates actual own PID/exact UUID memory and8GiB cap before release. EOF,
protocol/accounting error or timeout cancels with bounded stop-and-wait of only
its child; no unbounded read/join/cleanup. No inference padding or new engine/SDK
API, mean/hybrid wait or natural-campaign wait. This is fixture scaffolding for
the text-output oracle, not a separately claimed new passing coverage unit.

Implement engine_text and sdk_text as SEPARATE runtime RED/GREEN units using
existing text-only APIs. Compile-ready successful128-call/literal-ID proof must
precede the intended missing0vs128sample assertion. Hash/config/copy/WAL/device/
parse/API failures are NOT_RED. Freeze each oracle through its minimal GREEN;
retain16-record checkpoints and maintained manual selectors. No new API,
accessor, capability, policy exception or TGD9 closed-template completion.

## Remaining TGD-9 template subunits (bounded fit amendment)

Historical fit at6620b4c1d: its producer then emitted aggregate metadata rather
than the general completeness report; GAdapter::prepare_hybrid was only a tiny
two-row fixture. The following TGD9b/9c obligations were subsequently implemented
and accepted atc8015ee6/7664a255; retain their original oracles below. Accepted GN
host/G1 source prefixes now reach8e5823. They do not qualify generic G2/GO/GG or
remaining controller/budget/D work and final campaigns.

1. **TGD9b actual completeness report.** Introduce a private example/tool helper,
   shared by D and later normal G, using a read-only real SQLite connection.
   Join active canonical(kind,write_cursor) to mapping(kind,write_cursor),
   physical vector rowid=mapping.rowid, and span(vector_rowid=mapping.rowid,
   write_cursor=mapping.write_cursor). Do not assume write_cursor equals rowid.
   Per kind, require exact cursor-set coverage and cardinality at each join;
   reject missing, duplicate or extra mappings, physical rows and spans. Check
   physical f32 byte dimension, window0/start0, UTF8 body_length, actual nullable
   byte_end bounds and stored profile name/revision/dimension. Report per-kind counts,
   cursor coverage and known/unknown span ends, not only a total or terminal flag.
   Pending0 is the separate successful engine readiness observation after drain.
   Initial literal plus bounded real-store property oracles reuse D_TEMPLATE,
   independently verify its already-correct rows/copy/reopen controls, and then
   fail explicitly on absent new completeness-report fields. Missing files are
   handled as absent report values; compiler/parser/setup failures are NOT_RED.
   GREEN wires the helper into D's existing closed producer. No corrupt-store,
   tamper or deliberately broken product fixture is needed.
2. **TGD9c normal BGE closed producer.** Add a private G_TEMPLATE command branch
   to the existing normal SDK example, built with embed-cuda only. Use current
   Core Engine Default/public ProjectionSpec/write/drain/close APIs and its
   actual OpenReport.default_embedder identity, then the shared verifier. No
   test-hooks, slice30-measurement, SDK feature forwarding or public accessor in
   this producer. Input is owned ordered JSONL id/kind/body records; governed
   preparation uses fixed cohorts64 (no production knob), preserving order and
   exact completed prefix. Initial manual oracle includes a65-row doc/note
   boundary case (two governed calls) and bounded small Unicode cohorts. Stage
   literal AND property assertions before the first report GREEN. Compile-ready
   scaffold executes real preparation/close successfully; independent actual
   cursor/vector/span/profile SQL and fresh closed-copy/reopen controls must pass
   before explicit missing normal-template report RED. GREEN adds the report,
   not the already-correct product semantics. Existing D/control oracles retain
   their assertions and prior accepted meaning.

The general helper is called by the producers themselves before any template
is eligible for later retrieval; passing integration assertions alone is not
wiring. Independent verification may use a contained Rust test artifact with
existing engine proptest/rusqlite dev dependencies to inspect physical vec0;
its identity is distinct from the normal GN producer. Initialize SQLite through
the existing None open before independent SQLite use, without loading a model
in that verifier. Keep the normal producer's actual Cargo feature closure in
its proof; never infer normal identity from an engine test/example artifact.

D's current varying/perf-gates-dense768 recipe retains unknown byte_end=NULL.
Pinned BGE384 CandleBgeEmbedder::truncation_offset returns Some(end) on every
successful path. Require nonNULL UTF8-aligned ends in window0: short input
end==body UTF8 byte length; deliberately overlength Unicode input
end<body byte length. Record the actual end without a learned-vector or exact
token-offset golden. D NULL ends remain valid unknown coverage. Never infer
full-body coverage merely from cursor completeness. Use finite/dimension/profile
and literal ID/count invariants, not standalone CLS; small property cases
preserve count/order/span bounds and successful BGE nonNULL UTF8-aligned ends.

## TGD10 G actuation and phase framing amendment

This is new G scheduling actuation. Reuse P's Recorder/decoder and marker-format
concepts, not its scheduling loop: P writes before natural groups and does not
drain. G first completes10natural calls, then one atomic10-row governed write,
drain and10literal current-revision note probes; repeat after calls20, including
the final cohort before pass close. Reset10markers/drain remains preparation,
with0scheduled writes in the unchanged read-only control.

Before GREEN, stage literal20-call and bounded real10/20/30-call property
oracles. Compile-ready write selection initially only resets and makes successful
natural calls; missing ACTUAL work must fail0write events versus2 (for20calls),
not just absent metadata after implementing work. Existing natural IDs/doc
filter/order, reset, CUDA identity/accounting controls pass first. Fixed oracles
then require20written rows/20literal probes, phase timestamps/order/current
revisions and exact ACTIVE canonical(kind,cursor)→mapping/physical/span coverage
2doc+10note after every drain. Retained historical physical rows are separate:
write_commit intentionally retains superseded vector shadows; no purge or total
physical==12 assumption. Preserve the fresh-template no-extras verifier.

G write batches.jsonl has EXACTLY3phase events per completed cohort, in order:
`governed_write`, `projection_drained`, `active_coverage`. For20natural calls,
6phase records mean2actual governed calls/20rows,2drains,20separate marker-probe
records and2coverage observations; never6writes or auxiliary SQLite commits.
Each phase/probe records actual cohort and after_natural_calls10/20. One
pass-local monotonic origin timestamps natural record preparation and write/
drain/probe/coverage intervals. Natural record intervals include API plus
validation/formatting, not pure API latency. Existing API elapsed_ns still stops
before output work. Require natural group end before write start, then drain,
10ordered probes, coverage, then the next natural group. Literal final marker
bodies and independent read-only current mapping/span SQL reject false counts.

Initial literal/property assertions include every real completed event-prefix's
phase order and write/drain/coverage counts (coverage<=drain<=write), probe group
order and completed-prefix versus complete-pass distinction. Reuse unchanged P
strict UTF8-prefix/Recorder controls; no new fault/tamper/corrupt-store venue or
product hook. These assertions belong to the initial actuation unit, not a
separately claimed passing-on-arrival product proof. New event/timestamp fields
apply to G WRITE populations only; P and read-only schemas/oracles stay unchanged.

## Generic G2 reset precursor and placement oracle

The generic driver still lacks per-pass reset linkage. Keep the legacy fresh-two-
doc helper unchanged; reuse the existing current-versus-history coverage logic
for the generic SDK reset. Initial literal/property cases use actual successful
natural calls, then reject missing governed reset calls/rows and active notes
(0vs1call/10rows), not merely missing metadata. Include two passes on ONE writable
copy, bounded document counts and both positive/zero warmup requests. Check actual
current cursor/vector/span coverage and retained superseded history independently.

Generic-G2 API proof only: one pass-local monotonic origin after open/identity
checks and before reset anchors reset start/end, first warmup-record start and
first timed-record start. Reset start precedes batch construction; end follows
write, drain and current-coverage checks. Record starts are immediately BEFORE
shared observe, including fingerprint/record preparation BEFORE the API timer.
Require reset completion <= first warmup-record start when warmups>0, and <= first
timed-record start. For warmups0, first-warmup start is null/not_applicable;
missing positive-warmup or timed starts are incomplete, never numeric zero.
Future generic GWRITE record/cohort offsets use this SAME pass origin. The elapsed
loop timer is separate: it starts AFTER reset, includes warmup+timed work and
persistence (later scheduled work too), and is not compared as an absolute offset.
Existing query API timers, P/G1/legacy schemas remain unchanged. No new journal,
Engine API, observer venue, budget or workload; INITIAL later G2 write-coupling
oracles still require warmups2/timed20 and actual timed10/20 phase placement.

## Governed GG lifecycle inventory source fit

This narrow clarification fixes initial oracle population definitions before the
separate governed-stage source unit. Source baseline is
206405c65518c039f068e37361abbcf332301a73; no new runtime evidence is claimed.
Authoritative bodies under src/rust/crates/fathomdb-engine/src/ are:

- record_lifecycle.rs, Engine::transition: source Active→Deleted updates the
  non-superseded canonical row, refreshes vector eligibility and purges only
  Attribute/PropertyFts projections; its NodeFts/vector shadows remain.
- dependency_closure.rs, admit_soft_closure/apply_soft_effects: Complete soft
  closure marks an active derived node deleted with reason source_lifecycle and
  calls erase_row_projections for that dependent, retaining canonical provenance.
- projection_registry.rs, erase_row_projections/ROW_OWNED_PROJECTIONS: derived
  erasure includes both node FTS tables, managed mapping, physical vector and
  span rows. index_projector.rs, project_canonical_node_row writes BOTH
  search_index and search_index_v2 for these ordinary leaf nodes.

Use a fresh no-dependency base of N active doc nodes (tiny fixture N=65), then
append one canonical source doc and one derived-role note through ProvenancedNodeV1/
WriteProvenanceV1 and register their source dependency. Both are ordinary leaf
nodes with default BGE384 projections. Query a literal control term absent from
base bodies. BOTH governed stages use ReadView valid_as_of=100 with all other
view flags at their defaults. Valid control IDs are exactly source+derived;
after public transition(source,Deleted,None), neither is visible. A mismatching
ordinary control is NOT_RED/unqualified, never grounds to change the view or
lifecycle policy to force the count.
The default-view active population and controlled visibility are distinguished
from retained physical rows; no historical/superseded row is introduced.

| Population | Source valid | Source Deleted with complete derived closure |
| --- | --- | --- |
| All canonical rows / non-superseded canonical cursors | N+2 | N+2 |
| Canonical current rows by lifecycle state | active N+2, deleted0 | active N, deleted2 |
| Superseded canonical history |0|0|
| Mapped/physical vector rows and matching spans | N+2 | N+1: N active base rows plus retained deleted source |
| Source's own vector and each node-FTS shadow |1each|1each, retained but invisible|
| Derived vector/mapping/span and each node-FTS shadow |1each|0each, erased by closure|
| Eligible/visible nodes under the declared public view | N+2 | N |

Thus N=65 gives67 canonical rows,65 active+2deleted after transition,66 retained
vectors, no derived mapping, zero superseded history and65 visible base nodes.
The lifecycle source shadow is not called a historical superseded vector; the
deleted derived row without projections is not called an incomplete active row.

Keep the fresh-template verifier and G2 current_coverage helper UNCHANGED: their
active/superseded accounting contracts do not apply to this post-closure store.
A private GG inventory records canonical IDs/kinds/cursors/states, closure
phase/affected count and per-row mapping/physical/span/FTS disposition. Join
canonical(kind,cursor)→mapping(kind,cursor)→physical(mapping.rowid) and
span(mapping.rowid,cursor); never assume global cursor=rowid. Validate actual
profile/dimension/UTF8 spans for retained vectors and exact expected absence for
the one closed derived artifact. Independent read-only SQL checks these counts
and literal public-view controls check visibility before the intended RED.
Do not turn arbitrary missing active mappings into an allowed exception.

Initial literal/property tests must reach both valid/Deleted ordinary API
controls, then independently observe missing actual timed hybrid stage calls
(0vsrequested), not an exception from reusing an unsuitable verifier. Stage
records later require actual dependency/fallback invocation/outcome, skipped
all-nodes reason/null fields, reader snapshots and actual vector degradation;
no learned ranking or assumed complete dense route when the actual arm degrades.
The existing measured-stage caller may be mechanically shared with an explicit
query parameter, with unchanged time-stage oracles as relocation controls.
This is private tool reporting of existing behaviour: no product policy/public
stats schema/API/format/venue/gate change, no store corruption/purge experiment,
no G2 verifier relaxation and no campaign/full-size preparation authority.

### Successor fit: derived role is independent of dense-supported node kind

The first5f2fbad scaffold used node kind fact because the older dependency
predicate fixture used it. That payload is canonically committable but is NOT
in the current dense writer's locked kind domain. This invalidates only that
fixture assumption, not the derived provenance contract or lifecycle accounting.

At the unchanged206405 product source, vector_storage.rs
VECTOR_COMMITTABLE_NODE_KIND_SOURCE_TYPES/resolve_source_type/
kind_is_vector_committable accept email/article/paper/meeting/note/todo/doc,
not fact. write.rs batch_vector_kinds_needing_enrolment normalizes
ProvenancedNode through storage_write_shape; vector_kind_needs_enrolment refuses
unsupported kinds. There is no provenance-specific enrollment bug.
index_projector.rs project_canonical_node_row marks a never-vector-projected
cursor up_to_date without enrolling it, so terminal/readiness is not proof that
a physical vector exists. Do not extend this vocabulary or relax the verifier.

Actual failed-store read-only observations in
proofs/gd-gg-governed-fact-fit-observation.json: gg-source/doc/cursor66 is active
with one mapping; gg-derived/fact/cursor67 is active with zero mappings. Both
terminals are up_to_date; only doc is enrolled. No closure/transition occurred.
The write/register/first drain returned, but no public readiness getter was
reached before the inventory panic. gd-gg-governed-red is therefore NOT_RED;
it demonstrates no closure erasure or measured-stage failure. Original source/
oracle/binary preimages and the initial manifest are preserved under
proofs/gd-gg-governed-not-red-preimages in the owned task; no product fix follows.

Use KIND note for the derived provenance ROLE, retaining source KIND doc, the
same logical/revision/dependency identities, bodies/hash/locator semantics and
both fixed100 public views. Note's actual dense late enrollment is already
proved by accepted G2 source units. Thus valid kinds become doc=N+1/note=1;
after closure the retained vector kinds are doc=N+1/note=0. The N+2 canonical,
N active/2deleted, N+1 retained, source-shadow/derived-erasure and zero
superseded equations above stay unchanged. A derived role is not a requirement
to use node kind fact; no new kind or compatibility shim is introduced.

Before the meaningful missing-stage RED, capture the EXISTING public
EmbeddingReadiness after the first drain and after transition/drain: actual
state/usable_embedder/pending_count/affected_kinds/blocked, then independent
physical coverage. A ready report or up_to_date terminal alone never closes the
coverage control. Unexpected controls remain NOT_RED/unqualified. Preserve the
failed fact stores and all evidence; alter the fixture/oracle only after this
successor clarification receives independent design PASS and parent continuation.
This remains private fixture/source-fit work, without policy/API/public stats/
format/venue/numerical-gate or campaign changes.

## Generic-route/artifact/budget TDD seams (after reviewed source GO)

Pure sharing/refactor uses unchanged existing fixture controls; no fictional RED.
New host selectors/features/argv/config stay fixed across each actual pair.
Compile-ready scaffold may delegate the current single-query/one-pass caller;
it must not introduce an intentionally wrong product stub. Missing native/module/
selector/import errors are setup, not RED. The following are separate units,
not one whole-route GREEN; reuse already working pieces rather than force RED.

| Original unit / bounded addition | Initial runtime literal/property RED | Minimal GREEN |
| --- | --- | --- |
|10 input projection | Existing P converter selection is reused. Valid string JSONL versus required ordered id/kind/body records fails a literal/Unicode count-round-trip assertion, not a child parser exception. | Thin record encoder; preserve pinned corpus/question order and prefix. |
|10 retrieval limit | Existing hybrid caller returns2 on a real>10-vector fixture despite requested10; Core/SDK count oracles fail2vs10. No learned ordered-ID golden. | Private limit parameter; legacy fixture remains2. |
|10 query population | Fixed host scaffold delegates existing single-query calls; arbitrary query-file IDs/fingerprints and literal text alpha/beta/no-match results fail actual forwarding/order. Stage all six existing Core/SDK modes initially; hybrid/mean use actual argument fingerprint plus dimension/count/degradation controls, not learned rankings. | One shared query-feed loop, all requested IDs, existing API callers. |
|10 warmups | Working six-mode G1 timed calls precede the requested bounded warmup population0vsrequired RED; initial literal/property oracles cover actual warmup/timed counts and order. Generic G2 is not yet wired: its no-warmup-write, warmup/timed-count and phase-order assertions are staged in the INITIAL generic G2 actuation oracles BEFORE its GREEN, not added afterward. | Separate unscheduled G1 warmup loop and phase counters first. Preserve the same final G2 coupling requirement when its existing SDK-text actuation is adapted; no prior generic G2 or campaign qualification is inferred. |
|10 pass plan | Existing one pass versus requested six Core/SDK-mode passes fails actual call/open-close/pass/order counts, including reversed repeat. | Shared outer pass scheduler, same store and fresh Engine open/close perpass. |
|10 G2 mode adaptation | Existing SDK-text actuation is reused; hybrid/mean calls succeed but selected schedule has0actualcohorts/probes versus required2/20. Initial literal/property tests cover both missing mode adaptations, current/history and threephase/fourflush framing. | Route the existing SDK actuation after TIMED groups in both modes; no mean kind-filter claim. |
|10 GO adapter | Same observer-feature host succeeds through actual Core APIs; on requested but missing driver arm/transport has0vs1correlated search record. Stage off/GN absence, typed InvalidFilter, unavailable snapshot and mean-not-applicable controls before GREEN. | Host-only Core observer adapter; shared caller/timer, cumulative drop DELTAS and raw errors. |
|10 GG stages | Real governed/time-view controls pass first; missing requested stage calls/route observations fail actual literal IDs/guards/counts. Separate time stage and governed lifecycle stage pairs. | Public controlled-stage adapters; no duplicate product fix. |
|14 native deadline | Expired admission still executes a real tiny None-provider DB operation (1vs0); separate remaining-drain literal/property math/call-argument oracle catches fixed5000 versus actual remaining budget. No BGE/CPU substitution claim for these helper controls. | Tool clock/admission then remaining-phase bounds in separate pairs; never change Engine timeouts. |
|14 bounded probes/cleanup | Separate expired-probe prelaunch and in-flight owned subprocess deadline tests; current numeric du result after deadline or extended cleanup fails runtime refusal/time bound. Preserve diagnostics, group ownership/reaping and prior subreaper state. | Absolute-budget resource probes then cleanup clamping; source defaults unchanged. No delayed provider/model fault. |
|14 controller/manifest | Real tiny recording/DB children expose missing shared prep/copy/quiet/retry clock and26+2plan/outcome handling; literal/property counts/order and completed-tail resume checked before fixes. | Separate shared-clock admission then manifest/attempt wiring; not pure forecast treated as launch. |
|15 pairing/results | Actual tiny paired children/real DB pass records expose missing mode/query/template/adapter/repeat/error/drop/unpaired grouping and hand-computed raw-call math. | Separate grouping/report adaptation; preserve raw distributions, no percentile subtraction. |

These extend original10–15 only. D11worker intervals/12held-drain/13strict triple
intersection plus fully contained positive pending witness remain separate
real-runtime units with current deterministic768/test-hooks, not G substitutes.
All new suites/builders are indexed and selected explicitly; hardware suites use
the existing manual convention. Freeze stable source only after real generic
wiring, then one required full gate and fresh actual artifact/native proof.

## D11 initial worker-transport oracle (proposed source fit)

The [separate transport](gd-extension.md#d11-separate-worker-transport-and-shared-clock-fit-proposal)
leaves accepted GO query collection unchanged. A compile-ready RED scaffold may
use only a private driver delegating CURRENT real deterministic768 APIs and
read absent worker output as0. Alternatively, new hidden setter/drain methods
must expose an ACTUAL backed buffer, enablement and cumulative counter: empty
records/zero loss reflect genuinely unwired recording, and capacity comes from
the actual bound, never a fabricated product report or constant substitute for
state. No existing correct behavior is disabled to manufacture RED. The tiny D
driver uses real configure/write/drain/close APIs; worker/event recording remains
unwired before RED. Keep selector, features(test-hooks plus
slice30-measurement), argv/config and test bytes fixed through minimal GREEN;
missing symbols/failed build/setup/SQL controls are NOT_RED.

Initial literal65doc/note jobs plus bounded Unicode properties (1..129jobs,
1..65governed-cohort size, completed tail preserved) first
prove actual governed receipts, readiness pending0, independent canonical(kind,
cursor)→mapping→physical(mapping.rowid)/span/profile coverage, unknown D span ends
and closed WAL/copy/reopen controls. Capture configured2/5 separately. Existing
query API/result and exactly-one QueryComplete control succeeds; worker getter
must not consume or add query records. Only then assert observed worker cursor
coverage/job-count sum equals the written population: empty report fails0vs65
(and property0vsN), not a missing-file/parser exception.

Before GREEN stage exact cursor conservation/no duplicates or extras, unique
batch IDs/engine ownership, job_count==cursor-list length in1..64, actual worker
IDs in0..2, start<=end within the SAME driver attempt, and non-overlapping
successive batches per worker. Completion order may differ from cursor/admission
order; do not invent global batch ordering or require both workers to run.
Repeated quiescent drains have empty new records and stable cumulative drops;
normal/off paths remain empty controls. Actual observed distinct workers and
interval overlap supply concurrency; configuration alone never does. Preserve
literal AND property oracles before first GREEN, including query-stream isolation.

Use no new rendezvous in this first unit. Existing unbounded Barrier worker
hooks are not suitable. Later held diagnostics use cancellation-safe bounded
release-before-drain/join/close, while ordinary timed overlap has no holds/sleeps.
Stage a separate initial helper-level error oracle BEFORE the first transport
GREEN: an owned real SQLite readonly SELECT followed by the wrong typed column
read returns actual InvalidColumnType. Pass/propagate that original Err through
the observer lifetime helper; expect one aborted record, positive interval,
cleanup unconfirmed and unknown commit outcome null. The RED helper scaffold
propagates the ACTUAL Err unchanged; observation is honestly absent while event/
Drop transport remains unwired. It must not fabricate an outcome/report or
suppress already correct recording. Missing observation is read as0 and fails
the intended assertion explicitly, not an exception from absent output.
This is observer-helper error/Drop behavior, not an end-to-end worker failure or
injected product fault. Do not claim an actual worker panic/recovery occurred;
preserve the existing catch/recovery/cleanup source boundaries unchanged.

The feature-gated proposed methods/record types exist only in engine measurement
code. The D driver remains the current engine example/test-hooks identity, with
actual compiler closure recorded for DN(test-hooks) and DO(test-hooks plus
slice30-measurement); engine self-dev feature unification must be retained in
those proofs. No private G/TC5-host feature or SDK accessor changes. Default/no-
feature compilation and accepted query/GO suites are affected reuse controls.

## D12 empty-idle held precondition (method clarification)

Source subject9b1b252b6aa05b47e12bacff82c8c7a161faa020; this is read-only
source fit, not held runtime evidence. projection_runtime::ProjectionRuntime::
set_frozen(true) sets state.frozen and notifies, without cancelling work;
projection_worker::projection_dispatcher_loop checks it BEFORE incrementing
active_scans. A scan already past that check can
still fetch/admit work. Freeze never cancels admitted jobs or existing scans.

Use a FRESH caller-provider deterministic768 store: configure the reviewed
projection while empty, independently verify canonical0/vector0/pending0 and
actual profile/configuration, then create the tool-owned freeze guard. While
STILL EMPTY, call existing Engine::drain with a bounded source-fixture timeout.
runtime_lifecycle::Engine::drain takes the configured-runtime wait_for_idle route;
projection_runtime::ProjectionRuntime::wait_for_idle requires active_jobs0,
queued_jobs0 AND active_scans0, then a successful durable no-pending probe before
returning. Only Some(false), meaning NO durable pending work, completes idle;
Some(true), unavailable locking or failed probing cannot establish it. No
concurrent writer is allowed before this succeeds. Frozen then prevents a new scan before writes.
This is a fresh empty-work precondition, not a new product cancellation proof or
an assertion that a readiness count alone exposes admitted/in-flight work.

Only AFTER that idle proof, use existing governed writes for the literal3rows
(and bounded small Unicode/count cohorts). write::Engine::write late-enrolls
supported kinds transactionally; its notify_new_work sets pending_scan without
clearing frozen. Capture actual pending3/vector0 while held, then release and
use actual drain/readiness plus independent kind/cursor→mapping→physical/span/
profile completeness to establish pending0/vector3. D unknown covered ends stay
NULL; body byte lengths are separate. Existing readiness/freeze/completion
behavior is a setup control, never relabelled new product RED. Initial literal
AND property tests must first pass those controls and then fail ONLY the new
missing held/released phase reports/counts/order before reporting GREEN.

One tool-owned idempotent guard releases explicitly BEFORE post-release drain,
thread join, engine close or any orderly cleanup. Drop releases on early error/
unwind as fallback; it does not drain/join/close inside Drop. Cleanup must retain
its own original child hard deadline, stop/wait only that owned child and report
failure/partial data rather than wait forever. No unbounded Barrier, new hook,
freeze-setter repair, active-work cancellation or timed-overlap hold/sleep.

Create ONE attempt Instant before open/configuration. Record open/configure,
empty checks, freeze request, empty-idle drain, each governed write, held full
readiness/SQL observations, release, post-release drain/coverage and close with
start/end offsets on that SAME clock. Release-call return is not inferred worker
progress: actual subsequent drain/physical completion is required. API durations
exclude report serialization; observation/collection/persistence remain explicit
source work. The D11 collection window is one non-renewing5s source limit outside
API clocks, later clamped to original native work/hard/shared deadlines. No fresh
window can expand the existing150work/30cleanup source or870/30 cell bounds.
