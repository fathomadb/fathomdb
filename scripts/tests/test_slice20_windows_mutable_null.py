"""Actual mutable-lease readers must refuse a JSON null snapshot."""
import importlib.util
import json
import os
from pathlib import Path
import unittest
import uuid

ROOT = Path(__file__).resolve().parents[2]
PACKAGE = ROOT / 'dev/experiments/0.8.29-slice20-windows'
spec = importlib.util.spec_from_file_location('a6_null_operator', PACKAGE / 'a6_operator.py')
op = importlib.util.module_from_spec(spec)
spec.loader.exec_module(op)


@unittest.skipUnless(os.environ.get('FATHOM_WINDOWS_TESTS') == '1', 'explicit Windows control only')
class MutableNullTests(unittest.TestCase):
    def test_both_actual_readers_refuse_null(self):
        op.check_package()
        approved = op.read_json(op.TASK / 'actual-a6-batch-binaries-current.json')['binaries']
        supervisor = next(p for p in approved if p.endswith('Supervisor.exe'))
        native = (PACKAGE / 'native-bootstrap.ps1').read_text()
        reader = native[native.index('function Read-A6LeaseJson'):native.index('$s.LeaseReady=$true')]
        directory = op.GUEST + '\\control\\' + uuid.uuid4().hex
        script = reader + "\n$ErrorActionPreference='Stop';$dir=" + op.quote(directory) + ";[IO.Directory]::CreateDirectory($dir)|Out-Null;$p=Join-Path $dir 'lease.json';[IO.File]::WriteAllText($p,'null');$exe=" + op.quote(supervisor) + ";$hash=(Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant();if($hash -cne '" + approved[supervisor] + "'){throw 'actual helper hash differs'};$psFatal=$false;$csFatal=$false;try{$value=Read-A6LeaseJson $p}catch{$psFatal=$true};$assembly=[Reflection.Assembly]::LoadFile($exe);$method=$assembly.GetType('Supervisor').GetMethod('ReadMutable',[Reflection.BindingFlags]'Static,NonPublic');try{$value=$method.Invoke($null,[object[]]@([string]$p))}catch{$csFatal=$true};@{psNullFatal=$psFatal;csNullFatal=$csFatal;actualSupervisorSha256=$hash}|ConvertTo-Json -Compress"
        observed = json.loads(op.command(script, timeout=20))
        log = os.environ.get('FATHOM_WINDOWS_NULL_LOG')
        if log:
            Path(log).write_text(json.dumps(observed, indent=2) + '\n')
        self.assertTrue(observed['psNullFatal'], observed)
        self.assertTrue(observed['csNullFatal'], observed)


if __name__ == '__main__':
    unittest.main()
