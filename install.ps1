# Development installer: installs the DLLs built in this repo. Elevates itself (UAC) when not run as administrator.
# End users install with the setup .exe from Releases (installer\HangulIME.iss) instead.
# Build first:
#   cargo build --release -p hangul-tip --target x86_64-pc-windows-msvc
#   cargo build --release -p hangul-tip --target i686-pc-windows-msvc
$ErrorActionPreference = 'Stop'

$Root = $PSScriptRoot
$Dest = Join-Path $env:ProgramFiles 'HangulIME'

$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
    [Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    $p = Start-Process powershell.exe -Verb RunAs -Wait -PassThru -ArgumentList @(
        '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$PSCommandPath`"")
    exit $p.ExitCode
}

$builds = @(
    @{ Arch = 'x64'; Src = "$Root\target\x86_64-pc-windows-msvc\release\hangul_ime.dll"; RegSvr = "$env:WINDIR\System32\regsvr32.exe" },
    @{ Arch = 'x86'; Src = "$Root\target\i686-pc-windows-msvc\release\hangul_ime.dll"; RegSvr = "$env:WINDIR\SysWOW64\regsvr32.exe" }
)

foreach ($b in $builds) {
    if (-not (Test-Path $b.Src)) { throw "Build output not found: $($b.Src)" }
    $dir = Join-Path $Dest $b.Arch
    New-Item -ItemType Directory -Force $dir | Out-Null
    $dll = Join-Path $dir 'hangul_ime.dll'

    # A DLL that is loaded in running apps cannot be overwritten, but it can be renamed.
    Get-ChildItem $dir -Filter '*.old' -ErrorAction SilentlyContinue |
        ForEach-Object { try { Remove-Item $_.FullName -Force -ErrorAction Stop } catch {} }
    if (Test-Path $dll) {
        try { Remove-Item $dll -Force -ErrorAction Stop }
        catch { Rename-Item $dll ("hangul_ime.{0}.old" -f (Get-Date -Format 'yyyyMMddHHmmss')) }
    }
    Copy-Item $b.Src $dll

    $p = Start-Process $b.RegSvr -ArgumentList @('/s', "`"$dll`"") -Wait -PassThru
    if ($p.ExitCode -ne 0) { throw "regsvr32 ($($b.Arch)) failed with exit code $($p.ExitCode)" }
    Write-Host "Registered $($b.Arch): $dll"
}

# Add the IME to the user's Korean language entry.
& (Join-Path $Root 'installer\langlist.ps1') -Add

Write-Host ''
Write-Host 'Installed. Switch to "Hangul IME" with Win+Space.' -ForegroundColor Green
