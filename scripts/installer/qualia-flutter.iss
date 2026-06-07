; Inno Setup script — QualiaDB Flutter desktop (Windows x64)
; Build: iscc /DAppVersion=1.0.0 scripts\installer\qualia-flutter.iss

#ifndef AppVersion
  #define AppVersion "0.0.8"
#endif

#ifndef StageDir
  #define StageDir "..\..\dist\qualia-flutter-windows-x64"
#endif

[Setup]
AppId={{A7B3C9D1-4E2F-4A8B-9C1D-QualiaFlutter01}
AppName=QualiaDB
AppVersion={#AppVersion}
AppPublisher=QualiaDB
DefaultDirName={autopf}\QualiaDB
DefaultGroupName=QualiaDB
OutputDir=..\..\dist
OutputBaseFilename=QualiaDB-Setup-{#AppVersion}-x64
Compression=lzma2/ultra64
SolidCompression=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
WizardStyle=modern
LicenseFile=
InfoAfterFile=
UninstallDisplayIcon={app}\qualia_flutter.exe

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop shortcut"; GroupDescription: "Additional icons:"

[Files]
Source: "{#StageDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\QualiaDB"; Filename: "{app}\qualia_flutter.exe"
Name: "{group}\Uninstall QualiaDB"; Filename: "{uninstallexe}"
Name: "{autodesktop}\QualiaDB"; Filename: "{app}\qualia_flutter.exe"; Tasks: desktopicon

[Run]
; WebView2 Evergreen Bootstrapper (required for in-app WebViews)
Filename: "https://go.microsoft.com/fwlink/p/?LinkId=2124703"; \
  Description: "Install Microsoft WebView2 Runtime (required)"; \
  Flags: shellexec postinstall skipifsilent
; Visual C++ 2015–2022 x64 redistributable
Filename: "https://aka.ms/vs/17/release/vc_redist.x64.exe"; \
  Description: "Install Visual C++ Redistributable (recommended)"; \
  Parameters: "/install /quiet /norestart"; \
  Flags: shellexec postinstall skipifsilent

[Code]
function InitializeSetup: Boolean;
begin
  Result := True;
end;
