# Source proposal only. Loading this file defines functions; it performs no VM action.
Set-StrictMode -Version Latest
function Assert-A6Identity($Config) {
    if ($Config.runId -cnotmatch '^[0-9a-f]{32}$' -or $Config.candidateSha -cnotmatch '^[0-9a-f]{40}$' -or
        $Config.root -cne 'R:\fathomdb-s20-a6') { throw 'invalid run/source/root identity' }
}
function Assert-A6Path([string]$Path, [string]$Root) {
    $full = [IO.Path]::GetFullPath($Path)
    $prefix = [IO.Path]::GetFullPath($Root).TrimEnd('\') + '\'
    if (-not $full.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) { throw 'owned path escape' }
    $cursor = [IO.Path]::GetDirectoryName($full)
    while ($cursor -and $cursor.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) {
        if ([IO.Directory]::Exists($cursor) -and
            (([IO.File]::GetAttributes($cursor) -band [IO.FileAttributes]::ReparsePoint) -ne 0)) { throw 'reparse ancestor' }
        $cursor = [IO.Path]::GetDirectoryName($cursor)
    }
    return $full
}
function Get-A6Hash([string]$Path) { return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() }
function Write-A6Json([string]$Path, $Value) {
    $temporary = $Path + '.tmp-' + [Guid]::NewGuid().ToString('N')
    $bytes = [Text.Encoding]::UTF8.GetBytes(($Value | ConvertTo-Json -Depth 40 -Compress))
    $f = [IO.File]::Open($temporary, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
    try { $f.Write($bytes,0,$bytes.Length); $f.Flush($true) } finally { $f.Dispose() }
    if ([IO.File]::Exists($Path)) { [IO.File]::Replace($temporary,$Path,[NullString]::Value) } else { [IO.File]::Move($temporary,$Path) }
}
function Get-A6Facts {
    param($Config)
    $part = @(Get-CimInstance -Namespace 'root/Microsoft/Windows/Storage' -ClassName MSFT_Partition -Filter 'DriveLetter=82' -OperationTimeoutSec 2)
    if ($part.Count -ne 1 -or [Guid]$part[0].Guid -ne [Guid]$Config.partitionGuid) { throw 'R partition identity differs' }
    $disk = @(Get-CimInstance -Namespace 'root/Microsoft/Windows/Storage' -ClassName MSFT_Disk -Filter "Number=$($part[0].DiskNumber)" -OperationTimeoutSec 2)
    if ($disk.Count -ne 1 -or $disk[0].Number -eq 0 -or $disk[0].UniqueId -cne $Config.diskUniqueId -or
        $disk[0].SerialNumber.Trim() -cne 'FDB29S20A6' -or $disk[0].Size -ne 68719476736 -or
        $disk[0].IsBoot -or $disk[0].IsSystem -or $disk[0].IsOffline -or $disk[0].IsReadOnly) { throw 'owned disk differs' }
    $r = [IO.DriveInfo]::new('R'); $c = [IO.DriveInfo]::new('C')
    if (-not $r.IsReady -or $r.DriveFormat -cne 'NTFS' -or $r.VolumeLabel -cne 'FDB29S20A6') { throw 'owned volume differs' }
    $os = Get-CimInstance Win32_OperatingSystem -OperationTimeoutSec 2
    return @{cFree=$c.AvailableFreeSpace; rFree=$r.AvailableFreeSpace; memory=[long]$os.FreePhysicalMemory*1024; utc=[DateTime]::UtcNow.ToString('o')}
}
function Assert-A6Start($Config) {
    Assert-A6Identity $Config
    $f = Get-A6Facts $Config
    if ($f.cFree -lt 8GB -or $f.rFree -lt 32GB -or $f.memory -lt 16GB) { throw 'guest start floor' }
    return $f
}
function Read-A6Config([string]$Control,[string]$Hash) {
    if ($Control -cnotmatch '^R:\\fathomdb-s20-a6\\control\\[0-9a-f]{32}$' -or $Hash -cnotmatch '^[0-9a-f]{64}$') { throw 'control/hash syntax' }
    $path = Join-Path $Control 'run.json'
    if ((Get-A6Hash $path) -cne $Hash) { throw 'run manifest digest differs' }
    $c = Get-Content -Raw -Encoding UTF8 -LiteralPath $path | ConvertFrom-Json
    Assert-A6Identity $c
    if ($c.runId -cne (Split-Path -Leaf $Control)) { throw 'control run differs' }
    return $c
}
function Assert-A6Files($Config) {
    foreach ($entry in $Config.packageFiles.PSObject.Properties) {
        $path = Assert-A6Path (Join-Path $Config.packagePath $entry.Name) $Config.root
        if ((Get-A6Hash $path) -cne $entry.Value) { throw "package content differs: $($entry.Name)" }
    }
}
