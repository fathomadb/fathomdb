param([Parameter(Mandatory=$true)][string]$Control,[Parameter(Mandatory=$true)][string]$ConfigSha256)
$ErrorActionPreference='Stop'
$package=Split-Path -Parent $MyInvocation.MyCommand.Path
$configPath=Join-Path $Control 'run.json'
if($Control -cnotmatch '^R:\\fathomdb-s20-a6\\control\\[0-9a-f]{32}$' -or $ConfigSha256 -cnotmatch '^[0-9a-f]{64}$' -or (Get-FileHash -LiteralPath $configPath -Algorithm SHA256).Hash.ToLowerInvariant() -cne $ConfigSha256){throw 'control/config integrity'}
$config=Get-Content -Raw -Encoding UTF8 -LiteralPath $configPath|ConvertFrom-Json
foreach($entry in $config.packageFiles.PSObject.Properties){if($entry.Name -cnotmatch '^[A-Za-z0-9._-]+$' -or (Get-FileHash -LiteralPath (Join-Path $package $entry.Name) -Algorithm SHA256).Hash.ToLowerInvariant() -cne $entry.Value){throw 'package integrity'}}
. (Join-Path $package 'common.ps1')
Assert-A6Identity $config;Assert-A6Start $config|Out-Null
$archive=Assert-A6Path $config.sourceArchive $config.root
$destination=Assert-A6Path $config.workspace $config.root
if($destination -cne ($config.root+'\source\'+$config.candidateSha) -or (Test-Path -LiteralPath $destination)){throw 'source destination occupied or unallocated'}
. (Join-Path $package 'native-bootstrap.ps1')
$stageMonitor=New-A6StageMonitor $config $Control
$state=$stageMonitor.State
$source=$null;$zip=$null;$failure=$null;$count=0;$completed=$false
try {
 Assert-A6StageActivation $stageMonitor
 $state.Timeout=180;$state.StartAt=$state.Clock.ElapsedMilliseconds
 if((Get-A6Hash $archive) -cne $config.archiveSha256){throw 'source archive differs'}
 $assembly='C:\Windows\Microsoft.NET\Framework64\v4.0.30319\System.IO.Compression.dll'
 if(-not $config.compilerFiles.PSObject.Properties[$assembly] -or (Get-A6Hash $assembly) -cne $config.compilerFiles.PSObject.Properties[$assembly].Value){throw 'compression assembly identity missing/different'}
 [void][Reflection.Assembly]::LoadFrom($assembly)
[IO.Directory]::CreateDirectory($destination)|Out-Null
$source=[IO.File]::OpenRead($archive)
$zip=[IO.Compression.ZipArchive]::new($source,[IO.Compression.ZipArchiveMode]::Read,$false)
$seen=[Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase);$count=0
 foreach($entry in $zip.Entries){
    if($entry.FullName.EndsWith('/')){continue}
    $name=$entry.FullName
    if($name.StartsWith('/') -or $name.Contains('\') -or $name.Contains(':') -or (($entry.ExternalAttributes -shr 16) -band 0xf000) -eq 0xa000){throw 'unsafe ZIP entry'}
    foreach($segment in $name.Split('/')){if($segment -in @('','.', '..') -or $segment.EndsWith('.') -or $segment.EndsWith(' ') -or $segment -match '^(?i:CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(?:\.|$)'){throw 'invalid Windows source component'}}
    if(-not $seen.Add($name)){throw 'case-fold duplicate ZIP entry'}
    $expected=$config.sourceFiles.PSObject.Properties[$name];if(-not $expected){throw 'unexpected source entry'}
    $path=Assert-A6Path (Join-Path $destination $name.Replace('/','\')) $config.root
    [IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($path))|Out-Null
    $inputEntry=$entry.Open();$output=[IO.File]::Open($path,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
    try{
        $buffer=[byte[]]::new(65536)
        while(($bytes=$inputEntry.Read($buffer,0,$buffer.Length)) -gt 0){Assert-A6StageActivation $stageMonitor;$output.Write($buffer,0,$bytes)}
        $output.Flush($true)
    }finally{$inputEntry.Dispose();$output.Dispose()}
    if((Get-A6Hash $path) -cne $expected.Value){throw 'source file digest differs'}
    Assert-A6StageActivation $stageMonitor;$count++
 }
 if($count -ne @($config.sourceFiles.PSObject.Properties).Count){throw 'missing source entries'}
 $completed=$true
}catch{
 $failure=$_.Exception.ToString()
 Add-A6Diagnostic $state 'source extraction' $_.Exception $true
}finally{
 foreach($ownedStream in @($zip,$source)){
  if($null -ne $ownedStream){try{$ownedStream.Dispose()}catch{Add-A6Diagnostic $state 'extraction stream disposal' $_.Exception $true;if($null -eq $failure){$failure=$_.Exception.ToString()}}}
 }
}
try{
 if(-not $completed -or $null -ne $failure){throw 'UNKNOWN: extraction or stream cleanup failed'}
 Assert-A6StageActivation $stageMonitor
 Write-A6Json (Join-Path $Control 'source-extracted.json') @{runId=$config.runId;candidateSha=$config.candidateSha;archiveSha256=$config.archiveSha256;files=$count;workspace=$destination;status='EXACT_SOURCE_VERIFIED'}
}catch{
 # No child is created by this extractor. Forced self-exit may prevent a receipt;
 # the host keeps that outcome UNKNOWN and never launches Cargo after failure.
 Add-A6Diagnostic $state 'extraction completion/receipt' $_.Exception $true
 try{Write-A6Json (Join-Path $Control 'source-extraction-failure.json') @{runId=$config.runId;candidateSha=$config.candidateSha;status='UNKNOWN';error=$failure;stopReason=$state.Stop;partialFilesRetained=$true;diagnostics=@{lostOrOverflowed=$state.Lost;cleanupFailure=$state.CleanupFailed;entries=$state.Diagnostics.ToArray()}}}catch{Add-A6Diagnostic $state 'extraction failure receipt' $_.Exception $true}
 throw
}finally{$state.Done=$true}
