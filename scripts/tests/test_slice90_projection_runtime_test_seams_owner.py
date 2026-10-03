#!/usr/bin/env python3
"""Guard the approved Engine projection-runtime test seams and cfg paths."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
GATES = {
    "pause_projection_worker_after_wal_transaction_for_test": "#[cfg(debug_assertions)]",
    "pause_projection_worker_while_queued_for_test": '#[cfg(feature = "test-hooks")]',
    "pause_projection_worker_before_write_lock_for_test": '#[cfg(feature = "test-hooks")]',
    "force_next_projection_commit_failure_for_test": "#[cfg(debug_assertions)]",
    "force_next_projection_storage_failure_for_test": "#[cfg(debug_assertions)]",
    "pause_projection_commit_failure_cleanup_for_test": "#[cfg(debug_assertions)]",
    "acknowledge_projection_stop_for_test": "#[cfg(debug_assertions)]",
    "set_projection_scheduler_frozen_for_test": None,
    "projection_scheduler_pending_scan_for_test": None,
    "set_projection_retry_delays_for_test": None,
    "set_embed_timeout_ms_for_test": None,
    "projection_status_for_test": None,
    "has_vector_for_cursor_for_test": None,
    "projection_failure_count_for_test": None,
}
FAMILY_NAME = (
    r"(?:pause_projection_worker_|force_next_projection_|"
    r"pause_projection_commit_failure_cleanup|acknowledge_projection_stop|"
    r"set_projection_scheduler_frozen|projection_scheduler_pending_scan|"
    r"set_projection_retry_delays|set_embed_timeout_ms|projection_status_for_test|"
    r"has_vector_for_cursor|projection_failure_count)\w*"
)
FAMILY = re.compile(
    r"^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?fn[ \t]+(" + FAMILY_NAME + r")\b",
    re.M,
)
DECLARATION = re.compile(r"^[ \t]*(pub(?:\([^)]*\))?[ \t]+)?fn[ \t]+([a-z_]\w*)\b", re.M)
CFG = re.compile(r"#\[\s*cfg(?:_attr)?\b")


def prelude(source: str, start: int) -> list[str]:
    found = []
    for line in reversed(source[:start].splitlines()):
        stripped = line.strip()
        if stripped.startswith("#["):
            found.append(stripped)
        elif stripped and not stripped.startswith(("///", "//")):
            break
    return found


def source_inventory() -> tuple[str, str, dict[str, str]]:
    root = (SRC / "lib.rs").read_text()
    owner = (SRC / "projection_runtime.rs").read_text()
    other = {
        path.relative_to(SRC).as_posix(): path.read_text()
        for path in sorted(SRC.rglob("*.rs"))
        if path.relative_to(SRC).as_posix() not in ("lib.rs", "projection_runtime.rs")
    }
    return root, owner, other


def owner_errors(root: str, owner: str, other: dict[str, str]) -> list[str]:
    errors = [f"root wrongly defines {name}" for name in FAMILY.findall(root)]
    for path, source in other.items():
        errors.extend(f"{path} wrongly defines {name}" for name in FAMILY.findall(source))
    root_module = re.search(r"^mod projection_runtime;", root, re.M)
    if not root_module or any(CFG.match(attr) for attr in prelude(root, root_module.start())):
        errors.append("root gates projection runtime module")
    if re.search(r"(?m)^[ \t]*#!\[\s*cfg(?:_attr)?\b", owner):
        errors.append("projection runtime gates whole owner module")
    impls = list(re.finditer(r"(?ms)^impl Engine \{.*?^}", owner))
    for impl in impls:
        if any(CFG.match(attr) for attr in prelude(owner, impl.start())):
            errors.append("projection runtime gates Engine impl")
    methods = [(method, match, impl) for impl in impls for match in DECLARATION.finditer(impl.group())
               if (method := match.group(2)) in GATES or re.fullmatch(FAMILY_NAME, method)]
    for name in GATES:
        found = [(match, impl) for method, match, impl in methods if method == name]
        if len(found) != 1 or found[0][0].group(1) != "pub ":
            errors.append(f"projection runtime lacks public {name}")
            continue
        match, impl = found[0]
        attrs = prelude(impl.group(), match.start())
        actual_cfg = [attr for attr in attrs if CFG.match(attr)]
        expected_cfg = [GATES[name].strip()] if GATES[name] else []
        if actual_cfg != expected_cfg or "#[doc(hidden)]" not in attrs:
            errors.append(f"projection runtime changes attrs {name}")
        if name == "pause_projection_worker_after_wal_transaction_for_test" and "#[allow(dead_code)]" not in attrs:
            errors.append("projection runtime drops pause dead_code annotation")
    for name, _, _ in methods:
        if name not in GATES:
            errors.append(f"projection runtime adds unlisted {name}")
    return errors


def complete_owner_fixture() -> tuple[str, str, dict[str, str]]:
    root, owner, other = source_inventory()
    moved = []
    for name in GATES:
        pattern = re.compile(
            r"(?m)^(?:    (?:///[^\n]*|#\[[^\n]*\])\n)*    pub fn " + name + r"\b[\s\S]*?^    }\n"
        )
        match = pattern.search(root)
        if match:
            moved.append(match.group())
            root = root[:match.start()] + root[match.end():]
    if moved:
        owner += "\nimpl Engine {\n" + "\n".join(moved) + "}\n"
    return root, owner, other


class ProjectionRuntimeTestSeamsOwnerTest(unittest.TestCase):
    def test_current_source_has_one_owner(self) -> None:
        self.assertEqual(owner_errors(*source_inventory()), [])

    def test_complete_owner_fixture_and_all_source_mutants(self) -> None:
        root, owner, other = complete_owner_fixture()
        self.assertEqual(owner_errors(root, owner, other), [])
        for declaration, name in (
            ("impl Engine {\n    fn projection_status_for_test_new(&self) {}\n}", "projection_status_for_test_new"),
            ("impl Engine {\n    pub(crate) fn set_embed_timeout_ms_for_test(&self) {}\n}", "set_embed_timeout_ms_for_test"),
        ):
            self.assertIn(f"root wrongly defines {name}", owner_errors(root + "\n" + declaration, owner, other))
        for path in ("lifecycle.rs", "future_owner.rs"):
            changed = other | {path: other.get(path, "") + "\nimpl Engine {\n    fn projection_failure_count_new(&self) {}\n}"}
            self.assertIn(f"{path} wrongly defines projection_failure_count_new", owner_errors(root, owner, changed))
        changed = owner + "\nimpl Engine {\n    pub fn projection_status_for_test_new(&self) {}\n}"
        self.assertIn("projection runtime adds unlisted projection_status_for_test_new", owner_errors(root, changed, other))

    def test_cfg_cfg_attr_mutants(self) -> None:
        root, owner, other = complete_owner_fixture()
        self.assertEqual(owner_errors(root, owner, other), [])
        for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            changed = owner.replace("impl Engine {", attr + "\nimpl Engine {", 1)
            self.assertIn("projection runtime gates Engine impl", owner_errors(root, changed, other))
            changed = owner.replace("    pub fn projection_status_for_test(", "    " + attr + "\n    pub fn projection_status_for_test(", 1)
            self.assertIn("projection runtime changes attrs projection_status_for_test", owner_errors(root, changed, other))
            changed = root.replace("mod projection_runtime;", attr + "\nmod projection_runtime;", 1)
            self.assertIn("root gates projection runtime module", owner_errors(changed, owner, other))
            inner = "#!" + attr[1:]
            self.assertIn("projection runtime gates whole owner module", owner_errors(root, inner + "\n" + owner, other))
        changed = owner.replace("    #[cfg(debug_assertions)]\n    #[doc(hidden)]\n    pub fn force_next_projection_commit_failure_for_test(", "    #[doc(hidden)]\n    pub fn force_next_projection_commit_failure_for_test(", 1)
        self.assertIn("projection runtime changes attrs force_next_projection_commit_failure_for_test", owner_errors(root, changed, other))

    def test_fast_registration(self) -> None:
        self.assertIn("fast test-slice90-projection-runtime-test-seams-owner", (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
