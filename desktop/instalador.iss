; Compilar sólo con el payload público preparado por windows-installer.yml.
[Setup]
AppId=AIStackDesktopGPUI
AppName=AI Stack Desktop
AppVersion=0.0.7-dev
DefaultDirName={localappdata}\Programs\AI Stack Desktop
DisableDirPage=no
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.22000
UninstallDisplayIcon={app}\bin\ai-stack-gentle.exe
OutputDir=..\salida
OutputBaseFilename=AI-Stack-Desktop-0.0.7-dev-win-x64-Setup
Compression=lzma2/fast
WizardStyle=modern
CloseApplications=no
[Languages]
Name: "es"; MessagesFile: "compiler:Languages\Spanish.isl"
[Files]
Source: "..\entrega-gentle\bin\*"; DestDir: "{app}\bin"; Flags: ignoreversion
Source: "..\entrega-gentle\*.txt"; DestDir: "{app}"
Source: "..\entrega-gentle\*.md"; DestDir: "{app}"
Source: "..\entrega-gentle\*.json"; DestDir: "{app}"
Source: "..\vc_redist.x64.exe"; Flags: dontcopy
[Icons]
Name: "{userprograms}\AI Stack Desktop"; Filename: "{app}\bin\ai-stack-gentle.exe"
Name: "{userdesktop}\AI Stack Desktop"; Filename: "{app}\bin\ai-stack-gentle.exe"
[UninstallDelete]
Type: files; Name: "{app}\bin\perfil.txt"
[Run]
Filename: "{app}\bin\ai-stack-gentle.exe"; Description: "Abrir AI Stack Desktop y preparar componentes"; Flags: postinstall nowait skipifsilent
[Code]
var Profile: TInputDirWizardPage;
    State, Previous: String;

procedure InitializeWizard;
begin
  Profile := CreateInputDirPage(wpSelectDir, 'Cuentas, perfiles y sesiones',
    'Puede reutilizar su portable 0.0.6-dev, cerrado, sin moverlo.',
    'Conserve la ruta propuesta para un perfil privado nuevo. Para continuar con el portable, seleccione su carpeta raíz (la que contiene bin y datos).', False, '');
  Profile.Add('Perfil privado o raíz del portable:');
  Previous := GetPreviousData('ProfileRoot', '');
  Profile.Values[0] := ExpandConstant('{param:PORTABLE|{localappdata}\AI Stack Desktop\Perfil}');
end;

function ShouldSkipPage(PageID: Integer): Boolean;
begin Result := (PageID = Profile.ID) and (Previous <> ''); end;

function Inside(Child, Parent: String): Boolean;
begin Result := Pos(Uppercase(AddBackslash(Parent)), Uppercase(AddBackslash(Child))) = 1; end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var Code: Integer; V: String; Have, Need: Int64; Marker: AnsiString; Dest: String;
begin
  State := Previous;
  if State = '' then State := Profile.Values[0];
  if State = '' then State := ExpandConstant('{localappdata}\AI Stack Desktop\Perfil')
  else if (Previous = '') and not SameText(State, ExpandConstant('{localappdata}\AI Stack Desktop\Perfil')) then begin
    if not FileExists(AddBackslash(State)+'bin\ai-stack-gentle.exe') then begin Result := 'No se encontró el portable.'; exit; end;
    if CompareText(GetSHA256OfFile(AddBackslash(State)+'bin\ai-stack-gentle.exe'), '6d4c011cd36c398adad9618819dabfbb1b6a81eea11d6b493caba6d303c166ee') <> 0 then begin Result := 'Seleccione el portable validado 0.0.6-dev.'; exit; end;
  end;
  Dest := ExpandConstant('{app}');
  if Inside(Dest, State) or Inside(State, Dest) or FileExists(AddBackslash(Dest)+'.ai-stack-desktop-portable') then begin Result := 'Elija una carpeta de programas separada del perfil y del portable.'; exit; end;
  if DirExists(State) then begin
    if not LoadStringFromFile(AddBackslash(State)+'.ai-stack-desktop-portable', Marker) or (Marker <> 'AI Stack Desktop / portable / 1'+#10) then begin Result := 'Cierre la aplicación. El perfil debe pertenecer a AI Stack Desktop y estar libre.'; exit; end;
  end;
  ExtractTemporaryFile('vc_redist.x64.exe');
  if not GetPackedVersion(ExpandConstant('{tmp}\vc_redist.x64.exe'), Need) then begin Result := 'No se pudo identificar Microsoft VC++.'; exit; end;
  if RegQueryStringValue(HKLM64, 'SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64', 'Version', V) then begin
    if Copy(V,1,1) = 'v' then Delete(V,1,1);
    if StrToVersion(V, Have) and (ComparePackedVersion(Have, Need) >= 0) and FileExists(ExpandConstant('{sys}\vcruntime140.dll')) then exit;
  end;
  if not Exec(ExpandConstant('{tmp}\vc_redist.x64.exe'), '/install /quiet /norestart', '', SW_SHOW, ewWaitUntilTerminated, Code) then begin Result := 'No se pudo iniciar Microsoft VC++.'; exit; end;
  if Code = 3010 then NeedsRestart := True else if (Code <> 0) and (Code <> 1638) then Result := Format('Microsoft VC++ no se instaló (%d).', [Code]);
end;

procedure CurStepChanged(CurStep: TSetupStep);
var Lines: TArrayOfString;
begin
  if CurStep = ssPostInstall then begin
    if not ForceDirectories(ExtractFileDir(State)) then RaiseException('No se pudo preparar la carpeta privada.');
    SetArrayLength(Lines, 1); Lines[0] := State;
    if not SaveStringsToUTF8File(ExpandConstant('{app}\bin\perfil.txt'), Lines, False) then RaiseException('No se pudo registrar el perfil.');
  end;
end;

procedure RegisterPreviousData(PreviousDataKey: Integer);
begin SetPreviousData(PreviousDataKey, 'ProfileRoot', State); end;
