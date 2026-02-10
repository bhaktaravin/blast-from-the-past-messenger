#define AppName "Blast From The Past Messenger"
#define AppVersion "0.1.0"
#define AppExe "chatmessagediscordclone.exe"
#define RootDir "..\.."

[Setup]
AppId={{E3E0F0C5-7C1E-4B4F-9F7C-9B6B5B9E5D01}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=Blast From The Past
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
OutputDir={#RootDir}\dist
OutputBaseFilename=blast-from-the-past-messenger-setup
Compression=lzma
SolidCompression=yes

[Files]
Source: "{#RootDir}\target\release\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a desktop icon"; GroupDescription: "Additional icons:"; Flags: unchecked

[Run]
Filename: "{app}\{#AppExe}"; Description: "Launch {#AppName}"; Flags: nowait postinstall skipifsilent
