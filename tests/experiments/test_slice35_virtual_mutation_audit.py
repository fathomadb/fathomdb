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
