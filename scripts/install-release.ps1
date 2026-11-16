# One-line installer for Graphite on Windows 10/11 (64-bit).
#   irm https://raw.githubusercontent.com/sergerprojects/graphite/main/scripts/install-release.ps1 | iex
# Downloads the latest installer and runs it for the current user (no admin needed).
$ErrorActionPreference = 'Stop'

if (-not [Environment]::Is64BitOperatingSystem) { throw 'Graphite needs 64-bit Windows 10 or 11.' }

$url = 'https://github.com/sergerprojects/graphite/releases/latest/download/Graphite-windows-x64-setup.exe'
$file = Join-Path $env:TEMP 'Graphite-windows-x64-setup.exe'

Write-Host 'Downloading Graphite...'
Invoke-WebRequest -Uri $url -OutFile $file -UseBasicParsing
Unblock-File $file

Write-Host 'Installing...'
Get-Process -Name 'Graphite' -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Process -FilePath $file -ArgumentList '/S' -Wait
Remove-Item $file -ErrorAction SilentlyContinue

Write-Host 'Done. Open Graphite from the Start menu.'
Write-Host "Tip: summaries and chat use Claude Code (claude.com/claude-code). Install it and run 'claude' once to sign in."
