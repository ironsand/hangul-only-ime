# Hangul IME installer. Run from the repo root after building:
#   cargo build --release -p hangul-tip --target x86_64-pc-windows-msvc
#   cargo build --release -p hangul-tip --target i686-pc-windows-msvc
# Elevates itself (UAC) when not run as administrator.
$ErrorActionPreference = 'Stop'

$Tip = '0412:{17432EFE-970E-4BA4-A40A-014C6C458922}{6E2BAEEB-AA1C-4529-95D9-E8ACB1B58162}'
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
$list = Get-WinUserLanguageList
$ko = $list | Where-Object { $_.LanguageTag -eq 'ko' -or $_.LanguageTag -eq 'ko-KR' } | Select-Object -First 1
if (-not $ko) {
    $list.Add('ko')
    $ko = $list[$list.Count - 1]
    $ko.InputMethodTips.Clear()
}
if (-not $ko.InputMethodTips.Contains($Tip)) { $ko.InputMethodTips.Add($Tip) }
Set-WinUserLanguageList $list -Force

Write-Host ''
Write-Host 'Installed. Switch to "Hangul IME" with Win+Space.' -ForegroundColor Green
