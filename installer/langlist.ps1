# Adds or removes Hangul IME in the current user's Windows language list.
# Run by the installer (-Add, as the user who started setup) and the uninstaller (-Remove).
#
# Note: Windows PowerShell 5.1 passes the whole list returned by Get-WinUserLanguageList through a pipeline as
# a single object, so this script uses foreach statements, never Where-Object, on the list.
param([switch]$Add, [switch]$Remove)
$ErrorActionPreference = 'Stop'

$Tip = '0412:{17432EFE-970E-4BA4-A40A-014C6C458922}{6E2BAEEB-AA1C-4529-95D9-E8ACB1B58162}'

function Add-HangulIme($list) {
    $ko = $null
    foreach ($l in $list) {
        if ($l.LanguageTag -eq 'ko' -or $l.LanguageTag -eq 'ko-KR') { $ko = $l; break }
    }
    if (-not $ko) {
        $list.Add('ko')
        $ko = $list[$list.Count - 1]
        $ko.InputMethodTips.Clear()   # don't also enable Microsoft's Korean IME
    }
    if (-not $ko.InputMethodTips.Contains($Tip)) { $ko.InputMethodTips.Add($Tip) }
}

function Remove-HangulIme($list) {
    $emptied = @()
    foreach ($l in $list) {
        if ($l.InputMethodTips.Remove($Tip) -and $l.InputMethodTips.Count -eq 0) { $emptied += $l }
    }
    # Drop a language only if removing this IME left it with no input methods.
    foreach ($l in $emptied) { [void]$list.Remove($l) }
}

if ($Add -or $Remove) {
    $list = Get-WinUserLanguageList
    if ($Add) { Add-HangulIme $list } else { Remove-HangulIme $list }
    Set-WinUserLanguageList $list -Force
}
