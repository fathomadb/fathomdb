param([Parameter(Mandatory = $true)][string]$Version)
$ErrorActionPreference = 'Stop'
if ($Version -cnotmatch '^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$') {
  throw "smoke-pypi-mcp: invalid version '$Version'"
}
$Version = $Version -creplace '-rc\.', 'rc' -creplace '-alpha\.', 'a' -creplace '-beta\.', 'b'
$stdioSmoke = Join-Path $PSScriptRoot 'mcp-stdio-smoke.py'
$PSNativeCommandUseErrorActionPreference = $false
$work = Join-Path ([System.IO.Path]::GetTempPath()) ("fathomdb-pypi-mcp-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $work | Out-Null
try {
  python -m venv "$work/venv"
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  $python = Join-Path $work 'venv/Scripts/python.exe'
  & $python -m pip install --quiet --upgrade pip
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  $maxAttempts = if ($env:FATHOMDB_SMOKE_PYPI_INSTALL_ATTEMPTS) { [int]$env:FATHOMDB_SMOKE_PYPI_INSTALL_ATTEMPTS } else { 5 }
  if ($maxAttempts -lt 1) { throw "smoke-pypi-mcp: FATHOMDB_SMOKE_PYPI_INSTALL_ATTEMPTS must be >= 1" }
  $retryDelay = if ($env:FATHOMDB_SMOKE_PYPI_RETRY_DELAY_SECS) { [int]$env:FATHOMDB_SMOKE_PYPI_RETRY_DELAY_SECS } else { 15 }
  $installLog = Join-Path $work 'pip-install.log'
  $installExit = 0
  for ($attempt = 1; $attempt -le $maxAttempts; $attempt++) {
    & $python -m pip install --quiet "fathomdb[mcp]==$Version" *> $installLog
    $installExit = $LASTEXITCODE
    if ($installExit -eq 0) { break }
    $logText = if (Test-Path $installLog) { Get-Content -Raw $installLog } else { '' }
    $isSkew = $logText -match 'No matching distribution found for (fathomdb|fathomdb-mcp)==|Could not find a version that satisfies the requirement (fathomdb|fathomdb-mcp)=='
    if ($attempt -lt $maxAttempts -and $isSkew) {
      Write-Error "smoke-pypi-mcp: transient PyPI simple-index skew on attempt $attempt/$maxAttempts; retrying in ${retryDelay}s" -ErrorAction Continue
      if ($retryDelay -gt 0) { Start-Sleep -Seconds $retryDelay }
      continue
    }
    if ($logText) { [Console]::Error.Write($logText) }
    break
  }
  if ($installExit -ne 0) { exit $installExit }
  $mcp = Join-Path $work 'venv/Scripts/fathomdb-mcp.exe'
  if (-not (Test-Path $mcp)) { throw "smoke-pypi-mcp: expected MCP binary '$mcp' not installed by fathomdb[mcp]==$Version" }
  & $python -c "import fathomdb; print(fathomdb.__version__)"
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  & $python $stdioSmoke --binary $mcp
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally { Remove-Item -Recurse -Force $work }
