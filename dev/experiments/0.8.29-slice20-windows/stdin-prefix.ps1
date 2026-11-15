# Read exactly the script prefix; the receiver owns all following binary bytes.
$ErrorActionPreference='Stop'
$b=New-Object byte[] __A6_STDIN_BYTES__
$i=[Console]::OpenStandardInput()
$n=0
while($n -lt $b.Length){$r=$i.Read($b,$n,$b.Length-$n);if($r -eq 0){throw 'stdin length'};$n+=$r}
$p=[Text.Encoding]::ASCII.GetString($b)
if($p -cnotmatch '^[A-Za-z0-9+/]+={0,2}$'){throw 'stdin alphabet'}
$h=[Security.Cryptography.SHA256]::Create()
try{$v=([BitConverter]::ToString($h.ComputeHash($b))).Replace('-','').ToLowerInvariant()}finally{$h.Dispose()}
if($v -cne '__A6_STDIN_SHA256__'){throw 'stdin digest'}
$d=[Convert]::FromBase64String($p)
if([Convert]::ToBase64String($d) -cne $p){throw 'stdin canonical'}
& ([ScriptBlock]::Create(([Text.UTF8Encoding]::new($false,$true)).GetString($d)))
