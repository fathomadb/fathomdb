"""Require real guarded entry; namespace-root records are protocol coverage only."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from g_boundary import TaskBoundary
from g_campaign import make_g_plan
from g_freeze import git
from g_boundary_tests import PHASE


def prepare_entry(root):
    """Use harmless downstream work, without qualifying frozen inputs or GPU state."""
    frozen = root / "allocation/frozen"
    frozen.mkdir(parents=True)
    marker = root / "actual-dispatch"
    spec = json.loads(PHASE.with_name("g-cells.json").read_text())
    manifest = dict(schema="slice30-g-campaign/v1", source_sha=git("rev-parse", "HEAD"),
                    allocation=str(root / "allocation"), plan=make_g_plan(spec),
                    inputs={name: {"path": name} for name in ("corpus", "queries")},
                    binaries={}, gpu={"uuid": "GPU-protocol-control"}, device_policy="cuda:0")
    (frozen / "manifest.json").write_text(json.dumps(manifest))
    (root / "allocation/g-budget.json").write_text(json.dumps(dict(schema="slice30-gd-budget/v1",used_ns=17,inflight=None)))
    code = "\n".join([
        "import json,pathlib,subprocess,sys",
        "from unittest.mock import patch",
        f"sys.path.insert(0,{str(PHASE.parent)!r})",
        "from g_runner import run_frozen",
        f"frozen=pathlib.Path({str(frozen)!r}); manifest=json.loads((frozen/'manifest.json').read_text())",
        "def dispatch(_manifest,_root):",
        f"    subprocess.run([sys.executable,'-c',\"import pathlib; pathlib.Path({str(marker)!r}).touch()\"],check=True,timeout=3)",
        "    return []",
        "with patch('g_freeze.validate_frozen',return_value=manifest), patch('g_gpu.inspect_gpu',return_value={'applications':[]}), patch('g_runner.run_prepared',side_effect=dispatch):",
        "    run_frozen(frozen)",
    ])
    return code, marker


class GRequiredGuardEntryTests(unittest.TestCase):
    def test_missing_marker_refuses_actual_child_dispatch_from_normal_entry(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            code, marker = prepare_entry(root)
            child = subprocess.run([sys.executable, "-c", code], capture_output=True, text=True, timeout=10,
                                   env=dict(os.environ, S30_TASK=str(root)), check=False)
            self.assertFalse(marker.exists(), "bare normal G entry dispatched an actual child without a guardian marker")
            self.assertNotEqual(child.returncode, 0)
            self.assertIn("containment", child.stderr)
            self.assertEqual(json.loads((root / "allocation/g-budget.json").read_text())["used_ns"], 17)

    def test_live_guardian_actual_matching_cgroup_permits_normal_entry(self):
        # Compatibility coverage, not a new RED claim or positive model/GPU evidence.
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            code, marker = prepare_entry(root)
            for name in ("logs", "receipts"):
                (root / name).mkdir()
            harness = "\n".join([
                "import runpy,sys",
                f"sys.path.insert(0,{str(PHASE.parent)!r})",
                "import g_resource",
                "# Synthetic lifecycle start observations, not host-readiness qualification.",
                "g_resource.resource_sample=lambda *_:dict(root_free=40*1024**3,task_free=300*1024**3,memory_available=20*1024**3,owned=0,measurement_retries=[])",
                f"sys.argv=[{str(PHASE)!r},'positive-entry',sys.executable,'-c',{code!r}]",
                f"runpy.run_path({str(PHASE)!r},run_name='__main__')",
            ])
            child = subprocess.run([sys.executable, "-c", harness], capture_output=True, text=True, timeout=10,
                                   env=dict(os.environ, S30_SOURCE=str(PHASE.parents[3]), S30_TASK=str(root)), check=False)
            self.assertEqual(child.returncode, 0, "actual delegated containment unavailable or positive entry refused: " + child.stdout + child.stderr)
            self.assertTrue(marker.exists(), "live matching guardian failed to permit actual harmless dispatch")
            report = json.loads((root / "receipts/positive-entry.json").read_text())
            self.assertFalse(report["cleanup_uncertain"])
            self.assertFalse(report["boundary"]["populated_after"])
            self.assertTrue(report["boundary"]["removed_exact_empty_boundary"])
            self.assertFalse((root / "g-containment-state.json").exists())


class GNamespaceRootRecordTests(unittest.TestCase):
    def test_namespace_root_membership_record_has_one_separator(self):
        with tempfile.TemporaryDirectory() as temporary:
            boundary = TaskBoundary(Path(temporary))
            actual_read = Path.read_text
            def namespace_view(path, *args, **kwargs):
                if str(path) == "/proc/self/cgroup":
                    return "0::/\n"
                return actual_read(path, *args, **kwargs)
            try:
                # Stop before opening/creating any real global-root cgroup.
                with patch.object(Path, "read_text", namespace_view), \
                        patch("g_boundary.os.open", side_effect=PermissionError("controlled pre-create refusal")):
                    with self.assertRaises(PermissionError):
                        boundary.create()
                self.assertEqual(boundary.evidence["relative_path"], "/" + boundary.name,
                                 "namespace-root membership record has a duplicate separator")
            finally:
                boundary.cleanup()
