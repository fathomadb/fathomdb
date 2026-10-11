/// EXP-S (0.8.14 Slice 5, D1) — structural-role tag for a canonical row.
///
/// A SEPARATE axis from the doc-type `kind` (email/article/paper/meeting/
/// note/todo/doc/edge_fact): `row_kind` describes *what structural role* a row
/// plays in the "one store, many indexes" substrate, not what document type it
/// carries. Stored in `canonical_nodes.row_kind` (schema migration step 16).
///
/// `Leaf` is the default (a normal record; every existing/normal write is a
/// leaf — back-compat preserving). `Coverage` = coverage/summary rows;
/// `Graph` = graph structural rows. Engine-internal in 0.8.14 — there is NO
/// public Py/TS SDK surface for `row_kind` this release (`Leaf` for all normal
/// writes; `Coverage`/`Graph` are set only by internal paths). Cross-binding
/// parity (X1) is a Slice-40 concern.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RowKind {
    Leaf,
    Coverage,
    Graph,
}

impl RowKind {
    /// On-disk `canonical_nodes.row_kind` spelling. Must match the migration
    /// step-16 `DEFAULT 'leaf'` and the schema vocabulary (D1).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            RowKind::Leaf => "leaf",
            RowKind::Coverage => "coverage",
            RowKind::Graph => "graph",
        }
    }
}
