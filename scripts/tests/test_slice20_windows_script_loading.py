"""Load hash-verified owned script text without changing Windows policy."""
import base64
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import unittest
import uuid

ROOT = Path(__file__).resolve().parents[2]
PACKAGE = ROOT / "dev/experiments/0.8.29-slice20-windows"
spec = importlib.util.spec_from_file_location("a6_loading_operator", PACKAGE / "a6_operator.py")
op = importlib.util.module_from_spec(spec)
spec.loader.exec_module(op)


@unittest.skipUnless(os.environ.get("FATHOM_WINDOWS_TESTS") == "1", "actual Windows opt-in")
class ScriptLoadingTests(unittest.TestCase):
    def test_verified_script_receives_context_under_unchanged_policy(self):
        control = op.GUEST + "\\control\\" + uuid.uuid4().hex
        path = control + "\\loading.ps1"
        text = "param([string]$ScriptPath,[string]$Control);@{value='loaded';script=$ScriptPath;control=$Control;policy=(Get-ExecutionPolicy).ToString()}|ConvertTo-Json -Compress"
        payload = base64.b64encode(text.encode()).decode()
        op.command("[IO.Directory]::CreateDirectory(" + op.quote(control) + ")|Out-Null;[IO.File]::WriteAllBytes(" + op.quote(path) + ",[Convert]::FromBase64String('" + payload + "'))")
        policy = op.command("(Get-ExecutionPolicy).ToString()")
        digest = hashlib.sha256(text.encode()).hexdigest()
        invoke = op.script_invocation(path, digest, "-Control " + op.quote(control))
        self.assertEqual(json.loads(op.command(invoke)), {"value": "loaded", "script": path, "control": control, "policy": policy})
        with self.assertRaisesRegex(RuntimeError, "launch script digest differs"):
            op.command(op.script_invocation(path, "0" * 64, "-Control " + op.quote(control)))
        self.assertEqual(op.command("(Get-ExecutionPolicy).ToString()"), policy)


if __name__ == "__main__":
    unittest.main()
