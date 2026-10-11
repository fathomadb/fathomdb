#!/usr/bin/env python3
"""Guard open startup cleanup and fault hook ownership."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
TYPES = (
    "OpenEmbedDispatchGuard",
    "OpenPostProbeGuard",
    "OpenPostProbeParts",
    "AdmissionLockedHookForTest",
    "PostProbeStartupFaultForTest",
    "PostProbeStartupObservationForTest",
    "PostProbeVisibilityFaultForTest",
)
STATICS = (
    "ADMISSION_LOCKED_HOOK_FOR_TEST",
    "POST_PROBE_STARTUP_FAULT_FOR_TEST",
    "POST_PROBE_VISIBILITY_FAULT_FOR_TEST",
)
FUNCTIONS = (
    "install_post_probe_visibility_fault_for_test",
    "take_post_probe_visibility_fault_for_test",
    "install_post_probe_startup_fault_for_test",
    "take_post_probe_startup_fault_for_test",
    "install_admission_locked_hook_for_test",
    "run_admission_locked_hook_for_test",
)
IMPLS = (
    "impl OpenEmbedDispatchGuard",
    "impl Drop for OpenEmbedDispatchGuard",
    "impl OpenPostProbeGuard",
    "impl Drop for OpenPostProbeGuard",
)


def owner_errors(root: str, owner: str) -> list[str]:
    errors = []
    families = re.compile(
        r"^(?:(?:pub(?:\([^)]*\))?) )?"
        r"(?:(?:struct|enum|const|static) ((?:OpenEmbedDispatch|OpenPostProbe|"
        r"AdmissionLockedHook|PostProbeStartup|PostProbeVisibility|"
        r"ADMISSION_LOCKED_HOOK|POST_PROBE_STARTUP_FAULT|"
        r"POST_PROBE_VISIBILITY_FAULT)\w*)\b|"
        r"fn ((?:install_post_probe|take_post_probe|"
        r"install_admission_locked|run_admission_locked)\w*)\(|"
        r"(impl (?:Drop for )?(?:OpenEmbedDispatch|OpenPostProbe)\w*)\s*\{)",
        re.M,
    )
    for match in families.finditer(root):
        errors.append(f"root still defines open cleanup::{next(g for g in match.groups() if g)}")

    for name in TYPES:
        if len(re.findall(r"^(?:(?:pub(?:\([^)]*\))?) )?struct " + name + r"\b", owner, re.M)) != 1:
            errors.append(f"open owner does not define one {name}")
    for name in STATICS:
        if len(re.findall(r"^static " + name + r"\b", owner, re.M)) != 1:
            errors.append(f"open owner does not define one {name}")
    for name in FUNCTIONS:
        if len(re.findall(r"^(?:(?:pub(?:\([^)]*\))?) )?fn " + name + r"\(", owner, re.M)) != 1:
            errors.append(f"open owner does not define one {name}")
    for name in IMPLS:
        if len(re.findall(r"^" + re.escape(name) + r" \{", owner, re.M)) != 1:
            errors.append(f"open owner does not define one {name}")

    for name in (*TYPES[3:], *STATICS, *FUNCTIONS):
        kind = "struct" if name in TYPES else "static" if name in STATICS else "fn"
        suffix = r"\b" if kind != "fn" else r"\("
        pattern = r"^#\[cfg\(test\)\]\n(?:(?:pub(?:\([^)]*\))?) )?" + kind + " " + name + suffix
        if len(re.findall(pattern, owner, re.M)) != 1:
            errors.append(f"open owner lacks cfg(test) {name}")
    return errors


class OpenCleanupOwnerTest(unittest.TestCase):
    def sources(self) -> tuple[str, str]:
        return (SRC / "lib.rs").read_text(), (SRC / "open.rs").read_text()

    def test_complete_owner(self) -> None:
        self.assertEqual(owner_errors(*self.sources()), [])

    def test_unlisted_root_families_with_complete_owner(self) -> None:
        root, owner = self.sources()
        self.assertEqual(owner_errors(root, owner), [])
        for item, name in (
            ("struct OpenPostProbeExtra;", "OpenPostProbeExtra"),
            ("struct OpenEmbedDispatchExtra;", "OpenEmbedDispatchExtra"),
            ("struct AdmissionLockedHookExtra;", "AdmissionLockedHookExtra"),
            ("struct PostProbeStartupExtra;", "PostProbeStartupExtra"),
            ("struct PostProbeVisibilityExtra;", "PostProbeVisibilityExtra"),
            ("static ADMISSION_LOCKED_HOOK_NEW: u8 = 0;", "ADMISSION_LOCKED_HOOK_NEW"),
            ("const POST_PROBE_STARTUP_FAULT_NEW: u8 = 0;", "POST_PROBE_STARTUP_FAULT_NEW"),
            ("static POST_PROBE_VISIBILITY_FAULT_NEW: u8 = 0;", "POST_PROBE_VISIBILITY_FAULT_NEW"),
            ("fn install_post_probe_new() {}", "install_post_probe_new"),
            ("fn take_post_probe_new() {}", "take_post_probe_new"),
            ("fn install_admission_locked_new() {}", "install_admission_locked_new"),
            ("fn run_admission_locked_new() {}", "run_admission_locked_new"),
            ("impl OpenEmbedDispatchExtra {}", "impl OpenEmbedDispatchExtra"),
            ("impl OpenPostProbeExtra {}", "impl OpenPostProbeExtra"),
        ):
            with self.subTest(name=name):
                self.assertIn(f"root still defines open cleanup::{name}", owner_errors(root + "\n" + item, owner))

    def test_cfg_test_arm_is_exact(self) -> None:
        root, owner = self.sources()
        self.assertEqual(owner_errors(root, owner), [])
        mutated = owner.replace(
            "#[cfg(test)]\nstatic POST_PROBE_STARTUP_FAULT_FOR_TEST",
            "#[cfg(feature = \"test-hooks\")]\nstatic POST_PROBE_STARTUP_FAULT_FOR_TEST",
            1,
        )
        self.assertIn("open owner lacks cfg(test) POST_PROBE_STARTUP_FAULT_FOR_TEST", owner_errors(root, mutated))

    def test_fast_tier_registration(self) -> None:
        self.assertIn(
            "run_tier_suite fast test-slice90-open-cleanup-owner "
            "python3 scripts/tests/test_slice90_open_cleanup_owner.py",
            (ROOT / "scripts/agent-test.sh").read_text(),
        )


if __name__ == "__main__":
    unittest.main()
