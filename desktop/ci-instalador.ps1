param([Parameter(Mandatory)][string]$Setup,[Parameter(Mandatory)][string]$Portable)
$ErrorActionPreference='Stop'
$app="$env:RUNNER_TEMP\AI Stack ñ"; $fresh="$env:LOCALAPPDATA\AI Stack Desktop\Perfil"
function Installer($file,$opts) {
    $p=Start-Process $file -ArgumentList (@('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART') + $opts) -Wait -PassThru
    if ($p.ExitCode) {throw "Instalador: $($p.ExitCode)"}
}
function Snapshot($rootPid) {
    $rows=Get-CimInstance Win32_Process -Property ProcessId,ParentProcessId,PrivatePageCount,UserModeTime,KernelModeTime,WorkingSetSize
    $ids=@([uint32]$rootPid); $all=@()
    while ($ids.Count) {
        $all+=@($rows | Where-Object ProcessId -in $ids)
        $ids=@($rows | Where-Object ParentProcessId -in $ids | ForEach-Object ProcessId)
    }
    [pscustomobject]@{PrivateMiB=[math]::Round(($all|Measure-Object PrivatePageCount -Sum).Sum/1MB);Processes=$all.Count;
        CpuSeconds=(($all|Measure-Object UserModeTime -Sum).Sum+($all|Measure-Object KernelModeTime -Sum).Sum)/1e7}
}
function Open-Stack($exe,$root,$label) {
    $timer=[Diagnostics.Stopwatch]::StartNew();$out="$env:RUNNER_TEMP\stack-out.txt";$err="$env:RUNNER_TEMP\stack-err.txt"
    $p=Start-Process $exe -ArgumentList '--ci-cuentas' -RedirectStandardOutput $out -RedirectStandardError $err -PassThru
    $profile="$root\datos\cuentas-ci"; $sessionDir="$profile\gentle\agent\sessions"
    try {
        while (!(Select-String -LiteralPath $out -SimpleMatch 'GPUI lista' -Quiet)) {
            if ($p.HasExited -or $timer.Elapsed.TotalMinutes -gt 16){throw 'No llegó a GPUI'}; Start-Sleep -Milliseconds 300
        }
        $ready=[math]::Round($timer.Elapsed.TotalSeconds,1)
        $settings=Get-Content "$profile\gentle\agent\settings.json" -Raw|ConvertFrom-Json
        if($settings.defaultModel -ne 'cpa/lab-reasoner'){throw 'Modelo inicial no conversacional'}
        $deadline=[datetime]::UtcNow.AddMinutes(3)
        do {
            $turn=Get-ChildItem $sessionDir -Recurse -Filter *.jsonl -ErrorAction SilentlyContinue | Where-Object LastWriteTimeUtc -ge $p.StartTime.ToUniversalTime().AddSeconds(-1) | Select-String -SimpleMatch 'LABLABLAB' -Quiet
            if ($p.HasExited -or [datetime]::UtcNow -gt $deadline){throw 'Sin respuesta sintética nueva'};Start-Sleep -Milliseconds 500
        } while (!$turn)
        $first=[math]::Round($timer.Elapsed.TotalSeconds,1)
        Start-Sleep 3; $a=Snapshot $p.Id; $t=$timer.Elapsed.TotalSeconds;Start-Sleep 8;$b=Snapshot $p.Id
        $cpu=[math]::Round(100*($b.CpuSeconds-$a.CpuSeconds)/($timer.Elapsed.TotalSeconds-$t),1)
        Write-Output "MEDIDA $label GPUI_s=$ready respuesta_s=$first privados_MiB=$($b.PrivateMiB) procesos=$($b.Processes) CPU_pct_un_nucleo=$cpu"
        $gui=Get-Process -Id (Get-Content "$profile\gentle\desktop\instance.json" -Raw|ConvertFrom-Json).pid
        if (!$gui.CloseMainWindow() -or !$p.WaitForExit(60000) -or $p.ExitCode){throw 'Cierre incompleto'}
    } catch {Get-Content $err|Select-String -Pattern '^AI Stack Desktop|^Setup sintético:'; throw}
    finally {if (!$p.HasExited){$p.Kill();$p.WaitForExit()};$p.Dispose()}
}
$exe="$app\bin\ai-stack-gentle.exe"; $argsDir="/DIR=`"$app`""
Installer $Setup @($argsDir)
foreach($link in @([Environment]::GetFolderPath('Desktop'),[Environment]::GetFolderPath('Programs'))){if(!(Test-Path "$link\AI Stack Desktop.lnk")){throw 'Acceso directo ausente'}}
Open-Stack $exe $fresh '007-limpio'
Open-Stack $exe $fresh '007-reapertura'
py -3.12 -B desktop/medir-disco.py $fresh
Installer "$app\unins000.exe" @()
if (!(Test-Path "$fresh\datos\cuentas-ci\claves.dpapi") -or (Test-Path $exe)){throw 'Desinstalación incorrecta'}
Open-Stack "$Portable\bin\ai-stack-gentle.exe" $Portable '006-limpio'
Open-Stack "$Portable\bin\ai-stack-gentle.exe" $Portable '006-reapertura'
py -3.12 -B desktop/medir-disco.py $Portable
$dir="$Portable\datos\cuentas-ci";$vault=(Get-FileHash "$dir\claves.dpapi").Hash
$session=Get-ChildItem "$dir\gentle\agent\sessions" -Recurse -Filter *.jsonl|Select-Object -First 1
$marca=(Get-FileHash "$dir\gentle\launcher.json").Hash
$settings="$dir\gentle\agent\settings.json";$cfg=Get-Content $settings -Raw|ConvertFrom-Json;$cfg.defaultModel='cpa/codex-auto-review';$cfg | ConvertTo-Json -Depth 30 | Set-Content $settings -Encoding utf8NoBOM
Installer $Setup @($argsDir,"/PORTABLE=`"$Portable`"")
Open-Stack $exe $Portable '007-sobre-006'
Installer $Setup @($argsDir)
if((Get-Content "$app\bin\perfil.txt" -Raw).Trim() -ne $Portable -or (Get-FileHash "$dir\claves.dpapi").Hash -ne $vault -or (Get-FileHash "$dir\gentle\launcher.json").Hash -ne $marca -or !(Test-Path $session.FullName)){throw 'Se perdió estado en la actualización'}
Installer "$app\unins000.exe" @()
if(!(Test-Path "$Portable\bin\ai-stack-gentle.exe") -or !(Test-Path $session.FullName)){throw 'Recuperación dañada'}
