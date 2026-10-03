#!/usr/bin/env python3
"""Guard embedding readiness, dense capability, and event ownership."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
TYPES = ("EmbeddingReadinessState", "EmbeddingOperation", "EmbedderRequired", "EmbeddingReadiness")
METHODS = ("usable_dense_runtime", "drain_embedder_events")


def prelude(source: str, start: int) -> list[str]:
    lines = []
    for line in reversed(source[:start].splitlines()):
        stripped = line.strip()
        if stripped.startswith(("///", "#[")) or not stripped:
            lines.append(stripped)
        else:
            break
    return lines


def gates(source: str, start: int) -> bool:
    return any(re.match(r"#\[\s*cfg(?:_attr)?\b", line) for line in prelude(source, start))


def owner_errors(root: str, owner: str, generation: str, worker: str) -> list[str]:
    errors = []
    for name in re.findall(r"^(?:pub(?:\([^)]*\))? )?(?:struct|enum|type|trait) ((?:Embedding|EmbedderRequired)\w*)\b", root, re.M):
        errors.append(f"root still defines embedding::{name}")
    for name in re.findall(r"^impl[^\n{]*\b((?:Embedding|EmbedderRequired)\w*)\b[^\n{]*\{", root, re.M):
        errors.append(f"root still implements embedding::{name}")
    for name in re.findall(r"^    (?:pub(?:\([^)]*\))? )?fn ((?:usable_dense_runtime|drain_embedder_events)\w*)\(", root, re.M):
        errors.append(f"root still defines Engine::{name}")
    for name in re.findall(r"^(?:pub(?:\([^)]*\))? )?fn (embedder_required_for\w*)\(", root, re.M):
        errors.append(f"root still defines embedding helper {name}")
    for name in re.findall(r"^(?:pub(?:\([^)]*\))? )?const (EDGE_FACT_\w*)\b", root, re.M):
        errors.append(f"root still defines embedding::{name}")
    for name in TYPES:
        kind = "struct" if name in ("EmbedderRequired", "EmbeddingReadiness") else "enum"
        found = list(re.finditer(r"^pub " + kind + " " + name + r"\b", owner, re.M))
        if len(found) != 1:
            errors.append(f"embedding lacks one {name}")
        elif gates(owner, found[0].start()):
            errors.append(f"embedding gates always-on {name}")
    for name in METHODS:
        visibility = "pub " if name == "drain_embedder_events" else "pub\\(crate\\) "
        found = list(re.finditer(r"^    " + visibility + "fn " + name + r"\(", owner, re.M))
        if len(found) != 1:
            errors.append(f"embedding lacks one Engine::{name}")
        elif gates(owner, found[0].start()):
            errors.append(f"embedding gates always-on Engine::{name}")
    helper = list(re.finditer(r"^pub\(crate\) fn embedder_required_for\(", owner, re.M))
    if len(helper) != 1:
        errors.append("embedding lacks embedder_required_for")
    elif gates(owner, helper[0].start()):
        errors.append("embedding gates always-on embedder_required_for")
    kind = list(re.finditer(r"^pub\(crate\) const EDGE_FACT_KIND\b", owner, re.M))
    if len(kind) != 1:
        errors.append("embedding lacks EDGE_FACT_KIND")
    elif gates(owner, kind[0].start()):
        errors.append("embedding gates always-on EDGE_FACT_KIND")
    for name in ("EmbeddingReadinessState", "EmbeddingOperation"):
        implementations = list(re.finditer(r"^impl " + name + r" \{", owner, re.M))
        if len(implementations) != 1:
            errors.append(f"embedding lacks one impl {name}")
        elif gates(owner, implementations[0].start()):
            errors.append(f"embedding gates always-on impl {name}")
    exports = " ".join(re.findall(r"^pub use embedding::\{(.*?)\};", root, re.M | re.S))
    for name in TYPES:
        if not re.search(r"\b" + name + r"\b", exports):
            errors.append(f"root lacks embedding::{name} reexport")
    if "use crate::embedding::embedder_required_for;" not in generation:
        errors.append("projection generation does not import readiness owner")
    if "use crate::embedding::EDGE_FACT_KIND;" not in worker:
        errors.append("projection worker does not import edge fact owner")
    return errors


class EmbeddingOwnerTest(unittest.TestCase):
    def sources(self) -> tuple[str, str, str, str]:
        return tuple((SRC / name).read_text() for name in ("lib.rs", "embedding.rs", "projection_generation.rs", "projection_worker.rs"))

    def test_current_source_has_one_owner(self) -> None:
        self.assertEqual(owner_errors(*self.sources()), [])

    def test_complete_owner_rejects_unlisted_root_family(self) -> None:
        root, owner, generation, worker = self.sources()
        self.assertEqual(owner_errors(root, owner, generation, worker), [])
        for declaration, expected in (
            ("struct EmbeddingNew;", "root still defines embedding::EmbeddingNew"),
            ("pub(crate) enum EmbedderRequiredNew { A }", "root still defines embedding::EmbedderRequiredNew"),
            ("impl EmbeddingReadiness { fn extra(&self) {} }", "root still implements embedding::EmbeddingReadiness"),
            ("impl SomeTrait for EmbeddingReadiness {}", "root still implements embedding::EmbeddingReadiness"),
            ("impl Engine {\n    fn usable_dense_runtime_new(&self) {}\n}", "root still defines Engine::usable_dense_runtime_new"),
            ("impl Engine {\n    pub(crate) fn drain_embedder_events_new(&self) {}\n}", "root still defines Engine::drain_embedder_events_new"),
            ("fn embedder_required_for_new() {}", "root still defines embedding helper embedder_required_for_new"),
            ("const EDGE_FACT_NEW: &str = \"x\";", "root still defines embedding::EDGE_FACT_NEW"),
        ):
            with self.subTest(declaration=declaration):
                self.assertIn(expected, owner_errors(root + "\n" + declaration, owner, generation, worker))

    def test_cfg_and_reexport_mutants(self) -> None:
        root, owner, generation, worker = self.sources()
        self.assertEqual(owner_errors(root, owner, generation, worker), [])
        for method in METHODS:
            visibility = "pub " if method == "drain_embedder_events" else "pub(crate) "
            marker = f"    {visibility}fn {method}("
            for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "operator"))]'):
                altered = owner.replace(marker, f"    {attr}\n" + marker, 1)
                with self.subTest(method=method, attr=attr):
                    self.assertNotEqual(altered, owner)
                    self.assertIn(f"embedding gates always-on Engine::{method}", owner_errors(root, altered, generation, worker))
        for name in TYPES:
            kind = "struct" if name in ("EmbedderRequired", "EmbeddingReadiness") else "enum"
            marker = f"pub {kind} {name}"
            for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "operator"))]'):
                altered = owner.replace(marker, attr + "\n" + marker, 1)
                with self.subTest(type=name, attr=attr):
                    self.assertNotEqual(altered, owner)
                    self.assertIn(f"embedding gates always-on {name}", owner_errors(root, altered, generation, worker))
        for marker, expected in (
            ("impl EmbeddingReadinessState {", "embedding gates always-on impl EmbeddingReadinessState"),
            ("impl EmbeddingOperation {", "embedding gates always-on impl EmbeddingOperation"),
            ("pub(crate) fn embedder_required_for(", "embedding gates always-on embedder_required_for"),
            ("pub(crate) const EDGE_FACT_KIND", "embedding gates always-on EDGE_FACT_KIND"),
        ):
            altered = owner.replace(marker, '#[cfg_attr(feature = "default-embedder", cfg(feature = "operator"))]\n' + marker, 1)
            with self.subTest(marker=marker):
                self.assertNotEqual(altered, owner)
                self.assertIn(expected, owner_errors(root, altered, generation, worker))
        altered = root.replace("EmbeddingReadiness, EmbeddingReadinessState,", "", 1)
        self.assertNotEqual(altered, root)
        self.assertIn("root lacks embedding::EmbeddingReadiness reexport", owner_errors(altered, owner, generation, worker))

    def test_fast_tier_registration(self) -> None:
        self.assertIn("fast test-slice90-embedding-owner", (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
