# CORPUS-01 corpus matrix and human-gold contract

**Track:** `CORPUS-01`  
**Status:** preparation only; no corpus acquisition, payload inspection, human
review, answer-gold creation, or measurement occurred.  
**Authority:** `seq-249` authorizes corpus/license and human-gold work. It does
not replace factual preflight for a particular external payload or license.

## Purpose and boundary

`experiments/configs/corpus-01/corpus-matrix.v1.json` is the versioned
portfolio record. `experiments/configs/corpus-01/human-gold-protocol.v2.json`
binds an external-only qualification protocol to its canonical SHA-256. The
validator in `experiments.corpus_matrix` validates only committed metadata and
a future content-free manifest. It does not acquire, open, or write a corpus
payload.

Every payload, upstream license copy, question, answer, evidence excerpt, and
human-review worksheet remains outside the repository. A future external
manifest may contain only stable IDs, SHA-256 locators, final judgment,
hashed-reviewer identities, and adjudication status. The validator rejects raw
or verbatim fields, including `question`, `answer`, `answer_text`,
`evidence_text`, `raw_payload`, and `verbatim_quote`.

The state `external_payload_not_verified` means a local registry, acquisition
script, or historical register describes a source but this campaign worktree
has not established the selected external payload's presence or hash. It is
not evidence that the payload currently exists. `not_acquired` means no
payload or registry pin is claimed here.

`human-gold-protocol.v1.json` is retained only as the initial planning
prototype. It cannot qualify a corpus/category pair or support a product
claim. Qualification uses v2 and the amendment route below; no compatibility
path upgrades a v1 planning manifest into evidence.

## Corpus portfolio

| Corpus | License / payload / redistribution rule | Supports | Does not support | Metrics and power rule |
| --- | --- | --- | --- | --- |
| LOCOMO | CC-BY-NC-4.0; external, unverified; never commit, ship, or quote payload/gold | conversation, temporal, multi-session retrieval | knowledge update, supersession, source erasure | Retrieval recall, answer quality, latency/cost; paired class power before a claim; historical 841 factoid / 321 temporal / 281 multi-session |
| LongMemEval | MIT is a research-note fact only; not acquired | knowledge-update and session-ordered evaluation after factual preflight | source erasure and supersession without human gold | Answer and abstention quality; paired class power after actual class counts are frozen |
| TimelineQA | CC-BY-NC-4.0; generated external payload, unverified; never commit or ship | personal timeline and time-scoped validity | real-person data, update, supersession, erasure | Temporal accuracy and evidence recall; density-stratified paired power; registered atomic counts 282,643 / 640,935 / 1,534,761 |
| TimeQA | BSD-3-Clause; external, unverified; project policy still keeps it out of git | time-sensitive fact retrieval and abstention | personal memory or lifecycle claims | Temporal/abstention/evidence metrics; report easy (6,018) and hard (6,165) separately |
| Test of Time | CC-BY-4.0; external, unverified | contamination-resistant temporal probe | naturalistic personal memory or lifecycle claims | Temporal accuracy; keep semantic (2,800), arithmetic (1,850), and semantic-large (46,480) separate |
| IR-C | mixed upstream LicenseRefs; external gold/payload root, unverified | retrieval fidelity and lexical/dense diagnostics only | answer correctness or personal-memory lifecycle | Evidence Recall@K, MRR, NDCG; frozen hash and stratified denominators required before a claim |
| BEIR | per-dataset license; external, unverified | public-IR generalization | complete relevance, answer correctness, personal memory | Recall@K, MRR, NDCG; select and report each named dataset/split separately; do not pool sparse qrels |
| MuSiQue | CC-BY-4.0; external, unverified | bounded multi-hop supporting-evidence retrieval | lifecycle or unbounded-graph claims | Supporting-evidence Recall@K and answer F1; retain answerable/unanswerable and 2/3/4-hop strata |
| SummHay | Apache-2.0; external, unverified | global coverage and citation quality | personal-memory or graph-default claims | Coverage, citation quality, summary quality; report 10 topics and 92 subtopics separately |
| AP-News BenchmarkQED | Microsoft Research License; external, unverified; never commit, ship, or quote | historical global-sensemaking comparison | redistributable gold or a new claim without preflight | Comprehensiveness, diversity, empowerment; corpus-capped report only until a newly qualified question set exists |

The matrix records exact known or historical class counts only where a local
manifest or historical register provides them. Null counts are intentionally
`not_verified` or `not_acquired`; they must never be replaced by an estimate.
No row provides direct source-erasure or supersession evidence. Those two
categories require the governed human protocol below.

## Claim rules

The approved portfolio is deliberately complementary rather than interchangeable:

- LOCOMO is the personal-memory anchor, but it cannot establish a
  knowledge-update or source-erasure claim.
- LongMemEval may contribute knowledge-update evidence only after its source,
  selected split, license, content hash, and actual class counts are factually
  qualified.
- TimelineQA, TimeQA, and Test of Time measure different temporal questions;
  none is evidence that an agent correctly retains or erases personal records.
- IR-C and BEIR are retrieval measures. A higher Recall@K/MRR/NDCG does not
  establish answer correctness, lifecycle behavior, or a product default.
- MuSiQue supports multi-hop evidence coverage, not an unbounded graph claim.
- SummHay/AP-News support global-sensemaking evaluation, not personal-memory
  lifecycle behavior.

A broad mutable, provenance-preserving agent-memory statement is blocked until
the required category has qualified evidence and an explicit power analysis.
Counts are denominators, not automatic power: every measured class needs its
own paired analysis, confidence interval, exclusions, and corpus-specific
result. Do not pool unrelated corpora, class types, or sparse-qrel datasets.

## Native and qualified-human-gold eligibility

The matrix distinguishes the two evidence modes explicitly:

- A `native_corpus` mode is available only for a category named in that
  corpus's `supported_categories`. This says the corpus has the appropriate
  question shape; it does not waive payload, license, metric, or power
  preflight.
- A `qualified_human_gold` mode is the only path for an unsupported pair. For
  example, `LOCOMO × knowledge_update` fails closed unless a v2 qualified
  manifest and a matching versioned `corpus-01-human-gold-amendment.v1` are
  both supplied. The amendment binds the matrix SHA, qualified-manifest SHA,
  corpus/category pair, and an approval reference. It also must have an exact,
  matching entry in the content-free
  `corpus-01-approved-amendment-registry.v1` supplied by the coordinator. A
  `seq-N`-shaped string alone is never approval. The registry entry contains
  only amendment SHA-256, corpus ID, category, and the same approval reference.
  It is a future coordinator/HITL record, not created by this preparation lane.

A broad agent-memory claim requires qualified evidence for every lifecycle
category. Matrix intent, a registered recipe, or a pilot with a different
category is insufficient portfolio/category evidence.

## Human-review protocol

The `corpus-01-human-gold-protocol.v2` contract covers four categories:
`knowledge_update`, `supersession`, `source_erasure`, and
`time_scoped_validity`.

1. A coordinator first creates one content-free factual-preflight binding per
   selected corpus/category. It records the source-payload SHA-256, license
   copy SHA-256, source or generator revision, selected class counts,
   exclusions SHA-256, corpus-supported metric, paired-power SHA-256, claim
   ID, and claim SHA-256. Any missing, undeclared, over-large, or conflicting
   value fails closed before a human record can count.
2. Sampling happens against that external material. A reviewer receives the
   original source material outside the repo and determines whether the
   proposed lifecycle relation is `supported`, `contradicted`, or
   `insufficient_evidence`. Reviewers do not write or synthesize answer text.
3. Two independent human reviewers make blind initial judgments. Their
   detailed worksheets and any original text remain external. Disagreement is
   adjudicated externally; the final manifest says only `adjudicated` or
   `not_required`.
4. The resulting external manifest binds the matrix SHA, v2 protocol SHA, the
   factual-preflight entries, opaque source/evidence locators, and hashed
   reviewer IDs. It has no raw question, answer, evidence text, payload, or
   quote.
   A `pilot` manifest can cover one category; a `portfolio_complete` manifest
   must cover all four categories and may name only a corpus in the frozen
   matrix.
5. Before a category claim, report the sampled count, per-category agreement,
   exclusions, `insufficient_evidence` rate, final judgment distribution, and
   a paired power calculation appropriate to the selected comparison. A pilot
   or convenience sample is descriptive only.

Human judgment may establish an auditable relation between externally stored
records. It must not manufacture a model-answer oracle, fill absent evidence
with a plausible answer, or turn non-redistributable material into committed
fixtures.

## Factual prerequisites for the next CORPUS-01 step

Before any authorized acquisition or human review, the coordinator must choose
the specific source/split and qualify, outside this repository:

1. upstream revision or generator pin and license copy;
2. external payload-root hash (including selected questions/qrels where
   applicable);
3. actual selected-class counts and exclusion rules;
4. corpus-supported metric, paired-power artifact, claim ID, and claim hash;
5. source-specific redistribution posture, including whether derivatives or
   human annotation metadata inherit a non-commercial restriction; and
6. a versioned, review-accepted human-gold amendment if the selected category
   is unsupported natively by that corpus.
7. a coordinator-supplied approved-amendment registry entry whose amendment
   SHA-256, corpus/category pair, and steward `seq-N` reference exactly match
   that amendment. The coordinator records any decision through `ledgerwrite`;
   workers never hand-edit a ledger or invent a registry approval.

These are factual gates, not permissions to acquire a new source or conduct a
human review. New payloads, human work, or external artifacts still need the
normal coordinator release and Track Runner record.
