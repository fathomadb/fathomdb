#!/usr/bin/env python3
"""Guard canonical projector and row-kind ownership after extraction."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
INDEX_FUNCTIONS = (
    "restore_registered_derived_projections",
    "reproject_search_index_after_tokenizer_upgrade",
    "search_index_tokenizer_reproject_complete",
    "canonical_node_rows",
    "row_kind_from_column",
    "index_targets_for_row_kind",
    "project_canonical_node_row",
    "project_canonical_edge_row",
)
INDEX_TYPES = ("CanonicalNodeRow", "IndexTargetSet")
INDEX_CONSTANTS = (
    "SEARCH_INDEX_TOKENIZER_SCHEMA_VERSION",
    "SEARCH_INDEX_TOKENIZER_REPROJECT_MARKER_KEY",
)


def owner_errors(root: str, index: str, write_types: str) -> list[str]:
    errors = []
    root_declaration = re.compile(
        r"^(?:(?:pub(?:\([^)]*\))?) )?(?:fn|struct|enum|const) "
        r"((?:restore_registered_derived_projections|reproject_search_index|"
        r"search_index_tokenizer|canonical_node_rows|row_kind_from_column|"
        r"index_targets_for_row_kind|project_canonical_|CanonicalNodeRow|"
        r"IndexTargetSet|RowKind|SEARCH_INDEX_TOKENIZER)\w*)\b",
        re.M,
    )
    for match in root_declaration.finditer(root):
        errors.append(f"root still defines index::{match.group(1)}")
    for name in INDEX_FUNCTIONS:
        pattern = r"^pub\(crate\) fn " + name + r"\("
        if len(re.findall(pattern, index, re.M)) != 1:
            errors.append(f"index owner lacks one function {name}")
    for name in INDEX_TYPES:
        if len(re.findall(r"^pub\(crate\) struct " + name + r"\b", index, re.M)) != 1:
            errors.append(f"index owner lacks one struct {name}")
    for name in INDEX_CONSTANTS:
        if len(re.findall(r"^pub\(crate\) const " + name + r"\b", index, re.M)) != 1:
            errors.append(f"index owner lacks one constant {name}")
    if len(re.findall(r"^pub enum RowKind\b", write_types, re.M)) != 1:
        errors.append("write-types owner lacks RowKind")
    if len(re.findall(r"^impl RowKind \{", write_types, re.M)) != 1:
        errors.append("write-types owner lacks RowKind impl")
    if not re.search(r"^pub use write_types::RowKind;", root, re.M):
        errors.append("root does not re-export write_types::RowKind")
    return errors


class IndexProjectorOwnerTest(unittest.TestCase):
    def sources(self) -> tuple[str, str, str]:
        root = (SRC / "lib.rs").read_text()
        index_path = SRC / "index_projector.rs"
        write_types_path = SRC / "write_types.rs"
        return (
            root,
            index_path.read_text() if index_path.exists() else "",
            write_types_path.read_text() if write_types_path.exists() else "",
        )

    def test_current_source_has_one_owner_per_item(self) -> None:
        self.assertEqual(owner_errors(*self.sources()), [])

    def test_unlisted_root_family_with_complete_owners(self) -> None:
        root, index, write_types = self.sources()
        self.assertEqual(owner_errors(root, index, write_types), [])
        for declaration, name in (
            ("fn project_canonical_new() {}", "project_canonical_new"),
            ("fn search_index_tokenizer_new() {}", "search_index_tokenizer_new"),
            ("fn canonical_node_rows_new() {}", "canonical_node_rows_new"),
            ("fn restore_registered_derived_projections_new() {}", "restore_registered_derived_projections_new"),
            ("struct IndexTargetSetNew;", "IndexTargetSetNew"),
            ("enum RowKindNew {}", "RowKindNew"),
            ("const SEARCH_INDEX_TOKENIZER_NEW: u32 = 0;", "SEARCH_INDEX_TOKENIZER_NEW"),
        ):
            with self.subTest(name=name):
                self.assertIn(
                    f"root still defines index::{name}",
                    owner_errors(root + "\n" + declaration, index, write_types),
                )

    def test_missing_public_row_kind_reexport_fails(self) -> None:
        root, index, write_types = self.sources()
        self.assertIn(
            "root does not re-export write_types::RowKind",
            owner_errors(root.replace("pub use write_types::RowKind;", "", 1), index, write_types),
        )

    def test_fast_tier_registration(self) -> None:
        self.assertIn(
            "run_tier_suite fast test-slice90-index-projector-owner "
            "python3 scripts/tests/test_slice90_index_projector_owner.py",
            (ROOT / "scripts/agent-test.sh").read_text(),
        )


if __name__ == "__main__":
    unittest.main()
