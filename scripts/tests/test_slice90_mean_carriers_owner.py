#!/usr/bin/env python3
"""Guard mean report and public pin threshold ownership and root paths."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
REPORT = "MeanRecomputeReport"
THRESHOLD = "MEAN_VEC_PIN_THRESHOLD"
REEXPORT = "pub use mean::{MeanRecomputeReport, MEAN_VEC_PIN_THRESHOLD};"
FAMILY = re.compile(
    r"^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?(?:struct[ \t]+(MeanRecomputeReport\w*)\b|const[ \t]+(MEAN_VEC_PIN_THRESHOLD\w*)\b)",
    re.M,
)


def declarations(source: str) -> list[str]:
    return [left or right for left, right in FAMILY.findall(source)]


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
    owner = (SRC / "mean.rs").read_text()
    modules = {
        path.relative_to(SRC).as_posix(): path.read_text()
        for path in sorted(SRC.rglob("*.rs"))
        if path.relative_to(SRC).as_posix() not in ("lib.rs", "mean.rs")
    }
    return root, owner, modules


def owner_errors(root: str, owner: str, modules: dict[str, str]) -> list[str]:
    errors = [f"root wrongly defines {name}" for name in declarations(root)]
    for path, source in modules.items():
        errors.extend(f"{path} wrongly defines {name}" for name in declarations(source))
    module = re.search(r"^mod mean;", root, re.M)
    if not module or attrs(root, module.start()):
        errors.append("root gates mean module")
    if re.search(r"(?m)^[ \t]*#!\[\s*cfg(?:_attr)?\b", owner):
        errors.append("mean gates whole owner module")
    errors.extend(f"mean unexpectedly defines {name}" for name in declarations(owner) if name not in (REPORT, THRESHOLD))
    report = list(re.finditer(r"^pub struct MeanRecomputeReport \{", owner, re.M))
    if len(report) != 1 or declarations(owner).count(REPORT) != 1:
        errors.append("mean lacks one public report")
    elif attrs(owner, report[0].start()):
        errors.append("mean gates public report")
    constant = list(re.finditer(r"^pub const MEAN_VEC_PIN_THRESHOLD: u64 = 256;$", owner, re.M))
    if len(constant) != 1 or declarations(owner).count(THRESHOLD) != 1:
        errors.append("mean lacks exact public threshold")
    elif attrs(owner, constant[0].start()):
        errors.append("mean gates public threshold")
    reexport = list(re.finditer(r"^" + re.escape(REEXPORT) + r"$", root, re.M))
    if len(reexport) != 1 or attrs(root, reexport[0].start()):
        errors.append("root lacks ungated public mean reexport")
    for path in ("open.rs", "projection_commit.rs"):
        if "use crate::mean::MEAN_VEC_PIN_THRESHOLD;" not in modules.get(path, ""):
            errors.append(f"{path} lacks mean owner path")
    vector_write = re.search(r"(?ms)^    pub fn write_vector_for_test\(.*?^    }", modules.get("vector_storage.rs", ""))
    if not vector_write or "mean::MEAN_VEC_PIN_THRESHOLD" not in vector_write.group():
        errors.append("vector_storage test vector write lacks mean owner path")
    return errors


class MeanCarriersOwnerTest(unittest.TestCase):
    def test_current_source_has_one_owner_and_public_path(self) -> None:
        self.assertEqual(owner_errors(*source_inventory()), [])

    def test_complete_owner_rejects_root_and_every_other_owner(self) -> None:
        root, owner, modules = source_inventory()
        report = re.search(r"(?ms)^#\[derive\(Clone, Debug, PartialEq\)\]\npub struct MeanRecomputeReport \{.*?^}\n", root)
        if report:
            owner += "\n" + report.group()
            root = root[: report.start()] + root[report.end() :]
        constant = re.search(r"(?m)^pub const MEAN_VEC_PIN_THRESHOLD: u64 = 256;\n", root)
        if constant:
            owner += "\n" + constant.group()
            root = root[: constant.start()] + root[constant.end() :]
        if REEXPORT not in root:
            root += "\n" + REEXPORT
        for path in ("open.rs", "projection_commit.rs"):
            modules[path] += "\nuse crate::mean::MEAN_VEC_PIN_THRESHOLD;"
        self.assertEqual(owner_errors(root, owner, modules), [])
        for declaration, name in (
            ("struct MeanRecomputeReportNew;", "MeanRecomputeReportNew"),
            ("pub(crate) const MEAN_VEC_PIN_THRESHOLD: u64 = 1;", THRESHOLD),
        ):
            self.assertIn(f"root wrongly defines {name}", owner_errors(root + "\n" + declaration, owner, modules))
        for path, declaration, name in (
            ("open.rs", "pub struct MeanRecomputeReportNew;", "MeanRecomputeReportNew"),
            ("projection_commit.rs", "const MEAN_VEC_PIN_THRESHOLD: u64 = 1;", THRESHOLD),
            ("telemetry.rs", "pub(crate) const MEAN_VEC_PIN_THRESHOLD_NEW: u64 = 1;", "MEAN_VEC_PIN_THRESHOLD_NEW"),
        ):
            altered = modules | {path: modules[path] + "\n" + declaration}
            self.assertIn(f"{path} wrongly defines {name}", owner_errors(root, owner, altered))
        arbitrary = modules | {"future_owner.rs": "pub struct MeanRecomputeReportNew;"}
        self.assertIn("future_owner.rs wrongly defines MeanRecomputeReportNew", owner_errors(root, owner, arbitrary))

    def test_cfg_reexport_and_value_mutants(self) -> None:
        root, owner, modules = source_inventory()
        self.assertEqual(owner_errors(root, owner, modules), [])
        self.assertIn("mean lacks exact public threshold", owner_errors(root, owner.replace("= 256;", "= 255;", 1), modules))
        for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            for marker, error in (("pub struct MeanRecomputeReport {", "mean gates public report"), ("pub const MEAN_VEC_PIN_THRESHOLD:", "mean gates public threshold")):
                changed = owner.replace(marker, attr + "\n" + marker, 1)
                self.assertIn(error, owner_errors(root, changed, modules))
            changed = root.replace(REEXPORT, attr + "\n" + REEXPORT, 1)
            self.assertIn("root lacks ungated public mean reexport", owner_errors(changed, owner, modules))
            changed = root.replace("mod mean;", attr + "\nmod mean;", 1)
            self.assertIn("root gates mean module", owner_errors(changed, owner, modules))
        for attr in ('#![cfg(feature = "operator")]', '#![cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            self.assertIn("mean gates whole owner module", owner_errors(root, attr + "\n" + owner, modules))

    def test_old_root_vector_writer_decoy_cannot_replace_storage_owner_path(self) -> None:
        root, owner, modules = source_inventory()
        self.assertEqual(owner_errors(root, owner, modules), [])
        storage = modules["vector_storage.rs"]
        broken_storage = storage.replace("mean::MEAN_VEC_PIN_THRESHOLD", "MEAN_VEC_PIN_THRESHOLD", 1)
        self.assertNotEqual(broken_storage, storage)
        decoy_root = root + "\nfn write_vector_for_test() { let _ = mean::MEAN_VEC_PIN_THRESHOLD; }\n"
        errors = owner_errors(decoy_root, owner, modules | {"vector_storage.rs": broken_storage})
        self.assertIn("vector_storage test vector write lacks mean owner path", errors)
        wrong_owner = modules | {"future_owner.rs": "fn write_vector_for_test() { let _ = mean::MEAN_VEC_PIN_THRESHOLD; }"}
        self.assertIn("vector_storage test vector write lacks mean owner path",
                      owner_errors(root, owner, wrong_owner | {"vector_storage.rs": broken_storage}))
        literal_storage = storage.replace("mean::MEAN_VEC_PIN_THRESHOLD", "256", 1)
        self.assertIn("vector_storage test vector write lacks mean owner path",
                      owner_errors(root, owner, modules | {"vector_storage.rs": literal_storage}))

    def test_fast_tier_registration(self) -> None:
        self.assertIn("fast test-slice90-mean-carriers-owner", (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
