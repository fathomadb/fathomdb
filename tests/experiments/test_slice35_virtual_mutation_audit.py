from __future__ import annotations

from collections import Counter
from pathlib import Path

import pytest

from experiments import slice35_virtual_mutation_audit as audit


def test_production_virtual_mutations_and_helper_callers_are_closed() -> None:
    root = Path(__file__).resolve().parents[2]
    audit.validate_engine_tree(
        root / "src/rust/crates/fathomdb-engine/src",
        inventory=audit.PRODUCTION_INVENTORY,
        helper_callers=audit.PRODUCTION_HELPER_CALLERS,
    )


def test_moved_projector_and_open_sites_keep_their_exact_owners() -> None:
    expected_projectors = {
        audit.MutationSite("index_projector.rs", "project_canonical_node_row", "INSERT INTO", "search_index"),
        audit.MutationSite("index_projector.rs", "project_canonical_node_row", "INSERT INTO", "search_index_v2"),
        audit.MutationSite("index_projector.rs", "project_canonical_edge_row", "INSERT INTO", "search_index_edges"),
    }
    assert expected_projectors <= set(audit.PRODUCTION_INVENTORY)
    assert not any(
        site.module == "lib.rs" and site.function.startswith("project_canonical_")
        for site in audit.PRODUCTION_INVENTORY
    )
    assert audit.PRODUCTION_HELPER_CALLERS["delete_vector_partition_row"]["open.rs", "prune_orphaned_edge_vectors"] == 1
    assert audit.PRODUCTION_HELPER_CALLERS["truncate_row_projections_in"]["index_projector.rs", "reproject_search_index_after_tokenizer_upgrade"] == 1


def test_old_root_decoy_does_not_satisfy_moved_projector_owner(tmp_path: Path) -> None:
    (tmp_path / "lib.rs").write_text(
        'fn project_canonical_node_row() { let _ = "INSERT INTO search_index("; }',
        encoding="utf-8",
    )
    (tmp_path / "index_projector.rs").write_text("fn project_canonical_node_row() {}", encoding="utf-8")
    with pytest.raises(audit.VirtualMutationAuditError, match="unclassified"):
        audit.validate_engine_tree(
            tmp_path,
            inventory=[audit.MutationSite("index_projector.rs", "project_canonical_node_row", "INSERT INTO", "search_index")],
            helper_callers={helper: Counter() for helper in audit.HELPERS},
        )


@pytest.mark.parametrize(
    ("relative_path", "source"),
    [
        ("new_module.rs", 'fn leak(tx: &Db) { tx.execute("DELETE FROM search_index", []); }'),
        ("raw.rs", 'fn leak(tx: &Db) { tx.execute(r#"UPDATE vector_default SET status = \'x\'"#, []); }'),
        ("ddl.rs", 'fn leak(tx: &Db) { tx.execute("DROP TABLE property_search_index", []); }'),
        ("caller.rs", "fn leak(tx: &Db) { delete_vector_partition_row(tx, 1); }"),
        (
            "lifetime.rs",
            "fn leak(tx: &Transaction<'_>) { tx.execute(\"DELETE FROM search_index_edges\", []); }",
        ),
    ],
)
def test_unclassified_module_raw_ddl_and_helper_caller_fail(
    tmp_path: Path, relative_path: str, source: str
) -> None:
    (tmp_path / relative_path).write_text(source, encoding="utf-8")
    with pytest.raises(audit.VirtualMutationAuditError, match="unclassified"):
        audit.validate_engine_tree(
            tmp_path,
            inventory=[],
            helper_callers={helper: Counter() for helper in audit.HELPERS},
        )
