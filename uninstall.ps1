# Hangul IME uninstaller. Elevates itself (UAC) when not run as administrator.
$ErrorActionPreference = 'Stop'

$Tip = '0412:{17432EFE-970E-4BA4-A40A-014C6C458922}{6E2BAEEB-AA1C-4529-95D9-E8ACB1B58162}'
$Dest = Join-Path $env:ProgramFiles 'HangulIME'

$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
    [Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    $p = Start-Process powershell.exe -Verb RunAs -Wait -PassThru -ArgumentList @(
        '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$PSCommandPath`"")
    exit $p.ExitCode
}

# Remove from the language list (keeps the Korean language and other IMEs).
$list = Get-WinUserLanguageList
foreach ($l in $list) { if ($l.InputMethodTips.Contains($Tip)) { [void]$l.InputMethodTips.Remove($Tip) } }
$empty = @($list | Where-Object { $_.InputMethodTips.Count -eq 0 })
foreach ($l in $empty) { [void]$list.Remove($l) }
Set-WinUserLanguageList $list -Force

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
