$ErrorActionPreference = 'Stop'
Get-AppxPackage -Name 'Eatgrapes.WinIsland.Dev' -ErrorAction SilentlyContinue | Remove-AppxPackage
