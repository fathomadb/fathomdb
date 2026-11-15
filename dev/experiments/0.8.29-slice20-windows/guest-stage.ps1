param([Parameter(Mandatory=$true)][string]$Control,[Parameter(Mandatory=$true)][string]$ConfigSha256)
$ErrorActionPreference='Stop'
$package=Split-Path -Parent $MyInvocation.MyCommand.Path
$manifestPath=Join-Path $Control 'run.json'
if($Control -cnotmatch '^R:\\fathomdb-s20-a6\\control\\[0-9a-f]{32}$' -or $ConfigSha256 -cnotmatch '^[0-9a-f]{64}$' -or (Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256).Hash.ToLowerInvariant() -cne $ConfigSha256){throw 'control/config integrity'}
$initial=Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath|ConvertFrom-Json
foreach($entry in $initial.packageFiles.PSObject.Properties){if($entry.Name -cnotmatch '^[A-Za-z0-9._-]+$' -or (Get-FileHash -LiteralPath (Join-Path $package $entry.Name) -Algorithm SHA256).Hash.ToLowerInvariant() -cne $entry.Value){throw 'package integrity'}}
. (Join-Path $package 'common.ps1')
$config=Read-A6Config $Control $ConfigSha256
if ($config.packagePath -cne $package) { throw 'package path differs' }
Assert-A6Files $config
Assert-A6Start $config | Out-Null
. (Join-Path $package 'native-bootstrap.ps1')
$results=@();$phase='UNKNOWN';$errorText=$null;$controlResult=$null;$stageMonitor=$null
try {
    $stageMonitor=New-A6StageMonitor $config $Control
    if ($config.stage -eq 'compile') {
        $build=Assert-A6Path (Join-Path $config.root ('bootstrap\'+$config.runId)) $config.root
        if (Test-Path -LiteralPath $build) { throw 'bootstrap destination occupied' }
        foreach($sub in @('in','out','tmp','logs')) { [IO.Directory]::CreateDirectory((Join-Path $build $sub)) | Out-Null }
        foreach($name in @('NativeJob.cs','Supervisor.cs','OwnedChild.cs','Controls.cs')){[IO.File]::Copy((Join-Path $package $name),(Join-Path $build ('in\'+$name)),$false)}
        $framework='C:\Windows\Microsoft.NET\Framework64\v4.0.30319'
        $compiler=Join-Path $framework 'csc.exe'
        foreach($entry in $config.compilerFiles.PSObject.Properties){if((Get-A6Hash $entry.Name) -cne $entry.Value){throw "compiler/reference identity differs: $($entry.Name)"}}
        if(-not $config.compilerFiles.PSObject.Properties[$compiler]){throw 'compiler identity missing'}
        $references=@('mscorlib.dll','System.dll','System.Core.dll','System.Management.dll','System.Web.Extensions.dll')
        $referenceArgs=@();foreach($name in $references){$path=Join-Path $framework $name;if(-not $config.compilerFiles.PSObject.Properties[$path]){throw 'reference identity missing'};$referenceArgs+=('/reference:'+$path)}
        $env:TEMP=Join-Path $build 'tmp';$env:TMP=$env:TEMP
        $targets=@(@('Supervisor','Supervisor',@('NativeJob.cs','Supervisor.cs'),$false),@('OwnedChild','OwnedChild',@('OwnedChild.cs'),$false),@('Controls','Controls',@('NativeJob.cs','Controls.cs'),$true))
        foreach($target in $targets){
            $argsNative=@('/nologo','/noconfig','/nostdlib+','/target:exe','/platform:x64','/optimize+',('/main:'+$target[1]),('/out:'+(Join-Path $build ('out\'+$target[0]+'.exe'))))+$referenceArgs
            if($target[3]){$argsNative+='/define:A6_CONTROL'}
            foreach($name in $target[2]){$argsNative+=(Join-Path $build ('in\'+$name))}
            $result=Invoke-A6ContainedProcess $config $Control ('compile-'+$target[0]) $compiler $argsNative $build 30 $stageMonitor
            $results+=@($result);if($result.phase -cne 'COMPLETED' -or $result.rootExit -ne 0){throw 'compile or cleanup failed'}
        }
        $binaries=@{};foreach($name in @('Supervisor','OwnedChild','Controls')){$path=Join-Path $build ('out\'+$name+'.exe');$binaries[$path]=Get-A6Hash $path}
        Write-A6Json (Join-Path $Control 'compiled-binaries.json') @{runId=$config.runId;candidateSha=$config.candidateSha;binaries=$binaries;reviewRequired=$true}
    } elseif ($config.stage -eq 'controls') {
        foreach($entry in $config.reviewedBinaries.PSObject.Properties){if((Get-A6Hash $entry.Name) -cne $entry.Value){throw 'reviewed binary differs'}}
        $case=$config.controlCase
        $allowed=@('normal','orphan','output','breakaway','inherited-handle','cleanup-failure','diagnostic-overflow','diagnostic-truncation','death-before-create','death-after-create','death-after-resume','linger')
        if($case -cnotin $allowed){throw 'unsupported control case'}
        foreach($path in @($config.controlsExe,$config.ownedChildExe)){Assert-A6Path $path $config.root | Out-Null;if(-not $config.reviewedBinaries.PSObject.Properties[$path]){throw 'control binary review missing'}}
        $caseRoot=Join-Path $Control 'case';if(Test-Path -LiteralPath $caseRoot){throw 'control destination occupied'};[IO.Directory]::CreateDirectory($caseRoot)|Out-Null
        $seconds=if($case -eq 'linger'){90}else{30}
        $results+=@(Invoke-A6ContainedProcess $config $Control ('control-'+$case) $config.controlsExe @($case,$config.ownedChildExe,$caseRoot) $caseRoot $seconds $stageMonitor)
        if($results[0].phase -cne 'COMPLETED' -or $results[0].rootExit -ne 0){throw 'control did not complete normally'}
        $controlResult=Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $caseRoot 'control-result.json')|ConvertFrom-Json
        if($controlResult.exit -ne 0 -or $controlResult.verdict -like 'CONTROL_FAILED*'){throw 'owned-child control failed'}
    } else { throw 'only compile/controls accepted by guest-stage' }
    $phase='COMPLETED'
} catch { $errorText=$_.Exception.ToString();$phase='UNKNOWN';if($results.Count -gt 0 -and $results[-1].phase -eq 'STOPPED'){$phase='STOPPED'} }
Write-A6Json (Join-Path $Control 'termination.json') @{runId=$config.runId;candidateSha=$config.candidateSha;phase=$phase;stage=$config.stage;commands=$results;controlResult=$controlResult;error=$errorText;a6Qualified=$false}
if($null -ne $stageMonitor){$stageMonitor.State.Done=$true}
if($phase -cne 'COMPLETED'){exit 125}
