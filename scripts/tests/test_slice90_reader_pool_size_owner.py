#!/usr/bin/env python3
"""Guard the reader worker pool size owner across all engine modules."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
NAME = "READER_POOL_SIZE"
DECLARATION = "pub(crate) const READER_POOL_SIZE: usize = 8;"
FAMILY = re.compile(r"^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?const[ \t]+(READER_POOL_SIZE\w*)\b", re.M)
CALLERS = (
    ("open.rs", "use crate::reader_pool::READER_POOL_SIZE;"),
    ("wal_runtime.rs", "use crate::reader_pool::READER_POOL_SIZE;"),
    ("slice90_close_review_tests.rs", "use super::reader_pool::READER_POOL_SIZE;"),
)


def attrs(source: str, start: int) -> list[str]:
    found = []
    for line in reversed(source[:start].splitlines()):
        stripped = line.strip()
        if re.match(r"#\[\s*cfg(?:_attr)?\b", stripped):
            found.append(stripped)
        elif stripped and not stripped.startswith(("///", "#[", "//")):
            break
    return found


def source_inventory() -> tuple[str, str, dict[str, str]]:
    root = (SRC / "lib.rs").read_text()
    owner = (SRC / "reader_pool.rs").read_text()
    modules = {
        path.relative_to(SRC).as_posix(): path.read_text()
        for path in sorted(SRC.rglob("*.rs"))
        if path.relative_to(SRC).as_posix() not in ("lib.rs", "reader_pool.rs")
    }
    return root, owner, modules


def owner_errors(root: str, owner: str, modules: dict[str, str]) -> list[str]:
    errors = [f"root wrongly defines {name}" for name in FAMILY.findall(root)]
    for path, source in modules.items():
        errors.extend(f"{path.removesuffix('.rs')} wrongly defines {name}" for name in FAMILY.findall(source))
    module = re.search(r"^mod reader_pool;", root, re.M)
    if not module or attrs(root, module.start()):
        errors.append("root gates reader_pool module")
    if re.search(r"(?m)^[ \t]*#!\[\s*cfg(?:_attr)?\b", owner):
        errors.append("reader_pool gates whole owner module")
    errors.extend(f"reader_pool unexpectedly defines {name}" for name in FAMILY.findall(owner) if name != NAME)
    constant = list(re.finditer(r"^" + re.escape(DECLARATION) + r"$", owner, re.M))
    if len(constant) != 1:
        errors.append("reader_pool lacks exact size")
    elif attrs(owner, constant[0].start()):
        errors.append("reader_pool gates size")
    for path, marker in CALLERS:
        if marker not in modules.get(path, ""):
            errors.append(f"{path.removesuffix('.rs')} lacks reader_pool owner path")
    attribution = modules.get("wal_attribution.rs", "")
    if "use crate::reader_pool::READER_POOL_SIZE;" in attribution:
        errors.append("wal_attribution cycles through reader_pool")
    if not re.search(r"native_state_expected_roles\(\s*reader_count: usize,\s*worker_count: usize,", attribution) or "(0..reader_count)" not in attribution:
        errors.append("wal_attribution lacks passed reader count")
    if modules.get("wal_runtime.rs", "").count("native_state_expected_roles(READER_POOL_SIZE, worker_count)") != 2:
        errors.append("wal_runtime does not pass reader count")
    if "use super::reader_pool::{ReaderRequest, READER_POOL_SIZE};" not in root:
        errors.append("root test module lacks reader_pool owner path")
    return errors


class ReaderPoolSizeOwnerTest(unittest.TestCase):
    def test_current_source_has_one_owner(self) -> None:
        self.assertEqual(owner_errors(*source_inventory()), [])

    def test_complete_owner_rejects_wrong_module_exact_and_unlisted(self) -> None:
        root, owner, modules = source_inventory()
        root = re.sub(r"(?m)^const READER_POOL_SIZE.*\n", "", root, count=1)
        if DECLARATION not in owner:
            owner += "\n" + DECLARATION
        for path, marker in CALLERS:
            if marker not in modules[path]:
                modules[path] += "\n" + marker
        attribution = modules["wal_attribution.rs"]
        attribution = attribution.replace("use crate::reader_pool::READER_POOL_SIZE;", "")
        attribution = attribution.replace("native_state_expected_roles(\n    worker_count: usize,", "native_state_expected_roles(\n    reader_count: usize,\n    worker_count: usize,")
        attribution = attribution.replace("0..READER_POOL_SIZE", "0..reader_count")
        modules["wal_attribution.rs"] = attribution
        modules["wal_runtime.rs"] = modules["wal_runtime.rs"].replace("native_state_expected_roles(worker_count)", "native_state_expected_roles(READER_POOL_SIZE, worker_count)")
        marker = "use super::reader_pool::{ReaderRequest, READER_POOL_SIZE};"
        if marker not in root:
            root += "\n" + marker
        self.assertEqual(owner_errors(root, owner, modules), [])
        for declaration, name in (
            ("const READER_POOL_SIZE_NEW: usize = 1;", "READER_POOL_SIZE_NEW"),
            ("pub(crate) const READER_POOL_SIZE: usize = 1;", NAME),
        ):
            self.assertIn(f"root wrongly defines {name}", owner_errors(root + "\n" + declaration, owner, modules))
        for path, declaration, name in (
            ("open.rs", "const READER_POOL_SIZE: usize = 1;", NAME),
            ("wal_runtime.rs", "pub(crate) const READER_POOL_SIZE_NEW: usize = 1;", "READER_POOL_SIZE_NEW"),
            ("lifecycle.rs", "pub const READER_POOL_SIZE_NEW: usize = 1;", "READER_POOL_SIZE_NEW"),
        ):
            self.assertIn(path, modules)
            altered = modules | {path: modules[path] + "\n" + declaration}
            self.assertIn(f"{path.removesuffix('.rs')} wrongly defines {name}", owner_errors(root, owner, altered))
        arbitrary = modules | {"future_owner.rs": "const READER_POOL_SIZE_NEW: usize = 1;"}
        self.assertIn("future_owner wrongly defines READER_POOL_SIZE_NEW", owner_errors(root, owner, arbitrary))

    def test_cfg_value_and_owner_family_mutants(self) -> None:
        root, owner, modules = source_inventory()
        self.assertEqual(owner_errors(root, owner, modules), [])
        altered = owner + "\nconst READER_POOL_SIZE_NEW: usize = 1;"
        self.assertIn("reader_pool unexpectedly defines READER_POOL_SIZE_NEW", owner_errors(root, altered, modules))
        altered = owner.replace(DECLARATION, DECLARATION.replace("8", "7"), 1)
        self.assertIn("reader_pool lacks exact size", owner_errors(root, altered, modules))
        for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            altered = owner.replace(DECLARATION, attr + "\n" + DECLARATION, 1)
            self.assertIn("reader_pool gates size", owner_errors(root, altered, modules))
            altered = root.replace("mod reader_pool;", attr + "\nmod reader_pool;", 1)
            self.assertIn("root gates reader_pool module", owner_errors(altered, owner, modules))
        for attr in ('#![cfg(feature = "operator")]', '#![cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            self.assertIn("reader_pool gates whole owner module", owner_errors(root, attr + "\n" + owner, modules))

    def test_wal_attribution_uses_passed_reader_count(self) -> None:
        root, owner, modules = source_inventory()
        self.assertEqual(owner_errors(root, owner, modules), [])
        changed = modules | {"wal_attribution.rs": modules["wal_attribution.rs"].replace("0..reader_count", "0..7")}
        self.assertIn("wal_attribution lacks passed reader count", owner_errors(root, owner, changed))
        changed = modules | {"wal_attribution.rs": "use crate::reader_pool::READER_POOL_SIZE;\n" + modules["wal_attribution.rs"]}
        self.assertIn("wal_attribution cycles through reader_pool", owner_errors(root, owner, changed))

    def test_fast_tier_registration(self) -> None:
        self.assertIn("fast test-slice90-reader-pool-size-owner", (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
