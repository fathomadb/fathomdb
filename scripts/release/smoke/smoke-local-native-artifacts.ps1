param(
  [Parameter(Mandatory = $true)][string]$WheelDirectory,
  [Parameter(Mandatory = $true)][string]$TsDirectory,
  [Parameter(Mandatory = $true)][string]$PlatformPackageDirectory,
  [Parameter(Mandatory = $true)][string]$NapiLabel
)

$ErrorActionPreference = 'Stop'
$wheel = Get-ChildItem -Path $WheelDirectory -Filter '*.whl'
if ($wheel.Count -ne 1) {
  throw "smoke-local-native-artifacts: expected exactly one wheel in $WheelDirectory"
}
$native = Join-Path $TsDirectory "fathomdb.$NapiLabel.node"
if (-not (Test-Path $native -PathType Leaf)) {
  throw "smoke-local-native-artifacts: missing native N-API artifact $native"
}

$work = Join-Path ([System.IO.Path]::GetTempPath()) ("fathomdb-local-native-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $work | Out-Null
try {
  $venv = Join-Path $work 'python-venv'
  python -m venv $venv
  $python = Join-Path $venv 'Scripts/python.exe'
  & $python -m pip install --no-index --find-links $WheelDirectory fathomdb
  if ($LASTEXITCODE -ne 0) { throw 'smoke-local-native-artifacts: local wheel install failed' }
  @'
import sys
from fathomdb import Engine

engine = Engine.open(sys.argv[1])
engine.write([{
    "kind": "doc",
    "body": "AéB",
    "source_id": "smoke:local-native-wheel",
    "provenance": {
        "schema_version": 1,
        "role": "canonical",
        "artifact_revision_id": "wheel-source-revision",
        "source_version_id": "wheel-source-version",
    },
}, {
    "kind": "entity",
    "body": "local native wheel runtime validation",
    "source_id": "smoke:local-native-wheel",
    "provenance": {
        "schema_version": 1,
        "role": "derived",
        "artifact_revision_id": "wheel-derived-revision",
        "source_version_id": "wheel-source-version",
        "source_revision_id": "wheel-source-revision",
        "source_locator": {"kind": "whole_body"},
        "canonical_source_hash": {
            "algorithm": "sha256",
            "digest_hex": "0290cc0c54e573ce8b5150fcdaa22ee7506e99ede078ce66d012eaa901b6edbb",
        },
    },
}])
engine.search("runtime validation")
engine.close()
print("local Python wheel runtime validation: ok")
'@ | & $python - (Join-Path $work 'python-smoke.fdb')
  if ($LASTEXITCODE -ne 0) { throw 'smoke-local-native-artifacts: local Python wheel runtime smoke failed' }

  $main = Join-Path $work 'main'
  $npmRoot = Join-Path $work 'npm'
  $platform = Join-Path $npmRoot $NapiLabel
  $consumer = Join-Path $work 'consumer'
  New-Item -ItemType Directory -Force -Path $main, $platform, $consumer | Out-Null
  Copy-Item (Join-Path $TsDirectory 'package.json') $main
  Copy-Item (Join-Path $TsDirectory 'LICENSE') $main
  Copy-Item (Join-Path $TsDirectory 'dist') $main -Recurse
  Copy-Item (Join-Path $PlatformPackageDirectory 'package.json') $platform
  Copy-Item (Join-Path $PlatformPackageDirectory 'LICENSE') $platform
  Copy-Item $native (Join-Path $platform "fathomdb.$NapiLabel.node")

  $repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../../..')).Path
  & bash (Join-Path $repoRoot 'scripts/release/npm-inject-optional-deps.sh') $main $npmRoot
  if ($LASTEXITCODE -ne 0) { throw 'smoke-local-native-artifacts: optionalDependency injection failed' }

  $platformPackage = Get-Content (Join-Path $platform 'package.json') -Raw | ConvertFrom-Json
  $mainPackage = Get-Content (Join-Path $main 'package.json') -Raw | ConvertFrom-Json
  if ($mainPackage.optionalDependencies.($platformPackage.name) -ne $mainPackage.version) {
    throw "smoke-local-native-artifacts: matched optionalDependency is absent or version-skewed"
  }

  Push-Location $platform
  $platformTarball = (& npm pack --silent).Trim()
  Pop-Location
  Push-Location $main
  $mainTarball = (& npm pack --silent).Trim()
  Pop-Location
  $mainSpec = [System.Uri]::new((Join-Path $main $mainTarball)).AbsoluteUri
  $platformSpec = [System.Uri]::new((Join-Path $platform $platformTarball)).AbsoluteUri
  @{
    private = $true
    type = 'module'
    dependencies = @{
      fathomdb = $mainSpec
      $platformPackage.name = $platformSpec
    }
  } | ConvertTo-Json -Depth 3 | Set-Content (Join-Path $consumer 'package.json')
  @'
import { Engine } from "fathomdb";

const engine = await Engine.open(process.argv[2]);
await engine.write([{
  kind: "doc",
  body: "AéB",
  sourceId: "smoke:local-native-npm",
  provenance: {
    schemaVersion: 1,
    role: "canonical",
    artifactRevisionId: "npm-source-revision",
    sourceVersionId: "npm-source-version",
  },
}, {
  kind: "entity",
  body: "local native npm runtime validation",
  sourceId: "smoke:local-native-npm",
  provenance: {
    schemaVersion: 1,
    role: "derived",
    artifactRevisionId: "npm-derived-revision",
    sourceVersionId: "npm-source-version",
    sourceRevisionId: "npm-source-revision",
    sourceLocator: { kind: "whole_body" },
    canonicalSourceHash: {
      algorithm: "sha256",
      digestHex: "0290cc0c54e573ce8b5150fcdaa22ee7506e99ede078ce66d012eaa901b6edbb",
    },
  },
}]);
await engine.search("runtime validation");
await engine.close();
console.log("local N-API package runtime validation: ok");
'@ | Set-Content (Join-Path $consumer 'smoke.mjs')
  Push-Location $consumer
  & npm install --offline --ignore-scripts
  if ($LASTEXITCODE -ne 0) { throw 'smoke-local-native-artifacts: local npm install failed' }
  & node smoke.mjs (Join-Path $work 'npm-smoke.fdb')
  if ($LASTEXITCODE -ne 0) { throw 'smoke-local-native-artifacts: local npm runtime smoke failed' }
  Pop-Location
  Write-Output 'smoke-local-native-artifacts: ok — local wheel + matched N-API package validated'
} finally {
  Remove-Item -Recurse -Force $work
}
