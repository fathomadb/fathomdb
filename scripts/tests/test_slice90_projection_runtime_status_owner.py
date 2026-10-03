#!/usr/bin/env python3
"""Guard projection-runtime status carriers, cursor interpretation, and constants."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
TYPES = {
    "ProjectionRuntimeUnavailabilityReason": "enum",
    "ProjectionStatusDenseReadiness": "enum",
    "ProjectionRuntimeStatusEntry": "struct",
    "ProjectionRuntimeStatus": "struct",
}
IMPLS = ("ProjectionRuntimeUnavailabilityReason", "ProjectionStatusDenseReadiness")
CONSTANTS = (
    "PROJECTION_CURSOR_KEY",
    "PROJECTION_TEMPORAL_WAKE_POLL",
    "PROJECTION_RUNTIME_STARTUP_TIMEOUT",
    "DEFAULT_PROJECTION_RETRY_DELAYS_MS",
)
TYPE_FAMILY = r"(?:ProjectionRuntime(?:UnavailabilityReason|Status(?:Entry)?)|ProjectionStatusDenseReadiness)\w*"
CONST_FAMILY = r"(?:PROJECTION_CURSOR_KEY|PROJECTION_TEMPORAL_WAKE_POLL|PROJECTION_RUNTIME_STARTUP_TIMEOUT|DEFAULT_PROJECTION_RETRY_DELAYS_MS)\w*"


def gated(source: str, start: int) -> bool:
    for line in reversed(source[:start].splitlines()):
        stripped = line.strip()
        if re.match(r"#\[\s*cfg(?:_attr)?\b", stripped):
            return True
        if stripped and not stripped.startswith(("///", "#[")):
            break
    return False


def owner_errors(root: str, owner: str, search: str, commit: str, worker: str) -> list[str]:
    errors = []
    for name in re.findall(r"^(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum|type|trait|union)\s+(" + TYPE_FAMILY + r")\b", root, re.M):
        errors.append(f"root still defines runtime::{name}")
    for name in re.findall(r"^impl\b[^\n{]*\b(" + TYPE_FAMILY + r")\b[^\n{]*\{", root, re.M):
        errors.append(f"root still implements runtime::{name}")
    for name in re.findall(r"^(?:pub(?:\([^)]*\))?\s+)?const\s+(" + CONST_FAMILY + r")\b", root, re.M):
        errors.append(f"root still defines runtime::{name}")
    for name in re.findall(r"^(?:pub(?:\([^)]*\))?\s+)?fn\s+(projection_status\w*)\s*\(", root, re.M):
        errors.append(f"root still defines runtime helper {name}")
    for name, kind in TYPES.items():
        found = list(re.finditer(r"^pub " + kind + " " + name + r"\b", owner, re.M))
        if len(found) != 1:
            errors.append(f"runtime lacks one {name}")
        elif gated(owner, found[0].start()):
            errors.append(f"runtime gates always-on {name}")
    for name in IMPLS:
        found = list(re.finditer(r"^impl " + name + r" \{", owner, re.M))
        if len(found) != 1:
            errors.append(f"runtime lacks one impl {name}")
        elif gated(owner, found[0].start()):
            errors.append(f"runtime gates always-on impl {name}")
        if len(found) == 1:
            body_end = owner.find("\n}", found[0].end())
            body = owner[found[0].end():body_end]
            methods = list(re.finditer(r"^    pub fn as_str\(", body, re.M))
            if len(methods) != 1:
                errors.append(f"runtime lacks one {name}::as_str")
            elif gated(body, methods[0].start()):
                errors.append(f"runtime gates always-on {name}::as_str")
    for name in CONSTANTS:
        visibility = "pub\\(crate\\) " if name in ("PROJECTION_CURSOR_KEY", "PROJECTION_TEMPORAL_WAKE_POLL") else ""
        found = list(re.finditer(r"^" + visibility + "const " + name + r"\b", owner, re.M))
        if len(found) != 1:
            errors.append(f"runtime lacks one {name}")
        elif gated(owner, found[0].start()):
            errors.append(f"runtime gates always-on {name}")
    helper = list(re.finditer(r"^pub\(crate\) fn projection_status\(", owner, re.M))
    if len(helper) != 1:
        errors.append("runtime lacks one projection_status")
    elif gated(owner, helper[0].start()):
        errors.append("runtime gates always-on projection_status")
    exports = " ".join(re.findall(r"^pub use projection_runtime::\{(.*?)\};", root, re.M | re.S))
    for name in TYPES:
        if not re.search(r"\b" + name + r"\b", exports):
            errors.append(f"root lacks runtime::{name} reexport")
    for path, source, marker in (
        ("search", search, "use crate::projection_runtime::PROJECTION_CURSOR_KEY;"),
        ("commit", commit, "use crate::projection_runtime::PROJECTION_CURSOR_KEY;"),
        ("worker", worker, "use crate::projection_runtime::PROJECTION_TEMPORAL_WAKE_POLL;"),
    ):
        if marker not in source:
            errors.append(f"{path} lacks runtime-owner import")
    return errors


class ProjectionRuntimeStatusOwnerTest(unittest.TestCase):
    def sources(self) -> tuple[str, str, str, str, str]:
        return tuple((SRC / name).read_text() for name in ("lib.rs", "projection_runtime.rs", "search.rs", "projection_commit.rs", "projection_worker.rs"))

    def test_current_source_has_one_owner(self) -> None:
        self.assertEqual(owner_errors(*self.sources()), [])

    def test_complete_owner_rejects_unlisted_root_family(self) -> None:
        root, owner, search, commit, worker = self.sources()
        self.assertEqual(owner_errors(root, owner, search, commit, worker), [])
        for declaration, expected in (
            ("struct ProjectionRuntimeStatusNew;", "root still defines runtime::ProjectionRuntimeStatusNew"),
            ("pub(crate) enum ProjectionStatusDenseReadinessNew { A }", "root still defines runtime::ProjectionStatusDenseReadinessNew"),
            ("pub enum ProjectionRuntimeUnavailabilityReasonNew { A }", "root still defines runtime::ProjectionRuntimeUnavailabilityReasonNew"),
            ("impl ProjectionRuntimeStatus { fn extra(&self) {} }", "root still implements runtime::ProjectionRuntimeStatus"),
            ("impl SomeTrait for ProjectionStatusDenseReadiness {}", "root still implements runtime::ProjectionStatusDenseReadiness"),
            ("fn projection_status_new() {}", "root still defines runtime helper projection_status_new"),
            ("pub(crate) fn projection_status_new() {}", "root still defines runtime helper projection_status_new"),
            ("pub fn projection_status_new() {}", "root still defines runtime helper projection_status_new"),
            ("const PROJECTION_CURSOR_KEY_NEW: &str = \"x\";", "root still defines runtime::PROJECTION_CURSOR_KEY_NEW"),
            ("pub(crate) const PROJECTION_RUNTIME_STARTUP_TIMEOUT_NEW: u64 = 1;", "root still defines runtime::PROJECTION_RUNTIME_STARTUP_TIMEOUT_NEW"),
        ):
            with self.subTest(declaration=declaration):
                self.assertIn(expected, owner_errors(root + "\n" + declaration, owner, search, commit, worker))

    def test_cfg_and_reexport_mutants(self) -> None:
        root, owner, search, commit, worker = self.sources()
        self.assertEqual(owner_errors(root, owner, search, commit, worker), [])
        attrs = ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "operator"))]')
        markers = [(f"pub {kind} {name} {{", f"runtime gates always-on {name}") for name, kind in TYPES.items()]
        markers += [(f"impl {name} {{", f"runtime gates always-on impl {name}") for name in IMPLS]
        markers += [
            (
                ("pub(crate) " if name in ("PROJECTION_CURSOR_KEY", "PROJECTION_TEMPORAL_WAKE_POLL") else "") + f"const {name}",
                f"runtime gates always-on {name}",
            )
            for name in CONSTANTS
        ]
        markers += [("pub(crate) fn projection_status(", "runtime gates always-on projection_status")]
        for marker, expected in markers:
            for attr in attrs:
                altered = owner.replace(marker, attr + "\n" + marker, 1)
                with self.subTest(marker=marker, attr=attr):
                    self.assertNotEqual(altered, owner)
                    self.assertIn(expected, owner_errors(root, altered, search, commit, worker))
        for name in IMPLS:
            start = owner.index(f"impl {name} {{")
            end = owner.index("\n}", start)
            block = owner[start:end]
            marker = "    pub fn as_str("
            self.assertIn(marker, block)
            for attr in attrs:
                altered_block = block.replace(marker, "    " + attr + "\n" + marker, 1)
                altered = owner[:start] + altered_block + owner[end:]
                with self.subTest(method=f"{name}::as_str", attr=attr):
                    self.assertIn(f"runtime gates always-on {name}::as_str", owner_errors(root, altered, search, commit, worker))
        altered = root.replace("ProjectionRuntimeStatus,", "", 1)
        self.assertNotEqual(altered, root)
        self.assertIn("root lacks runtime::ProjectionRuntimeStatus reexport", owner_errors(altered, owner, search, commit, worker))

    def test_fast_tier_registration(self) -> None:
        self.assertIn("fast test-slice90-projection-runtime-status-owner", (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
