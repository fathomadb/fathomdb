#!/usr/bin/env python3
"""Guard operator integrity and safe-export ownership."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
TYPES = ("CheckIntegrityOpts", "Section", "Finding", "IntegrityReport", "SafeExportArtifact")
METHODS = ("check_integrity", "safe_export")
HELPERS = (
    "physical_section",
    "logical_section",
    "semantic_section",
    "collect_integrity_check_findings",
    "locator_from_rusqlite_error",
)


METHOD_DECL = re.compile(
    r"^    (?:(?:pub(?:\([^)]*\))?) )?fn ((?:check_integrity|safe_export)\w*)\(",
    re.M,
)


def mask_comments(source: str) -> str:
    """Keep offsets while removing legal comments outside quoted strings."""
    result = list(source)
    index = 0
    while index < len(source):
        if source[index] == '"':
            index += 1
            while index < len(source):
                if source[index] == "\\":
                    index += 2
                elif source[index] == '"':
                    index += 1
                    break
                else:
                    index += 1
            continue
        if source.startswith("//", index):
            end = source.find("\n", index)
            end = len(source) if end < 0 else end
            result[index:end] = " " * (end - index)
            index = end
            continue
        if source.startswith("/*", index):
            start = index
            depth = 1
            index += 2
            while index < len(source) and depth:
                if source.startswith("/*", index):
                    depth += 1
                    index += 2
                elif source.startswith("*/", index):
                    depth -= 1
                    index += 2
                else:
                    index += 1
            result[start:index] = "".join("\n" if char == "\n" else " " for char in source[start:index])
            continue
        index += 1
    return "".join(result)


def attribute_end(source: str, start: int) -> int | None:
    """Return the end of one balanced outer attribute, including nested cfg calls."""
    if not source.startswith("#[", start):
        return None
    closes = {"[": "]", "(": ")", "{": "}"}
    stack = []
    quoted = False
    index = start + 1
    while index < len(source):
        char = source[index]
        if quoted:
            if char == "\\":
                index += 2
                continue
            if char == '"':
                quoted = False
        elif char == '"':
            quoted = True
        elif char in closes:
            stack.append(closes[char])
        elif char in "])}":
            if not stack or stack.pop() != char:
                return None
            if not stack:
                return index + 1
        index += 1
    return None


def source_prelude(source: str, start: int) -> list[str]:
    prefix = mask_comments(source[:start])
    cursor = len(prefix.rstrip())
    attrs = []
    while cursor:
        candidate = prefix.rfind("#[", 0, cursor)
        while candidate >= 0 and attribute_end(prefix, candidate) != cursor:
            candidate = prefix.rfind("#[", 0, candidate)
        if candidate < 0:
            break
        attrs.append(re.sub(r"\s+", "", prefix[candidate:cursor]))
        cursor = len(prefix[:candidate].rstrip())
    return attrs


def gates_item(attrs: list[str]) -> bool:
    return any(attr.startswith(("#[cfg(", "#[cfg_attr(")) for attr in attrs)


def owner_errors(root: str, owner: str, others: dict[str, str] | None = None) -> list[str]:
    errors = []
    if others is None:
        others = {
            str(path.relative_to(SRC)): path.read_text()
            for path in SRC.rglob("*.rs")
            if path not in (SRC / "lib.rs", SRC / "operator.rs")
        }
    root_module = re.search(r"(?m)^mod operator;$", root)
    if root_module is None or gates_item(source_prelude(root, root_module.start())):
        errors.append("root gates operator owner module")
    root_types = re.compile(
        r"^(?:(?:pub(?:\([^)]*\))?) )?(?:struct|enum) "
        r"((?:CheckIntegrity|Section|Finding|IntegrityReport|SafeExport)\w*)\b",
        re.M,
    )
    for match in root_types.finditer(root):
        errors.append(f"root still defines operator::{match.group(1)}")
    root_impls = re.compile(
        r"^impl[^\n{]*\b((?:CheckIntegrity|Section|Finding|IntegrityReport|SafeExport)\w*)\b"
        r"(?:\s*<[^>\n]*>)?\s*(?:\{|where\b)",
        re.M,
    )
    for match in root_impls.finditer(root):
        errors.append(f"root still defines operator::{match.group(1)}")
    for name in re.findall(
        r"^    (?:(?:pub(?:\([^)]*\))?) )?fn ((?:check_integrity|safe_export)\w*)\(",
        root,
        re.M,
    ):
        errors.append(f"root still defines Engine::{name}")
    for name in re.findall(
        r"^(?:(?:pub(?:\([^)]*\))?) )?fn "
        r"((?:physical_section|logical_section|semantic_section|"
        r"collect_integrity_check_findings|locator_from_rusqlite_error)\w*)\(",
        root,
        re.M,
    ):
        errors.append(f"root still defines operator helper {name}")
    for name in TYPES:
        kind = "enum" if name == "Section" else "struct"
        if len(re.findall(r"^pub " + kind + " " + name + r"\b", owner, re.M)) != 1:
            errors.append(f"operator owner lacks one {name}")
    owner_impls = list(re.finditer(r"^impl Engine \{.*?^\}", owner, re.M | re.S))
    for name in METHODS:
        declarations = [match for match in METHOD_DECL.finditer(owner) if match.group(1) == name]
        valid = False
        if len(declarations) == 1:
            declaration = declarations[0]
            enclosing = next(
                (impl for impl in owner_impls if impl.start() < declaration.start() < impl.end()),
                None,
            )
            attrs = source_prelude(owner, declaration.start())
            valid = (
                enclosing is not None
                and not source_prelude(owner, enclosing.start())
                and attrs == ['#[cfg(feature="operator")]']
                and declaration.group(0).startswith(f"    pub fn {name}(")
                and not re.search(r"^#!\[cfg(?:_attr)?\(", owner, re.M)
            )
        if not valid:
            errors.append(f"operator owner lacks cfg(operator) Engine::{name}")
    for path, source in others.items():
        for match in METHOD_DECL.finditer(source):
            errors.append(f"{path} still defines Engine::{match.group(1)}")
    for name in HELPERS:
        pattern = r'^#\[cfg\(feature = "operator"\)\]\nfn ' + name + r"\("
        if len(re.findall(pattern, owner, re.M)) != 1:
            errors.append(f"operator owner lacks cfg(operator) helper {name}")
    exports = " ".join(re.findall(r"^pub use operator::\{(.*?)\};", root, re.M | re.S))
    for name in TYPES:
        if not re.search(r"\b" + name + r"\b", exports):
            errors.append(f"root does not re-export operator::{name}")
    return errors


class OperatorIntegrityOwnerTest(unittest.TestCase):
    def sources(self) -> tuple[str, str]:
        return (SRC / "lib.rs").read_text(), (SRC / "operator.rs").read_text()

    def test_current_source_has_one_operator_owner(self) -> None:
        self.assertEqual(owner_errors(*self.sources()), [])

    def test_unlisted_root_family_with_complete_owner(self) -> None:
        root, owner = self.sources()
        self.assertEqual(owner_errors(root, owner), [])
        for declaration, expected in (
            ("struct SafeExportNew;", "root still defines operator::SafeExportNew"),
            ("impl IntegrityReport { fn extra(&self) {} }", "root still defines operator::IntegrityReport"),
            ("impl Engine {\n    pub fn check_integrity_new(&self) {}\n}", "root still defines Engine::check_integrity_new"),
            ("fn physical_section_new() {}", "root still defines operator helper physical_section_new"),
        ):
            with self.subTest(expected=expected):
                self.assertIn(expected, owner_errors(root + "\n" + declaration, owner))

    def test_cfg_and_reexport_mutants_fail(self) -> None:
        root, owner = self.sources()
        old = '#[cfg(feature = "operator")]\n    pub fn safe_export('
        self.assertIn(old, owner)
        self.assertIn(
            "operator owner lacks cfg(operator) Engine::safe_export",
            owner_errors(root, owner.replace(old, '#[cfg(test)]\n    pub fn safe_export(', 1)),
        )
        self.assertIn(
            "root does not re-export operator::SafeExportArtifact",
            owner_errors(root.replace("SafeExportArtifact,", "RemovedSafeExportArtifact,", 1), owner),
        )

    def test_multiple_engine_impls_and_method_mutants(self) -> None:
        root, owner = self.sources()
        self.assertEqual(owner_errors(root, owner), [])
        for name in METHODS:
            declaration = (
                '\nimpl Engine {\n    #[cfg(feature = "operator")]\n'
                f'    pub fn {name}(&self) {{}}\n}}\n'
            )
            with self.subTest(name=name, mutation="duplicate"):
                self.assertIn(
                    f"operator owner lacks cfg(operator) Engine::{name}",
                    owner_errors(root, owner + declaration),
                )
            with self.subTest(name=name, mutation="removed"):
                self.assertIn(
                    f"operator owner lacks cfg(operator) Engine::{name}",
                    owner_errors(root, owner.replace(f"pub fn {name}(", f"pub fn removed_{name}(", 1)),
                )
        self.assertIn(
            "operator owner lacks cfg(operator) Engine::check_integrity",
            owner_errors(root, owner.replace(
                '    #[cfg(feature = "operator")]\n    pub fn check_integrity(',
                '    #[cfg_attr(feature = "default-embedder", cfg(test))]\n'
                '    #[cfg(feature = "operator")]\n    pub fn check_integrity(',
                1,
            )),
        )
        self.assertIn(
            "operator owner lacks cfg(operator) Engine::safe_export",
            owner_errors(root, owner.replace(
                "impl Engine {", '#[cfg(test)]\nimpl Engine {', 1
            )),
        )
        for attr in ('#[cfg(test)]',
                     '#[cfg_attr(feature = "default-embedder", cfg(test))]',
                     '#[cfg(\n    all(\n        feature = "operator",\n        test,\n    )\n)]',
                     '#[cfg_attr(\n    all(feature = "default-embedder", test),\n    cfg(test)\n)]'):
            for comment in ('// legal comment', '/* legal block comment */',
                            '/* legal\n   block comment */', '/// legal doc comment'):
                with self.subTest(attr=attr, comment=comment):
                    changed = owner.replace("impl Engine {", attr + "\n" + comment + "\nimpl Engine {", 1)
                    self.assertIn(
                        "operator owner lacks cfg(operator) Engine::check_integrity",
                        owner_errors(root, changed),
                    )
                    changed = owner.replace(
                        '    #[cfg(feature = "operator")]\n    pub fn check_integrity(',
                        '    #[cfg(feature = "operator")]\n    ' + attr + '\n    ' + comment +
                        '\n    pub fn check_integrity(',
                        1,
                    )
                    self.assertIn(
                        "operator owner lacks cfg(operator) Engine::check_integrity",
                        owner_errors(root, changed),
                    )

    def test_wrong_owner_method_family_mutants(self) -> None:
        root, owner = self.sources()
        other = (SRC / "evidence.rs").read_text()
        for name in ("check_integrity", "safe_export_new"):
            with self.subTest(name=name):
                self.assertIn(
                    f"evidence.rs still defines Engine::{name}",
                    owner_errors(
                        root,
                        owner,
                        {"evidence.rs": other + f"\nimpl Engine {{\n    fn {name}(&self) {{}}\n}}\n"},
                    ),
                )

    def test_root_module_gate_with_multiline_attribute(self) -> None:
        root, owner = self.sources()
        attr = '#[cfg_attr(\n    all(feature = "default-embedder", test),\n    cfg(test)\n)]'
        mutant = root.replace("mod operator;", attr + "\n/* nested /* comment */ gap */\nmod operator;", 1)
        self.assertIn("root gates operator owner module", owner_errors(mutant, owner))

    def test_equivalent_multiline_operator_gate_remains_valid(self) -> None:
        root, owner = self.sources()
        multiline = '#[cfg(\n    feature = "operator"\n)]'
        changed = owner.replace(
            '    #[cfg(feature = "operator")]\n    pub fn check_integrity(',
            '    ' + multiline + '\n    /* explanatory gap */\n    pub fn check_integrity(',
            1,
        )
        self.assertEqual(owner_errors(root, changed), [])

    def test_fast_tier_registration(self) -> None:
        self.assertIn(
            "run_tier_suite fast test-slice90-operator-integrity-owner "
            "python3 scripts/tests/test_slice90_operator_integrity_owner.py",
            (ROOT / "scripts/agent-test.sh").read_text(),
        )


if __name__ == "__main__":
    unittest.main()
