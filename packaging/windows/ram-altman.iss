; Inno Setup script for RAM Altman (Windows installer).
; Build:  iscc /DAppVersion=0.1.0 packaging\windows\ram-altman.iss
; Output: dist\RAM-Altman-Setup-<version>.exe
;
; Installs per-user (no admin prompt), adds Start menu entry, optional desktop
; shortcut, optional "start with Windows", and a normal uninstaller.

#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif
#define AppName "RAM Altman"
#define AppExe "ram-altman.exe"
#define Root "..\.."

[Setup]
AppId={{56B768FF-0BB0-4DF2-B65C-FF1958A21949}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppComments=Memory Is All You Need
DefaultDirName={localappdata}\Programs\RAM Altman
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
OutputDir={#Root}\dist
OutputBaseFilename=RAM-Altman-Setup-{#AppVersion}
LicenseFile={#Root}\LICENSE
SetupIconFile={#Root}\assets\icon.ico
UninstallDisplayIcon={app}\{#AppExe}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
CloseApplications=yes

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop shortcut"; GroupDescription: "Shortcuts:"
Name: "autostart"; Description: "Start RAM Altman when I sign in to Windows"; GroupDescription: "Startup:"; Flags: unchecked

[Files]
Source: "{#Root}\target\release\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{group}\Uninstall {#AppName}"; Filename: "{uninstallexe}"
Name: "{userdesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "RAM Altman"; ValueData: """{app}\{#AppExe}"""; Tasks: autostart; Flags: uninsdeletevalue

[Run]
Filename: "{app}\{#AppExe}"; Description: "Launch {#AppName} now"; Flags: nowait postinstall skipifsilent
