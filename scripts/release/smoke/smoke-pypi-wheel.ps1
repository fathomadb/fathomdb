param([Parameter(Mandatory = $true)][string]$Version)
$ErrorActionPreference = 'Stop'
if ($Version -cnotmatch '^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$') {
  throw "smoke-pypi-wheel: invalid version '$Version'"
}
$Version = $Version -creplace '-rc\.', 'rc' -creplace '-alpha\.', 'a' -creplace '-beta\.', 'b'
$consumer = Join-Path $PSScriptRoot 'registry-python.py'
$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '../../..')
$slice55 = Join-Path $repoRoot 'src/python/tests/smoke_slice55_installed.py'
$PSNativeCommandUseErrorActionPreference = $false
$work = Join-Path ([System.IO.Path]::GetTempPath()) ("fathomdb-pypi-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $work | Out-Null
try {
  python -m venv "$work/venv"
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  $python = Join-Path $work 'venv/Scripts/python.exe'
  & $python -m pip install --quiet --upgrade pip
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  $maxAttempts = if ($env:FATHOMDB_SMOKE_PYPI_INSTALL_ATTEMPTS) { [int]$env:FATHOMDB_SMOKE_PYPI_INSTALL_ATTEMPTS } else { 5 }
  if ($maxAttempts -lt 1) { throw "smoke-pypi-wheel: FATHOMDB_SMOKE_PYPI_INSTALL_ATTEMPTS must be >= 1" }
  $retryDelay = if ($env:FATHOMDB_SMOKE_PYPI_RETRY_DELAY_SECS) { [int]$env:FATHOMDB_SMOKE_PYPI_RETRY_DELAY_SECS } else { 15 }
  $installLog = Join-Path $work 'pip-install.log'
  $installExit = 0
  for ($attempt = 1; $attempt -le $maxAttempts; $attempt++) {
    & $python -m pip install --quiet "fathomdb[cli]==$Version" *> $installLog
    $installExit = $LASTEXITCODE
    if ($installExit -eq 0) { break }
    $logText = if (Test-Path $installLog) { Get-Content -Raw $installLog } else { '' }
    $isSkew = $logText -match 'No matching distribution found for (fathomdb|fathomdb-cli)==|Could not find a version that satisfies the requirement (fathomdb|fathomdb-cli)=='
    if ($attempt -lt $maxAttempts -and $isSkew) {
      Write-Error "smoke-pypi-wheel: transient PyPI simple-index skew on attempt $attempt/$maxAttempts; retrying in ${retryDelay}s" -ErrorAction Continue
      if ($retryDelay -gt 0) { Start-Sleep -Seconds $retryDelay }
      continue
    }
    if ($logText) { [Console]::Error.Write($logText) }
    break
  }
  if ($installExit -ne 0) { exit $installExit }
  $cli = Join-Path $work 'venv/Scripts/fathomdb.exe'
  if (-not (Test-Path $cli)) { throw "smoke-pypi-wheel: expected CLI binary '$cli' not installed by fathomdb[cli]==$Version" }
  $forbiddenCli = Join-Path $work 'venv/Scripts/perf-regression-check.exe'
  if (Test-Path $forbiddenCli) { throw "smoke-pypi-wheel: unexpected perf-regression-check.exe shipped in fathomdb-cli wheel" }
  & $python -c "import fathomdb; print(fathomdb.__version__)"
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  & $cli --version
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  & $cli doctor platform --json
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  $db = Join-Path $work 'smoke.fdb'
  & $python -I $consumer $Version $db write
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  & $python -I $consumer $Version $db reopen
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  $prevPythonPath = $env:PYTHONPATH
  try {
    Remove-Item Env:PYTHONPATH -ErrorAction SilentlyContinue
  & $python $slice55
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  } finally {
    if ($null -ne $prevPythonPath) { $env:PYTHONPATH = $prevPythonPath }
  }
} finally { Remove-Item -Recurse -Force $work }
