#ifndef MyAppVersion
  #define MyAppVersion "1.0.2"
#endif

[Setup]
AppId={{8F6E2A14-4C9B-4D7E-A912-6E3F0C8D2B51}
AppName=NG Helper
AppVersion={#MyAppVersion}
AppPublisher=Frezze98
AppPublisherURL=https://github.com/Frezze98/ng-helper
DefaultDirName={autopf}\NG Helper
DefaultGroupName=NG Helper
OutputDir=..\dist
OutputBaseFilename=Setup-NG-Helper
SetupIconFile=icon.ico
Compression=lzma2
SolidCompression=yes
PrivilegesRequired=lowest
WizardStyle=modern
CloseApplications=force
UninstallDisplayIcon={app}\icon.ico

[Languages]
Name: "ukrainian"; MessagesFile: "compiler:Languages\Ukrainian.isl"
Name: "russian"; MessagesFile: "compiler:Languages\Russian.isl"

[CustomMessages]
ukrainian.CreateDesktopIcon=Створити ярлик на робочому столі
russian.CreateDesktopIcon=Создать ярлык на рабочем столе
ukrainian.AutoStartApp=Запускати NG Helper разом із Windows
russian.AutoStartApp=Запускать NG Helper вместе с Windows
ukrainian.LaunchApp=Запустити NG Helper
russian.LaunchApp=Запустить NG Helper

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "autostart"; Description: "{cm:AutoStartApp}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\overlay\target\release\ng-helper.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "icon.ico"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\NG Helper"; Filename: "{app}\ng-helper.exe"; IconFilename: "{app}\icon.ico"
Name: "{group}\Видалити NG Helper"; Filename: "{uninstallexe}"
Name: "{autodesktop}\NG Helper"; Filename: "{app}\ng-helper.exe"; Tasks: desktopicon; IconFilename: "{app}\icon.ico"

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "NG Helper"; ValueData: """{app}\ng-helper.exe"""; Flags: uninsdeletevalue; Tasks: autostart

[Run]
Filename: "{app}\ng-helper.exe"; Description: "{cm:LaunchApp}"; Flags: nowait postinstall skipifsilent
; после тихого автообновления приложение должно запуститься снова
Filename: "{app}\ng-helper.exe"; Flags: nowait; Check: WizardSilent