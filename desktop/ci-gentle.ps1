# Ejecutar exclusivamente en Actions, con una raíz portable limpia y perfil exterior sintético.
param([Parameter(Mandatory)][string]$Root, [Parameter(Mandatory)][string]$Report)
$ErrorActionPreference='Stop'
$Root=(Resolve-Path $Root).Path
$exe="$Root/bin/ai-stack-gentle.exe"
$pidsObserved=@()
function Wait-Until($Proc,$Test,$Seconds,$Stage) {
    $deadline=[datetime]::UtcNow.AddSeconds($Seconds)
    do {
        if ($Proc.HasExited) { throw "Terminó el supervisor: $Stage ($($Proc.ExitCode))" }
        if (& $Test) { return }
        Start-Sleep -Milliseconds 400
    } while ([datetime]::UtcNow -lt $deadline)
    throw "Plazo agotado: $Stage"
}
foreach($mode in '--ci-turno','--ci-delegar') {
    $before=@(Get-ChildItem "$Root/datos/gentle/agent/sessions" -Recurse -Filter *.jsonl -ErrorAction SilentlyContinue | ForEach-Object FullName)
    $p=Start-Process $exe -ArgumentList $mode -WorkingDirectory $Root -RedirectStandardOutput "$Root/progreso.txt" -RedirectStandardError "$Root/error.txt" -PassThru
    try {
        Wait-Until $p { (Test-Path "$Root/datos/gentle/desktop/instance.json") -and (Select-String -LiteralPath "$Root/progreso.txt" -SimpleMatch 'GPUI lista' -Quiet) } 1050 'preparación y ventana'
        $gui=Get-Process -Id (Get-Content -Raw "$Root/datos/gentle/desktop/instance.json" | ConvertFrom-Json).pid
        if ($gui.Path -ne "$Root\runtimes\gpui\gentle-shell-gpui.exe".Replace('/','\')) {throw 'GPUI no pertenece al portable'}
        if (Test-Path "$Root/runtimes/oc") {throw 'La preparación incluyó OpenCode'}
        $models=(Get-Content -Raw "$Root/datos/gentle/agent/models.json" | ConvertFrom-Json).providers.omniroute.models
        if ((($models.id | Sort-Object) -join ',') -ne 'labcpa/lab-helper,labcpa/lab-reasoner') {throw 'Catálogo integrado incorrecto'}
        foreach ($m in $models) {
            $tiers=@($m.thinkingLevelMap.psobject.Properties | Where-Object {$null -ne $_.Value})
            if ((($tiers.Name | Sort-Object) -join ',') -ne 'high,low' -or $m.thinkingLevelMap.low -ne 'low' -or $m.thinkingLevelMap.high -ne 'high' -or $m.contextWindow -ne 262144) {throw 'Capacidades integradas incorrectas'}
        }
        if ($mode -eq '--ci-delegar' -and (Select-String -LiteralPath "$Root/progreso.txt" -SimpleMatch 'Instalado y verificado' -Quiet)) {throw 'Se reinstalaron dependencias íntegras'}
        if ($mode -eq '--ci-turno') {
            Wait-Until $p { @(Get-ChildItem "$Root/datos/gentle/agent/sessions" -Recurse -Filter *.jsonl | Where-Object { $_.FullName -notin $before -and (Select-String -LiteralPath $_.FullName -Pattern 'LABLABLAB' -SimpleMatch -Quiet) }).Count } 140 'respuesta en sesión Pi'
            if (!$gui.CloseMainWindow()) { throw 'Sin ventana para cierre normal' }
            if (!$p.WaitForExit(60000) -or $p.ExitCode -ne 0) { throw 'No terminó limpiamente' }
        } else {
            # El worker → Pi separado debe estar generando el turno lento. La sonda del receptor sólo contiene rol/esfuerzo.
            Wait-Until $p { @(Get-ChildItem "$Root/preparacion" -Recurse -Filter hijo-activo.txt | Where-Object { (Get-Content -Raw $_.FullName) -eq 'high' }).Count } 180 'delegación activa en receptor'
            $rows=Get-CimInstance Win32_Process | Select-Object ProcessId,ParentProcessId,Name,ExecutablePath
            $ids=@($p.Id); do {$new=@($rows | Where-Object {$_.ParentProcessId -in $ids -and $_.ProcessId -notin $ids} | ForEach-Object ProcessId); $ids+=$new} while($new.Count)
            $tree=@($rows|Where-Object {$_.ProcessId -in $ids}); if ($tree.Name -contains 'opencode.exe') {throw 'OpenCode activo'}
            foreach ($name in 'engram.exe','cli-proxy-api.exe','gentle-shell-gpui.exe') {
                if (@($tree | Where-Object Name -eq $name).Count -ne 1) {throw 'Árbol incompleto o servicio duplicado'}
            }
            if (@($tree | Where-Object {$_.Name -eq 'node.exe' -and $_.ExecutablePath -and !$_.ExecutablePath.StartsWith($Root+'\',[StringComparison]::OrdinalIgnoreCase)}).Count) {throw 'Node externo en el árbol'}
            $handles=@($ids | ForEach-Object {Get-Process -Id $_ -ErrorAction SilentlyContinue}); foreach($h in $handles){try {$null=$h.Handle} catch {}}
            $p.Kill(); $p.WaitForExit(); foreach($h in $handles){if (!$h.HasExited -and !$h.WaitForExit(10000)){throw 'Quedó un descendiente al perder el propietario'}}
            $pidsObserved=$handles.Count
        }
    } catch {
        if (Test-Path "$Root/error.txt") { Get-Content "$Root/error.txt" | Select-String -SimpleMatch 'AI Stack Desktop 0.0.5-dev:' }
        throw
    } finally { if (!$p.HasExited){$p.Kill();$p.WaitForExit()}; $p.Dispose() }
}
$summary="Preparación desde cero y reutilización; dos modelos con low/high; turno GPUI; cierre normal y pérdida del propietario durante delegación high: $pidsObserved procesos observados; sin OpenCode."
$summary | Set-Content -Encoding utf8 -LiteralPath $Report
Write-Host $summary
