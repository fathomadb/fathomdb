"""Closed source audit for serving virtual-table mutations in Slice 35."""

from __future__ import annotations

import re
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


SERVING_VIRTUAL_TABLES = {
    "search_index",
    "search_index_v2",
    "search_index_edges",
    "property_search_index",
    "vector_default",
}
MUTATION_PATTERN = re.compile(
    r"\b(INSERT(?:\s+OR\s+IGNORE)?\s+INTO|UPDATE|DELETE\s+FROM|"
    r"CREATE\s+VIRTUAL\s+TABLE(?:\s+IF\s+NOT\s+EXISTS)?|DROP\s+TABLE)"
    r"\s+(?:IF\s+EXISTS\s+)?([A-Za-z0-9_{}]+)",
    re.IGNORECASE,
)
FUNCTION_PATTERN = re.compile(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(")
HELPERS = {
    "delete_vector_partition_row",
    "delete_row_owned_projection",
    "truncate_row_projections_in",
}


class VirtualMutationAuditError(AssertionError):
    """The production mutation or helper-caller inventory drifted."""


@dataclass(frozen=True, order=True)
class MutationSite:
    """One SQL mutation form classified by module, function, verb, and table."""

    module: str
    function: str
    verb: str
    table: str


def _string_literals(source: str) -> list[tuple[int, str]]:
    """Lex ordinary/raw Rust strings while excluding comments and character literals."""
    result: list[tuple[int, str]] = []
    index = 0
    length = len(source)
    while index < length:
        if source.startswith("//", index):
            newline = source.find("\n", index + 2)
            index = length if newline < 0 else newline + 1
            continue
        if source.startswith("/*", index):
            depth = 1
            index += 2
            while index < length and depth:
                if source.startswith("/*", index):
                    depth += 1
                    index += 2
                elif source.startswith("*/", index):
                    depth -= 1
                    index += 2
                else:
                    index += 1
            continue
        raw_prefix = index
        if source.startswith("br", index):
            raw_prefix += 1
        if raw_prefix < length and source[raw_prefix] == "r":
            cursor = raw_prefix + 1
            while cursor < length and source[cursor] == "#":
                cursor += 1
            raw = cursor < length and source[cursor] == '"'
        else:
            raw = False
        if raw:
            hashes = source[raw_prefix + 1 : cursor]
            start = index
            body_start = cursor + 1
            terminator = '"' + hashes
            end = source.find(terminator, body_start)
            if end < 0:
                break
            result.append((start, source[body_start:end]))
            index = end + len(terminator)
            continue
        quote_offset = 1 if source.startswith('b"', index) else 0
        if source.startswith('"', index) or quote_offset:
            start = index
            index += quote_offset + 1
            body: list[str] = []
            while index < length:
                char = source[index]
                if char == '"':
                    index += 1
                    break
                if char == "\\" and index + 1 < length:
                    next_char = source[index + 1]
                    if next_char == "\n":
                        index += 2
                        while index < length and source[index] in " \t":
                            index += 1
                        continue
                    body.extend((char, next_char))
                    index += 2
                    continue
                body.append(char)
                index += 1
            result.append((start, "".join(body)))
            continue
        if source[index] == "'":
            index += 1
            while index < length:
                if source[index] == "\\":
                    index += 2
                elif source[index] == "'":
                    index += 1
                    break
                else:
                    index += 1
            continue
        index += 1
    return result


def _enclosing_function(source: str, offset: int) -> str:
    names = [match.group(1) for match in FUNCTION_PATTERN.finditer(source, 0, offset)]
    return names[-1] if names else "<module>"


def _canonical_table(token: str) -> str | None:
    value = token.lower()
    if value.endswith("{default_vector_partition}"):
        return "vector_default"
    if value in {"{}", "{table}"}:
        return "<allowlisted>"
    return value if value in SERVING_VIRTUAL_TABLES else None


def scan_mutations(root: Path) -> list[MutationSite]:
    """Return every serving-table DML/DDL mutation in production Engine Rust."""
    sites: list[MutationSite] = []
    for path in sorted(root.rglob("*.rs")):
        source = path.read_text(encoding="utf-8")
        for offset, literal in _string_literals(source):
            normalized = " ".join(literal.split())
            for match in MUTATION_PATTERN.finditer(normalized):
                table = _canonical_table(match.group(2))
                if table is None:
                    continue
                verb = " ".join(match.group(1).upper().split())
                sites.append(
                    MutationSite(
                        module=str(path.relative_to(root)),
                        function=_enclosing_function(source, offset),
                        verb=verb,
                        table=table,
                    )
                )
    return sorted(sites)


def _code_without_comments_or_strings(source: str) -> str:
    """Mask Rust comments and strings while preserving code offsets/newlines."""
    masked = list(source)
    index = 0
    length = len(source)

    def blank(start: int, end: int) -> None:
        for position in range(start, end):
            if masked[position] != "\n":
                masked[position] = " "

    while index < length:
        if source.startswith("//", index):
            end = source.find("\n", index + 2)
            end = length if end < 0 else end
            blank(index, end)
            index = end
            continue
        if source.startswith("/*", index):
            start = index
            depth = 1
            index += 2
            while index < length and depth:
                if source.startswith("/*", index):
                    depth += 1
                    index += 2
                elif source.startswith("*/", index):
                    depth -= 1
                    index += 2
                else:
                    index += 1
            blank(start, index)
            continue
        raw_prefix = index + 1 if source.startswith("br", index) else index
        if raw_prefix < length and source[raw_prefix] == "r":
            cursor = raw_prefix + 1
            while cursor < length and source[cursor] == "#":
                cursor += 1
            if cursor < length and source[cursor] == '"':
                terminator = '"' + source[raw_prefix + 1 : cursor]
                end = source.find(terminator, cursor + 1)
                end = length if end < 0 else end + len(terminator)
                blank(index, end)
                index = end
                continue
        quote_offset = 1 if source.startswith('b"', index) else 0
        if source.startswith('"', index) or quote_offset:
            start = index
            index += quote_offset + 1
            while index < length:
                if source[index] == "\\":
                    index += 2
                elif source[index] == '"':
                    index += 1
                    break
                else:
                    index += 1
            blank(start, min(index, length))
            continue
        index += 1
    return "".join(masked)


def scan_helper_callers(root: Path) -> dict[str, Counter[tuple[str, str]]]:
    """Return exact immediate caller counts for inventoried mutation helpers."""
    observed = {helper: Counter() for helper in HELPERS}
    for path in sorted(root.rglob("*.rs")):
        raw = path.read_text(encoding="utf-8")
        source = _code_without_comments_or_strings(raw)
        for helper in HELPERS:
            for match in re.finditer(rf"\b{re.escape(helper)}\s*\(", source):
                if re.search(r"\bfn\s*$", source[max(0, match.start() - 12) : match.start()]):
                    continue
                function = _enclosing_function(source, match.start())
                observed[helper][(str(path.relative_to(root)), function)] += 1
    return observed


def validate_engine_tree(
    root: Path,
    *,
    inventory: Iterable[MutationSite],
    helper_callers: dict[str, Counter[tuple[str, str]]],
) -> None:
    """Fail closed when mutation signatures or helper callers drift."""
    observed = Counter(scan_mutations(root))
    expected = Counter(inventory)
    if observed != expected:
        raise VirtualMutationAuditError(
            f"unclassified virtual mutation: observed={observed!r}, expected={expected!r}"
        )
    observed_callers = scan_helper_callers(root)
    if observed_callers != helper_callers:
        raise VirtualMutationAuditError(
            "unclassified virtual mutation helper caller: "
            f"observed={observed_callers!r}, expected={helper_callers!r}"
        )


PRODUCTION_INVENTORY = [
    MutationSite("lib.rs", function, verb, table)
    for function, verb, table in [
        ("clear_attribute_projection", "DELETE FROM", "property_search_index"),
        ("commit_projection_outcomes", "INSERT OR IGNORE INTO", "vector_default"),
        ("commit_projection_outcomes", "INSERT OR IGNORE INTO", "vector_default"),
        ("delete_row_owned_projection", "DELETE FROM", "<allowlisted>"),
        ("delete_vector_partition_row", "DELETE FROM", "vector_default"),
        ("migrate_vector_partition_pack1_to_pack2", "DROP TABLE", "vector_default"),
        ("migrate_vector_partition_pack1_to_pack2", "INSERT INTO", "vector_default"),
        ("migrate_vector_partition_to_pack1", "DROP TABLE", "vector_default"),
        ("migrate_vector_partition_to_pack1", "INSERT INTO", "vector_default"),
        ("project_canonical_edge_row", "INSERT INTO", "search_index_edges"),
        ("project_canonical_node_row", "INSERT INTO", "search_index"),
        ("project_canonical_node_row", "INSERT INTO", "search_index_v2"),
        ("project_one_attribute", "INSERT INTO", "property_search_index"),
        ("refresh_vector_attr_values_for_row", "UPDATE", "vector_default"),
        ("reshape_vector_partition_nondestructive", "DROP TABLE", "vector_default"),
        ("reshape_vector_partition_nondestructive", "INSERT INTO", "vector_default"),
        ("run_pin_and_requantize_pass", "INSERT INTO", "vector_default"),
        ("truncate_row_projections_in", "DELETE FROM", "<allowlisted>"),
        ("vector_partition_create_sql", "CREATE VIRTUAL TABLE", "vector_default"),
        ("write_vector_for_test", "INSERT INTO", "vector_default"),
    ]
]

PRODUCTION_HELPER_CALLERS = {
    "delete_vector_partition_row": Counter(
        {
            ("lib.rs", "apply_batch_in_transaction"): 2,
            ("lib.rs", "prune_edge_projection_shadows"): 1,
            ("lib.rs", "run_pin_and_requantize_pass"): 1,
            ("lib.rs", "prune_orphaned_edge_vectors"): 1,
            ("lib.rs", "commit_projection_outcomes"): 1,
            ("lib.rs", "delete_row_owned_projection"): 1,
        }
    ),
    "delete_row_owned_projection": Counter(
        {
            ("lib.rs", "erase_row_projections"): 1,
            ("lib.rs", "purge_row_projections_for_cursor_in"): 1,
        }
    ),
    "truncate_row_projections_in": Counter(
        {
            ("lib.rs", "rebuild_shadow_state"): 1,
            ("lib.rs", "reproject_search_index_after_tokenizer_upgrade"): 1,
            ("lib.rs", "truncate_all_row_projections"): 1,
        }
    ),
}
