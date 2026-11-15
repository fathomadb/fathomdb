function Invoke-A6Receive([string]$ConfigBase64) {
    $ErrorActionPreference='Stop'
    $c=[Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($ConfigBase64))|ConvertFrom-Json
    Assert-A6Start $c | Out-Null
    if($c.expectedSha256 -cnotmatch '^[0-9a-f]{64}$' -or $c.length -lt 1 -or $c.length -gt 1073741824){throw 'transfer size/digest invalid'}
    if($c.relative -cnotmatch '^(packages/[0-9a-f]{64}/[A-Za-z0-9._-]+|control/[0-9a-f]{32}/run.json|inputs/[0-9a-f]{32}/source.zip)$'){throw 'transfer destination not allocated'}
    $owner=Join-Path $c.root '.a6-owner.json'
    if(Test-Path -LiteralPath $c.root){
        if(-not(Test-Path -LiteralPath $owner)){throw 'existing root has no ownership receipt'}
        $o=Get-Content -Raw -Encoding UTF8 -LiteralPath $owner|ConvertFrom-Json
        if($o.task -cne '0829-continuation-slice20' -or $o.diskUniqueId -cne $c.diskUniqueId -or [Guid]$o.partitionGuid -ne [Guid]$c.partitionGuid){throw 'root owner differs'}
    }else{[IO.Directory]::CreateDirectory($c.root)|Out-Null;Write-A6Json $owner @{task='0829-continuation-slice20';diskUniqueId=$c.diskUniqueId;partitionGuid=$c.partitionGuid}}
    $destination=Assert-A6Path (Join-Path $c.root $c.relative.Replace('/','\')) $c.root
    $parent=[IO.Path]::GetDirectoryName($destination);[IO.Directory]::CreateDirectory($parent)|Out-Null
    if([IO.File]::Exists($destination)){
        if((Get-A6Hash $destination) -cne $c.expectedSha256 -or (Get-Item -LiteralPath $destination).Length -ne $c.length){throw 'occupied destination differs'}
        @{runId=$c.runId;candidateSha=$c.candidateSha;status='EXISTING_VERIFIED_NO_WRITE';path=$destination;sha256=$c.expectedSha256}|ConvertTo-Json -Compress;return
    }
    $s=[hashtable]::Synchronized(@{Clock=[Diagnostics.Stopwatch]::StartNew();Done=$false;ResourceAt=0L;Stop=$false})
    $facts='function Get-A6Facts {'+${function:Get-A6Facts}.ToString()+'}'
    $resource=[PowerShell]::Create();[void]$resource.AddScript("param(`$s,`$c)`n"+$facts+@'
try{while(-not $s.Done){$f=Get-A6Facts $c;if($f.cFree -lt 5GB -or $f.rFree -lt 20GB -or $f.memory -lt 8GB){$s.Stop=$true;break};$s.ResourceAt=$s.Clock.ElapsedMilliseconds;[Threading.Thread]::Sleep(1000)}}catch{$s.Stop=$true}
'@).AddArgument($s).AddArgument($c);$r=$resource.BeginInvoke()
    $watch=[PowerShell]::Create();[void]$watch.AddScript(@'
param($s)
while(-not $s.Done){$now=$s.Clock.ElapsedMilliseconds;if($s.Stop -or $now -ge 180000 -or ($s.ResourceAt -eq 0 -and $now -gt 6000) -or ($s.ResourceAt -gt 0 -and $now-$s.ResourceAt -gt 3000)){[Diagnostics.Process]::GetCurrentProcess().Kill()};[Threading.Thread]::Sleep(200)}
'@).AddArgument($s);$w=$watch.BeginInvoke()
    $partial=$destination+'.partial-'+[Guid]::NewGuid().ToString('N')
    $stream=[IO.File]::Open($partial,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
    $inputStream=[Console]::OpenStandardInput();$buffer=[byte[]]::new(65536);$total=0L
    try{while(($n=$inputStream.Read($buffer,0,$buffer.Length)) -gt 0){$total+=$n;if($total -gt $c.length){throw 'transfer longer than declared'};$stream.Write($buffer,0,$n)};$stream.Flush($true)}finally{$stream.Dispose()}
    if($total -ne $c.length -or (Get-A6Hash $partial) -cne $c.expectedSha256){throw 'transfer truncated or digest differs'}
    if($s.Stop -or [IO.File]::Exists($destination)){throw 'transfer stop or destination raced'}
    [IO.File]::Move($partial,$destination)
    $s.Done=$true
    @{runId=$c.runId;candidateSha=$c.candidateSha;status='TRANSFER_VERIFIED';path=$destination;bytes=$total;sha256=$c.expectedSha256}|ConvertTo-Json -Compress
}
