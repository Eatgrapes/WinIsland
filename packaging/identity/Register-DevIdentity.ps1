$ErrorActionPreference = 'Stop'

function Test-IsElevated {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

if (-not (Test-IsElevated)) {
    $arguments = "-NoLogo -NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`""
    $process = Start-Process powershell.exe -ArgumentList $arguments -Verb RunAs -Wait -PassThru
    if ($process.ExitCode -ne 0) {
        throw "Elevated Dev identity registration failed with exit code $($process.ExitCode)."
    }
    return
}

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$targetDirectory = if ([string]::IsNullOrWhiteSpace($env:WINISLAND_DEV_TARGET_DIR)) {
    Join-Path $repositoryRoot 'target'
} else {
    $env:WINISLAND_DEV_TARGET_DIR
}
$certificateDirectory = Join-Path $targetDirectory 'identity-dev'
$outputDirectory = Join-Path $targetDirectory 'identity-dev\package'
$packageName = 'Eatgrapes.WinIsland.Dev'
$externalLocation = Join-Path $targetDirectory 'debug'

& (Join-Path $PSScriptRoot 'New-DevCertificate.ps1') -CertificateDirectory $certificateDirectory

New-Item -ItemType Directory -Force -Path (Join-Path $externalLocation 'resources') | Out-Null
Copy-Item `
    -Path (Join-Path $repositoryRoot 'resources\icon-dark.png') `
    -Destination (Join-Path $externalLocation 'resources\icon-dark.png') `
    -Force

$previous = Get-AppxPackage -Name $packageName -ErrorAction SilentlyContinue
if ($null -ne $previous) {
    $previous | Remove-AppxPackage
}

& (Join-Path $PSScriptRoot 'Build-IdentityPackage.ps1') `
    -Channel dev `
    -Version '1.0.0.0' `
    -CertificatePath (Join-Path $certificateDirectory 'WinIsland.Dev.pfx') `
    -CertificatePassword 'WinIslandDevelopment' `
    -OutputDirectory $outputDirectory

Import-Certificate `
    -FilePath (Join-Path $outputDirectory 'Eatgrapes.WinIsland.Dev.cer') `
    -CertStoreLocation 'Cert:\LocalMachine\TrustedPeople' | Out-Null

Add-AppxPackage `
    -Path (Join-Path $outputDirectory 'Eatgrapes.WinIsland.Dev.msix') `
    -ExternalLocation $externalLocation
