param(
    [Parameter(Mandatory)]
    [string]$PackageName
)

$ErrorActionPreference = 'Stop'
Get-AppxPackage -Name $PackageName -ErrorAction SilentlyContinue | Remove-AppxPackage
