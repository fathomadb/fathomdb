#!/usr/bin/env python3
"""Guard open result carriers, embedder admission, and GPU witness ownership."""

from pathlib import Path
import re
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
ENGINE_SRC = REPO_ROOT / "src/rust/crates/fathomdb-engine/src"
TYPES = ("OpenReport", "OpenedEngine", "EmbedderChoice", "LoaderInfo")
CONSTANTS = (
    "DEFAULT_EMBEDDER_NAME",
    "DEFAULT_EMBEDDER_REVISION",
    "DEFAULT_EMBEDDER_DIMENSION",
    "BGE_SMALL_EMBEDDER_NAME",
    "ENV_GPU_ALLOCATION_WITNESS",
    "EXPLANATION_OPEN_NONCE_SEQUENCE",
)
FUNCTIONS = {
    "default_embedder_identity": 1,
    "check_embedder_profile": 1,
    "parse_gpu_allocation_witness_opt_in": 1,
    "witness_gpu_allocation_if_requested": 1,
    "gpu_allocation_witness_refusal": 1,
    "run_requested_gpu_allocation_witness": 2,
    "mint_explanation_open_nonce": 1,
}
PUBLIC_ROOT = (
    "OpenReport",
    "OpenedEngine",
    "EmbedderChoice",
    "ENV_GPU_ALLOCATION_WITNESS",
)


def owner_errors(root: str, owner: str) -> list[str]:
    errors = []
    declaration = r"^(?:(?:pub(?:\([^)]*\))?) )?"
    root_items = re.compile(
        declaration + r"(?:(?:struct|enum|const|static) "
        r"((?:OpenReport|OpenedEngine|EmbedderChoice|LoaderInfo|"
        r"DEFAULT_EMBEDDER_\w*|BGE_SMALL_EMBEDDER_NAME|"
        r"ENV_GPU_ALLOCATION_WITNESS|EXPLANATION_OPEN_NONCE_SEQUENCE)\w*)\b|"
        r"fn ((?:default_embedder\w*|check_embedder_profile\w*|"
        r"parse_gpu_allocation_witness\w*|witness_gpu_allocation\w*|"
        r"gpu_allocation_witness\w*|run_requested_gpu_allocation_witness\w*|"
        r"mint_explanation_open_nonce\w*))\()",
        re.M,
    )
    for match in root_items.finditer(root):
        errors.append(
            f"root still defines open embedder::{match.group(1) or match.group(2)}"
        )
    if re.search(r"^impl OpenedEngine \{", root, re.M):
        errors.append("root still defines impl OpenedEngine")

    for name in TYPES:
        kind = "enum" if name == "EmbedderChoice" else "struct"
        pattern = re.compile(declaration + kind + " " + name + r"\b", re.M)
        if len(pattern.findall(owner)) != 1:
            errors.append(f"open owner does not define one {name}")
    if len(re.findall(r"^impl OpenedEngine \{", owner, re.M)) != 1:
        errors.append("open owner does not define one impl OpenedEngine")
    for name in CONSTANTS:
        kind = "static" if name == "EXPLANATION_OPEN_NONCE_SEQUENCE" else "const"
        pattern = re.compile(declaration + kind + " " + name + r"\b", re.M)
        if len(pattern.findall(owner)) != 1:
            errors.append(f"open owner does not define one {name}")
    for name, expected in FUNCTIONS.items():
        pattern = re.compile(declaration + r"fn " + name + r"\(", re.M)
        if len(pattern.findall(owner)) != expected:
            errors.append(f"open owner does not define {expected} {name}")

    arms = (
        'all(feature = "default-embedder", feature = "embed-cuda")',
        'all(feature = "default-embedder", not(feature = "embed-cuda"))',
    )
    for predicate in arms:
        pattern = re.compile(
            r"^#\[cfg\(" + re.escape(predicate) + r"\)\]\n"
            r"fn run_requested_gpu_allocation_witness\(",
            re.M,
        )
        if len(pattern.findall(owner)) != 1:
            errors.append(f"open owner lacks cfg({predicate}) GPU witness arm")
    root_exports = " ".join(
        match.group(1)
        for match in re.finditer(r"^pub use open::\{(.*?)\};", root, re.M | re.S)
    )
    for name in PUBLIC_ROOT:
        if not re.search(r"\b" + name + r"\b", root_exports):
            errors.append(f"root does not re-export open::{name}")
    return errors


class OpenEmbedderOwnerTest(unittest.TestCase):
    def test_current_source_has_one_open_embedder_owner(self) -> None:
        root = (ENGINE_SRC / "lib.rs").read_text()
        owner = (ENGINE_SRC / "open.rs").read_text()
        self.assertEqual(owner_errors(root, owner), [])

    def test_unlisted_root_item_fails_with_complete_owner(self) -> None:
        root = (ENGINE_SRC / "lib.rs").read_text()
        owner = (ENGINE_SRC / "open.rs").read_text()
        self.assertEqual(owner_errors(root, owner), [])
        for injected, name in (
            ("fn default_embedder_new() {}", "default_embedder_new"),
            ("fn check_embedder_profile_new() {}", "check_embedder_profile_new"),
            (
                "fn parse_gpu_allocation_witness_new() {}",
                "parse_gpu_allocation_witness_new",
            ),
            ("fn witness_gpu_allocation_new() {}", "witness_gpu_allocation_new"),
            ("fn gpu_allocation_witness_new() {}", "gpu_allocation_witness_new"),
            (
                "fn run_requested_gpu_allocation_witness_new() {}",
                "run_requested_gpu_allocation_witness_new",
            ),
            (
                "fn mint_explanation_open_nonce_new() {}",
                "mint_explanation_open_nonce_new",
            ),
            ("struct OpenReportExtra {}", "OpenReportExtra"),
            ("struct LoaderInfoExtra {}", "LoaderInfoExtra"),
            ("const DEFAULT_EMBEDDER_NEW: u32 = 1;", "DEFAULT_EMBEDDER_NEW"),
            (
                'const ENV_GPU_ALLOCATION_WITNESS_NEW: &str = "x";',
                "ENV_GPU_ALLOCATION_WITNESS_NEW",
            ),
            (
                "static EXPLANATION_OPEN_NONCE_SEQUENCE_NEW: u64 = 1;",
                "EXPLANATION_OPEN_NONCE_SEQUENCE_NEW",
            ),
        ):
            with self.subTest(name=name):
                self.assertIn(
                    f"root still defines open embedder::{name}",
                    owner_errors(root + "\n" + injected, owner),
                )

    def test_cfg_twin_predicates_are_exact(self) -> None:
        root = (ENGINE_SRC / "lib.rs").read_text()
        owner = (ENGINE_SRC / "open.rs").read_text()
        self.assertEqual(owner_errors(root, owner), [])
        mutated = owner.replace(
            '#[cfg(all(feature = "default-embedder", not(feature = "embed-cuda")))]',
            '#[cfg(all(feature = "default-embedder", feature = "embed-cuda"))]',
            1,
        )
        self.assertIn(
            'open owner lacks cfg(all(feature = "default-embedder", not(feature = "embed-cuda"))) GPU witness arm',
            owner_errors(root, mutated),
        )

    def test_missing_public_reexport_fails(self) -> None:
        root = (ENGINE_SRC / "lib.rs").read_text()
        owner = (ENGINE_SRC / "open.rs").read_text()
        self.assertEqual(owner_errors(root, owner), [])
        missing, changed = re.subn(
            r"(^pub use open::\{[^}]*?)\bOpenReport\b",
            r"\1RemovedOpenReport",
            root,
            count=1,
            flags=re.M | re.S,
        )
        self.assertEqual(changed, 1)
        self.assertIn(
            "root does not re-export open::OpenReport", owner_errors(missing, owner)
        )

    def test_fast_tier_runs_this_guard(self) -> None:
        registration = (
            "run_tier_suite fast test-slice90-open-embedder-owner "
            "python3 scripts/tests/test_slice90_open_embedder_owner.py"
        )
        self.assertIn(registration, (REPO_ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
