# Remove Graphite from Windows. Your meetings and notes are KEPT unless you pass -Everything.
#   powershell -ExecutionPolicy Bypass -File scripts\uninstall.ps1
#   powershell -ExecutionPolicy Bypass -File scripts\uninstall.ps1 -Everything
param([switch]$Everything)
$ErrorActionPreference = 'Continue'

Get-Process -Name 'Graphite' -ErrorAction SilentlyContinue | Stop-Process -Force
$un = Get-ChildItem "$env:LOCALAPPDATA\Graphite" -Filter 'uninstall*.exe' -ErrorAction SilentlyContinue | Select-Object -First 1
if ($un) { Start-Process -FilePath $un.FullName -ArgumentList '/S' -Wait }

if ($Everything) {
  Remove-Item -Recurse -Force "$env:USERPROFILE\Videos\graphite-recordings", "$env:USERPROFILE\.graphite",
    "$env:APPDATA\com.graphite.ai", "$env:LOCALAPPDATA\com.graphite.ai", "$env:LOCALAPPDATA\Graphite" -ErrorAction SilentlyContinue
  Write-Host 'Graphite and all of its data are gone.'
} else {
  Write-Host 'Graphite is removed. Your meetings are still in Videos\graphite-recordings.'
}
