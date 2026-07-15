param(
    [Parameter(Mandatory)]
    [string]$PackagePath,

    [Parameter(Mandatory)]
    [string]$CertificatePath,

    [Parameter(Mandatory)]
    [string]$ExternalLocation,

    [Parameter(Mandatory)]
    [string]$PackageName
)

$ErrorActionPreference = 'Stop'

function Test-IsElevated {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Quote-Argument([string]$Value) {
    '"{0}"' -f $Value
}

if (-not (Test-IsElevated)) {
    $arguments = @(
        '-NoLogo',
        '-NoProfile',
        '-NonInteractive',
        '-ExecutionPolicy',
        'Bypass',
        '-File',
        (Quote-Argument $PSCommandPath),
        '-PackagePath',
        (Quote-Argument $PackagePath),
        '-CertificatePath',
        (Quote-Argument $CertificatePath),
        '-ExternalLocation',
        (Quote-Argument $ExternalLocation),
        '-PackageName',
        (Quote-Argument $PackageName)
    ) -join ' '
    $process = Start-Process powershell.exe -ArgumentList $arguments -Verb RunAs -Wait -PassThru
    if ($process.ExitCode -ne 0) {
        throw "Elevated identity installation failed with exit code $($process.ExitCode)."
    }
    return
}

Import-Certificate -FilePath $CertificatePath -CertStoreLocation 'Cert:\LocalMachine\TrustedPeople' | Out-Null

$previous = Get-AppxPackage -Name $PackageName -ErrorAction SilentlyContinue
if ($null -ne $previous) {
    $previous | Remove-AppxPackage
}

Add-AppxPackage -Path $PackagePath -ExternalLocation $ExternalLocation
