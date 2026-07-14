$ErrorActionPreference = 'Stop'

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$env:WINISLAND_PACKAGE_CHANNEL = 'dev'
$targetDirectory = if ([string]::IsNullOrWhiteSpace($env:WINISLAND_DEV_TARGET_DIR)) {
    Join-Path $repositoryRoot 'target'
} else {
    $env:WINISLAND_DEV_TARGET_DIR
}
$env:WINISLAND_DEV_TARGET_DIR = $targetDirectory

Push-Location $repositoryRoot
try {
    cargo build --target-dir $targetDirectory
    & (Join-Path $PSScriptRoot 'Register-DevIdentity.ps1')
    Start-Process (Join-Path $targetDirectory 'debug\WinIsland.exe')
} finally {
    Pop-Location
}
