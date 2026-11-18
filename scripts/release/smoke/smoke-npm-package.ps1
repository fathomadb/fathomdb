param([Parameter(Mandatory = $true)][string]$Version)
$ErrorActionPreference = 'Stop'
if ($Version -cnotmatch '^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$') {
  throw "smoke-npm-package: invalid version '$Version'"
}
$PSNativeCommandUseErrorActionPreference = $false
$consumer = Join-Path $PSScriptRoot 'registry-node.mjs'
$work = Join-Path ([System.IO.Path]::GetTempPath()) ("fathomdb-npm-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $work | Out-Null
try {
  Push-Location $work
  npm init -y | Out-Null
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  $installLog = Join-Path $work 'npm-install.log'
  npm install --loglevel=warn "fathomdb@$Version" *> $installLog
  $installExit = $LASTEXITCODE
  node $consumer inspect $work $Version $installLog $installExit | Out-Null
  $checkExit = $LASTEXITCODE
  if ($installExit -ne 0) { exit $installExit }
  if ($checkExit -ne 0) { exit $checkExit }
  node $consumer write $work $Version
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  node $consumer reopen $work $Version
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
  Pop-Location
  Remove-Item -Recurse -Force $work
}
