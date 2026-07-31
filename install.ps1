[CmdletBinding()]
param(
    [switch]$PrintPlan,
    [string]$PlanArchitecture,
    [string]$ParseChecksumAsset,
    [string]$ParseChecksumManifest
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if ($PlanArchitecture -and -not $PrintPlan) {
    throw '-PlanArchitecture is only available with -PrintPlan.'
}

$Repository = 'madao02515634/llm-wikis'
$AssetName = 'llm-wikis-windows-amd64.exe'
$InstallDirectory = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'llm-wikis\bin'
$InstallPath = Join-Path $InstallDirectory 'llm-wikis.exe'
$PathMarker = '# LLM-WIKIS-PATH'

function Get-ReleaseTag {
    if ([string]::IsNullOrWhiteSpace($env:LLM_WIKIS_VERSION)) { return $null }
    return ('v' + $env:LLM_WIKIS_VERSION.Trim().TrimStart('v'))
}

function Get-Architecture {
    if ($PrintPlan -and $PlanArchitecture) { return $PlanArchitecture }
    return $env:PROCESSOR_ARCHITECTURE
}

function Get-ReleaseBase([string]$Tag) {
    if ([string]::IsNullOrEmpty($Tag)) { return "https://github.com/$Repository/releases/latest/download" }
    return "https://github.com/$Repository/releases/download/$Tag"
}

function Get-ManifestChecksum {
    param([Parameter(Mandatory)][string]$Asset, [Parameter(Mandatory)][string]$Manifest)
    $escapedAsset = [regex]::Escape($Asset)
    $matches = @(Get-Content -LiteralPath $Manifest | Where-Object { $_ -match "^(?<hash>[0-9a-fA-F]{64})[ *]+$escapedAsset$" })
    if ($matches.Count -ne 1) { throw "SHA256SUMS must contain exactly one checksum for $Asset." }
    return ([regex]::Match($matches[0], '^[0-9a-fA-F]{64}')).Value.ToLowerInvariant()
}

$parseChecksum = $PSBoundParameters.ContainsKey('ParseChecksumAsset') -or $PSBoundParameters.ContainsKey('ParseChecksumManifest')
if ($parseChecksum) {
    if ($PrintPlan -or $PlanArchitecture) { throw 'Checksum parsing cannot be combined with plan options.' }
    if ([string]::IsNullOrWhiteSpace($ParseChecksumAsset) -or [string]::IsNullOrWhiteSpace($ParseChecksumManifest)) {
        throw 'Provide both -ParseChecksumAsset and -ParseChecksumManifest.'
    }
    Write-Output "sha256=$(Get-ManifestChecksum -Asset $ParseChecksumAsset -Manifest $ParseChecksumManifest)"
    exit 0
}

$architecture = Get-Architecture
if ($architecture -notin @('AMD64', 'x86_64')) {
    throw "Unsupported Windows architecture: $architecture. Only x64 is supported."
}

$tag = Get-ReleaseTag
$resolvedVersion = if ([string]::IsNullOrEmpty($tag)) { 'latest' } else { $tag }
$baseUrl = Get-ReleaseBase $tag
$assetUrl = "$baseUrl/$AssetName"
$manifestUrl = "$baseUrl/SHA256SUMS"

if ($PrintPlan) {
    @(
        "version=$resolvedVersion"
        "asset=$AssetName"
        "asset_url=$assetUrl"
        "manifest_url=$manifestUrl"
        "install_directory=$InstallDirectory"
        "install_path=$InstallPath"
        'user_path_action=prepend-if-absent'
    ) | Write-Output
    exit 0
}

New-Item -ItemType Directory -Force -Path $InstallDirectory | Out-Null
$temporaryPath = Join-Path $InstallDirectory (".$AssetName.$([guid]::NewGuid().ToString('N')).tmp")
$manifestPath = Join-Path $InstallDirectory (".SHA256SUMS.$([guid]::NewGuid().ToString('N')).tmp")

try {
    Invoke-WebRequest -UseBasicParsing -Uri $assetUrl -OutFile $temporaryPath
    Invoke-WebRequest -UseBasicParsing -Uri $manifestUrl -OutFile $manifestPath

    $expectedHash = Get-ManifestChecksum -Asset $AssetName -Manifest $manifestPath
    $actualHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $temporaryPath).Hash.ToLowerInvariant()
    if ($expectedHash -ne $actualHash) { throw "Checksum verification failed for $AssetName." }

    Move-Item -Force -LiteralPath $temporaryPath -Destination $InstallPath
}
finally {
    Remove-Item -Force -ErrorAction SilentlyContinue -LiteralPath $temporaryPath, $manifestPath
}

$currentPath = [Environment]::GetEnvironmentVariable('Path', 'User')
$pathEntries = @($currentPath -split ';' | Where-Object { $_ -and $_ -ne $InstallDirectory })
$newPath = @($InstallDirectory) + $pathEntries -join ';'
if ($newPath -ne $currentPath) { [Environment]::SetEnvironmentVariable('Path', $newPath, 'User') }

Write-Host "Installed $InstallPath"
Write-Host 'Run llm-wikis --version manually to verify the installation.'
