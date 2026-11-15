# No Add-Type or compiler is used to emit these P/Invoke declarations.
function New-A6KernelType {
    if (-not [Environment]::Is64BitProcess) { throw 'bootstrap requires x64 Windows PowerShell' }
    $assembly = [AppDomain]::CurrentDomain.DefineDynamicAssembly([Reflection.AssemblyName]::new('A6Kernel'+[Guid]::NewGuid().ToString('N')),[Reflection.Emit.AssemblyBuilderAccess]::Run)
    $module = $assembly.DefineDynamicModule('Native')
    $builder = $module.DefineType('Kernel',[Reflection.TypeAttributes]'Public,Sealed,Abstract')
    $refUInt = [UIntPtr].MakeByRefType(); $refInt = [int].MakeByRefType(); $refBool = [bool].MakeByRefType()
    $specs = @(
        @('CreateJobObjectW',[IntPtr],@([IntPtr],[string])),
        @('SetHandleInformation',[bool],@([IntPtr],[uint32],[uint32])),
        @('GetHandleInformation',[bool],@([IntPtr],[uint32].MakeByRefType())),
        @('SetInformationJobObject',[bool],@([IntPtr],[int],[IntPtr],[int])),
        @('QueryInformationJobObject',[bool],@([IntPtr],[int],[IntPtr],[int],$refInt)),
        @('InitializeProcThreadAttributeList',[bool],@([IntPtr],[int],[int],$refUInt)),
        @('UpdateProcThreadAttribute',[bool],@([IntPtr],[uint32],[UIntPtr],[IntPtr],[UIntPtr],[IntPtr],[IntPtr])),
        @('DeleteProcThreadAttributeList',[void],@([IntPtr])),
        @('CreateProcessW',[bool],@([string],[Text.StringBuilder],[IntPtr],[IntPtr],[bool],[uint32],[IntPtr],[string],[IntPtr],[IntPtr])),
        @('IsProcessInJob',[bool],@([IntPtr],[IntPtr],$refBool)),
        @('ResumeThread',[uint32],@([IntPtr])),
        @('TerminateJobObject',[bool],@([IntPtr],[uint32])),
        @('WaitForSingleObject',[uint32],@([IntPtr],[uint32])),
        @('GetExitCodeProcess',[bool],@([IntPtr],[uint32].MakeByRefType())),
        @('GetProcessTimes',[bool],@([IntPtr],[long].MakeByRefType(),[long].MakeByRefType(),[long].MakeByRefType(),[long].MakeByRefType())),
        @('QueryFullProcessImageNameW',[bool],@([IntPtr],[uint32],[Text.StringBuilder],[int].MakeByRefType())),
        @('CreateFileW',[IntPtr],@([string],[uint32],[uint32],[IntPtr],[uint32],[uint32],[IntPtr])),
        @('CloseHandle',[bool],@([IntPtr])),
        @('ExitProcess',[void],@([uint32]))
    )
    foreach ($spec in $specs) {
        $method = $builder.DefineMethod($spec[0],[Reflection.MethodAttributes]'Public,Static,PinvokeImpl',$spec[1],[Type[]]$spec[2])
        $fields = [Reflection.FieldInfo[]]@([Runtime.InteropServices.DllImportAttribute].GetField('EntryPoint'),[Runtime.InteropServices.DllImportAttribute].GetField('SetLastError'),[Runtime.InteropServices.DllImportAttribute].GetField('ExactSpelling'),[Runtime.InteropServices.DllImportAttribute].GetField('CharSet'),[Runtime.InteropServices.DllImportAttribute].GetField('CallingConvention'))
        $values = [object[]]@($spec[0],$true,$true,[Runtime.InteropServices.CharSet]::Unicode,[Runtime.InteropServices.CallingConvention]::Winapi)
        $attribute = [Reflection.Emit.CustomAttributeBuilder]::new([Runtime.InteropServices.DllImportAttribute].GetConstructor(@([string])),[object[]]@('kernel32.dll'),$fields,$values)
        $method.SetCustomAttribute($attribute)
        $method.SetImplementationFlags([Reflection.MethodImplAttributes]::PreserveSig)
    }
    return $builder.CreateType()
}
function Add-A6Diagnostic {
    param($State,[string]$Operation,$Exception,[bool]$Cleanup=$true)
    [Threading.Monitor]::Enter($State.Gate)
    try {
        if ($Cleanup) { $State.CleanupFailed=$true }
        $text=$Exception.GetType().FullName+': '+$Exception.Message; $cut=$text.Length -gt 4096
        if ($cut) { $text=$text.Substring(0,4096);$State.Lost=$true }
        if ($State.Diagnostics.Count -ge 128) { $State.Lost=$true;return }
        $native=$null;$inner=$Exception;$depth=0
        while($inner -and $depth -lt 8){if($inner -is [ComponentModel.Win32Exception]){$native=$inner.NativeErrorCode;break};$inner=$inner.InnerException;$depth++}
        if($inner -and $depth -ge 8){$State.Lost=$true}
        $State.Diagnostics.Add(@{operation=$Operation;exception=$text;win32NativeErrorCode=$native;cleanup=$Cleanup;truncated=$cut;utc=[DateTime]::UtcNow.ToString('o')})
    } finally { [Threading.Monitor]::Exit($State.Gate) }
}
function Assert-A6Native([bool]$Ok,[string]$Operation) {
    if (-not $Ok) { throw [ComponentModel.Win32Exception]::new([Runtime.InteropServices.Marshal]::GetLastWin32Error(),$Operation) }
}
function New-A6Buffer([int]$Size) { $p=[Runtime.InteropServices.Marshal]::AllocHGlobal($Size);[Runtime.InteropServices.Marshal]::Copy([byte[]]::new($Size),0,$p,$Size);return $p }
function Quote-A6Argument([string]$Value) {
    $b=[Text.StringBuilder]::new([string][char]34);$slashes=0
    foreach($c in $Value.ToCharArray()) {
        if($c -eq [char]92){$slashes++;continue}
        if($c -eq [char]34){[void]$b.Append([char]92,2*$slashes+1)}else{[void]$b.Append([char]92,$slashes)}
        [void]$b.Append($c);$slashes=0
    }
    [void]$b.Append([char]92,2*$slashes);[void]$b.Append([char]34);return $b.ToString()
}
function New-A6StageMonitor($Config,[string]$Control) {
    Assert-A6Start $Config | Out-Null
    $api=New-A6KernelType; $gate=[object]::new()
    $state=[hashtable]::Synchronized(@{Gate=$gate;Config=$Config;Control=$Control;Clock=[Diagnostics.Stopwatch]::StartNew();Job=[IntPtr]::Zero;Done=$false;Stop=$null;StopAt=-1L;LeaseAt=0L;Generation=-1L;ResourceAt=0L;Resource=$null;StartAt=-1L;Timeout=0;Lost=$false;CleanupFailed=$false;Diagnostics=[Collections.Generic.List[object]]::new();LeaseReady=$false;ResourceReady=$false;WatchReady=$false})
    $diag='function Add-A6Diagnostic {'+${function:Add-A6Diagnostic}.ToString()+"}`n"
    $facts='function Get-A6Facts {'+${function:Get-A6Facts}.ToString()+"}`n"
    $leaseScript="param(`$s)`n"+$diag+@'
$s.LeaseReady=$true
try {
 while(-not $s.Done) {
  $cancel=Join-Path $s.Control 'cancel.json'
  if([IO.File]::Exists($cancel)){$c=[IO.File]::ReadAllText($cancel)|ConvertFrom-Json;if($c.runId -cne $s.Config.runId -or $c.candidateSha -cne $s.Config.candidateSha){throw 'cancel identity'};throw 'explicit cancellation'}
  $l=[IO.File]::ReadAllText((Join-Path $s.Control 'lease.json'))|ConvertFrom-Json
  if($l.runId -cne $s.Config.runId -or $l.candidateSha -cne $s.Config.candidateSha -or ($l.generation -isnot [long] -and $l.generation -isnot [int])){throw 'lease identity/type'}
  [Threading.Monitor]::Enter($s.Gate)
  try{if($s.Generation -ge 0 -and $s.Clock.ElapsedMilliseconds-$s.LeaseAt -ge 60000){throw 'lease expired'};if($l.generation -lt $s.Generation){throw 'lease regressed'};if($l.generation -gt $s.Generation){$s.Generation=[long]$l.generation;$s.LeaseAt=$s.Clock.ElapsedMilliseconds}}finally{[Threading.Monitor]::Exit($s.Gate)}
  [Threading.Thread]::Sleep(250)
 }
}catch{Add-A6Diagnostic $s 'lease reader' $_.Exception $false;[Threading.Monitor]::Enter($s.Gate);try{if(-not $s.Stop){$s.Stop=$_.Exception.Message;$s.StopAt=$s.Clock.ElapsedMilliseconds}}finally{[Threading.Monitor]::Exit($s.Gate)}}
'@
    $resourceScript="param(`$s)`n"+$diag+$facts+@'
$s.ResourceReady=$true
try{while(-not $s.Done){$f=Get-A6Facts $s.Config;[Threading.Monitor]::Enter($s.Gate);try{$s.Resource=$f;$s.ResourceAt=$s.Clock.ElapsedMilliseconds}finally{[Threading.Monitor]::Exit($s.Gate)};if($f.cFree -lt 5GB -or $f.rFree -lt 20GB -or $f.memory -lt 8GB){throw 'resource stop floor'};[Threading.Thread]::Sleep(1000)}}catch{Add-A6Diagnostic $s 'resource reader' $_.Exception $false;[Threading.Monitor]::Enter($s.Gate);try{if(-not $s.Stop){$s.Stop='resource failure';$s.StopAt=$s.Clock.ElapsedMilliseconds}}finally{[Threading.Monitor]::Exit($s.Gate)}}
'@
    $watchScript="param(`$s,[type]`$api)`n"+$diag+@'
$s.WatchReady=$true
try{while(-not $s.Done){$now=$s.Clock.ElapsedMilliseconds;[Threading.Monitor]::Enter($s.Gate);try{
 if(-not $s.Stop){
  if($s.Generation -lt 0 -and $now -gt 6000){$s.Stop='initial lease unavailable'}
  elseif($s.Generation -ge 0 -and $now-$s.LeaseAt -ge 60000){$s.Stop='lease expired'}
  elseif(($s.Resource -eq $null -and $now -gt 6000) -or ($s.Resource -ne $null -and $now-$s.ResourceAt -gt 3000)){$s.Stop='resource sample unavailable/stale'}
  elseif($s.StartAt -ge 0 -and $now-$s.StartAt -ge 1000*$s.Timeout){$s.Stop='command deadline'}
  elseif($now -ge 1800000){$s.Stop='overall deadline'}
  elseif($s.Lost -or $s.CleanupFailed){$s.Stop='diagnostic loss/cleanup failure'}
  if($s.Stop){$s.StopAt=$now}
 }
 if($s.Stop -and $s.Job -ne [IntPtr]::Zero){if(-not $api::TerminateJobObject($s.Job,125)){Add-A6Diagnostic $s 'watchdog TerminateJobObject' ([ComponentModel.Win32Exception]::new([Runtime.InteropServices.Marshal]::GetLastWin32Error())) $true}}
 if($s.StopAt -ge 0 -and $now-$s.StopAt -gt 22000){$api::ExitProcess(125)}
 }finally{[Threading.Monitor]::Exit($s.Gate)};[Threading.Thread]::Sleep(200)}}catch{Add-A6Diagnostic $s 'watchdog' $_.Exception $true;$api::ExitProcess(125)}
'@
    $workers=@()
    foreach($script in @($leaseScript,$resourceScript,$watchScript)){$ps=[PowerShell]::Create();[void]$ps.AddScript($script).AddArgument($state);if($script -eq $watchScript){[void]$ps.AddArgument($api)};$async=$ps.BeginInvoke();$workers+=@(@{ps=$ps;async=$async})}
    # One clock/generation belongs to this entire guest stage, including gaps
    # between compiler invocations. Reading the same generation never renews it.
    return @{State=$state;Api=$api;Workers=$workers}
}
function Assert-A6StageActivation($Stage) {
    $s=$Stage.State
    while($s.Generation -lt 0 -or $s.Resource -eq $null -or -not $s.LeaseReady -or -not $s.ResourceReady -or -not $s.WatchReady){
        if($s.Stop -or $s.Clock.ElapsedMilliseconds -gt 6000 -or @($Stage.Workers|Where-Object{$_.async.IsCompleted}).Count -gt 0){throw 'bootstrap worker startup failed/unavailable'}
        [Threading.Thread]::Sleep(50)
    }
    [Threading.Monitor]::Enter($s.Gate)
    try{
        $now=$s.Clock.ElapsedMilliseconds
        if($s.Stop -or $s.Lost -or $s.CleanupFailed -or $s.Diagnostics.Count -gt 0){throw 'UNKNOWN: stage stopped or diagnostic/setup failure'}
        if($s.Generation -lt 0 -or $now-$s.LeaseAt -ge 60000){throw 'stage lease expired before activation'}
        if($null -eq $s.Resource -or $now-$s.ResourceAt -gt 3000 -or $s.Resource.cFree -lt 5GB -or $s.Resource.rFree -lt 20GB -or $s.Resource.memory -lt 8GB){throw 'stage resource unavailable/stale/stop floor before activation'}
        if(($s.StartAt -ge 0 -and $now-$s.StartAt -ge 1000*$s.Timeout) -or $now -ge 1800000 -or @($Stage.Workers|Where-Object{$_.async.IsCompleted}).Count -gt 0){throw 'stage deadline/worker failure before activation'}
    }finally{[Threading.Monitor]::Exit($s.Gate)}
}
function Invoke-A6ContainedProcess($Config,[string]$Control,[string]$Label,[string]$Exe,[string[]]$Arguments,[string]$WorkingDirectory,[int]$TimeoutSeconds,$Stage) {
    Assert-A6Start $Config | Out-Null
    foreach($p in @($Control,$WorkingDirectory)) { Assert-A6Path ($p+'\owned-sentinel') $Config.root | Out-Null }
    $state=$Stage.State;$api=$Stage.Api;$gate=$state.Gate;$workers=$Stage.Workers
    if($state.Config.runId -cne $Config.runId -or $state.Control -cne $Control -or $state.Job -ne [IntPtr]::Zero -or $state.StartAt -ge 0){throw 'stage identity or prior invocation not quiescent'}
    $buffers=[Collections.Generic.List[IntPtr]]::new();$files=[Collections.Generic.List[IntPtr]]::new();$process=[IntPtr]::Zero;$thread=[IntPtr]::Zero;$attributes=[IntPtr]::Zero;$attributesReady=$false;$job=[IntPtr]::Zero;$zero=$false;$exit=$null;$unknown=$false
    $record=@{runId=$Config.runId;candidateSha=$Config.candidateSha;label=$Label;phase='UNKNOWN';authoritativeEmpty=$false;rootExit=$null;commands=@()}
    try {
        Assert-A6StageActivation $Stage
        $invalidFlags=0;$invalidResult=$api::GetHandleInformation([IntPtr]::Zero,[ref]$invalidFlags);$nativeError=[Runtime.InteropServices.Marshal]::GetLastWin32Error();if($invalidResult -or $nativeError -ne 6){throw 'emitted P/Invoke LastError contract not established'}
        $job=$api::CreateJobObjectW([IntPtr]::Zero,$null);if($job -eq [IntPtr]::Zero){throw [ComponentModel.Win32Exception]::new([Runtime.InteropServices.Marshal]::GetLastWin32Error())}
        [Threading.Monitor]::Enter($gate);try{$state.Job=$job}finally{[Threading.Monitor]::Exit($gate)}
        Assert-A6Native ($api::SetHandleInformation($job,1,0)) 'clear job inheritance'
        $limits=New-A6Buffer 144;$buffers.Add($limits);[Runtime.InteropServices.Marshal]::WriteInt32($limits,16,0x2000)
        Assert-A6Native ($api::SetInformationJobObject($job,9,$limits,144)) 'set kill-on-close/no-breakaway'
        $returned=0;Assert-A6Native ($api::QueryInformationJobObject($job,9,$limits,144,[ref]$returned)) 'query limits'
        $flags=0;Assert-A6Native ($api::GetHandleInformation($job,[ref]$flags)) 'query job inheritance'
        if($returned -ne 144 -or [Runtime.InteropServices.Marshal]::ReadInt32($limits,16) -ne 0x2000 -or ($flags -band 1)){throw 'job setup identity/limits'}
        $security=New-A6Buffer 24;$buffers.Add($security);[Runtime.InteropServices.Marshal]::WriteInt32($security,24);[Runtime.InteropServices.Marshal]::WriteInt32($security,16,1)
        $stdout=Join-Path $Control ($Label+'.stdout.log');$stderr=Join-Path $Control ($Label+'.stderr.log')
        $opened=@()
        foreach($spec in @(@('NUL',[uint32]2147483648,3),@($stdout,[uint32]1073741824,1),@($stderr,[uint32]1073741824,1))){$h=$api::CreateFileW($spec[0],$spec[1],1,$security,$spec[2],0x80,[IntPtr]::Zero);if($h -eq [IntPtr](-1)){throw [ComponentModel.Win32Exception]::new([Runtime.InteropServices.Marshal]::GetLastWin32Error(),'open stdio')};$files.Add($h);$opened+=@($h)}
        $stdInput=$opened[0];$output=$opened[1];$errorFile=$opened[2]
        $bytes=[UIntPtr]::Zero;$initial=$api::InitializeProcThreadAttributeList([IntPtr]::Zero,2,0,[ref]$bytes);$code=[Runtime.InteropServices.Marshal]::GetLastWin32Error();if($initial -or $code -ne 122 -or $bytes -eq [UIntPtr]::Zero){throw 'attribute-size query failed'}
        $attributes=New-A6Buffer ([int]$bytes.ToUInt64());$buffers.Add($attributes);Assert-A6Native ($api::InitializeProcThreadAttributeList($attributes,2,0,[ref]$bytes)) 'initialize attributes';$attributesReady=$true
        $jobList=New-A6Buffer 8;$buffers.Add($jobList);[Runtime.InteropServices.Marshal]::WriteIntPtr($jobList,$job)
        Assert-A6Native ($api::UpdateProcThreadAttribute($attributes,0,[UIntPtr]0x2000d,$jobList,[UIntPtr]8,[IntPtr]::Zero,[IntPtr]::Zero)) 'atomic creation job list'
        $handleList=New-A6Buffer 24;$buffers.Add($handleList);[Runtime.InteropServices.Marshal]::WriteIntPtr($handleList,0,$stdInput);[Runtime.InteropServices.Marshal]::WriteIntPtr($handleList,8,$output);[Runtime.InteropServices.Marshal]::WriteIntPtr($handleList,16,$errorFile)
        Assert-A6Native ($api::UpdateProcThreadAttribute($attributes,0,[UIntPtr]0x20002,$handleList,[UIntPtr]24,[IntPtr]::Zero,[IntPtr]::Zero)) 'stdio-only inherited handles'
        $startup=New-A6Buffer 112;$buffers.Add($startup);[Runtime.InteropServices.Marshal]::WriteInt32($startup,112);[Runtime.InteropServices.Marshal]::WriteInt32($startup,60,0x100);[Runtime.InteropServices.Marshal]::WriteIntPtr($startup,80,$stdInput);[Runtime.InteropServices.Marshal]::WriteIntPtr($startup,88,$output);[Runtime.InteropServices.Marshal]::WriteIntPtr($startup,96,$errorFile);[Runtime.InteropServices.Marshal]::WriteIntPtr($startup,104,$attributes)
        $pi=New-A6Buffer 24;$buffers.Add($pi)
        $line=[Text.StringBuilder]::new((Quote-A6Argument $Exe));foreach($a in $Arguments){[void]$line.Append(' ').Append((Quote-A6Argument $a))}
        Assert-A6StageActivation $Stage
        [Threading.Monitor]::Enter($gate);try{$state.Timeout=$TimeoutSeconds;$state.StartAt=$state.Clock.ElapsedMilliseconds}finally{[Threading.Monitor]::Exit($gate)}
        Assert-A6Native ($api::CreateProcessW($Exe,$line,[IntPtr]::Zero,[IntPtr]::Zero,$true,0x80004,[IntPtr]::Zero,$WorkingDirectory,$startup,$pi)) 'create suspended contained process'
        $process=[Runtime.InteropServices.Marshal]::ReadIntPtr($pi,0);$thread=[Runtime.InteropServices.Marshal]::ReadIntPtr($pi,8);$pidOwned=[Runtime.InteropServices.Marshal]::ReadInt32($pi,16);$inside=$false
        Assert-A6Native ($api::IsProcessInJob($process,$job,[ref]$inside)) 'root membership';if(-not $inside){throw 'root not in job'}
        $created=0L;$exited=0L;$kernel=0L;$user=0L;Assert-A6Native ($api::GetProcessTimes($process,[ref]$created,[ref]$exited,[ref]$kernel,[ref]$user)) 'root creation time';$record.creationFileTime=$created
        $image=[Text.StringBuilder]::new(32768);$imageSize=$image.Capacity;Assert-A6Native ($api::QueryFullProcessImageNameW($process,0,$image,[ref]$imageSize)) 'root image';if(-not [string]::Equals([IO.Path]::GetFullPath($image.ToString()),[IO.Path]::GetFullPath($Exe),[StringComparison]::OrdinalIgnoreCase)){throw 'root image differs'}
        $initialAccounting=New-A6Buffer 48;$buffers.Add($initialAccounting);$initialReturned=0;Assert-A6Native ($api::QueryInformationJobObject($job,1,$initialAccounting,48,[ref]$initialReturned)) 'initial accounting';if($initialReturned -ne 48 -or [Runtime.InteropServices.Marshal]::ReadInt32($initialAccounting,40) -ne 1){throw 'suspended root accounting differs'}
        foreach($h in $files){if(-not $api::CloseHandle($h)){Add-A6Diagnostic $state 'parent stdio CloseHandle' ([ComponentModel.Win32Exception]::new([Runtime.InteropServices.Marshal]::GetLastWin32Error())) $true}};$files.Clear()
        $record.rootPid=$pidOwned;$record.commandLine=$line.ToString();$record.stdout=$stdout;$record.stderr=$stderr;$record.creationTimeJobAssignment=$true;Write-A6Json (Join-Path $Control ($Label+'.created.json')) $record
        Assert-A6StageActivation $Stage
        $previous=$api::ResumeThread($thread);if($previous -ne 1){throw [ComponentModel.Win32Exception]::new([Runtime.InteropServices.Marshal]::GetLastWin32Error(),'resume count must equal1')}
        if(-not $api::CloseHandle($thread)){Add-A6Diagnostic $state 'thread CloseHandle' ([ComponentModel.Win32Exception]::new([Runtime.InteropServices.Marshal]::GetLastWin32Error())) $true};$thread=[IntPtr]::Zero
        $accounting=New-A6Buffer 48;$buffers.Add($accounting);$rootExitedAt=-1L;$cleanupAt=-1L
        while($true){
            if(-not $state.Stop -and @($workers|Where-Object{$_.async.IsCompleted}).Count -gt 0){throw 'bootstrap worker terminated unexpectedly'}
            if($state.Stop -and $cleanupAt -lt 0){Assert-A6Native ($api::TerminateJobObject($job,125)) 'stop private job';$cleanupAt=$state.Clock.ElapsedMilliseconds}
            if($process -ne [IntPtr]::Zero){$wait=$api::WaitForSingleObject($process,0);if($wait -eq 0){$code=0;Assert-A6Native ($api::GetExitCodeProcess($process,[ref]$code)) 'root exit';$exit=[long]$code;if(-not $api::CloseHandle($process)){Add-A6Diagnostic $state 'process CloseHandle' ([ComponentModel.Win32Exception]::new([Runtime.InteropServices.Marshal]::GetLastWin32Error())) $true};$process=[IntPtr]::Zero;$rootExitedAt=$state.Clock.ElapsedMilliseconds;if($exit -ne 0 -and -not $state.Stop){$state.Stop='root nonzero';$state.StopAt=$state.Clock.ElapsedMilliseconds}}elseif($wait -ne 258){throw 'root wait failed'}}
            $returned=0;Assert-A6Native ($api::QueryInformationJobObject($job,1,$accounting,48,[ref]$returned)) 'authoritative accounting';if($returned -ne 48){throw 'accounting size differs'}
            $count=[Runtime.InteropServices.Marshal]::ReadInt32($accounting,40)
            if($count -eq 0 -and $null -ne $exit){$zero=$true;break}
            if($rootExitedAt -ge 0 -and -not $state.Stop -and $state.Clock.ElapsedMilliseconds-$rootExitedAt -gt 10000){$state.Stop='descendants not quiescent';$state.StopAt=$state.Clock.ElapsedMilliseconds}
            if($cleanupAt -ge 0 -and $state.Clock.ElapsedMilliseconds-$cleanupAt -gt 10000){throw 'authoritative empty timeout'}
            [Threading.Thread]::Sleep(200)
        }
        $record.authoritativeEmpty=$true;$record.rootExit=$exit;$record.counts=@{ActiveProcesses=0;TotalProcesses=[Runtime.InteropServices.Marshal]::ReadInt32($accounting,36);TotalTerminatedProcesses=[Runtime.InteropServices.Marshal]::ReadInt32($accounting,44)};$record.stdoutSha256=Get-A6Hash $stdout;$record.stderrSha256=Get-A6Hash $stderr;$record.stdoutBytes=(Get-Item -LiteralPath $stdout).Length;$record.stderrBytes=(Get-Item -LiteralPath $stderr).Length;$record.phase=if($state.Stop){'STOPPED'}else{'COMPLETED'}
    } catch {
        Add-A6Diagnostic $state 'contained process' $_.Exception $true;$unknown=$true
        if($job -ne [IntPtr]::Zero -and -not $api::TerminateJobObject($job,125)){Add-A6Diagnostic $state 'exception TerminateJobObject' ([ComponentModel.Win32Exception]::new([Runtime.InteropServices.Marshal]::GetLastWin32Error())) $true}
    } finally {
        if($attributesReady){$api::DeleteProcThreadAttributeList($attributes)}
        foreach($h in @($thread,$process)+$files.ToArray()){if($h -ne [IntPtr]::Zero -and -not $api::CloseHandle($h)){Add-A6Diagnostic $state 'secondary CloseHandle' ([ComponentModel.Win32Exception]::new([Runtime.InteropServices.Marshal]::GetLastWin32Error())) $true}}
        [Threading.Monitor]::Enter($gate);try{$state.Job=[IntPtr]::Zero;if($job -ne [IntPtr]::Zero -and -not $api::CloseHandle($job)){Add-A6Diagnostic $state 'job CloseHandle' ([ComponentModel.Win32Exception]::new([Runtime.InteropServices.Marshal]::GetLastWin32Error())) $true}}finally{[Threading.Monitor]::Exit($gate)}
        foreach($p in $buffers){[Runtime.InteropServices.Marshal]::FreeHGlobal($p)}
        $record.diagnostics=@{lostOrOverflowed=$state.Lost;cleanupFailure=$state.CleanupFailed;entries=$state.Diagnostics.ToArray()}
        if($unknown -or -not $zero -or $state.Lost -or $state.CleanupFailed){$record.phase='UNKNOWN'}
        $record.stopReason=$state.Stop;$record.lastLeaseGeneration=$state.Generation
        Write-A6Json (Join-Path $Control ($Label+'.result.json')) $record
        # The stage monitor remains alive between commands; its lease age and
        # diagnostic failures are never reset by the next compiler invocation.
        if($zero -and -not $unknown -and -not $state.Lost -and -not $state.CleanupFailed){$state.StartAt=-1L}
    }
    return $record
}
