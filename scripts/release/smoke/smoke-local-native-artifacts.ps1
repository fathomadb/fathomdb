param(
  [Parameter(Mandatory = $true)][string]$WheelDirectory,
  [Parameter(Mandatory = $true)][string]$TsDirectory,
  [Parameter(Mandatory = $true)][string]$PlatformPackageDirectory,
  [Parameter(Mandatory = $true)][string]$NapiLabel,
  [Parameter(Mandatory = $true)][string]$RetainedTestManifest
)

$ErrorActionPreference = 'Stop'
# Windows PowerShell 5.1 otherwise transcodes here-strings through its legacy
# native-pipeline encoding. Keep the Unicode provenance fixture byte-stable
# when Python and Node consume their scripts from stdin.
$OutputEncoding = [System.Text.UTF8Encoding]::new($false)

function Test-PathInside {
  param([string]$Child, [string]$Parent)
  $childPath = [System.IO.Path]::GetFullPath($Child)
  $parentPath = [System.IO.Path]::GetFullPath($Parent).TrimEnd('\', '/')
  return $childPath.StartsWith(
    $parentPath + [System.IO.Path]::DirectorySeparatorChar,
    [System.StringComparison]::OrdinalIgnoreCase
  )
}

function Assert-RegularPathFromRoot {
  param([string]$Path, [string]$Root)
  $rootPath = (Resolve-Path -LiteralPath $Root).Path.TrimEnd('\', '/')
  $pathValue = (Resolve-Path -LiteralPath $Path).Path
  if (-not (Test-PathInside $pathValue $rootPath)) {
    throw "smoke-local-native-artifacts: path escaped trusted root: $pathValue"
  }
  $currentPath = $pathValue
  while ($true) {
    $item = Get-Item -LiteralPath $currentPath -Force
    if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
      throw "smoke-local-native-artifacts: reparse point is not allowed in trusted path: $currentPath"
    }
    if ($currentPath.Equals($rootPath, [System.StringComparison]::OrdinalIgnoreCase)) {
      break
    }
    $parentPath = Split-Path -Parent $currentPath
    if ([string]::IsNullOrEmpty($parentPath) -or $parentPath -eq $currentPath) {
      throw "smoke-local-native-artifacts: trusted path did not reach root: $pathValue"
    }
    $currentPath = $parentPath.TrimEnd('\', '/')
  }
}

function Get-TreeDigest {
  param([Parameter(Mandatory = $true)][string]$Root)
  $rootPath = (Resolve-Path -LiteralPath $Root).Path.TrimEnd('\', '/')
  $rootItem = Get-Item -LiteralPath $rootPath -Force
  if ($rootItem -isnot [System.IO.DirectoryInfo] -or
      ($rootItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw "smoke-local-native-artifacts: digest root is not an ordinary directory: $rootPath"
  }
  $directories = New-Object 'System.Collections.Generic.Stack[string]'
  $directories.Push($rootPath)
  $lines = @()
  while ($directories.Count -gt 0) {
    $directory = $directories.Pop()
    foreach ($entry in @(Get-ChildItem -LiteralPath $directory -Force)) {
      if (($entry.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "smoke-local-native-artifacts: reparse point is not allowed in digest tree: $($entry.FullName)"
      }
      if ($entry -is [System.IO.DirectoryInfo]) {
        $directories.Push($entry.FullName)
        continue
      }
      if ($entry -isnot [System.IO.FileInfo]) {
        throw "smoke-local-native-artifacts: non-regular entry is not allowed in digest tree: $($entry.FullName)"
      }
      $relative = $entry.FullName.Substring($rootPath.Length).TrimStart('\', '/').Replace('\', '/')
      $hash = (Get-FileHash -LiteralPath $entry.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
      $lines += "$relative`t$hash"
    }
  }
  $lines = @($lines | Sort-Object)
  $bytes = [System.Text.Encoding]::UTF8.GetBytes(([string]::Join("`n", $lines) + "`n"))
  $sha256 = [System.Security.Cryptography.SHA256]::Create()
  try {
    return ([System.BitConverter]::ToString($sha256.ComputeHash($bytes))).Replace('-', '').ToLowerInvariant()
  } finally {
    $sha256.Dispose()
  }
}

function Get-NodeSummaryCount {
  param([string]$Output, [string]$Name)
  $matches = [regex]::Matches($Output, "(?m)^\s*# $Name (\d+)\s*$")
  if ($matches.Count -ne 1) {
    throw "smoke-local-native-artifacts: expected one Node summary field '$Name', found $($matches.Count)"
  }
  return [int]$matches[0].Groups[1].Value
}

$wheel = @(Get-ChildItem -Path $WheelDirectory -Filter '*.whl')
if ($wheel.Count -ne 1) {
  throw "smoke-local-native-artifacts: expected exactly one wheel in $WheelDirectory"
}
$native = Join-Path $TsDirectory "fathomdb.$NapiLabel.node"
if (-not (Test-Path $native -PathType Leaf)) {
  throw "smoke-local-native-artifacts: missing native N-API artifact $native"
}
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../../..')).Path
$manifestPath = (Resolve-Path -LiteralPath $RetainedTestManifest).Path
if (-not (Test-PathInside $manifestPath $repoRoot)) {
  throw 'smoke-local-native-artifacts: retained test manifest escaped the source checkout'
}
Assert-RegularPathFromRoot $manifestPath $repoRoot
if ((Get-Item -LiteralPath $manifestPath -Force) -isnot [System.IO.FileInfo]) {
  throw 'smoke-local-native-artifacts: retained test manifest is not a regular file'
}
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
if ($manifest.schema_version -ne 'fathomdb.slice73.windows-napi/v1') {
  throw "smoke-local-native-artifacts: unsupported retained test manifest schema $($manifest.schema_version)"
}
$manifestModules = @($manifest.modules)
$manifestFixtures = @($manifest.fixtures)
if ($manifestModules.Count -eq 0 -or $manifestFixtures.Count -eq 0) {
  throw 'smoke-local-native-artifacts: retained test manifest must select modules and fixtures'
}
if (@($manifestModules | Select-Object -Unique).Count -ne $manifestModules.Count) {
  throw 'smoke-local-native-artifacts: retained test manifest contains duplicate modules'
}
if (@($manifestFixtures | Select-Object -Unique).Count -ne $manifestFixtures.Count) {
  throw 'smoke-local-native-artifacts: retained test manifest contains duplicate fixtures'
}
foreach ($module in $manifestModules) {
  if ($module -notmatch '^[A-Za-z0-9][A-Za-z0-9-]*\.test\.js$') {
    throw "smoke-local-native-artifacts: unsafe retained test module $module"
  }
  $sourceModule = Join-Path $TsDirectory ("tests/" + $module.Replace('.js', '.ts'))
  if (-not (Test-Path -LiteralPath $sourceModule -PathType Leaf)) {
    throw "smoke-local-native-artifacts: missing retained test source $sourceModule"
  }
  $sourceModule = (Resolve-Path -LiteralPath $sourceModule).Path
  Assert-RegularPathFromRoot $sourceModule $repoRoot
  if ((Get-Item -LiteralPath $sourceModule -Force) -isnot [System.IO.FileInfo]) {
    throw "smoke-local-native-artifacts: retained test source is not a regular file: $sourceModule"
  }
}
foreach ($fixture in $manifestFixtures) {
  if ($fixture -notmatch '^[A-Za-z0-9][A-Za-z0-9._/-]*$' -or $fixture.Contains('..')) {
    throw "smoke-local-native-artifacts: unsafe retained fixture path $fixture"
  }
  $sourceFixture = Join-Path $repoRoot $fixture
  if (-not (Test-Path -LiteralPath $sourceFixture -PathType Leaf) -or
      -not (Test-PathInside $sourceFixture $repoRoot)) {
    throw "smoke-local-native-artifacts: missing or escaped retained fixture $fixture"
  }
  $sourceFixture = (Resolve-Path -LiteralPath $sourceFixture).Path
  Assert-RegularPathFromRoot $sourceFixture $repoRoot
  if ((Get-Item -LiteralPath $sourceFixture -Force) -isnot [System.IO.FileInfo]) {
    throw "smoke-local-native-artifacts: retained fixture is not a regular file: $fixture"
  }
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
    "body": "A\u00e9B",
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
dependency = engine.register_source_dependency({
    "schema_version": 1,
    "dependency_id": "wheel-dependency",
    "source_revision_id": "wheel-source-revision",
    "derived_revision_id": "wheel-derived-revision",
})
assert dependency.registered_dependency_generation == "1"
assert engine.dependencies_for_source({
    "schema_version": 1,
    "source_revision_id": "wheel-source-revision",
}).items == (dependency,)
assert engine.dependency_for_derived({
    "schema_version": 1,
    "derived_revision_id": "wheel-derived-revision",
}) == dependency
engine.search("runtime validation")
engine.close()
print("local Python wheel runtime validation: ok")
'@ | & $python -X utf8 - (Join-Path $work 'python-smoke.fdb')
  if ($LASTEXITCODE -ne 0) { throw 'smoke-local-native-artifacts: local Python wheel runtime smoke failed' }

  $main = Join-Path $work 'main'
  $npmRoot = Join-Path $work 'npm'
  $platform = Join-Path $npmRoot $NapiLabel
  $mirrorRoot = Join-Path $work 'mirror'
  $consumer = Join-Path $mirrorRoot 'src/ts'
  New-Item -ItemType Directory -Force -Path $main, $platform, $consumer | Out-Null
  Copy-Item (Join-Path $TsDirectory 'package.json') $main
  Copy-Item (Join-Path $TsDirectory 'LICENSE') $main
  Copy-Item (Join-Path $TsDirectory 'dist') $main -Recurse
  Copy-Item (Join-Path $PlatformPackageDirectory 'package.json') $platform
  Copy-Item (Join-Path $PlatformPackageDirectory 'LICENSE') $platform
  Copy-Item $native (Join-Path $platform "fathomdb.$NapiLabel.node")

  & bash (Join-Path $repoRoot 'scripts/release/npm-inject-optional-deps.sh') $main $npmRoot
  if ($LASTEXITCODE -ne 0) { throw 'smoke-local-native-artifacts: optionalDependency injection failed' }

  $platformPackage = Get-Content (Join-Path $platform 'package.json') -Raw | ConvertFrom-Json
  $mainPackage = Get-Content (Join-Path $main 'package.json') -Raw | ConvertFrom-Json
  if ($mainPackage.optionalDependencies.($platformPackage.name) -ne $mainPackage.version) {
    throw 'smoke-local-native-artifacts: matched optionalDependency is absent or version-skewed'
  }

  Push-Location $platform
  try {
    $platformTarball = (& npm pack --silent).Trim()
    if ($LASTEXITCODE -ne 0) { throw 'smoke-local-native-artifacts: platform npm pack failed' }
  } finally {
    Pop-Location
  }
  Push-Location $main
  try {
    $mainTarball = (& npm pack --silent).Trim()
    if ($LASTEXITCODE -ne 0) { throw 'smoke-local-native-artifacts: main npm pack failed' }
  } finally {
    Pop-Location
  }
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
  body: "A\u00e9B",
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
const dependency = await engine.registerSourceDependency({
  schemaVersion: 1,
  dependencyId: "npm-dependency",
  sourceRevisionId: "npm-source-revision",
  derivedRevisionId: "npm-derived-revision",
});
if (dependency.registeredDependencyGeneration !== "1") throw new Error("bad generation");
const bySource = await engine.dependenciesForSource({
  schemaVersion: 1,
  sourceRevisionId: "npm-source-revision",
});
if (bySource.items.length !== 1 || bySource.items[0].dependencyId !== "npm-dependency") {
  throw new Error("source dependency lookup failed");
}
const byDerived = await engine.dependencyForDerived({
  schemaVersion: 1,
  derivedRevisionId: "npm-derived-revision",
});
if (byDerived?.dependencyId !== "npm-dependency") {
  throw new Error("derived dependency lookup failed");
}
await engine.search("runtime validation");
await engine.close();
console.log("local N-API package runtime validation: ok");
'@ | Set-Content (Join-Path $consumer 'smoke.mjs')
  Push-Location $consumer
  try {
    & npm install --offline --ignore-scripts
    if ($LASTEXITCODE -ne 0) { throw 'smoke-local-native-artifacts: local npm install failed' }
    & node smoke.mjs (Join-Path $work 'npm-smoke.fdb')
    if ($LASTEXITCODE -ne 0) { throw 'smoke-local-native-artifacts: local npm runtime smoke failed' }
  } finally {
    Pop-Location
  }

  Push-Location $consumer
  try {
    $resolvedMainModule = (& node -e 'process.stdout.write(require.resolve(process.argv[1]))' 'fathomdb').Trim()
    if ($LASTEXITCODE -ne 0) { throw 'smoke-local-native-artifacts: main module resolution failed' }
    $resolvedNativeModule = (& node -e 'process.stdout.write(require.resolve(process.argv[1]))' $platformPackage.name).Trim()
    if ($LASTEXITCODE -ne 0) { throw 'smoke-local-native-artifacts: native module resolution failed' }
  } finally {
    Pop-Location
  }
  $resolvedMainModule = (Resolve-Path -LiteralPath $resolvedMainModule).Path
  $resolvedNativeModule = (Resolve-Path -LiteralPath $resolvedNativeModule).Path
  if (-not (Test-PathInside $resolvedMainModule $consumer) -or (Test-PathInside $resolvedMainModule $repoRoot)) {
    throw 'smoke-local-native-artifacts: resolved main module escaped isolated consumer'
  }
  if (-not (Test-PathInside $resolvedNativeModule $consumer) -or (Test-PathInside $resolvedNativeModule $repoRoot)) {
    throw 'smoke-local-native-artifacts: resolved native module escaped isolated consumer'
  }
  Assert-RegularPathFromRoot $resolvedMainModule $consumer
  Assert-RegularPathFromRoot $resolvedNativeModule $consumer
  $builtNativeSha256 = (Get-FileHash -LiteralPath $native -Algorithm SHA256).Hash.ToLowerInvariant()
  $installedNativeSha256 = (Get-FileHash -LiteralPath $resolvedNativeModule -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($installedNativeSha256 -ne $builtNativeSha256) {
    throw 'smoke-local-native-artifacts: installed native artifact digest mismatch'
  }

  $compiledDist = Join-Path $consumer 'dist'
  $tsc = Join-Path $TsDirectory 'node_modules/typescript/bin/tsc'
  & node $tsc -p (Join-Path $TsDirectory 'tsconfig.json') --outDir $compiledDist
  if ($LASTEXITCODE -ne 0) { throw 'smoke-local-native-artifacts: retained TypeScript test compilation failed' }
  $installedSdk = Join-Path $consumer 'node_modules/fathomdb/dist'
  $installedSdkTreeSha256 = Get-TreeDigest $installedSdk
  $stagedSdk = Join-Path $compiledDist 'src'
  Remove-Item -LiteralPath $stagedSdk -Recurse -Force
  New-Item -ItemType Directory -Path $stagedSdk | Out-Null
  foreach ($installedEntry in @(Get-ChildItem -LiteralPath $installedSdk -Force)) {
    Copy-Item -LiteralPath $installedEntry.FullName -Destination $stagedSdk -Recurse -Force
  }
  $stagedSdkTreeSha256 = Get-TreeDigest $stagedSdk
  if ($stagedSdkTreeSha256 -ne $installedSdkTreeSha256) {
    throw 'smoke-local-native-artifacts: sdk tree digest mismatch'
  }

  foreach ($fixture in $manifest.fixtures) {
    $sourceFixture = (Resolve-Path -LiteralPath (Join-Path $repoRoot $fixture)).Path
    Assert-RegularPathFromRoot $sourceFixture $repoRoot
    $stagedFixture = Join-Path $mirrorRoot $fixture
    if (-not (Test-PathInside $stagedFixture $mirrorRoot)) {
      throw "smoke-local-native-artifacts: retained fixture destination escaped mirror: $fixture"
    }
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $stagedFixture) | Out-Null
    Copy-Item -LiteralPath $sourceFixture -Destination $stagedFixture
  }

  $testTemp = Join-Path $work 'test-temp'
  New-Item -ItemType Directory -Path $testTemp | Out-Null
  $oldTemp = $env:TEMP
  $oldTmp = $env:TMP
  $env:TEMP = $testTemp
  $env:TMP = $testTemp
  $moduleResults = @()
  $totalTests = 0
  $totalPass = 0
  Push-Location $consumer
  try {
    foreach ($module in $manifest.modules) {
      $modulePath = Join-Path $compiledDist ("tests/" + $module)
      if (-not (Test-Path -LiteralPath $modulePath -PathType Leaf) -or
          -not (Test-PathInside $modulePath $compiledDist)) {
        throw "smoke-local-native-artifacts: retained compiled module is missing or escaped: $module"
      }
      $moduleOutput = @(& node --test --test-reporter=tap $modulePath 2>&1)
      $moduleExit = $LASTEXITCODE
      $moduleText = [string]::Join("`n", @($moduleOutput | ForEach-Object { $_.ToString() }))
      Write-Output $moduleText
      $counts = [ordered]@{
        tests = Get-NodeSummaryCount $moduleText 'tests'
        pass = Get-NodeSummaryCount $moduleText 'pass'
        fail = Get-NodeSummaryCount $moduleText 'fail'
        cancelled = Get-NodeSummaryCount $moduleText 'cancelled'
        skipped = Get-NodeSummaryCount $moduleText 'skipped'
        todo = Get-NodeSummaryCount $moduleText 'todo'
      }
      if ($moduleExit -ne 0 -or $counts.tests -le 0 -or $counts.pass -ne $counts.tests -or
          $counts.fail -ne 0 -or $counts.cancelled -ne 0 -or $counts.skipped -ne 0 -or
          $counts.todo -ne 0) {
        throw "smoke-local-native-artifacts: retained module failed closed: $module"
      }
      $totalTests += $counts.tests
      $totalPass += $counts.pass
      $moduleResults += [ordered]@{
        module = $module
        tests = $counts.tests
        pass = $counts.pass
      }
    }
  } finally {
    Pop-Location
    $env:TEMP = $oldTemp
    $env:TMP = $oldTmp
  }
  if ($moduleResults.Count -ne $manifestModules.Count -or $totalTests -le 0 -or $totalPass -ne $totalTests) {
    throw 'smoke-local-native-artifacts: retained module aggregate failed closed'
  }

  $result = [ordered]@{
    schemaVersion = 'fathomdb.slice73.windows-napi-result/v1'
    outcome = 'pass'
    manifestSha256 = (Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256).Hash.ToLowerInvariant()
    resolvedMainModule = $resolvedMainModule
    resolvedNativeModule = $resolvedNativeModule
    nativeSha256 = $installedNativeSha256
    installedSdkTreeSha256 = $installedSdkTreeSha256
    stagedSdkTreeSha256 = $stagedSdkTreeSha256
    modules = $moduleResults
    totals = [ordered]@{
      modules = $moduleResults.Count
      tests = $totalTests
      pass = $totalPass
      fail = 0
      cancelled = 0
      skipped = 0
      todo = 0
    }
  }
  Write-Output ('slice73-windows-napi-result:' + ($result | ConvertTo-Json -Depth 6 -Compress))
  Write-Output 'smoke-local-native-artifacts: ok - local wheel + matched N-API package + retained modules validated'
} finally {
  Remove-Item -LiteralPath $work -Recurse -Force
}
