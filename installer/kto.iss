; Inno Setup script for KTO (Windows x64 installer).
;
; Build locally:
;   iscc /DMyAppVersion=3.0.0 /DSourceDir=dist installer\kto.iss
; where `dist\` holds kto.exe, kto-gui.exe, icon.ico, README.md, LICENSE, CHANGELOG.md.
; CI passes MyAppVersion and an absolute SourceDir. Output lands in
; installer\installer-out\.

#define MyAppName "KTO"
#define MyAppPublisher "Londopy"
#define MyAppURL "https://github.com/Londopy/kto"
#define MyAppExeName "kto.exe"

#ifndef MyAppVersion
  #define MyAppVersion "0.0.0"
#endif
#ifndef SourceDir
  #define SourceDir "dist"
#endif

[Setup]
; Keep this AppId constant across releases so upgrades stay clean.
AppId={{8F2C2D1E-7B4A-49C5-9E0A-2D3F1A6B7C84}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}/issues
AppUpdatesURL={#MyAppURL}/releases
DefaultDirName={autopf}\KTO
DefaultGroupName=KTO
DisableProgramGroupPage=yes
; Shows the MIT license from the repo as a page you must accept.
LicenseFile={#SourceDir}\LICENSE
; The installer's own icon (wizard + Add/Remove Programs).
SetupIconFile={#SourceDir}\icon.ico
OutputDir=installer-out
OutputBaseFilename=kto-{#MyAppVersion}-setup-x64
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
ChangesEnvironment=yes
UninstallDisplayIcon={app}\{#MyAppExeName}
UninstallDisplayName={#MyAppName} {#MyAppVersion}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Shortcuts:"
Name: "addtopath";   Description: "Add KTO to the system PATH (recommended)"; GroupDescription: "Integration:"

[Files]
Source: "{#SourceDir}\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\kto-gui.exe";     DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\icon.ico";         DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\README.md";        DestDir: "{app}"; Flags: ignoreversion isreadme
Source: "{#SourceDir}\LICENSE";          DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\CHANGELOG.md";     DestDir: "{app}"; Flags: ignoreversion

[Icons]
; The GUI is the main entry; a console shortcut is there for CLI fans.
Name: "{group}\KTO";           Filename: "{app}\kto-gui.exe"; WorkingDir: "{app}"; Comment: "KTO control panel"
Name: "{group}\KTO console";   Filename: "{cmd}"; Parameters: "/k ""{app}\{#MyAppExeName}"" --help"; WorkingDir: "{app}"; IconFilename: "{app}\icon.ico"; Comment: "Open a KTO console"
Name: "{group}\KTO on GitHub"; Filename: "{#MyAppURL}"; IconFilename: "{app}\icon.ico"
Name: "{group}\Uninstall KTO"; Filename: "{uninstallexe}"
Name: "{autodesktop}\KTO";     Filename: "{app}\kto-gui.exe"; WorkingDir: "{app}"; Comment: "KTO control panel"; Tasks: desktopicon

[Run]
; Optional checkboxes on the final page.
Filename: "{app}\kto-gui.exe"; Description: "Launch KTO"; Flags: postinstall nowait skipifsilent
Filename: "{cmd}"; Parameters: "/k ""{app}\{#MyAppExeName}"" --help"; Description: "Open a KTO console now"; Flags: postinstall shellexec skipifsilent nowait unchecked
Filename: "{#MyAppURL}"; Description: "Visit the KTO project on GitHub"; Flags: postinstall shellexec skipifsilent nowait unchecked

[Code]
const
  EnvironmentKey = 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment';

{ True if the app dir isn't already on the system PATH. }
function NeedsAddPath(Param: string): Boolean;
var
  OrigPath: string;
begin
  if not RegQueryStringValue(HKEY_LOCAL_MACHINE, EnvironmentKey, 'Path', OrigPath) then
  begin
    Result := True;
    exit;
  end;
  Result := Pos(';' + Uppercase(Param) + ';', ';' + Uppercase(OrigPath) + ';') = 0;
end;

procedure AddToPath();
var
  OrigPath: string;
begin
  if not WizardIsTaskSelected('addtopath') then
    exit;
  if not NeedsAddPath(ExpandConstant('{app}')) then
    exit;
  if not RegQueryStringValue(HKEY_LOCAL_MACHINE, EnvironmentKey, 'Path', OrigPath) then
    OrigPath := '';
  if (OrigPath <> '') and (Copy(OrigPath, Length(OrigPath), 1) <> ';') then
    OrigPath := OrigPath + ';';
  RegWriteStringValue(HKEY_LOCAL_MACHINE, EnvironmentKey, 'Path',
    OrigPath + ExpandConstant('{app}'));
end;

procedure RemoveFromPath();
var
  OrigPath: string;
  AppDir: string;
  P: Integer;
begin
  if not RegQueryStringValue(HKEY_LOCAL_MACHINE, EnvironmentKey, 'Path', OrigPath) then
    exit;
  AppDir := ExpandConstant('{app}');
  P := Pos(Uppercase(';' + AppDir), Uppercase(OrigPath));
  if P > 0 then
    Delete(OrigPath, P, Length(AppDir) + 1)
  else
  begin
    P := Pos(Uppercase(AppDir + ';'), Uppercase(OrigPath));
    if P > 0 then
      Delete(OrigPath, P, Length(AppDir) + 1)
    else
    begin
      P := Pos(Uppercase(AppDir), Uppercase(OrigPath));
      if P > 0 then
        Delete(OrigPath, P, Length(AppDir));
    end;
  end;
  RegWriteStringValue(HKEY_LOCAL_MACHINE, EnvironmentKey, 'Path', OrigPath);
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
    AddToPath();
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usUninstall then
    RemoveFromPath();
end;
