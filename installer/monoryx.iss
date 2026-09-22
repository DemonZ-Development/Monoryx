#ifndef AppVersion
  #define AppVersion "1.0.0-beta"
#endif
#define AppName "MONORYX"
#define AppPublisher "DemonZDevelopment"
#define AppURL "https://github.com/DemonZDevelopment/monoryx"
#define AppExe "monoryx.exe"

[Setup]
AppId={{A6CF77C4-A848-4D19-B0BD-3713A4A565EA}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} v1.0.0 Beta
AppPublisher={#AppPublisher}
AppPublisherURL={#AppURL}
AppSupportURL={#AppURL}
AppUpdatesURL={#AppURL}/releases
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputDir=dist
OutputBaseFilename=MONORYX-Setup-v1.0.0-Beta
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
WizardResizable=no
CloseApplications=yes
RestartApplications=no
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
LicenseFile=..\LICENSE
SetupIconFile=..\assets\icon.ico
UninstallDisplayIcon={app}\{#AppExe}
UninstallDisplayName={#AppName} v1.0.0 Beta

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: checkedonce
Name: "startupicon"; Description: "Start {#AppName} automatically when I sign in"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "associate_mrpack"; Description: "Associate .mrpack (Modrinth Modpack) files with {#AppName}"; GroupDescription: "File Associations:"

[Files]
Source: "..\target\release\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\assets\icon.ico"; DestDir: "{app}\assets"; Flags: ignoreversion

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExe}"; IconFilename: "{app}\{#AppExe}"
Name: "{group}\Uninstall {#AppName}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon; IconFilename: "{app}\{#AppExe}"
Name: "{userstartup}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: startupicon; IconFilename: "{app}\{#AppExe}"

[Registry]
; File association for .mrpack
Root: HKA; Subkey: "Software\Classes\.mrpack"; ValueType: string; ValueName: ""; ValueData: "MonoryxModpack"; Flags: uninsdeletevalue; Tasks: associate_mrpack
Root: HKA; Subkey: "Software\Classes\.mrpack\OpenWithProgids"; ValueType: string; ValueName: "MonoryxModpack"; ValueData: ""; Flags: uninsdeletevalue; Tasks: associate_mrpack
Root: HKA; Subkey: "Software\Classes\MonoryxModpack"; ValueType: string; ValueName: ""; ValueData: "Modrinth Modpack"; Flags: uninsdeletekey; Tasks: associate_mrpack
Root: HKA; Subkey: "Software\Classes\MonoryxModpack\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#AppExe},0"; Tasks: associate_mrpack
Root: HKA; Subkey: "Software\Classes\MonoryxModpack\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExe}"" ""%1"""; Tasks: associate_mrpack

; Custom URL protocol scheme monoryx://
Root: HKA; Subkey: "Software\Classes\monoryx"; ValueType: string; ValueName: ""; ValueData: "URL:MONORYX Protocol"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\monoryx"; ValueType: string; ValueName: "URL Protocol"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\monoryx\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#AppExe},0"
Root: HKA; Subkey: "Software\Classes\monoryx\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExe}"" ""%1"""

[Run]
Filename: "{app}\{#AppExe}"; Description: "{cm:LaunchProgram,{#StringChange(AppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent
