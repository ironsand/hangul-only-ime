; Inno Setup 6 script for Hangul IME.
; Build the DLLs first (see docs/DEVELOPMENT.md), then:
;   ISCC.exe /DAppVersion=0.1.0 installer\HangulIME.iss
; Output: dist\HangulIME-Setup-<version>.exe

#ifndef AppVersion
  #define AppVersion "0.0.0-dev"
#endif
#define BuildDir "..\target"
; Windows file versions must be numeric: "0.2.0-beta" -> "0.2.0".
#define FileVersion Copy(AppVersion, 1, Pos("-", AppVersion + "-") - 1)

[Setup]
; Never change AppId: upgrades and uninstall are keyed on it.
AppId={{F07788B5-E188-4B4E-B08B-29B3D4AC5A63}
AppName=Hangul IME
AppVerName=Hangul IME {#AppVersion}
AppVersion={#AppVersion}
AppPublisher=ironsand
AppPublisherURL=https://github.com/ironsand/hangul-only-ime
AppSupportURL=https://github.com/ironsand/hangul-only-ime/issues
VersionInfoVersion={#FileVersion}
DefaultDirName={autopf}\HangulIME
DisableDirPage=yes
DisableProgramGroupPage=yes
PrivilegesRequired=admin
ArchitecturesAllowed=x64os
ArchitecturesInstallIn64BitMode=x64os
; The IME DLL is loaded by nearly every app; never ask to close them (it is renamed instead, see [Code]).
CloseApplications=no
RestartApplications=no
LicenseFile=..\LICENSE
OutputDir=..\dist
OutputBaseFilename=HangulIME-Setup-{#AppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayName=Hangul IME (두벌식)
UninstallDisplayIcon={sys}\input.dll,0

[Languages]
Name: "en"; MessagesFile: "compiler:Default.isl"; InfoBeforeFile: "info-en.txt"
Name: "ja"; MessagesFile: "compiler:Languages\Japanese.isl"; InfoBeforeFile: "info-ja.txt"
Name: "ko"; MessagesFile: "compiler:Languages\Korean.isl"; InfoBeforeFile: "info-ko.txt"

[Files]
Source: "{#BuildDir}\x86_64-pc-windows-msvc\release\hangul_ime.dll"; DestDir: "{app}\x64"; Flags: ignoreversion regserver 64bit
Source: "{#BuildDir}\i686-pc-windows-msvc\release\hangul_ime.dll"; DestDir: "{app}\x86"; Flags: ignoreversion regserver 32bit
Source: "langlist.ps1"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Run]
; The language list is per user, so run as the user who started setup (not the elevated admin).
Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; \
  Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\langlist.ps1"" -Add"; \
  Flags: runhidden runasoriginaluser waituntilterminated

[UninstallRun]
Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; \
  Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\langlist.ps1"" -Remove"; \
  Flags: runhidden waituntilterminated; RunOnceId: "RemoveFromLanguageList"

[Code]
const
  MOVEFILE_DELAY_UNTIL_REBOOT = 4;

function MoveFileExW(lpExistingFileName: String; lpNewFileName: Cardinal; dwFlags: Cardinal): Boolean;
  external 'MoveFileExW@kernel32.dll stdcall';

{ A DLL loaded by running apps cannot be deleted or overwritten, but it can be renamed.
  Move it aside so the new version can be copied; apps pick up the new DLL when restarted. }
procedure MoveAsideInUse(const Dir: String);
var
  FindRec: TFindRec;
  Dll: String;
begin
  if FindFirst(Dir + '\*.old', FindRec) then
  begin
    try
      repeat
        DeleteFile(Dir + '\' + FindRec.Name);
      until not FindNext(FindRec);
    finally
      FindClose(FindRec);
    end;
  end;
  Dll := Dir + '\hangul_ime.dll';
  if FileExists(Dll) and not DeleteFile(Dll) then
    RenameFile(Dll, Dir + '\hangul_ime.' + GetDateTimeString('yyyymmddhhnnss', #0, #0) + '.old');
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  MoveAsideInUse(ExpandConstant('{app}\x64'));
  MoveAsideInUse(ExpandConstant('{app}\x86'));
  Result := '';
end;

{ Uninstall: files still loaded by running apps cannot be deleted. Rename them to a unique name (so a reinstall
  before the next restart is not affected) and let Windows delete them at the next restart. }
procedure DeleteNowOrAtReboot(const Dir: String);
var
  FindRec: TFindRec;
  Path, Pending: String;
  N: Integer;
begin
  N := 0;
  if FindFirst(Dir + '\*', FindRec) then
  begin
    try
      repeat
        if FindRec.Attributes and FILE_ATTRIBUTE_DIRECTORY = 0 then
        begin
          Path := Dir + '\' + FindRec.Name;
          if not DeleteFile(Path) then
          begin
            N := N + 1;
            Pending := Dir + '\uninstalled.' + GetDateTimeString('yyyymmddhhnnss', #0, #0) + '.' + IntToStr(N) + '.del';
            if not RenameFile(Path, Pending) then
              Pending := Path;
            MoveFileExW(Pending, 0, MOVEFILE_DELAY_UNTIL_REBOOT);
          end;
        end;
      until not FindNext(FindRec);
    finally
      FindClose(FindRec);
    end;
  end;
  if DirExists(Dir) and not RemoveDir(Dir) then
    MoveFileExW(Dir, 0, MOVEFILE_DELAY_UNTIL_REBOOT);
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
  begin
    DeleteNowOrAtReboot(ExpandConstant('{app}\x64'));
    DeleteNowOrAtReboot(ExpandConstant('{app}\x86'));
    { Queued after its subfolders, so it is removed at restart once they are gone (fails harmlessly if not empty). }
    if DirExists(ExpandConstant('{app}')) then
      MoveFileExW(ExpandConstant('{app}'), 0, MOVEFILE_DELAY_UNTIL_REBOOT);
  end;
end;
