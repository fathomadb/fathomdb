#!/usr/bin/env python3
"""Structural contracts for focused installed Windows N-API coverage."""

from __future__ import annotations

import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / "scripts/release/smoke/slice73-windows-napi-modules.json"
CI = ROOT / ".github/workflows/ci.yml"
SMOKE = ROOT / "scripts/release/smoke/smoke-local-native-artifacts.ps1"
AGENT_TEST = ROOT / "scripts/agent-test.sh"

EXPECTED_MODULES = [
    "slice15-identity-provenance.test.js",
    "slice15b-search-validity.test.js",
    "slice15d-projection-registry.test.js",
    "slice20-source-dependencies.test.js",
    "slice22-projection-status.test.js",
    "slice25-actuation.test.js",
    "slice30-dependency-closure.test.js",
    "slice35-frozen-read.test.js",
    "slice45-pagination.test.js",
    "slice50-evidence.test.js",
    "slice50-evidence-response-validation.test.js",
    "slice55-candidate-native-explanation.test.js",
    "slice60-graph-expand.test.js",
]

EXPECTED_FIXTURES = [
    "src/conformance/provenance-v1.json",
    "src/conformance/governed-surface-allowlist.json",
    "dev/fixtures/slice20-dependency-conformance-v1.json",
    "dev/fixtures/slice25-actuation-conformance-v1.json",
    "dev/fixtures/slice60-graph-expand-conformance-v1.json",
]


class Slice73WindowsNapiContract(unittest.TestCase):
    def manifest(self) -> dict:
        self.assertTrue(MANIFEST.is_file(), f"missing manifest: {MANIFEST}")
        return json.loads(MANIFEST.read_text(encoding="utf-8"))

    def test_manifest_freezes_the_human_approved_inputs(self) -> None:
        manifest = self.manifest()
        self.assertEqual(manifest.get("schema_version"), "fathomdb.slice73.windows-napi/v1")
        self.assertEqual(manifest.get("modules"), EXPECTED_MODULES)
        self.assertEqual(manifest.get("fixtures"), EXPECTED_FIXTURES)
        self.assertEqual(set(manifest), {"schema_version", "modules", "fixtures"})

        for module in EXPECTED_MODULES:
            source = ROOT / "src/ts/tests" / module.replace(".js", ".ts")
            self.assertTrue(source.is_file(), f"missing selected source module: {source}")
        for fixture in EXPECTED_FIXTURES:
            self.assertTrue((ROOT / fixture).is_file(), f"missing selected fixture: {fixture}")

    def test_ci_routes_every_external_input_and_wires_windows_only(self) -> None:
        ci = CI.read_text(encoding="utf-8")
        route_start = ci.index("            native_artifact_harness:\n")
        route_end = ci.index("      - uses: dorny/paths-filter@", route_start)
        route = ci[route_start:route_end]
        for path in [
            "scripts/release/smoke/slice73-windows-napi-modules.json",
            *EXPECTED_FIXTURES,
        ]:
            self.assertIn(f"              - '{path}'", route)

        windows_step = ci.index("      - name: Validate local wheel and N-API package (Windows)\n")
        windows_end = ci.index("\n  # Docs-only fast-path", windows_step)
        block = ci[windows_step:windows_end]
        self.assertIn("if: matrix.runner == 'windows-latest'", block)
        self.assertIn("-RetainedTestManifest", block)
        self.assertIn("scripts/release/smoke/slice73-windows-napi-modules.json", block)

        non_windows_step = ci.index("      - name: Validate local wheel and N-API package\n")
        non_windows_block = ci[non_windows_step:windows_step]
        self.assertNotIn("slice73-windows-napi-modules.json", non_windows_block)

    def test_powershell_smoke_fails_closed_and_proves_installed_bytes(self) -> None:
        smoke = SMOKE.read_text(encoding="utf-8")
        required = [
            "[string]$RetainedTestManifest",
            "ConvertFrom-Json",
            "Get-TreeDigest",
            "installedSdkTreeSha256",
            "stagedSdkTreeSha256",
            "sdk tree digest mismatch",
            "resolved main module escaped isolated consumer",
            "resolved native module escaped isolated consumer",
            "$env:TEMP = $testTemp",
            "$env:TMP = $testTemp",
            "foreach ($module in $manifest.modules)",
            "& node --test $modulePath",
            "tests -le 0",
            "pass -ne $counts.tests",
            "fail -ne 0",
            "cancelled -ne 0",
            "skipped -ne 0",
            "todo -ne 0",
            "slice73-windows-napi-result:",
        ]
        for fragment in required:
            self.assertIn(fragment, smoke)
        self.assertNotIn("dist/tests/*.test.js", smoke)

    def test_powershell_rejects_reparse_ancestors_and_hidden_tree_omissions(self) -> None:
        smoke = SMOKE.read_text(encoding="utf-8")
        required = [
            "function Assert-RegularPathFromRoot",
            "Assert-RegularPathFromRoot $manifestPath $repoRoot",
            "Assert-RegularPathFromRoot $sourceModule $repoRoot",
            "Assert-RegularPathFromRoot $sourceFixture $repoRoot",
            "Get-ChildItem -LiteralPath $rootPath -Recurse -Force",
            "Get-ChildItem -LiteralPath $installedSdk -Force",
            "non-regular entry is not allowed in digest tree",
        ]
        for fragment in required:
            self.assertIn(fragment, smoke)
        self.assertNotIn("Copy-Item -Path (Join-Path $installedSdk '*')", smoke)

    def test_fast_tier_registers_the_structural_contract(self) -> None:
        self.assertIn(
            "run_tier_suite fast test-slice73-windows-napi-ci "
            "python3 scripts/tests/test_slice73_windows_napi_ci.py",
            AGENT_TEST.read_text(encoding="utf-8"),
        )


if __name__ == "__main__":
    unittest.main()
