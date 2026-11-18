param([Parameter(Mandatory = $true)][string]$Version)
$ErrorActionPreference = 'Stop'
if ($Version -cnotmatch '^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$') {
  throw "smoke-pypi-wheel: invalid version '$Version'"
}
$Version = $Version -creplace '-rc\.', 'rc' -creplace '-alpha\.', 'a' -creplace '-beta\.', 'b'
$consumer = Join-Path $PSScriptRoot 'registry-python.py'
$PSNativeCommandUseErrorActionPreference = $false
$work = Join-Path ([System.IO.Path]::GetTempPath()) ("fathomdb-pypi-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $work | Out-Null
try {
  python -m venv "$work/venv"
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  $python = Join-Path $work 'venv/Scripts/python.exe'
  & $python -m pip install --quiet --upgrade pip
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  & $python -m pip install --quiet "fathomdb==$Version"
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  $db = Join-Path $work 'smoke.fdb'
  & $python -I $consumer $Version $db write
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  & $python -I $consumer $Version $db reopen
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally { Remove-Item -Recurse -Force $work }
