param(
    [Parameter(Mandatory)]
    [string]$CertificateDirectory
)

$ErrorActionPreference = 'Stop'

New-Item -ItemType Directory -Force -Path $CertificateDirectory | Out-Null

$certificate = Get-ChildItem Cert:\CurrentUser\My |
    Where-Object {
        $_.Subject -eq 'CN=Eatgrapes.WinIsland' -and
        $_.FriendlyName -eq 'WinIsland Development Package Signing' -and
        $_.HasPrivateKey
    } |
    Select-Object -First 1

if ($null -eq $certificate) {
    $certificate = New-SelfSignedCertificate `
        -Type Custom `
        -KeyUsage DigitalSignature `
        -CertStoreLocation 'Cert:\CurrentUser\My' `
        -TextExtension @('2.5.29.37={text}1.3.6.1.5.5.7.3.3', '2.5.29.19={text}') `
        -Subject 'CN=Eatgrapes.WinIsland' `
        -FriendlyName 'WinIsland Development Package Signing'
}

$password = ConvertTo-SecureString 'WinIslandDevelopment' -AsPlainText -Force
$pfxPath = Join-Path $CertificateDirectory 'WinIsland.Dev.pfx'

Export-PfxCertificate -Cert $certificate -FilePath $pfxPath -Password $password -Force | Out-Null
