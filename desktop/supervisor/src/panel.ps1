$ErrorActionPreference='Stop'
[Console]::InputEncoding=[Text.UTF8Encoding]::new($false)
$cfg=[Console]::In.ReadToEnd() | ConvertFrom-Json
Add-Type -AssemblyName System.Windows.Forms,System.Drawing
$form=New-Object Windows.Forms.Form
$form.Text='AI Stack Desktop · Cuentas y administración'; $form.ClientSize=New-Object Drawing.Size(555,230)
$form.StartPosition='CenterScreen'; $form.FormBorderStyle='FixedDialog'; $form.MaximizeBox=$false
$flow=New-Object Windows.Forms.FlowLayoutPanel; $flow.Dock='Fill'; $flow.Padding=New-Object Windows.Forms.Padding(14); $form.Controls.Add($flow)
function Label($s) { $l=New-Object Windows.Forms.Label; $l.Text=$s; $l.AutoSize=$true; $flow.Controls.Add($l); $flow.SetFlowBreak($l,$true) }
function Button($s,$click) {$b=New-Object Windows.Forms.Button; $b.Text=$s; $b.AutoSize=$true; $b.Add_Click($click); $flow.Controls.Add($b)}
function Open-Admin($service) {
    $edge=@([Environment]::GetFolderPath('ProgramFilesX86'),[Environment]::GetFolderPath('ProgramFiles')) | ForEach-Object {Join-Path $_ 'Microsoft/Edge/Application/msedge.exe'} | Where-Object {Test-Path -LiteralPath $_} | Select-Object -First 1
    if (!$edge) {[Windows.Forms.MessageBox]::Show('Se necesita Microsoft Edge para abrir los paneles privados.');return}
    $dir=Join-Path (Get-Location) "panel-$service"
    $url=if ($service -eq 'cpa') {"http://127.0.0.1:$($cfg.cpa)/management.html"} else {"http://127.0.0.1:$($cfg.omni)/dashboard"}
    Start-Process -FilePath $edge -ArgumentList @("--user-data-dir=`"$dir`"",'--no-first-run','--disable-background-mode',"--app=$url")
}
Label '1. Abra CLIProxyAPI → OAuth y complete el flujo oficial del proveedor.'
Button 'CLIProxyAPI' {Open-Admin cpa}; Button 'OmniRoute' {Open-Admin omni}
$flow.SetFlowBreak($flow.Controls[$flow.Controls.Count-1],$true)
Label '2. Si el panel pide su clave local, muéstrela y péguela en ese panel.'
$key=New-Object Windows.Forms.TextBox; $key.ReadOnly=$true; $key.Width=500
Button 'Mostrar clave CPA' {$key.Text=$cfg.admin;$key.SelectAll();$key.Focus()};Button 'Mostrar contraseña OmniRoute' {$key.Text=$cfg.password;$key.SelectAll();$key.Focus()};Button 'Ocultar' {$key.Clear()};$flow.Controls.Add($key)
Label '3. Los modelos se sincronizan automáticamente. Abra una sesión nueva.'
Button 'Sincronizar ahora' {[IO.File]::WriteAllText($cfg.signal,'sync')};Button 'Volver a Gentle Shell' {$form.Hide()}
$tray=New-Object Windows.Forms.NotifyIcon; $tray.Icon=[Drawing.SystemIcons]::Application; $tray.Text='AI Stack Desktop · Cuentas y administración'; $tray.Visible=$true
$menu=New-Object Windows.Forms.ContextMenuStrip; [void]$menu.Items.Add('Cuentas y administración',$null,{$form.Show();$form.Activate()});$tray.ContextMenuStrip=$menu;$tray.Add_DoubleClick({$form.Show();$form.Activate()})
$form.Add_FormClosing({param($sender,$evt) $evt.Cancel=$true;$form.Hide()});$form.Add_VisibleChanged({if(!$form.Visible){$key.Clear()}})
$timer=New-Object Windows.Forms.Timer; $timer.Interval=700
$timer.Add_Tick({foreach ($name in 'cuentas','cliproxyapi','omniroute') {
    $file=Join-Path (Split-Path $cfg.signal) "abrir-$name"
    if ([IO.File]::Exists($file)) {[IO.File]::Delete($file); switch ($name) {
        cuentas {$form.Show();$form.Activate()} cliproxyapi {Open-Admin cpa} omniroute {Open-Admin omni}
    }}
}});$timer.Start()
if ($cfg.open){$form.Show()};[Windows.Forms.Application]::Run()
