# Sólo para Windows Actions: prueba el mismo EXE distribuido, sin PowerShell en el paquete.
param([Parameter(Mandatory)][string]$Root, [Parameter(Mandatory)][string]$Output)
$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path $Root).Path
$Output = (Resolve-Path $Output).Path
$exe = Join-Path $Root 'bin/ai-stack-desktop.exe'
$runtime = Join-Path $Root 'runtimes'
$cache = Join-Path $Root 'cache'
$scratch = Join-Path $Root 'preparacion'
$results = [System.Collections.Generic.List[string]]::new()

function Start-Test([string]$Mode, [string]$Report, [string]$App = $exe) {
    Start-Process -FilePath $App -WorkingDirectory (Split-Path $App) -ArgumentList @($Mode, "`"$Report`"") -PassThru
}

function Invoke-Test([string]$Mode, [string]$Name, [bool]$Success = $true, [string]$App = $exe) {
    $report = Join-Path $Output "$Name.txt"
    $p = Start-Test $Mode $report $App
    try {
        if (!$p.WaitForExit(1200000)) { throw "Plazo agotado: $Name" }
        if (!(Test-Path -LiteralPath $report)) { throw "No se obtuvo informe: $Name ($($p.ExitCode))" }
        $text = Get-Content -Raw -LiteralPath $report -Encoding utf8
        if (($p.ExitCode -eq 0) -ne $Success) { throw "Resultado inesperado: $Name ($($p.ExitCode)): $text" }
        $text.Trim()
    } finally {
        if (!$p.HasExited) { $p.Kill(); $p.WaitForExit() }
        $p.Dispose()
    }
}

function Assert-Stats([string]$Text, [int]$Reused, [int]$Installed, [int]$Downloads) {
    if ($Text -notmatch "reutilizados=$Reused; instalados=$Installed; descargas=$Downloads$") { throw "Contadores incorrectos: $Text" }
}

if ((Test-Path $runtime) -or (Test-Path $cache)) { throw 'La prueba exige una instalación nueva' }
$first = Invoke-Test --ci-preparar primera
Assert-Stats $first 0 5 6
$results.Add('Primera preparación en ruta con espacios y ñ: cinco componentes, seis descargas directas fijadas y SQLite x64.')

$hashesBefore = Get-FileHash "$runtime/node/node-v24.21.0-win-x64/node.exe", "$runtime/oc/opencode.exe"
$timeBefore = (Get-Item "$runtime/oc/opencode.exe").LastWriteTimeUtc
$downloadsBefore = Get-ChildItem "$cache/descargas" -File | Select-Object Name,LastWriteTimeUtc
Invoke-Test --ci-verificar reapertura | Out-Null
$results.Add('Nueva instancia: verifica el árbol existente sin preparar de nuevo.')

# Obligar a reutilizar lo instalado: no hay una caché visible y no debe invocarse npm.
Rename-Item -LiteralPath $cache -NewName 'cache-apartada-ci'
try {
    Assert-Stats (Invoke-Test --ci-preparar repetida) 5 0 0
    if (Get-ChildItem "$cache/descargas" -File) { throw 'La preparación repetida descargó archivos' }
    if ((Get-Item "$runtime/oc/opencode.exe").LastWriteTimeUtc -ne $timeBefore) { throw 'OpenCode se reinstaló' }
} finally {
    Remove-Item -LiteralPath $cache -Recurse -Force
    Rename-Item -LiteralPath (Join-Path $Root 'cache-apartada-ci') -NewName 'cache'
}
$results.Add('Preparación repetida: cinco reutilizados, cero instalaciones y cero descargas, incluso sin caché previa.')

$cpa = Join-Path $runtime 'cpa/cli-proxy-api.exe'
$archive = Join-Path $cache 'descargas/CLIProxyAPI_8.1.7_windows_amd64.zip'
Set-Content -LiteralPath $cpa 'corrupto'
Set-Content -LiteralPath $archive 'corrupto'
$bad = Invoke-Test --ci-verificar corrupto $false
if ($bad -notmatch 'SHA-256 incorrecto') { throw "Diagnóstico inesperado: $bad" }
Assert-Stats (Invoke-Test --ci-preparar reparada) 4 1 1
foreach ($file in $downloadsBefore | Where-Object Name -ne (Split-Path $archive -Leaf)) {
    if ((Get-Item (Join-Path "$cache/descargas" $file.Name)).LastWriteTimeUtc -ne $file.LastWriteTimeUtc) { throw 'Se redescargó un componente válido' }
}
$results.Add('EXE y ZIP corruptos detectados; sólo se descargó y sustituyó CLIProxyAPI.')

Remove-Item -LiteralPath $cpa
$work = Join-Path $scratch 'trabajo'
New-Item -ItemType Directory -Path "$work/cpa" | Out-Null
Set-Content -LiteralPath "$work/cpa/incompleto.txt" 'staging anterior'
Set-Content -LiteralPath "$archive.part" 'descarga interrumpida'
Invoke-Test --ci-verificar incompleta $false | Out-Null
Assert-Stats (Invoke-Test --ci-preparar recuperada) 4 1 0
if ((Test-Path $work) -or (Test-Path "$archive.part")) { throw 'Restos de preparación no limpiados' }
foreach ($h in $hashesBefore) {
    if ((Get-FileHash -LiteralPath $h.Path).Hash -ne $h.Hash) { throw 'Se alteró un runtime válido' }
}
$results.Add('Instalación parcial y archivos .part: recuperación desde caché verificada, conservando los otros runtimes.')

$denied = Join-Path $Output 'sin permiso'
$deniedBin = New-Item -ItemType Directory -Path "$denied/bin" -Force
$deniedExe = Join-Path $deniedBin.FullName 'ai-stack-desktop.exe'
Copy-Item -LiteralPath $exe -Destination $deniedExe
$sid = [System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value
$principal = "*$sid"
& icacls $denied /deny "${principal}:(WD)" | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'No se pudo aplicar la ACL de ensayo' }
try {
    $err = Invoke-Test --ci-preparar permiso $false $deniedExe
    if ($err -notmatch 'permiso denegado') { throw "Diagnóstico de permisos inesperado: $err" }
    if (Test-Path "$denied/runtimes") { throw 'Se intentó escribir en una ubicación denegada' }
} finally {
    & icacls $denied /remove:d $principal | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'No se pudo retirar la ACL de ensayo' }
}
$results.Add('ACL de escritura denegada: error legible, sin crear runtimes en esa ubicación.')

$cmd = Invoke-Test --prueba-backend backend
$cmd
if ($cmd -notmatch '(?s)OpenCode: Job vacío.*OmniRoute: Job vacío.*CLIProxyAPI: Job vacío.*Proveedor sintético: Job vacío') { throw 'Falta la evidencia de cierre inverso' }
$results.Add('Backend del EXE: turno sintético y cuatro Jobs vacíos, cierre inverso.')

$pidFile = Join-Path $Output 'pids-temporales.txt'
$owner = Start-Test --ci-cierre-forzado $pidFile
$owned = @()
try {
    $until = [DateTime]::UtcNow.AddMinutes(5)
    do {
        if ($owner.HasExited -or [DateTime]::UtcNow -gt $until) { throw 'No se pudo arrancar el ensayo de cierre forzado' }
        Start-Sleep -Milliseconds 200
        $list = if (Test-Path $pidFile) { Get-Content -Raw -LiteralPath $pidFile }
    } until ($list -match '^\d+( \d+){7,}$')
    $owned = @($list.Split(' ') | ForEach-Object { Get-Process -Id ([int]$_) -ErrorAction Stop })
    $owned | ForEach-Object { $null = $_.Handle }
    $blocked = Invoke-Test --ci-verificar bloqueo $false
    if ($blocked -notmatch 'otra instancia') { throw 'No se respetó el bloqueo de instalación' }
    $owner.Kill()
    $owner.WaitForExit()
    foreach ($process in $owned) {
        if (!$process.WaitForExit(15000)) { throw 'Quedaron procesos del Job tras terminar el EXE' }
    }
} finally {
    if (!$owner.HasExited) { $owner.Kill(); $owner.WaitForExit() }
    $owned | ForEach-Object { $_.Dispose() }
    $owner.Dispose()
    if (Test-Path $pidFile) { Remove-Item -LiteralPath $pidFile }
}
Invoke-Test --ci-verificar tras-cierre | Out-Null
$results.Add('Cierre forzado del EXE: todos los procesos observados de sus cuatro Jobs terminaron; nuevo propietario admitido.')
$results | Set-Content -Encoding utf8 (Join-Path $Output 'preparacion-ci.txt')
$results
