param(
    [Parameter(Mandatory)]
    [ValidateSet('dev', 'stable', 'nightly')]
    [string]$Channel,

    [Parameter(Mandatory)]
    [string]$Version,

    [Parameter(Mandatory)]
    [string]$CertificatePath,

    [Parameter(Mandatory)]
    [string]$CertificatePassword,

    [Parameter(Mandatory)]
    [string]$OutputDirectory
)

$ErrorActionPreference = 'Stop'

function Get-SdkTool([string]$Name) {
    $roots = @(
        "${env:ProgramFiles(x86)}\Windows Kits\10\bin",
        "$env:ProgramFiles\Windows Kits\10\bin"
    )
    foreach ($root in $roots) {
        $tool = Get-ChildItem -Path $root -Filter $Name -Recurse -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -match '\\x64\\' } |
            Sort-Object FullName -Descending |
            Select-Object -First 1
        if ($null -ne $tool) {
            return $tool.FullName
        }
    }
    throw "$Name was not found. Install the Windows 10 or Windows 11 SDK."
}

$identity = switch ($Channel) {
    'stable' { @{ Name = 'Eatgrapes.WinIsland'; DisplayName = 'WinIsland' } }
    'nightly' { @{ Name = 'Eatgrapes.WinIsland.Nightly'; DisplayName = 'WinIsland Nightly' } }
    'dev' { @{ Name = 'Eatgrapes.WinIsland.Dev'; DisplayName = 'WinIsland Development' } }
}

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$stagingDirectory = Join-Path $OutputDirectory 'identity-staging'
$manifestPath = Join-Path $stagingDirectory 'AppxManifest.xml'
$packagePath = Join-Path $OutputDirectory "$($identity.Name).msix"

Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $stagingDirectory
New-Item -ItemType Directory -Force -Path $stagingDirectory | Out-Null
New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null

$manifest = Get-Content -Raw (Join-Path $PSScriptRoot 'AppxManifest.xml.template')
$manifest = $manifest.Replace('{{PACKAGE_NAME}}', $identity.Name)
$manifest = $manifest.Replace('{{PACKAGE_VERSION}}', $Version)
$manifest = $manifest.Replace('{{DISPLAY_NAME}}', $identity.DisplayName)
Set-Content -Path $manifestPath -Value $manifest -Encoding utf8

$makeAppx = Get-SdkTool 'MakeAppx.exe'
$signTool = Get-SdkTool 'SignTool.exe'

Remove-Item -Force -ErrorAction SilentlyContinue $packagePath
& $makeAppx pack /o /d $stagingDirectory /nv /p $packagePath
if ($LASTEXITCODE -ne 0) {
    throw 'MakeAppx failed to build the identity package.'
}

& $signTool sign /fd SHA256 /f $CertificatePath /p $CertificatePassword $packagePath
if ($LASTEXITCODE -ne 0) {
    throw 'SignTool failed to sign the identity package.'
}

$certificate = [System.Security.Cryptography.X509Certificates.X509Certificate2]::new(
    $CertificatePath,
    $CertificatePassword
)
$certificatePath = Join-Path $OutputDirectory "$($identity.Name).cer"
[IO.File]::WriteAllBytes(
    $certificatePath,
    $certificate.Export([System.Security.Cryptography.X509Certificates.X509ContentType]::Cert)
)
$certificate.Dispose()
