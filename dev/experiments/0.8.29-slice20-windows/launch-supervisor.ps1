param(
  [Parameter(Mandatory=$true)][string]$Control,
  [Parameter(Mandatory=$true)][ValidatePattern('^[0-9a-f]{64}$')][string]$ConfigSha256,
  [Parameter(Mandatory=$true)][ValidatePattern('^[0-9a-f]{64}$')][string]$WrapperSha256
)
# EXTERNAL REVIEW ARTIFACT. NOT_RUN. Requires actual approval and binary review.
$ErrorActionPreference = 'Stop'
if ($Control -cnotmatch '^R:\\fathomdb-s20-a6\\control\\[0-9a-f]{32}$') {
  throw 'control directory differs'
}
$Config = Join-Path $Control 'run.json'
if ((Get-FileHash -LiteralPath $Config -Algorithm SHA256).Hash.ToLowerInvariant() -cne $ConfigSha256) {
  throw 'run manifest hash differs'
}
$c = Get-Content -Raw -Encoding UTF8 -LiteralPath $Config | ConvertFrom-Json
if ($c.runId -cne (Split-Path -Leaf $Control) -or
    $c.candidateSha -cnotmatch '^[0-9a-f]{40}$' -or
    $c.root -cne 'R:\fathomdb-s20-a6' -or $c.wrapperSha256 -cne $WrapperSha256) {
  throw 'run/source/root identity differs'
}
$Exe = $c.wrapperPath
if (-not [IO.Path]::GetFullPath($Exe).StartsWith('R:\fathomdb-s20-a6\bootstrap\',[StringComparison]::OrdinalIgnoreCase)) { throw 'wrapper path unowned' }
if ((Get-FileHash -LiteralPath $Exe -Algorithm SHA256).Hash.ToLowerInvariant() -cne $WrapperSha256) {
  throw 'compiled wrapper identity differs'
}
if (Test-Path -LiteralPath (Join-Path $Control 'launch.json')) { throw 'run already launched' }
if (-not (Test-Path -LiteralPath (Join-Path $Control 'lease.json'))) { throw 'initial lease missing' }
$Command = '"' + $Exe + '" "' + $Control + '" ' + $ConfigSha256
$r = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{
  CommandLine = $Command; CurrentDirectory = $Control
}
if ($r.ReturnValue -ne 0 -or -not $r.ProcessId) { throw 'independent supervisor creation failed' }
$p = Get-CimInstance Win32_Process -Filter "ProcessId=$($r.ProcessId)" -OperationTimeoutSec 2
if (-not $p -or $p.ExecutablePath -cne $Exe -or $p.CommandLine -cne $Command) {
  throw 'launch identity unconfirmed; no blind retry; stop lease and collect receipt'
}
$receipt = [ordered]@{
  runId=$c.runId; candidateSha=$c.candidateSha; configSha256=$ConfigSha256
  wrapperSha256=$WrapperSha256; pid=$p.ProcessId; creationDate=$p.CreationDate
  executable=$p.ExecutablePath; commandLine=$p.CommandLine
}
$Text = $receipt | ConvertTo-Json -Depth 5 -Compress
$Temp = Join-Path $Control ('launch.' + [Guid]::NewGuid().ToString('N') + '.tmp')
[IO.File]::WriteAllText($Temp, $Text, [Text.UTF8Encoding]::new($false))
[IO.File]::Move($Temp, (Join-Path $Control 'launch.json'))
$Text
# Hold a process handle for actual supervisor exit evidence on the normal path.
# This is an observer of the WMI-created process, not its lifetime parent.
# A closed SSH observer never counts as guest cleanup; lease/job containment applies.
$Observed = [Diagnostics.Process]::GetProcessById([int]$r.ProcessId)
$Handle = $Observed.Handle
if ($Observed.MainModule.FileName -cne $Exe) { throw 'observer process identity differs' }
if (-not $Observed.WaitForExit(1740000)) { throw 'supervisor observer29min bound reached' }
$ExitReceipt = [ordered]@{
  runId=$c.runId; candidateSha=$c.candidateSha; pid=$r.ProcessId
  creationUtc=$Observed.StartTime.ToUniversalTime().ToString('o')
  actualExitCode=$Observed.ExitCode; wrapperSha256=$WrapperSha256
  observedUtc=[DateTime]::UtcNow.ToString('o')
}
$ExitText = $ExitReceipt | ConvertTo-Json -Compress
$Temp = Join-Path $Control ('exit.' + [Guid]::NewGuid().ToString('N') + '.tmp')
[IO.File]::WriteAllText($Temp, $ExitText, [Text.UTF8Encoding]::new($false))
[IO.File]::Move($Temp, (Join-Path $Control 'supervisor-exit.json'))
$Observed.Dispose()
$ExitText
if ($ExitReceipt.actualExitCode -ne 0) { exit 125 }
