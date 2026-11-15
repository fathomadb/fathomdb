"""The actual bootstrap attribute arguments must construct pointer-sized values."""
import importlib.util
import json
import os
from pathlib import Path
import unittest
import uuid

ROOT = Path(__file__).resolve().parents[2]
PACKAGE = ROOT / "dev/experiments/0.8.29-slice20-windows"
spec = importlib.util.spec_from_file_location("a6_uintptr_operator", PACKAGE / "a6_operator.py")
op = importlib.util.module_from_spec(spec)
spec.loader.exec_module(op)


@unittest.skipUnless(os.environ.get("FATHOM_WINDOWS_TESTS") == "1", "actual Windows opt-in")
class UIntPtrTests(unittest.TestCase):
    def test_attribute_arguments_keep_exact_pointer_values(self):
        control = op.GUEST + "\\control\\" + uuid.uuid4().hex
        path = control + "\\native-bootstrap.ps1"
        write = "$ErrorActionPreference='Stop';[IO.Directory]::CreateDirectory(" + op.quote(control) + ")|Out-Null;$f=[IO.File]::Open(" + op.quote(path) + ",[IO.FileMode]::CreateNew);try{[Console]::OpenStandardInput().CopyTo($f)}finally{$f.Dispose()}"
        op.command(write, input_bytes=(PACKAGE / "native-bootstrap.ps1").read_bytes())
        script = "$ErrorActionPreference='Stop';$s=Get-Content -Raw -LiteralPath " + op.quote(path) + ";$t=$null;$e=$null;$a=[Management.Automation.Language.Parser]::ParseInput($s,[ref]$t,[ref]$e);"
        script += "$calls=$a.FindAll({param($n)$n -is [Management.Automation.Language.InvokeMemberExpressionAst] -and $n.Member.Value -eq 'UpdateProcThreadAttribute'},$true);"
        script += "$values=@(foreach($call in $calls){foreach($index in @(2,4)){$v=& ([ScriptBlock]::Create($call.Arguments[$index].Extent.Text));@{type=$v.GetType().FullName;value=$v.ToUInt64()}}});$values|ConvertTo-Json -Compress"
        self.assertEqual(json.loads(op.command(script)), [{"type": "System.UIntPtr", "value": n} for n in [0x2000D, 8, 0x20002, 24]])


if __name__ == "__main__":
    unittest.main()
