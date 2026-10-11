#!/usr/bin/env python3
"""Guard Slice 45 timing carriers, methods, cfg, and public paths."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
GATE = '#[cfg(feature = "test-hooks")]'
TYPES = {
    "Slice45FrozenStageTiming": (
        "cursor_authentication_ns",
        "token_authentication_ns",
        "snapshot_binding_ns",
    ),
    "Slice45MintStageTiming": (
        "context_validation_ns",
        "snapshot_validation_ns",
        "binding_ns",
        "token_codec_ns",
    ),
}
METHODS = (
    "measure_slice45_frozen_stages_for_test",
    "measure_slice45_mint_stages_for_test",
)
TYPE_FAMILY = re.compile(r"^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?struct[ \t]+(Slice45(?:Frozen|Mint)StageTiming\w*)\b", re.M)
METHOD_FAMILY = re.compile(r"^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?fn[ \t]+(measure_slice45_(?:frozen|mint)_stages_for_test\w*)\b", re.M)
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


def inventory() -> tuple[str, str, dict[str, str]]:
    root = (SRC / "lib.rs").read_text()
    owner = (SRC / "frozen_read.rs").read_text()
    other = {
        path.relative_to(SRC).as_posix(): path.read_text()
        for path in sorted(SRC.rglob("*.rs"))
        if path.relative_to(SRC).as_posix() not in ("lib.rs", "frozen_read.rs")
    }
    return root, owner, other


def owner_errors(root: str, owner: str, other: dict[str, str]) -> list[str]:
    errors = [f"root wrongly defines {name}" for name in TYPE_FAMILY.findall(root) + METHOD_FAMILY.findall(root)]
    for path, source in other.items():
        errors.extend(f"{path} wrongly defines {name}" for name in TYPE_FAMILY.findall(source) + METHOD_FAMILY.findall(source))
    module = re.search(r"^mod frozen_read;", root, re.M)
    if not module or any(CFG.match(attr) for attr in prelude(root, module.start())):
        errors.append("root gates frozen owner module")
    if re.search(r"(?m)^[ \t]*#!\[\s*cfg(?:_attr)?\b", owner):
        errors.append("frozen owner gates whole module")
    for name, fields in TYPES.items():
        found = list(re.finditer(r"^pub struct " + name + r" \{", owner, re.M))
        if len(found) != 1 or TYPE_FAMILY.findall(owner).count(name) != 1:
            errors.append(f"frozen owner lacks public {name}")
        else:
            attrs = prelude(owner, found[0].start())
            if [attr for attr in attrs if CFG.match(attr)] != [GATE] or "#[doc(hidden)]" not in attrs or "#[derive(Clone, Copy, Debug)]" not in attrs:
                errors.append(f"frozen owner changes {name} attrs")
            end = owner.find("\n}", found[0].end())
            body = owner[found[0].end():end]
            if tuple(re.findall(r"^    pub (\w+): u128,$", body, re.M)) != fields:
                errors.append(f"frozen owner changes {name} fields")
        export = re.search(r"^pub use frozen_read::" + name + r";$", root, re.M)
        if not export:
            errors.append(f"root loses {name} public path")
        else:
            attrs = prelude(root, export.start())
            if [attr for attr in attrs if CFG.match(attr)] != [GATE] or "#[doc(hidden)]" not in attrs:
                errors.append(f"root changes {name} export attrs")
    impls = list(re.finditer(r"(?ms)^impl Engine \{.*?^}", owner))
    for impl in impls:
        if any(CFG.match(attr) for attr in prelude(owner, impl.start())):
            errors.append("frozen owner gates Engine impl")
    for name in METHODS:
        found = [(match, impl) for impl in impls for match in re.finditer(r"^    pub fn " + name + r"\b", impl.group(), re.M)]
        if len(found) != 1 or METHOD_FAMILY.findall(owner).count(name) != 1:
            errors.append(f"frozen owner lacks public Engine::{name}")
        else:
            match, impl = found[0]
            attrs = prelude(impl.group(), match.start())
            if [attr for attr in attrs if CFG.match(attr)] != [GATE] or "#[doc(hidden)]" not in attrs:
                errors.append(f"frozen owner changes Engine::{name} attrs")
    for name in TYPE_FAMILY.findall(owner) + METHOD_FAMILY.findall(owner):
        if name not in TYPES and name not in METHODS:
            errors.append(f"frozen owner adds unlisted {name}")
    return errors


def complete_fixture() -> tuple[str, str, dict[str, str]]:
    root, owner, other = inventory()
    for name in TYPES:
        pattern = re.compile(r"(?m)^(?:(?:///[^\n]*|#\[[^\n]*\])\n)*pub struct " + name + r"\b[\s\S]*?^}\n")
        match = pattern.search(root)
        if match:
            owner += "\n" + match.group()
            root = root[:match.start()] + root[match.end():]
        export = GATE + "\n#[doc(hidden)]\npub use frozen_read::" + name + ";"
        if export not in root:
            root += "\n" + export + "\n"
    moved = []
    for name in METHODS:
        pattern = re.compile(r"(?m)^(?:    (?:///[^\n]*|#\[[^\n]*\])\n)*    pub fn " + name + r"\b[\s\S]*?^    }\n")
        match = pattern.search(root)
        if match:
            moved.append(match.group())
            root = root[:match.start()] + root[match.end():]
    if moved:
        owner += "\nimpl Engine {\n" + "\n".join(moved) + "}\n"
    return root, owner, other


class FrozenTimingOwnerTest(unittest.TestCase):
    def test_current_source_and_qualified_paths(self) -> None:
        self.assertEqual(owner_errors(*inventory()), [])

    def test_complete_owner_and_wrong_owner_mutants(self) -> None:
        root, owner, other = complete_fixture()
        self.assertEqual(owner_errors(root, owner, other), [])
        for decl, name in (
            ("pub(crate) struct Slice45FrozenStageTimingNew {}", "Slice45FrozenStageTimingNew"),
            ("impl Engine {\n    fn measure_slice45_mint_stages_for_test_new(&self) {}\n}", "measure_slice45_mint_stages_for_test_new"),
        ):
            self.assertIn(f"root wrongly defines {name}", owner_errors(root + "\n" + decl, owner, other))
        for path in ("pagination.rs", "future_owner.rs"):
            changed = other | {path: other.get(path, "") + "\npub struct Slice45MintStageTimingNew {}"}
            self.assertIn(f"{path} wrongly defines Slice45MintStageTimingNew", owner_errors(root, owner, changed))
        changed = owner + "\nimpl Engine {\n    pub fn measure_slice45_frozen_stages_for_test_new(&self) {}\n}"
        self.assertIn("frozen owner adds unlisted measure_slice45_frozen_stages_for_test_new", owner_errors(root, changed, other))

    def test_cfg_cfg_attr_and_field_mutants(self) -> None:
        root, owner, other = complete_fixture()
        self.assertEqual(owner_errors(root, owner, other), [])
        for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            changed = owner.replace("pub struct Slice45FrozenStageTiming {", attr + "\npub struct Slice45FrozenStageTiming {", 1)
            self.assertIn("frozen owner changes Slice45FrozenStageTiming attrs", owner_errors(root, changed, other))
            impl_start = owner.rfind("impl Engine {", 0, owner.index("pub fn measure_slice45_frozen_stages_for_test"))
            changed = owner[:impl_start] + attr + "\n" + owner[impl_start:]
            self.assertIn("frozen owner gates Engine impl", owner_errors(root, changed, other))
            changed = owner.replace("    pub fn measure_slice45_mint_stages_for_test(", "    " + attr + "\n    pub fn measure_slice45_mint_stages_for_test(", 1)
            self.assertIn("frozen owner changes Engine::measure_slice45_mint_stages_for_test attrs", owner_errors(root, changed, other))
            changed = root.replace("mod frozen_read;", attr + "\nmod frozen_read;", 1)
            self.assertIn("root gates frozen owner module", owner_errors(changed, owner, other))
            changed = root.replace("pub use frozen_read::Slice45MintStageTiming;", attr + "\npub use frozen_read::Slice45MintStageTiming;", 1)
            self.assertIn("root changes Slice45MintStageTiming export attrs", owner_errors(changed, owner, other))
            self.assertIn("frozen owner gates whole module", owner_errors(root, "#!" + attr[1:] + "\n" + owner, other))
        changed = owner.replace("pub cursor_authentication_ns: u128,", "pub cursor_authentication_ns: u64,", 1)
        self.assertIn("frozen owner changes Slice45FrozenStageTiming fields", owner_errors(root, changed, other))

    def test_fast_registration(self) -> None:
        self.assertIn("fast test-slice90-frozen-timing-owner", (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
