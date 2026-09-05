---
title: 0.8.25 Slice 35 — truthful legacy-path measurement amendment
status: READY
design_version: 7
amends: design.md
triggered_by: implementation-review-fix-6
---

# Slice 35 design amendment — FIX-6

The legacy non-regression campaign measures the public Python
`Engine.search_text_only` operation used by SCALE-02, not `Engine.search`.
`search_text_only` is the intended ordinary FTS path because the experiment
opens both artifacts without a default embedder and isolates eligibility's
legacy lexical overhead. The claim remains limited to this path; it does not
claim vector, reranker, hybrid, or automatic-routing performance.

The replacement measurement contract must:

- name `Engine.search_text_only` in both arm call paths and components;
- bind commit-addressed blobs for `experiments/scale_02_slice35.py` and the
  actual invocation in `experiments/scale_02.py`;
- increment an observed call counter at the invocation seam and persist the
  aggregate in each arm summary;
- build the exact witness count from those observed summaries, never from the
  requested workload alone;
- run from a clean committed checkout against the already pinned baseline and
  candidate native artifacts; and
- retain but quarantine every earlier Slice 35 receipt that made the false
  `Engine.search` claim.

The product performance boundary, five repetitions, 100 warm-ups, 1,000
steady queries, fixed 10k corpus, seeds, and one-sided 95% upper bound of 3%
remain unchanged. Only warm-up and steady calls contribute to the recorded
measurement witness. Setup, mutation probes, and other SCALE-02 observations
remain excluded run control.

The virtual-table mutation manifest also becomes a closed inventory of every
live mutation form. A scanner traverses every production `*.rs` file under
`fathomdb-engine/src/`, lexes ordinary and raw Rust string literals while
excluding comments, normalizes whitespace, and reports every SQL DML or DDL
form capable of changing the exact serving virtual-table set: `search_index`,
`search_index_v2`, `search_index_edges`, `property_search_index`, and
`vector_default`. It recognizes `INSERT`, `INSERT OR IGNORE`, `UPDATE`,
`DELETE`, `CREATE VIRTUAL TABLE`, and `DROP TABLE`, including formatted
`{DEFAULT_VECTOR_PARTITION}` and allowlisted generic `{table}` forms. Negative
scanner tests inject an unclassified mutation in another production module,
raw-string SQL, and virtual-table DDL and prove each fails.

The authoritative inventory is:

| Function | Mutation form | Classification/coupling |
| --- | --- | --- |
| `project_canonical_node_row` | insert `search_index`; insert `search_index_v2` | canonical-node owner in the same transaction |
| `project_canonical_edge_row` | insert `search_index_edges` | canonical-edge owner in the same transaction |
| `prune_edge_projection_shadows` | delete `search_index_edges` | canonical-edge lifecycle in the same transaction |
| `project_one_attribute` | insert `property_search_index` | paired `canonical_attributes` insert |
| `clear_attribute_projection` | delete `property_search_index` | paired `canonical_attributes` delete |
| `write_vector_for_test` | formatted insert `vector_default` | paired `_fathomdb_vector_rows`; test-only transaction |
| `run_pin_and_requantize_pass` | formatted vector delete/reinsert | retains `_fathomdb_vector_rows` owner in one transaction |
| `commit_projection_outcomes` | literal/dynamic `INSERT OR IGNORE` vector insert | paired vector owner and projection terminal in the worker transaction |
| `delete_vector_partition_row` | formatted vector delete | helper whose callers retain or remove the real owner in one transaction |
| `refresh_vector_attr_values_for_row` | formatted vector update | canonical owner/attribute activation transaction |
| `reshape_vector_partition_nondestructive` | formatted drop, create, and bulk vector reinsert | pre-token open or registry-triggered configuration transaction |
| `vector_partition_create_sql` | formatted create `vector_default` | pre-token open or configuration transaction paired to registry generation |
| `migrate_vector_partition_pack1_to_pack2` | literal drop and bulk vector reinsert | database-open migration before token minting |
| `migrate_vector_partition_to_pack1` | literal drop and bulk vector reinsert | database-open migration before token minting |
| `delete_row_owned_projection` | generic formatted delete | allowlisted table and canonical lifecycle owner |
| `truncate_row_projections_in` | generic formatted delete | governed rebuild/configuration generation owner |

The scanner's observed normalized signatures, source module, enclosing
function, and counts must equal this inventory exactly. Function-coupling
assertions pin the named real owner, triggered registry/generation row, or
pre-token call boundary. Adding a new static/raw or dynamically formatted DML
or DDL mutation in any production Engine module therefore fails until it is
classified here and in the executable inventory. Temporary tokenizer-only
tables outside the exact serving set are not frozen evidence and are excluded
by table identity, not by file or SQL form.

The executable inventory also closes over mutation-helper call sites, because
a new caller can violate coupling without adding a new SQL literal. It records
every caller of `delete_vector_partition_row`, `delete_row_owned_projection`,
and `truncate_row_projections_in` and fails when the source call-site set
changes. The current caller classifications are:

- canonical edge pruning: `prune_edge_projection_shadows` and the G0/G11 edge
  supersession arms in `apply_batch_in_transaction`, each paired with
  canonical lifecycle updates, vector-owner deletion, and terminal readiness;
- vector publication maintenance: `run_pin_and_requantize_pass` and
  `commit_projection_outcomes`, each within its vector-owner transaction;
- open-time repair: `prune_orphaned_edge_vectors`, before token minting and
  protected by its durable completion marker;
- row lifecycle: `erase_row_projections` and
  `purge_row_projections_for_cursor_in`, which reach the helper only through
  the closed `ROW_OWNED_PROJECTIONS` allowlist in the canonical lifecycle
  transaction; and
- governed rebuilds: `rebuild_shadow_state`,
  `reproject_search_index_after_tokenizer_upgrade`, and
  `truncate_all_row_projections`, which execute before serving resumes or in a
  generation-triggered configuration transaction.

Both direct and transitive helper-call inventories use enclosing-function plus
callee identity rather than line numbers. Negative tests add a new uncoupled
caller without a new SQL string and prove the gate fails.

This amendment becomes READY only after independent design review with no
unresolved P1/P2 findings.
