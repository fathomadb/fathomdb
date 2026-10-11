#!/usr/bin/env python3
"""Guard telemetry counters, controls, and platform RSS ownership."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
METHODS = ("counters", "set_profiling", "set_slow_threshold_ms")
RSS = ("process_current_rss_bytes", "process_peak_rss_bytes")
ARMS = (
    '#[cfg(all(feature = "test-hooks", target_os = "linux"))]',
    '#[cfg(all(feature = "test-hooks", not(target_os = "linux")))]',
)


def attrs(source: str, start: int) -> list[str]:
    found = []
    for line in reversed(source[:start].splitlines()):
        stripped = line.strip()
        if re.match(r"#\[\s*cfg(?:_attr)?\b", stripped):
            found.append(stripped)
        elif stripped and not stripped.startswith(("///", "#[")):
            break
    return found


def owner_errors(root: str, owner: str, lifecycle: str, config: str, graph: str, execution: str) -> list[str]:
    errors = []
    for name in re.findall(r"^(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum|type|trait|union)\s+(CounterSnapshot\w*)\b", root, re.M):
        errors.append(f"root still defines telemetry::{name}")
    for name in re.findall(r"^impl\b[^\n{]*\b(CounterSnapshot\w*)\b[^\n{]*\{", root, re.M):
        errors.append(f"root still implements telemetry::{name}")
    for name in re.findall(r"^(?:pub(?:\([^)]*\))?\s+)?const\s+(DEFAULT_SLOW_THRESHOLD_MS\w*)\b", root, re.M):
        errors.append(f"root still defines telemetry::{name}")
    for name in re.findall(r"^(?:pub(?:\([^)]*\))?\s+)?fn\s+((?:process_current_rss_bytes|process_peak_rss_bytes)\w*)\s*\(", root, re.M):
        errors.append(f"root still defines telemetry helper {name}")
    for name in re.findall(r"^    (?:pub(?:\([^)]*\))?\s+)?fn\s+((?:counters|set_profiling|set_slow_threshold_ms)\w*)\s*\(", root, re.M):
        errors.append(f"root still defines Engine::{name}")
    module = re.search(r"^mod telemetry;", root, re.M)
    if not module or attrs(root, module.start()):
        errors.append("root lacks ungated telemetry module")
    snapshot = list(re.finditer(r"^pub struct CounterSnapshot\b", owner, re.M))
    if len(snapshot) != 1:
        errors.append("telemetry lacks one CounterSnapshot")
    elif attrs(owner, snapshot[0].start()):
        errors.append("telemetry gates always-on CounterSnapshot")
    constant = list(re.finditer(r"^pub\(crate\) const DEFAULT_SLOW_THRESHOLD_MS\b", owner, re.M))
    if len(constant) != 1:
        errors.append("telemetry lacks one DEFAULT_SLOW_THRESHOLD_MS")
    elif attrs(owner, constant[0].start()):
        errors.append("telemetry gates always-on DEFAULT_SLOW_THRESHOLD_MS")
    for name in METHODS:
        found = list(re.finditer(r"^    pub fn " + name + r"\(", owner, re.M))
        if len(found) != 1:
            errors.append(f"telemetry lacks one Engine::{name}")
        elif attrs(owner, found[0].start()):
            errors.append(f"telemetry gates always-on Engine::{name}")
    for name in RSS:
        found = list(re.finditer(r"^pub\(crate\) fn " + name + r"\(", owner, re.M))
        if len(found) != 2:
            errors.append(f"telemetry lacks two {name} cfg arms")
        else:
            actual = [attrs(owner, match.start()) for match in found]
            if actual != [[ARMS[0]], [ARMS[1]]]:
                errors.append(f"telemetry changes {name} cfg arms")
    export = re.search(r"^pub use telemetry::CounterSnapshot;", root, re.M)
    if not export or attrs(root, export.start()):
        errors.append("root lacks ungated telemetry::CounterSnapshot reexport")
    for label, source, marker in (
        ("lifecycle", lifecycle, "use crate::telemetry::CounterSnapshot;"),
        ("configuration", config, "use super::telemetry::DEFAULT_SLOW_THRESHOLD_MS;"),
        ("graph", graph, "crate::telemetry::process_current_rss_bytes()"),
        ("execution", execution, "crate::telemetry::process_current_rss_bytes()"),
    ):
        if marker not in source:
            errors.append(f"{label} lacks telemetry owner path")
    return errors


class TelemetryOwnerTest(unittest.TestCase):
    def sources(self) -> tuple[str, str, str, str, str, str]:
        return tuple((SRC / name).read_text() for name in ("lib.rs", "telemetry.rs", "lifecycle.rs", "runtime_configuration.rs", "graph_api.rs", "graph_expand/execution.rs"))

    def test_current_source_has_one_owner(self) -> None:
        self.assertEqual(owner_errors(*self.sources()), [])

    def test_complete_owner_rejects_unlisted_root_family(self) -> None:
        root, owner, lifecycle, config, graph, execution = self.sources()
        self.assertEqual(owner_errors(root, owner, lifecycle, config, graph, execution), [])
        for declaration, expected in (
            ("struct CounterSnapshotNew;", "root still defines telemetry::CounterSnapshotNew"),
            ("pub(crate) enum CounterSnapshotNew { A }", "root still defines telemetry::CounterSnapshotNew"),
            ("impl SomeTrait for CounterSnapshot {}", "root still implements telemetry::CounterSnapshot"),
            ("const DEFAULT_SLOW_THRESHOLD_MS_NEW: u64 = 1;", "root still defines telemetry::DEFAULT_SLOW_THRESHOLD_MS_NEW"),
            ("pub(crate) fn process_current_rss_bytes_new() {}", "root still defines telemetry helper process_current_rss_bytes_new"),
            ("pub fn process_peak_rss_bytes_new() {}", "root still defines telemetry helper process_peak_rss_bytes_new"),
            ("impl Engine {\n    fn counters_new(&self) {}\n}", "root still defines Engine::counters_new"),
            ("impl Engine {\n    pub(crate) fn set_profiling_new(&self) {}\n}", "root still defines Engine::set_profiling_new"),
        ):
            with self.subTest(declaration=declaration):
                self.assertIn(expected, owner_errors(root + "\n" + declaration, owner, lifecycle, config, graph, execution))

    def test_cfg_and_public_path_mutants(self) -> None:
        root, owner, lifecycle, config, graph, execution = self.sources()
        self.assertEqual(owner_errors(root, owner, lifecycle, config, graph, execution), [])
        for marker, expected in (
            ("pub struct CounterSnapshot {", "telemetry gates always-on CounterSnapshot"),
            ("pub(crate) const DEFAULT_SLOW_THRESHOLD_MS", "telemetry gates always-on DEFAULT_SLOW_THRESHOLD_MS"),
            *((f"    pub fn {name}(", f"telemetry gates always-on Engine::{name}") for name in METHODS),
        ):
            for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
                altered = owner.replace(marker, attr + "\n" + marker, 1)
                with self.subTest(marker=marker, attr=attr):
                    self.assertNotEqual(altered, owner)
                    self.assertIn(expected, owner_errors(root, altered, lifecycle, config, graph, execution))
        for name in RSS:
            marker = f"pub(crate) fn {name}("
            for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "operator"))]'):
                altered = owner.replace(marker, attr + "\n" + marker, 1)
                self.assertIn(f"telemetry changes {name} cfg arms", owner_errors(root, altered, lifecycle, config, graph, execution))
            altered = owner.replace(ARMS[1] + "\n" + marker, ARMS[0] + "\n" + marker, 1)
            self.assertNotEqual(altered, owner)
            self.assertIn(f"telemetry changes {name} cfg arms", owner_errors(root, altered, lifecycle, config, graph, execution))
        for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            altered = root.replace("mod telemetry;", attr + "\nmod telemetry;", 1)
            self.assertIn("root lacks ungated telemetry module", owner_errors(altered, owner, lifecycle, config, graph, execution))
            altered = root.replace("pub use telemetry::CounterSnapshot;", attr + "\npub use telemetry::CounterSnapshot;", 1)
            self.assertIn("root lacks ungated telemetry::CounterSnapshot reexport", owner_errors(altered, owner, lifecycle, config, graph, execution))

    def test_fast_tier_registration(self) -> None:
        self.assertIn("fast test-slice90-telemetry-owner", (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
