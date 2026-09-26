#define AppName "Blast From The Past Messenger"
#define AppExe "chatmessagediscordclone.exe"
; CI passes the release tag's version: iscc /DAppVersion=1.3.0 installer.iss
#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

#define EnvRoot GetEnv("GITHUB_WORKSPACE")
#if EnvRoot != ""
  #define RootDir EnvRoot + "\\"
#else
  #define RootDir SourcePath + "..\\..\\"
#endif

[Setup]
AppId={{E3E0F0C5-7C1E-4B4F-9F7C-9B6B5B9E5D01}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher=Blast From The Past
AppPublisherURL=https://github.com/bhaktaravin/blast-from-the-past-messenger
VersionInfoVersion={#AppVersion}
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
; The app is a 64-bit build: install into "Program Files", not "Program Files (x86)"
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#RootDir}dist
OutputBaseFilename=blast-from-the-past-messenger-setup
Compression=lzma
SolidCompression=yes
SetupIconFile={#RootDir}assets\icons\icon.ico
UninstallDisplayIcon={app}\icon.ico
; Updating while the app is open: ask to close it rather than failing on a locked exe
CloseApplications=yes

[Files]
Source: "{#RootDir}target\release\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#RootDir}assets\icons\icon.ico"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"; IconFilename: "{app}\icon.ico"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; IconFilename: "{app}\icon.ico"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a desktop icon"; GroupDescription: "Additional icons:"; Flags: unchecked

[Run]
Filename: "{app}\{#AppExe}"; Description: "Launch {#AppName}"; Flags: nowait postinstall skipifsilent
