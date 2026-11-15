param([Parameter(Mandatory=$true)][string]$ScriptPath,[Parameter(Mandatory=$true)][string]$Control,[Parameter(Mandatory=$true)][string]$ConfigSha256)
$ErrorActionPreference='Stop'
$package=Split-Path -Parent $ScriptPath
$manifestPath=Join-Path $Control 'run.json'
if($Control -cnotmatch '^R:\\fathomdb-s20-a6\\control\\[0-9a-f]{32}$' -or $ConfigSha256 -cnotmatch '^[0-9a-f]{64}$' -or (Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256).Hash.ToLowerInvariant() -cne $ConfigSha256){throw 'control/config integrity'}
$initial=Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath|ConvertFrom-Json
foreach($entry in $initial.packageFiles.PSObject.Properties){if($entry.Name -cnotmatch '^[A-Za-z0-9._-]+$' -or (Get-FileHash -LiteralPath (Join-Path $package $entry.Name) -Algorithm SHA256).Hash.ToLowerInvariant() -cne $entry.Value){throw 'package integrity'}}
. ([ScriptBlock]::Create([IO.File]::ReadAllText((Join-Path $package 'common.ps1'),[Text.Encoding]::UTF8)))
$c=Read-A6Config $Control $ConfigSha256
Assert-A6Files $c
Assert-A6Start $c | Out-Null
$exe='C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe'
if((Get-A6Hash $exe) -cne $c.powershellSha256){throw 'interpreter identity differs'}
$script=Join-Path $package 'guest-stage.ps1'
$stageHash=$c.packageFiles.PSObject.Properties['guest-stage.ps1'].Value
$invoke='$ErrorActionPreference=''Stop'';$p=''+$script+'''';if((Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash.ToLowerInvariant() -cne ''+$stageHash+''''){throw ''launch script digest differs''};& ([ScriptBlock]::Create((Get-Content -Raw -Encoding UTF8 -LiteralPath $p))) -ScriptPath $p -Control ''+$Control+'''' -ConfigSha256 '+$ConfigSha256
$command='"'+$exe+'" -NoLogo -NoProfile -NonInteractive -EncodedCommand '+[Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($invoke))
if(Test-Path -LiteralPath (Join-Path $Control 'launch.json')){throw 'occupied launch; no retry'}
$r=Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{CommandLine=$command;CurrentDirectory=$Control}
if($r.ReturnValue -ne 0 -or -not $r.ProcessId){throw 'WMI stage creation failed'}
$observed=[Diagnostics.Process]::GetProcessById([int]$r.ProcessId);$held=$observed.Handle
if(-not [string]::Equals($observed.MainModule.FileName,$exe,[StringComparison]::OrdinalIgnoreCase)){throw 'stage executable mismatch'}
Write-A6Json (Join-Path $Control 'launch.json') @{runId=$c.runId;candidateSha=$c.candidateSha;pid=$r.ProcessId;creationUtc=$observed.StartTime.ToUniversalTime().ToString('o');interpreterSha256=$c.powershellSha256;commandLine=$command}
if(-not $observed.WaitForExit(600000)){throw 'stage observer10min bound'}
Write-A6Json (Join-Path $Control 'supervisor-exit.json') @{runId=$c.runId;candidateSha=$c.candidateSha;pid=$r.ProcessId;actualExitCode=$observed.ExitCode;creationUtc=$observed.StartTime.ToUniversalTime().ToString('o')}
$code=$observed.ExitCode;$observed.Dispose();if($code -ne 0){exit 125}
