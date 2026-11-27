param([Parameter(Mandatory)][string]$Root,[Parameter(Mandatory)][string]$Report)
$ErrorActionPreference='Stop'
$dir="$Root/datos/cuentas-ci"; $exe="$Root/bin/ai-stack-gentle.exe"
foreach($pass in 1,2){
    $p=Start-Process -FilePath $exe -ArgumentList '--ci-cuentas' -WorkingDirectory $Root -RedirectStandardOutput "$Root/out.txt" -RedirectStandardError "$Root/err.txt" -PassThru
    try {
        $limit=[datetime]::UtcNow.AddMinutes(22)
        while (!(Select-String -LiteralPath "$Root/out.txt" -SimpleMatch 'GPUI lista' -Quiet)){
            if ($p.HasExited -or [datetime]::UtcNow -gt $limit){throw 'El arranque no llegó a GPUI'};Start-Sleep 1
        }
        $acl=Get-Acl $dir; if (!$acl.AreAccessRulesProtected -or @($acl.Access).Count -ne 1){throw 'ACL no privada'}
        $marker=Get-Content -LiteralPath "$dir/gentle/launcher.json" -Raw | ConvertFrom-Json
        $mark=@($marker.provisioned.psobject.Properties.Value)[0];if ($mark.gentleAi -ne '5.0.0' -or $mark.gentlePi -ne '5.0.0'){throw 'Companion no aprovisionado'}
        # Mismo GOROOT que GPUI: el enlace duro del toolchain no es la raíz de la distribución.
        $prev=$env:GOROOT
        try {$env:GOROOT="$Root/runtimes/go/go"; $go=& "$dir/gentle/desktop/toolchain/bin/go.exe" version; if ($LASTEXITCODE -ne 0 -or $go -notmatch 'go1\.27\.4 windows/amd64'){throw 'Go privado no disponible'}} finally {$env:GOROOT=$prev}
        $provider=(Get-Content "$dir/gentle/agent/models.json" -Raw|ConvertFrom-Json).providers.omniroute
        if ($provider.models.Count -ne 1 -or $provider.models[0].id -ne 'cpa/lab-reasoner' -or $provider.models[0].thinkingLevelMap.high -ne 'high' -or $provider.models[0].thinkingLevelMap.low -ne 'low' -or $provider.apiKey -ne '$GS_INFERENCE'){throw 'Proyección dinámica de cuenta incorrecta'}
        if ($pass -eq 1){$saved=@($mark.at,(Get-FileHash "$dir/claves.dpapi").Hash,$provider.baseUrl);$limit=[datetime]::UtcNow.AddMinutes(3)
            while (!(Get-ChildItem "$dir/gentle/agent/sessions" -Recurse -Filter *.jsonl -ErrorAction SilentlyContinue|Select-String -SimpleMatch 'LABLABLAB' -Quiet)){if ($p.HasExited -or [datetime]::UtcNow -gt $limit){throw 'El proveedor sintético no respondió'};Start-Sleep 1}
        }elseif($saved[0] -ne $mark.at -or $saved[1] -ne (Get-FileHash "$dir/claves.dpapi").Hash -or $saved[2] -ne $provider.baseUrl){throw 'Se repitió setup o se reemplazó la cuenta'}
        $gui=Get-Process -Id (Get-Content "$dir/gentle/desktop/instance.json" -Raw | ConvertFrom-Json).pid
        $limit=[datetime]::UtcNow.AddSeconds(45)
        do {$gui.Refresh(); if($gui.MainWindowHandle -ne 0){break}; Start-Sleep -Milliseconds 300} while([datetime]::UtcNow -lt $limit -and !$gui.HasExited)
        if (!$gui.CloseMainWindow()){throw "GPUI sin ventana de cierre en apertura $pass"}
        if (!$p.WaitForExit(60000)){throw "Plazo de cierre en apertura $pass (GPUI terminada: $($gui.HasExited))"};if ($p.ExitCode){throw "Error al cerrar en apertura $pass ($($p.ExitCode))"}
    } catch {
        Get-Content "$Root/err.txt" | Select-String -SimpleMatch 'AI Stack Desktop 0.0.6-dev:','Setup sintético:'
        Get-Content "$Root/out.txt" | Select-Object -Last 3
        throw
    } finally {if (!$p.HasExited){$p.Kill();$p.WaitForExit()};$p.Dispose()}
}
# La primera apertura normal todavía no tiene cuentas: debe permitir acceder a los paneles
# sin una inferencia ni un proveedor sintético que disimule un catálogo vacío.
$p=Start-Process $exe -WorkingDirectory $Root -PassThru -RedirectStandardError "$Root/empty-error.txt" -RedirectStandardOutput "$Root/empty-out.txt"
try {
    $dir="$Root/datos/cuentas/gentle"; $limit=[datetime]::UtcNow.AddMinutes(4)
    do {if ($p.HasExited -or (Select-String -LiteralPath "$Root/empty-error.txt" -SimpleMatch 'AI Stack Desktop 0.0.6-dev:' -Quiet)){throw 'Terminó la apertura sin cuentas'};Start-Sleep 1} while (!(Test-Path "$dir/desktop/instance.json") -and [datetime]::UtcNow -lt $limit)
    $provider=(Get-Content "$dir/agent/models.json" -Raw|ConvertFrom-Json).providers.omniroute
    if ($provider.models.Count -ne 0){throw 'El perfil sin cuentas publicó modelos'}
    $gui=Get-Process -Id (Get-Content "$dir/desktop/instance.json" -Raw|ConvertFrom-Json).pid
    $limit=[datetime]::UtcNow.AddSeconds(40);do {$gui.Refresh();if ($gui.MainWindowHandle -ne 0){break};Start-Sleep 1}while([datetime]::UtcNow -lt $limit)
    if (!$gui.CloseMainWindow() -or !$p.WaitForExit(60000) -or $p.ExitCode){throw 'El perfil sin cuentas no cerró'}
}catch{Get-Content "$Root/empty-error.txt" | Select-String -SimpleMatch 'AI Stack Desktop 0.0.6-dev:';Get-Content "$Root/empty-out.txt"|Select-Object -Last 2;throw}finally{if (!$p.HasExited){$p.Kill();$p.WaitForExit()};$p.Dispose()}
'Windows x64: setup real, Go privado, ACL y DPAPI, inicio y cancelación OAuth sin login, catálogo de CPA, esfuerzos, turno sintético y reapertura del perfil.' | Set-Content $Report
