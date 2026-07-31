[CmdletBinding()]
param(
    [string]$StagingDirectory,
    [switch]$SelfTest
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$ExpectedFiles = @(
    'llm-wikis-windows-amd64.exe',
    'llm-wikis-linux-amd64',
    'llm-wikis-darwin-arm64',
    'SHA256SUMS'
)
$ChecksummedFiles = $ExpectedFiles | Where-Object { $_ -ne 'SHA256SUMS' }

function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw "Verification failed: $Message" }
}

function Test-ReleaseBundle {
    param([Parameter(Mandatory)][string]$Directory)
    $directoryItem = Get-Item -LiteralPath $Directory
    Assert-True $directoryItem.PSIsContainer 'staging path is a directory'
    $actualFiles = @(Get-ChildItem -LiteralPath $directoryItem.FullName -File | Select-Object -ExpandProperty Name | Sort-Object)
    $expectedFiles = @($ExpectedFiles | Sort-Object)
    Assert-True ($actualFiles.Count -eq $expectedFiles.Count) 'bundle contains exactly four files'
    Assert-True (@(Compare-Object $actualFiles $expectedFiles).Count -eq 0) 'bundle file set is exact'

    $manifestPath = Join-Path $directoryItem.FullName 'SHA256SUMS'
    $entries = @(Get-Content -LiteralPath $manifestPath | Where-Object { $_ -ne '' })
    Assert-True ($entries.Count -eq 3) 'SHA256SUMS has exactly three entries'
    $names = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
    foreach ($line in $entries) {
        $match = [regex]::Match($line, '^(?<hash>[0-9A-Fa-f]{64})[ ]{2}(?<name>[^/\\]+)$')
        Assert-True $match.Success "manifest entry is valid: $line"
        $name = $match.Groups['name'].Value
        Assert-True ($ChecksummedFiles -contains $name) "manifest entry names a release asset: $name"
        Assert-True $names.Add($name) "manifest entry is unique: $name"
        $actualHash = (Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $directoryItem.FullName $name)).Hash
        Assert-True ($actualHash -ieq $match.Groups['hash'].Value) "checksum matches: $name"
    }
    Assert-True ($names.SetEquals([string[]]$ChecksummedFiles)) 'every non-manifest asset is checksummed'
}

function New-TextBundle {
    param([Parameter(Mandatory)][string]$Directory)
    New-Item -ItemType Directory -Force -Path $Directory | Out-Null
    foreach ($name in $ChecksummedFiles) {
        [System.IO.File]::WriteAllText((Join-Path $Directory $name), "inert $name`n")
    }
    $lines = foreach ($name in $ChecksummedFiles) {
        $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $Directory $name)).Hash.ToLowerInvariant()
        "$hash  $name"
    }
    [System.IO.File]::WriteAllLines((Join-Path $Directory 'SHA256SUMS'), [string[]]$lines)
}

function Assert-Rejected {
    param([scriptblock]$Action, [string]$Message)
    try { & $Action } catch { return }
    throw "Self-test did not reject: $Message"
}

function Test-PublishContract {
    $repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
    $workflow = Get-Content -Raw -LiteralPath (Join-Path $repositoryRoot '.github\workflows\release.yml')
    $readme = Get-Content -Raw -LiteralPath (Join-Path $repositoryRoot 'README.md')

    Assert-True ($workflow -notmatch 'release-bundle/install\.(ps1|sh)') 'Release assets exclude installer scripts'
    Assert-True ($readme -notmatch 'releases/(latest/)?download/[^\s]*/?install\.(ps1|sh)') 'README does not download installers from Releases'
    Assert-True ($readme -match 'raw\.githubusercontent\.com/madao02515634/llm-wikis/refs/heads/main/install\.ps1') 'README uses the raw repository PowerShell installer'
    Assert-True ($readme -match 'raw\.githubusercontent\.com/madao02515634/llm-wikis/refs/heads/main/install\.sh') 'README uses the raw repository POSIX installer'
}

if ($SelfTest) {
    Test-PublishContract
    $root = Join-Path ([System.IO.Path]::GetTempPath()) ("llm-wikis-verifier-" + [guid]::NewGuid().ToString('N'))
    try {
        New-TextBundle $root
        Test-ReleaseBundle $root

        Remove-Item -LiteralPath (Join-Path $root 'llm-wikis-darwin-arm64')
        Assert-Rejected { Test-ReleaseBundle $root } 'missing file'
        New-TextBundle $root

        [System.IO.File]::WriteAllText((Join-Path $root 'unexpected.txt'), 'inert')
        Assert-Rejected { Test-ReleaseBundle $root } 'unexpected file'
        Remove-Item -LiteralPath (Join-Path $root 'unexpected.txt')

        Add-Content -LiteralPath (Join-Path $root 'SHA256SUMS') -Value ((Get-Content -LiteralPath (Join-Path $root 'SHA256SUMS'))[0])
        Assert-Rejected { Test-ReleaseBundle $root } 'duplicate manifest entry'
        New-TextBundle $root

        [System.IO.File]::WriteAllText((Join-Path $root 'SHA256SUMS'), 'not a checksum')
        Assert-Rejected { Test-ReleaseBundle $root } 'malformed manifest entry'
        New-TextBundle $root

        Add-Content -LiteralPath (Join-Path $root 'llm-wikis-linux-amd64') -Value 'changed'
        Assert-Rejected { Test-ReleaseBundle $root } 'checksum mismatch'
    }
    finally {
        Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
    }
    Write-Host 'PowerShell release bundle verifier self-test passed.'
    exit 0
}

if ([string]::IsNullOrWhiteSpace($StagingDirectory)) { throw 'Provide -StagingDirectory or -SelfTest.' }
Test-ReleaseBundle $StagingDirectory
Write-Host 'Release bundle verified.'
