# Development uninstaller for install.ps1. End users uninstall from Settings > Apps.
# Elevates itself (UAC) when not run as administrator.
$ErrorActionPreference = 'Stop'

$Dest = Join-Path $env:ProgramFiles 'HangulIME'

$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
    [Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    $p = Start-Process powershell.exe -Verb RunAs -Wait -PassThru -ArgumentList @(
        '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$PSCommandPath`"")
    exit $p.ExitCode
}

# Remove from the language list (keeps the Korean language and other IMEs).
& (Join-Path $PSScriptRoot 'installer\langlist.ps1') -Remove

foreach ($b in @(
        @{ Arch = 'x64'; RegSvr = "$env:WINDIR\System32\regsvr32.exe" },
        @{ Arch = 'x86'; RegSvr = "$env:WINDIR\SysWOW64\regsvr32.exe" })) {
    $dll = Join-Path $Dest "$($b.Arch)\hangul_ime.dll"
    if (Test-Path $dll) {
        Start-Process $b.RegSvr -ArgumentList @('/u', '/s', "`"$dll`"") -Wait | Out-Null
    }
}

try {
    Remove-Item $Dest -Recurse -Force -ErrorAction Stop
} catch {
    Write-Host "Some files are still in use; delete $Dest after signing out or rebooting." -ForegroundColor Yellow
}
Write-Host 'Uninstalled.' -ForegroundColor Green
