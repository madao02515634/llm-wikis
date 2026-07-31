[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$RepositoryRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$Installer = Join-Path $RepositoryRoot 'install.ps1'

function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw "Assertion failed: $Message" }
}

function Get-Plan {
    param([string]$Version, [string]$Architecture = 'AMD64')
    $previousVersion = $env:LLM_WIKIS_VERSION
    try {
        if ($null -eq $Version) { Remove-Item Env:LLM_WIKIS_VERSION -ErrorAction SilentlyContinue }
        else { $env:LLM_WIKIS_VERSION = $Version }
        $output = & $Installer -PrintPlan -PlanArchitecture $Architecture 2>&1
        if ($LASTEXITCODE -ne 0) { throw "PrintPlan failed: $output" }
        return ($output -join "`n")
    }
    finally {
        if ($null -eq $previousVersion) { Remove-Item Env:LLM_WIKIS_VERSION -ErrorAction SilentlyContinue }
        else { $env:LLM_WIKIS_VERSION = $previousVersion }
    }
}

if (-not (Test-Path -LiteralPath $Installer)) { throw "Missing installer: $Installer" }
$tokens = $null
$errors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile($Installer, [ref]$tokens, [ref]$errors)
Assert-True ($errors.Count -eq 0) 'install.ps1 parses without syntax errors'

$latest = Get-Plan -Version $null
Assert-True ($latest -match 'releases/latest/download/llm-wikis-windows-amd64\.exe') 'latest URL is selected'
$expectedPath = [regex]::Escape((Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'llm-wikis\bin\llm-wikis.exe'))
Assert-True ($latest -match $expectedPath) 'Windows install path is selected'
Assert-True ($latest -match 'version=latest') 'latest resolved version is reported'
Assert-True ($latest -match ('install_directory=' + [regex]::Escape((Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'llm-wikis\bin')))) 'install directory is reported'
Assert-True ($latest -match 'user_path_action=prepend-if-absent') 'user PATH action is reported'

foreach ($version in @('0.1.0', 'v0.1.0')) {
    $plan = Get-Plan -Version $version
    Assert-True ($plan -match 'releases/download/v0\.1\.0/llm-wikis-windows-amd64\.exe') "pinned version $version is normalized"
    Assert-True ($plan -match 'version=v0\.1\.0') "pinned version $version is reported"
}

function Assert-Rejected {
    param([scriptblock]$Action, [string]$Message)
    try { & $Action } catch { return }
    throw "Parser accepted: $Message"
}

$fixtureDirectory = Join-Path ([System.IO.Path]::GetTempPath()) ("llm-wikis-installer-parser-" + [guid]::NewGuid().ToString('N'))
try {
    New-Item -ItemType Directory -Path $fixtureDirectory | Out-Null
    $asset = 'llm-wikis-windows-amd64.exe'
    $hash = 'a' * 64
    $validManifest = Join-Path $fixtureDirectory 'valid.txt'
    [System.IO.File]::WriteAllText($validManifest, "$hash  $asset`n")
    $parsed = & $Installer -ParseChecksumAsset $asset -ParseChecksumManifest $validManifest
    Assert-True ($LASTEXITCODE -eq 0) 'parser mode accepts exactly one valid entry'
    Assert-True (($parsed -join "`n") -match "sha256=$hash") 'parser mode returns the selected checksum'

    $malformedManifest = Join-Path $fixtureDirectory 'malformed.txt'
    [System.IO.File]::WriteAllText($malformedManifest, "not-a-hash  $asset`n")
    Assert-Rejected { & $Installer -ParseChecksumAsset $asset -ParseChecksumManifest $malformedManifest } 'malformed matching entry'

    $duplicateManifest = Join-Path $fixtureDirectory 'duplicate.txt'
    [System.IO.File]::WriteAllText($duplicateManifest, "$hash  $asset`n$hash  $asset`n")
    Assert-Rejected { & $Installer -ParseChecksumAsset $asset -ParseChecksumManifest $duplicateManifest } 'duplicate matching entries'
}
finally {
    Remove-Item -LiteralPath $fixtureDirectory -Recurse -Force -ErrorAction SilentlyContinue
}

try {
    Get-Plan -Version $null -Architecture 'ARM64' | Out-Null
    throw 'unsupported architecture was accepted'
}
catch {
    if ($_.Exception.Message -eq 'unsupported architecture was accepted') { throw }
}

$source = Get-Content -Raw -LiteralPath $Installer
$forbidden = @('fake', ('test' + '-origin'), ('Start' + '-Process'))
foreach ($term in $forbidden) { Assert-True (-not $source.Contains($term)) "source rejects $term" }
Assert-True (-not ($source -match '&\s*\$?(?:temporary|download|asset|binary)')) 'downloaded path is not invoked'
Assert-True ($source -match 'Set-StrictMode') 'strict mode enabled'
Assert-True ($source -match 'Get-FileHash') 'checksum verification enabled'
Assert-True ($source -match 'madao02515634/llm-wikis') 'repository is fixed'
Assert-True (([regex]::Matches($source, 'LLM-WIKIS-PATH')).Count -eq 1) 'one managed PATH marker exists'
$webRequests = @($ast.FindAll({
            param($node)
            $node -is [System.Management.Automation.Language.CommandAst] -and
            $node.GetCommandName() -eq 'Invoke-WebRequest'
        }, $true))
Assert-True ($webRequests.Count -eq 2) 'exactly two web requests download the asset and manifest'
foreach ($webRequest in $webRequests) {
    $basicParsing = @($webRequest.CommandElements | Where-Object {
            $_ -is [System.Management.Automation.Language.CommandParameterAst] -and
            $_.ParameterName -eq 'UseBasicParsing'
        })
    Assert-True ($basicParsing.Count -eq 1) 'every web request enables Windows PowerShell 5.1 basic parsing'
}

Write-Host 'PowerShell installer contract checks passed.'
