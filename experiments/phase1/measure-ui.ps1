# Medición preliminar del árbol de una ventana que el operador ha identificado en la VM.
param([Parameter(Mandatory)][int]$RootPid, [ValidateSet('A','B')][string]$Arm='A',
      [ValidateRange(2,1800)][int]$Seconds=120)
$ErrorActionPreference = 'Stop'
$root = Get-Process -Id $RootPid
$started = $root.StartTime
$out = Join-Path $PSScriptRoot ('.lab/ui-{0}-{1}.jsonl' -f $Arm, [guid]::NewGuid())
for ($i=0; $i -lt $Seconds; $i++) {
    $current = Get-Process -Id $RootPid -ErrorAction SilentlyContinue
    if (!$current -or $current.StartTime -ne $started) { break }
    $seen = @{}
    $pending = [Collections.Generic.Queue[int]]::new()
    $pending.Enqueue($RootPid)
    while ($pending.Count) {
        $n = $pending.Dequeue()
        if ($seen.ContainsKey($n)) { continue }
        $seen[$n] = $true
        $p = Get-Process -Id $n -ErrorAction SilentlyContinue
        if (!$p -or $p.StartTime -lt $started) { continue }
        [ordered]@{utc=[datetime]::UtcNow.ToString('o');role=$p.ProcessName;pid=$n;
            created=$p.StartTime.ToUniversalTime().ToString('o');cpu_s=$p.CPU;
            private_bytes=$p.PrivateMemorySize64;working_set=$p.WorkingSet64} |
            ConvertTo-Json -Compress | Add-Content -Encoding UTF8 -Path $out
        Get-CimInstance Win32_Process -Filter "ParentProcessId=$n" -Property ProcessId |
            ForEach-Object { $pending.Enqueue([int]$_.ProcessId) }
    }
    Start-Sleep -Seconds 1
}
Write-Output $out
