function Invoke-A6Parse([string]$ConfigBase64) {
    $ErrorActionPreference='Stop'
    $c=[Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($ConfigBase64))|ConvertFrom-Json
    Assert-A6Start $c|Out-Null
    $rows=@();$valid=$true
    foreach($entry in $c.packageFiles.PSObject.Properties){
        if($entry.Name -cnotmatch '^[A-Za-z0-9._-]+$'){throw 'package filename syntax'}
        $path=Assert-A6Path (Join-Path $c.packagePath $entry.Name) $c.root
        if((Get-A6Hash $path) -cne $entry.Value){throw 'package hash differs'}
        if($entry.Name.EndsWith('.ps1')){
            $tokens=$null;$errors=$null
            [void][Management.Automation.Language.Parser]::ParseFile($path,[ref]$tokens,[ref]$errors)
            if($errors.Count -ne 0){$valid=$false}
            $rows+=@(@{file=$entry.Name;sha256=$entry.Value;errors=@($errors|ForEach-Object{$_.ToString()})})
        }
    }
    @{runId=$c.runId;candidateSha=$c.candidateSha;packageManifestSha256=$c.packageManifestSha256;valid=$valid;psVersion=$PSVersionTable.PSVersion.ToString();files=$rows;kind='ACTUAL_POWERSHELL_PARSE_ONLY'}|ConvertTo-Json -Depth 15 -Compress
    if(-not $valid){throw 'PowerShell source parsing failed'}
}
