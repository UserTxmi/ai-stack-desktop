# Preparación de desarrollo por usuario. Python portátil sólo durante la preparación.
[CmdletBinding()]param()
$ErrorActionPreference = 'Stop'
$phase = Join-Path $PSScriptRoot 'experiments/phase1'
$lab = Join-Path $phase '.lab'
$tools = Join-Path $lab 'preparacion'
if (Test-Path "$lab/runtime") { throw 'Ya existe el runtime. Para una preparación nueva extraiga el artifact en otra carpeta vacía.' }
New-Item -ItemType Directory -Force $tools | Out-Null
$archive = Join-Path $tools 'python.zip'
Invoke-WebRequest -UseBasicParsing 'https://www.python.org/ftp/python/3.12.10/python-3.12.10-embed-amd64.zip' -OutFile $archive
if ((Get-FileHash $archive -Algorithm SHA256).Hash -ne '4acbed6dd1c744b0376e3b1cf57ce906f9dc9e95e68824584c8099a63025a3c3') { throw 'Python de preparación no coincide' }
Expand-Archive $archive -DestinationPath $tools -Force
foreach ($stage in 'binaries','gateway') {
    & "$tools/python.exe" -IB -c "import sys,runpy; sys.path.insert(0,sys.argv.pop(1)); runpy.run_module('prepare',run_name='__main__')" $phase --ci-stage $stage
    if ($LASTEXITCODE -ne 0) { throw "Falló la preparación: $stage" }
}
Remove-Item -LiteralPath $tools -Recurse -Force
Write-Host 'W1 preparado. Abra ai-stack-desktop.exe como usuario normal.'
