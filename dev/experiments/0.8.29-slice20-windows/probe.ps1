function Invoke-A6Probe([string]$ConfigBase64) {
    $ErrorActionPreference='Stop'
    $c=[Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($ConfigBase64))|ConvertFrom-Json
    Assert-A6Start $c|Out-Null
    if($PSVersionTable.PSVersion.Major -ne 5 -or $PSVersionTable.PSVersion.Minor -lt 1){throw 'requires existing Windows PowerShell5.1; no installation fallback'}
    if([Environment]::MachineName -cne 'MEMEX-AFE8V4I00' -or -not [Environment]::Is64BitProcess){throw 'venue/architecture differs'}
    $framework='C:\Windows\Microsoft.NET\Framework64\v4.0.30319'
    $paths=@('csc.exe','mscorlib.dll','System.dll','System.Core.dll','System.Management.dll','System.Web.Extensions.dll','System.IO.Compression.dll')|ForEach-Object{Join-Path $framework $_}
    $paths+=@('C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe',$c.cargoExe,$c.rustcExe)
    $hashes=@{};foreach($path in $paths){if(-not(Test-Path -LiteralPath $path -PathType Leaf)){throw "missing immutable tool/reference: $path"};$hashes[$path]=Get-A6Hash $path}
    @{runId=$c.runId;candidateSha=$c.candidateSha;hostname=[Environment]::MachineName;diskUniqueId=$c.diskUniqueId;partitionGuid=$c.partitionGuid;psVersion=$PSVersionTable.PSVersion.ToString();frameworkRuntime=[Environment]::Version.ToString();executionPolicies=@(Get-ExecutionPolicy -List|ForEach-Object{@{scope=$_.Scope.ToString();policy=$_.ExecutionPolicy.ToString()}});files=$hashes;facts=(Get-A6Facts $c);status='READ_ONLY_IDENTITY_OBSERVATION';utc=[DateTime]::UtcNow.ToString('o')}|ConvertTo-Json -Depth 8 -Compress
}
