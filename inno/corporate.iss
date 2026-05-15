; TR-300 — Corporate Edition installer (perUser, no admin required).
;
; Built by .github/workflows/windows-installers.yml after release.yml finishes
; publishing the GitHub Release. Companion to inno/global.iss (perMachine
; sibling). The MSI sibling lives at wix/corporate.wxs.
;
; Builds with Inno Setup 6 (`iscc` from JRSoftware) on a Windows runner. CI
; passes the version via `iscc /DMyAppVersion=3.15.0`.
;
; All four installers (MSI Global, MSI Corporate, EXE Global, EXE Corporate)
; target only TWO actual install paths — one per edition. The Corporate
; edition (both MSI and EXE) installs to
;     %LocalAppData%\Programs\tr300\bin\tr300.exe
; and modifies USER PATH (HKCU\Environment\Path), not system PATH. README
; documents "pick one format per edition" since coexistence creates duplicate
; Add/Remove Programs entries.

#ifndef MyAppVersion
  #define MyAppVersion "0.0.0-dev"
#endif

#define MyAppName "tr300"
#define MyAppFullName "tr300 (Corporate Edition)"
#define MyAppPublisher "Emmett S"
#define MyAppURL "https://github.com/QubeTX/qube-machine-report"
#define MyAppExeName "tr300.exe"

[Setup]
; AppId is the immutable identity of the Corporate EXE installer.
; Different GUID from both the Corporate MSI's UpgradeCode and the Global
; EXE's AppId so the four installer products are distinct to Windows.
AppId={{76A253EB-3A17-4730-9C54-5BE755A9BC4C}
AppName={#MyAppFullName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppFullName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}/releases
; perUser install location: %LocalAppData%\Programs\tr300 — same path as the
; Corporate MSI by design. {userpf} is Inno Setup's per-user "Program Files"
; equivalent and resolves to %LocalAppData%\Programs.
DefaultDirName={userpf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
DisableDirPage=auto
; The core perUser switch: lowest privileges, no admin elevation, no UAC.
; PrivilegesRequiredOverridesAllowed= prevents Inno from offering the user
; the choice to elevate (we deliberately install per-user only).
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
OutputBaseFilename=tr300-x86_64-pc-windows-msvc-corporate-setup
OutputDir=Output
Compression=lzma
SolidCompression=yes
WizardStyle=modern
ChangesEnvironment=yes
; ARP display name. Matches the Corporate MSI's Product Name so the two
; installer formats show consistent labels.
UninstallDisplayName={#MyAppFullName}
LicenseFile=..\LICENSE
SetupLogging=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Files]
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}\bin"; Flags: ignoreversion

[Registry]
; Install-source marker. tr300 update reads HKCU\Software\TR300\InstallSource
; and picks the matching installer to download for in-place upgrades. Value
; must match the `exe-corporate` arm in src/update.rs.
Root: HKCU; Subkey: "Software\TR300"; ValueType: string; ValueName: "InstallSource"; ValueData: "exe-corporate"; Flags: uninsdeletevalue
Root: HKCU; Subkey: "Software\TR300"; Flags: uninsdeletekeyifempty

[Code]
{
  PATH management — user PATH (HKCU\Environment\Path) for the Corporate
  perUser edition. Same canonical pattern as inno/global.iss but pointing
  at the HKCU\Environment key instead of HKLM\...\Session Manager\Environment.
}
const
  EnvironmentKey = 'Environment';

procedure EnvAddPath(Path: string);
var
  Paths: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', Paths) then
    Paths := '';

  // Skip if already in PATH (case-insensitive substring match with
  // ;-padding so we don't match a prefix of a different directory).
  if Pos(';' + Uppercase(Path) + ';', ';' + Uppercase(Paths) + ';') > 0 then exit;

  if Length(Paths) > 0 then
    Paths := Paths + ';' + Path
  else
    Paths := Path;

  RegWriteExpandStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', Paths);
end;

procedure EnvRemovePath(Path: string);
var
  Paths: string;
  P: Integer;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', Paths) then
    exit;

  P := Pos(';' + Uppercase(Path) + ';', ';' + Uppercase(Paths) + ';');
  if P = 0 then exit;

  Delete(Paths, P - 1, Length(Path) + 1);

  RegWriteExpandStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', Paths);
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
    EnvAddPath(ExpandConstant('{app}') + '\bin');
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
    EnvRemovePath(ExpandConstant('{app}') + '\bin');
end;
