#!/usr/bin/env python3
"""Guard embed dispatch timeout policy ownership across engine sources."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
NAME = "DEFAULT_EMBED_TIMEOUT_MS"
DECLARATION = "pub(crate) const DEFAULT_EMBED_TIMEOUT_MS: u64 = 30_000;"
IMPORT = "use super::embed_dispatch::DEFAULT_EMBED_TIMEOUT_MS;"
FAMILY = re.compile(r"^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?const[ \t]+(DEFAULT_EMBED_TIMEOUT_MS\w*)\b", re.M)


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
    owner = (SRC / "embed_dispatch.rs").read_text()
    modules = {
        path.relative_to(SRC).as_posix(): path.read_text()
        for path in sorted(SRC.rglob("*.rs"))
        if path.relative_to(SRC).as_posix() not in ("lib.rs", "embed_dispatch.rs")
    }
    return root, owner, modules


def owner_errors(root: str, owner: str, modules: dict[str, str]) -> list[str]:
    errors = [f"root wrongly defines {name}" for name in FAMILY.findall(root)]
    for path, source in modules.items():
        errors.extend(f"{path} wrongly defines {name}" for name in FAMILY.findall(source))
    module = re.search(r"^mod embed_dispatch;", root, re.M)
    if not module or attrs(root, module.start()):
        errors.append("root gates embed_dispatch module")
    if re.search(r"(?m)^[ \t]*#!\[\s*cfg(?:_attr)?\b", owner):
        errors.append("embed_dispatch gates whole owner module")
    errors.extend(f"embed_dispatch unexpectedly defines {name}" for name in FAMILY.findall(owner) if name != NAME)
    constant = list(re.finditer(r"^" + re.escape(DECLARATION) + r"$", owner, re.M))
    if len(constant) != 1 or FAMILY.findall(owner).count(NAME) != 1:
        errors.append("embed_dispatch lacks exact timeout")
    elif attrs(owner, constant[0].start()):
        errors.append("embed_dispatch gates timeout")
    configuration = modules.get("runtime_configuration.rs", "")
    if IMPORT not in configuration:
        errors.append("runtime_configuration lacks dispatch owner path")
    if "requested.embedder_call_timeout_ms,\n            DEFAULT_EMBED_TIMEOUT_MS," not in configuration:
        errors.append("runtime_configuration lacks timeout default consumption")
    return errors


class EmbedTimeoutOwnerTest(unittest.TestCase):
    def test_current_source_has_one_owner(self) -> None:
        self.assertEqual(owner_errors(*source_inventory()), [])

    def test_complete_owner_rejects_wrong_module_exact_and_unlisted(self) -> None:
        root, owner, modules = source_inventory()
        root = re.sub(r"(?m)^const DEFAULT_EMBED_TIMEOUT_MS: u64 = 30_000;\n", "", root, count=1)
        if DECLARATION not in owner:
            owner += "\n" + DECLARATION
        if IMPORT not in modules["runtime_configuration.rs"]:
            modules["runtime_configuration.rs"] += "\n" + IMPORT
        self.assertEqual(owner_errors(root, owner, modules), [])
        for declaration, name in (
            ("const DEFAULT_EMBED_TIMEOUT_MS_NEW: u64 = 1;", "DEFAULT_EMBED_TIMEOUT_MS_NEW"),
            ("pub(crate) const DEFAULT_EMBED_TIMEOUT_MS: u64 = 1;", NAME),
        ):
            self.assertIn(f"root wrongly defines {name}", owner_errors(root + "\n" + declaration, owner, modules))
        for path, declaration, name in (
            ("runtime_configuration.rs", "pub const DEFAULT_EMBED_TIMEOUT_MS_NEW: u64 = 1;", "DEFAULT_EMBED_TIMEOUT_MS_NEW"),
            ("projection_runtime.rs", "const DEFAULT_EMBED_TIMEOUT_MS: u64 = 1;", NAME),
        ):
            altered = modules | {path: modules[path] + "\n" + declaration}
            self.assertIn(f"{path} wrongly defines {name}", owner_errors(root, owner, altered))
        arbitrary = modules | {"future_owner.rs": "pub(crate) const DEFAULT_EMBED_TIMEOUT_MS_NEW: u64 = 1;"}
        self.assertIn("future_owner.rs wrongly defines DEFAULT_EMBED_TIMEOUT_MS_NEW", owner_errors(root, owner, arbitrary))

    def test_cfg_cfg_attr_value_and_consumer_mutants(self) -> None:
        root, owner, modules = source_inventory()
        self.assertEqual(owner_errors(root, owner, modules), [])
        changed = owner.replace(DECLARATION, DECLARATION.replace("30_000", "30_001"), 1)
        self.assertIn("embed_dispatch lacks exact timeout", owner_errors(root, changed, modules))
        for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            changed = owner.replace(DECLARATION, attr + "\n" + DECLARATION, 1)
            self.assertIn("embed_dispatch gates timeout", owner_errors(root, changed, modules))
            changed = root.replace("mod embed_dispatch;", attr + "\nmod embed_dispatch;", 1)
            self.assertIn("root gates embed_dispatch module", owner_errors(changed, owner, modules))
        for attr in ('#![cfg(feature = "operator")]', '#![cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            self.assertIn("embed_dispatch gates whole owner module", owner_errors(root, attr + "\n" + owner, modules))
        changed = modules | {"runtime_configuration.rs": modules["runtime_configuration.rs"].replace(IMPORT, "")}
        self.assertIn("runtime_configuration lacks dispatch owner path", owner_errors(root, owner, changed))

    def test_fast_tier_registration(self) -> None:
        self.assertIn("fast test-slice90-embed-timeout-owner", (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
