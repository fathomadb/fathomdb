# Slice 100 needs, requirements and local acceptance

N100 refines the release N29-R/R29-REL-R need: interruption must preserve truthful
erasure state and authorized surviving data within the declared fault model.
[Completed3](../../prework/slice-3/requirements.md) allocates R29-3P-16/O3-16;
[its oracle](../../prework/slice-3/acceptance.md) and completed4/5 retain the
post-state comparison. [Design](design.md#obligation-and-disposition-map) separates
original D28-09 minimum, later allocation and optional proof expansion.

Canonical ancestry remains NEED-001/020/021 and existing erasure/reliability
requirements. AC-028a/b/c and R-20-E1/E5→AC-080 constrain retained audit,
projection, survivor and raw-byte success proof. Process exit supplies no new
REQ-031b/AC-034 power/OS-crash proof. A100 labels are local assessment criteria,
not additions to locked ACs or new thresholds. Current documentation base is
`1099978cd7d7aef6e745cfc850ddb8f9b57050d3`; no new product run occurs here.

| Need → requirement → local acceptance | Independent oracle and present assessment |
| --- | --- |
| N100 → R100-1 atomic intent/deletion/sanitation and truthful phases. A100-1: reached named process exit, rollback of uncommitted state, survival of committed state, no invented success. | C1–C6, S1/S2/S9. Exit73 with target/survivor/ticket logical state and retry is direct crash proof; exact phase/root scrub at every exit is not asserted. Fault-position minimum is evidenced; broader post-state is bounded. |
| N100 → R100-2 exact policy and fences across release/reopen. A100-2: Queued visibility unchanged; explicit activated debt stays fenced and never starts autonomous cleanup; autonomous reopen finishes without follow-up target calls. | C7/C10/C11; S3/S4/S7. Independent observer and live worker prove autonomy separately; error/BUSY/reopen controls prove explicit fences. These separate controls are not one all-phase abrupt-crash proof. |
| N100 → R100-3 physical success covers registered boundaries. A100-3: target structural zero, original-sink discharge, identity-free queue/root and successful TRUNCATE precede Complete, with raw main/live-WAL witness before close. | C5/C8/C9/C12; S4–S8. Positive population, raw tables/file bytes, original fingerprint and unchanged control records. Strong success/refusal proof; retained G100-2P lacks direct post-crash projection/span state, and G100-2W lacks independent main/live-WAL absence after post-crash completion before close. Proposed H/P/W cells are in the test plan. |
| N100 → R100-4 exact scope, survivors and replay. A100-4: preserve survivor history/rowIDs and independent survivor-only scores/order; retry converges; offline leaves Queued untouched and publishes same activated receipt. | C8–C12; S4–S8. Exact snapshots, independent stores, actual retry/offline controls. Existing crash fixtures assert survivor presence/count only: retained G100-2H requires unchanged historical versions after interruption. Proposed H cell compares literal-intent-validated pre-crash raw rows and post-reopen history; no whole-policy qualification follows. |
| N100 → R100-5 honest fault-model attribution. A100-5: identify actual reached transaction boundaries; preserve absent active-checkpoint proof, and infer no power/host fault. | C2/C3/C6/C13. Original “transaction or WAL checkpoint” minimum has real transaction-interior evidence. G100-1 active-native-checkpoint is NOT QUALIFIED; optional expansion is not an automatic additional minimum. No debugger/fault alternative is selected. |
| N100 → R100-6 exact evidence and minimal changes. A100-6: retain source/configuration/selector/exits/reachability and provenance limits, actual closed10 ancestry, independent current design/final reviews and scoped documentation checks. | C14/S1–S9, selective22-file hash receipt and retained log. Historical bounded actual20 design PASS is preserved. Fresh v1/v2 semantic and exact v3 design PASS are recorded; parent scoped checks and different fresh final DOC/OUTPUT PASS apply to exact imported11-file diffc0e80622, as bound in review-log. No runtime qualification follows. Only docs are selected; a later real repair needs genuine RED→separate GREEN. |

R100-1–6 all map to N100 and the release FAULT row; each C/S cell maps back.
Three retained predicates remain unmet: G100-2H interrupted survivor history,
G100-2P direct post-crash projection/span state, and G100-2W independent main/live-
WAL absence after post-crash completion. Every-family/every-hook conjunction and
active-native-checkpoint interruption are not established additional gates.
No defect, coverage exception, deferral or whole100/D28-09 acceptance is recorded.
The compact proposed cells in the test plan are unapproved coverage-only work.
