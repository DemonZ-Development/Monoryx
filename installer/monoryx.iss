#ifndef AppVersion
  #define AppVersion "1.3.0"
#endif
#define AppName "MONORYX"
#define AppPublisher "DemonZDevelopment"
#define AppURL "https://github.com/DemonZ-Development/Monoryx"
#define AppExe "monoryx.exe"
#define AppDataDir "{userappdata}\DemonZDevelopment\MONORYX"

[Setup]
AppId={{A6CF77C4-A848-4D19-B0BD-3713A4A565EA}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} v{#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppURL}
AppSupportURL={#AppURL}
AppUpdatesURL={#AppURL}/releases
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputDir=dist
OutputBaseFilename=MONORYX-Setup-{#AppVersion}
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
CloseApplications=force
CloseApplicationsFilter={#AppExe}
RestartApplications=no
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
LicenseFile=..\LICENSE
SetupIconFile=..\assets\icon.ico
UninstallDisplayIcon={app}\{#AppExe}
UninstallDisplayName={#AppName} v{#AppVersion}

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
Root: HKA; Subkey: "Software\Classes\.mrpack"; ValueType: string; ValueName: ""; ValueData: "MonoryxModpack"; Flags: uninsdeletevalue; Tasks: associate_mrpack
Root: HKA; Subkey: "Software\Classes\.mrpack\OpenWithProgids"; ValueType: string; ValueName: "MonoryxModpack"; ValueData: ""; Flags: uninsdeletevalue; Tasks: associate_mrpack
Root: HKA; Subkey: "Software\Classes\MonoryxModpack"; ValueType: string; ValueName: ""; ValueData: "Modrinth Modpack"; Flags: uninsdeletekey; Tasks: associate_mrpack
Root: HKA; Subkey: "Software\Classes\MonoryxModpack\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#AppExe},0"; Tasks: associate_mrpack
Root: HKA; Subkey: "Software\Classes\MonoryxModpack\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExe}"" ""%1"""; Tasks: associate_mrpack

Root: HKA; Subkey: "Software\Classes\monoryx"; ValueType: string; ValueName: ""; ValueData: "URL:MONORYX Protocol"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\monoryx"; ValueType: string; ValueName: "URL Protocol"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\monoryx\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#AppExe},0"
Root: HKA; Subkey: "Software\Classes\monoryx\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExe}"" ""%1"""

[Run]
Filename: "{app}\{#AppExe}"; Description: "{cm:LaunchProgram,{#StringChange(AppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Code]
var
  RemoveSavedData: Boolean;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  ResultCode: Integer;
begin
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM {#AppExe}', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Result := '';
end;

function TreeSize(const Root: String): Int64;
var
  R: TFindRec;
begin
  Result := 0;
  if not DirExists(Root) then
    Exit;
  if FindFirst(Root + '\*', R) then
  begin
    try
      repeat
        if (R.Name <> '.') and (R.Name <> '..') then
        begin
          if (R.Attributes and FILE_ATTRIBUTE_DIRECTORY) <> 0 then
            Result := Result + TreeSize(Root + '\' + R.Name)
          else
            Result := Result + (Int64(R.SizeHigh) * 4294967296) + Int64(R.SizeLow);
        end;
      until not FindNext(R);
    finally
      FindClose(R);
    end;
  end;
end;

function HumanSize(Bytes: Int64): String;
begin
  if Bytes >= 1073741824 then
    Result := Format('%.1f GB', [Bytes / 1073741824.0])
  else if Bytes >= 1048576 then
    Result := Format('%.0f MB', [Bytes / 1048576.0])
  else if Bytes >= 1024 then
    Result := Format('%.0f KB', [Bytes / 1024.0])
  else
    Result := IntToStr(Bytes) + ' bytes';
end;

function InitializeUninstall(): Boolean;
var
  ResultCode: Integer;
  Answer: Integer;
  Saved: Int64;
  SavedText: String;
begin
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM {#AppExe}', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  RemoveSavedData := False;

  Saved := TreeSize(ExpandConstant('{#AppDataDir}'));
  if Saved > 0 then
    SavedText := 'It currently holds ' + HumanSize(Saved) + ' of your data.'
  else
    SavedText := 'No saved data was found.';

  Answer := MsgBox(
    'Uninstall {#AppName}' + #13#10 + #13#10 +
    'Do you also want to delete everything {#AppName} has saved?' + #13#10 + #13#10 +
    SavedText + #13#10 + #13#10 +
    'This permanently deletes:' + #13#10 +
    '    every instance and its mods' + #13#10 +
    '    every world, including your world backups' + #13#10 +
    '    screenshots' + #13#10 +
    '    downloaded Minecraft, Java and assets' + #13#10 +
    '    your settings and account' + #13#10 + #13#10 +
    'Worlds and world backups are NOT moved anywhere else first. There is no undo.' + #13#10 + #13#10 +
    'Yes    delete it all' + #13#10 +
    'No     keep my data' + #13#10 +
    'Cancel stop the uninstall',
    mbConfirmation, MB_YESNOCANCEL);

  if Answer = IDCANCEL then
  begin
    Result := False;
    Exit;
  end;
  RemoveSavedData := (Answer = IDYES);
  Result := True;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  DataDir: String;
begin
  if (CurUninstallStep = usPostUninstall) and RemoveSavedData then
  begin
    DataDir := ExpandConstant('{#AppDataDir}');
    if DirExists(DataDir) then
      DelTree(DataDir, True, True, True);
  end;
end;
