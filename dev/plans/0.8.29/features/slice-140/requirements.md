# Slice 140 requirements reconciliation

Active .29 repair selection follows owner seq-376/377: necessary110 source/seam
repairs and independently required existing .29 artifact repairs only. New x86
CUDA13 delivery/compiler work formerly in120 is [.31](../../../0.8.31/cuda-delivery-plan.md)
under seq-378 and cannot create a mandatory .29 repair. Real Orin/3090 reset
children are HELD .30; no .29 live-fault gate, survival or original-defect closure
is inferred. Fork B/C maintenance remains .30. No repair is selected by this note.

Status: WAITING_FOR_REQUIRED_REPAIR_SELECTION. R140/A140 identities below retain
the original local plan's six obligations; these are not canonical AC additions.

Authority is N29-R/G, R29-REL-R/G and R29-3P-17/18 (O3-17/18), with
[110](../slice-110/plan.md), [120](../slice-120/plan.md) and the accepted
[pool/fork](../../../../adr/ADR-0.8.28-tegra-private-cuda-pool.md),
[driverless](../../../../adr/ADR-0.8.25-driverless-cuda-runtime-linkage.md) and
[CUDA policy](../../../../adr/ADR-0.8.28-cuda-version-support-policy.md) contracts.

| Requirement | Human intent and canonical join | Current gap |
| --- | --- | --- |
| R140-1 minimal selected repair | Every diff is necessary for an accepted110 or independently required existing .29 artifact outcome; compare a smaller remedy and exclude .30 B/C. | Selected110 policy still needs an exact fork-local repair witness and proof; no concrete diff is selected here. |
| R140-2 preserve capabilities | Preserve pool isolation/ownership, allocator symmetry, ARM CPU safety, lazy loading, typed refusal and affected existing behavior. REQ-020a/b/021/023, REQ-056/068; AC-022a/b/c,023a/b,025,060a,082a–d apply in their actual scope. | Existing unsupported-reset statement cannot become support without accepted replacement and evidence. Five-second exit bounds start at close-return/main-return; they are not a universal close-call bound. |
| R140-3 source and consumer closure | Qualified code, features, lock closure, version and package bytes have reviewed provenance; external Rust consumers are distinct from root-patched artifacts. REQ-047/049/052; AC-051a,053,056 and pool ADR consumption amendment. | Published current sources authenticated; no repair source/package exists. Historical qualification is not .29 repair proof. |
| R140-4 behavioral proof before repair | Own observed RED before separate GREEN per changed behavior; prove positive safe lifecycle and healthy/recoverable-OOM controls, not expected crash. REQ-020/021/056 and affected ACs above. | SIGSEGV characterization and pure loss classification do not prove cleanup. Seq-376 selects policy; seq-377 holds real device fault execution .30. Source/seam proof cannot close original reset survival. |
| R140-5 bounded version and pin movement | New version only for included repair; semver/API impact, exact upstream base and package source are reviewed before adoption. Preserve one cudarc family and static kernel patch. REQ-047/049/050; AC-051a,053,054 as applicable. | FT-1 is conditional; no version guessed and no policy to track latest upstream. Publication is held. |
| R140-6 truthful bounded closure | Each selected obligation is qualified or explicitly disposed; no-repair-needed requires evidence. .30 maintenance never gates .29. | WAITING status is not completion or no-repair-needed. Final130 follows chosen pins/integration and cannot gate initial fork proof. |

Format37/no migration, no shims and existing query/spans/hints/queue remain fixed.
REQ-048/AC-052 historical W/E wording reconciliation stays with160; it does not
authorize coupling fork versions or bumping independent E0.8.0. No existing
canonical acceptance wording is changed by this proposal.
