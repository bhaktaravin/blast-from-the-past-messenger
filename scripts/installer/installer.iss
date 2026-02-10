[Setup]
AppId={{E3E0F0C5-7C1E-4B4F-9F7C-9B6B5B9E5D01}
AppName=Blast From The Past Messenger
AppVersion=0.1.0
AppPublisher=Blast From The Past
DefaultDirName={autopf}\Blast From The Past Messenger
DefaultGroupName=Blast From The Past Messenger
DisableProgramGroupPage=yes
OutputDir=dist
OutputBaseFilename=blast-from-the-past-messenger-setup
Compression=lzma
SolidCompression=yes

[Files]
Source: "target\release\chatmessagediscordclone.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Blast From The Past Messenger"; Filename: "{app}\chatmessagediscordclone.exe"
Name: "{autodesktop}\Blast From The Past Messenger"; Filename: "{app}\chatmessagediscordclone.exe"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a desktop icon"; GroupDescription: "Additional icons:"; Flags: unchecked

[Run]
Filename: "{app}\chatmessagediscordclone.exe"; Description: "Launch Blast From The Past Messenger"; Flags: nowait postinstall skipifsilent
