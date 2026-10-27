; Compile through build-installer.ps1 so the binary and version come from Cargo.
#ifndef AppVersion
  #error Run scripts/build-installer.ps1 to supply AppVersion.
#endif
#ifndef AppVersionNumeric
  #error Run scripts/build-installer.ps1 to supply AppVersionNumeric.
#endif
#ifndef AppBinary
  #error Run scripts/build-installer.ps1 to supply AppBinary.
#endif
#ifndef InstallerOutput
  #error Run scripts/build-installer.ps1 to supply InstallerOutput.
#endif

[Setup]
; Keep this identity stable across versions so upgrades share one uninstaller.
AppId=CagePlate
AppName=CagePlate
AppVersion={#AppVersion}
VersionInfoVersion={#AppVersionNumeric}
DefaultDirName={localappdata}\Programs\CagePlate
PrivilegesRequired=lowest
DisableProgramGroupPage=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
SourceDir=..
OutputDir={#InstallerOutput}
OutputBaseFilename=CagePlate-{#AppVersion}-Setup
Compression=lzma2
SolidCompression=yes
UninstallDisplayIcon={app}\cageplate.exe
CloseApplications=yes
RestartApplications=no

[Files]
Source: "{#AppBinary}"; DestDir: "{app}"; Flags: ignoreversion
Source: "assets\fonts\UFL.txt"; DestDir: "{app}\licenses\ubuntu"; Flags: ignoreversion
Source: "assets\fonts\README.md"; DestDir: "{app}\licenses\ubuntu"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\CagePlate"; Filename: "{app}\cageplate.exe"

[Run]
Filename: "{app}\cageplate.exe"; Description: "Launch CagePlate"; Flags: nowait postinstall skipifsilent

; Deliberately do not install defaults.json or delete user settings on uninstall.
; All files are explicitly listed: no clipboard data, fixtures, or CLI are bundled.
