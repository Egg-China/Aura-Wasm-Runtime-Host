[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem

function Assert-Condition {
    param([bool] $Condition, [string] $Message)
    if (-not $Condition) { throw $Message }
}

function Assert-Fails {
    param([scriptblock] $Action, [string] $ExpectedMessage)
    try {
        & $Action
    } catch {
        Assert-Condition ($_.Exception.Message -like "*$ExpectedMessage*") `
            "Expected '$ExpectedMessage', got '$($_.Exception.Message)'"
        return
    }
    throw "Expected failure containing '$ExpectedMessage'"
}

function New-HostBinary {
    param([string] $Path, [string] $Platform)
    $bytes = [byte[]]::new(512)
    if ($Platform.StartsWith('windows-')) {
        $bytes[0] = 0x4d
        $bytes[1] = 0x5a
        [BitConverter]::GetBytes([int32]0x80).CopyTo($bytes, 0x3c)
        $bytes[0x80] = 0x50
        $bytes[0x81] = 0x45
        $machine = if ($Platform.EndsWith('-x64')) { [uint16]0x8664 } else { [uint16]0xaa64 }
        [BitConverter]::GetBytes($machine).CopyTo($bytes, 0x84)
    } elseif ($Platform.StartsWith('linux-')) {
        $magic = [byte[]]@(0x7f, 0x45, 0x4c, 0x46)
        $magic.CopyTo($bytes, 0)
        $bytes[4] = 2
        $bytes[5] = 1
        $machine = if ($Platform.EndsWith('-x64')) { [uint16]62 } else { [uint16]183 }
        [BitConverter]::GetBytes($machine).CopyTo($bytes, 18)
    } else {
        $magic = [byte[]]@(0xcf, 0xfa, 0xed, 0xfe)
        $magic.CopyTo($bytes, 0)
        $cpu = if ($Platform.EndsWith('-x64')) { [uint32]0x01000007 } else { [uint32]0x0100000c }
        [BitConverter]::GetBytes($cpu).CopyTo($bytes, 4)
    }
    [System.IO.File]::WriteAllBytes($Path, $bytes)
}

function New-Package {
    param(
        [string] $Root,
        [string] $ContractRoot,
        [string] $Platform,
        [string] $BinaryPlatform = $Platform,
        [switch] $DuplicatePluginJson
    )
    $executableName = if ($Platform.StartsWith('windows-')) {
        'aura-wasm-host.exe'
    } else {
        'aura-wasm-host'
    }
    $binary = Join-Path $Root "$Platform-$executableName"
    New-HostBinary -Path $binary -Platform $BinaryPlatform
    $jar = Join-Path $Root "$Platform-provider.jar"
    [System.IO.File]::WriteAllBytes($jar, [byte[]](0x50, 0x4b, 0x03, 0x04))
    $package = Join-Path $Root "dev.hmclce.runtime.wasm-host-v0.1.0-beta.1-$Platform.npl"
    if (Test-Path -LiteralPath $package) {
        Remove-Item -LiteralPath $package -Force
    }
    $archive = [System.IO.Compression.ZipFile]::Open(
        $package,
        [System.IO.Compression.ZipArchiveMode]::Create
    )
    try {
        $files = [ordered]@{
            'LICENSE' = (Join-Path $PSScriptRoot '..\LICENSE')
            'THIRD-PARTY-NOTICES.txt' = (Join-Path $ContractRoot 'THIRD-PARTY-NOTICES.txt')
            'libs/aura-wasm-runtime-host-plugin.jar' = $jar
            "native/$Platform/$executableName" = $binary
            'plugin.json' = (Join-Path $PSScriptRoot '..\host-plugin\plugin.json')
        }
        foreach ($entryName in $files.Keys) {
            [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile(
                $archive,
                $files[$entryName],
                $entryName
            ) | Out-Null
        }
        if ($DuplicatePluginJson) {
            [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile(
                $archive,
                (Join-Path $PSScriptRoot '..\host-plugin\plugin.json'),
                'plugin.json'
            ) | Out-Null
        }
    } finally {
        $archive.Dispose()
    }
    return $package
}

function New-Record {
    param([string] $Platform, [string] $Package)
    return [pscustomobject][ordered]@{
        platform = $Platform
        package = Split-Path -Leaf $Package
        sha256 = (Get-FileHash -LiteralPath $Package -Algorithm SHA256).Hash.ToLowerInvariant()
        size = (Get-Item -LiteralPath $Package).Length
    }
}

function Write-Manifest {
    param(
        [string] $Path,
        [object[]] $Artifacts,
        [string] $AuraCommit = '636b06aad03c5d21946369c836280c891c13054d',
        [string] $AuraJarHash = '674f717f5f97a5b7e8f7f20e4d60aa2e25451d71a96ab475f4595d0482f99d4b'
    )
    $manifest = [pscustomobject][ordered]@{
        schemaVersion = 1
        version = '0.1.0-beta.1'
        aura = [pscustomobject][ordered]@{
            repository = 'Egg-China/Aura-Launcher'
            commit = $AuraCommit
            runId = '33931508945'
            jarSha256 = $AuraJarHash
        }
        artifacts = $Artifacts
    }
    [System.IO.File]::WriteAllText(
        $Path,
        ($manifest | ConvertTo-Json -Depth 8) + "`n",
        [System.Text.UTF8Encoding]::new($false)
    )
}

$platforms = @(
    'windows-x64',
    'windows-arm64',
    'linux-x64',
    'linux-arm64',
    'macos-x64',
    'macos-arm64'
)
$temporary = Join-Path ([System.IO.Path]::GetTempPath()) `
    ('aura-wasm-artifact-test-' + [guid]::NewGuid().ToString('N'))
$verifier = Join-Path $PSScriptRoot 'verify-wasm-host-artifacts.ps1'

try {
    New-Item -ItemType Directory -Path $temporary | Out-Null
    $contractRoot = Join-Path $temporary 'contract'
    New-Item -ItemType Directory -Path (Join-Path $contractRoot 'sdk/wit') -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $contractRoot 'sdk/rust/aura-wasm-guest/src') -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot '../sdk/wit/aura-runtime.wit') `
        -Destination (Join-Path $contractRoot 'sdk/wit/aura-runtime.wit')
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot '../sdk/rust/aura-wasm-guest/Cargo.toml') `
        -Destination (Join-Path $contractRoot 'sdk/rust/aura-wasm-guest/Cargo.toml')
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot '../sdk/rust/aura-wasm-guest/src/lib.rs') `
        -Destination (Join-Path $contractRoot 'sdk/rust/aura-wasm-guest/src/lib.rs')
    Set-Content -LiteralPath (Join-Path $contractRoot 'THIRD-PARTY-NOTICES.txt') `
        -Value 'Wasmtime 48.0.1 is provided under Apache-2.0 WITH LLVM-exception and MIT.' -NoNewline
    $records = @()
    foreach ($platform in $platforms) {
        $package = New-Package -Root $temporary -ContractRoot $contractRoot -Platform $platform
        $records += New-Record -Platform $platform -Package $package
    }
    $manifestPath = Join-Path $temporary 'manifest.json'
    Write-Manifest -Path $manifestPath -Artifacts $records
    & $verifier -ArtifactManifest $manifestPath -PackageDirectory $temporary -ContractRoot $contractRoot

    $wrongProvenance = Join-Path $temporary 'wrong-provenance.json'
    Write-Manifest -Path $wrongProvenance -Artifacts $records -AuraCommit ('0' * 40)
    Assert-Fails {
        & $verifier -ArtifactManifest $wrongProvenance -PackageDirectory $temporary -ContractRoot $contractRoot
    } 'Aura commit'

    $wrongAuraHash = Join-Path $temporary 'wrong-aura-hash.json'
    Write-Manifest -Path $wrongAuraHash -Artifacts $records -AuraJarHash ('0' * 64)
    Assert-Fails {
        & $verifier -ArtifactManifest $wrongAuraHash -PackageDirectory $temporary -ContractRoot $contractRoot
    } 'Aura JAR SHA-256'

    $badHashRecord = New-Record -Platform 'windows-x64' `
        -Package (Join-Path $temporary 'dev.hmclce.runtime.wasm-host-v0.1.0-beta.1-windows-x64.npl')
    $badHashRecord.sha256 = '0' * 64
    $badHashManifest = Join-Path $temporary 'bad-hash.json'
    Write-Manifest -Path $badHashManifest -Artifacts @($badHashRecord)
    Assert-Fails {
        & $verifier -ArtifactManifest $badHashManifest -PackageDirectory $temporary -ContractRoot $contractRoot
    } 'SHA-256'

    $wrongArchitecture = New-Package -Root $temporary -ContractRoot $contractRoot `
        -Platform 'windows-x64' -BinaryPlatform 'windows-arm64'
    $wrongArchitectureManifest = Join-Path $temporary 'wrong-architecture.json'
    Write-Manifest -Path $wrongArchitectureManifest `
        -Artifacts @((New-Record -Platform 'windows-x64' -Package $wrongArchitecture))
    Assert-Fails {
        & $verifier -ArtifactManifest $wrongArchitectureManifest -PackageDirectory $temporary -ContractRoot $contractRoot
    } 'architecture'

    $duplicate = New-Package -Root $temporary -ContractRoot $contractRoot `
        -Platform 'linux-x64' -DuplicatePluginJson
    $duplicateManifest = Join-Path $temporary 'duplicate.json'
    Write-Manifest -Path $duplicateManifest `
        -Artifacts @((New-Record -Platform 'linux-x64' -Package $duplicate))
    Assert-Fails {
        & $verifier -ArtifactManifest $duplicateManifest -PackageDirectory $temporary -ContractRoot $contractRoot
    } 'duplicate ZIP entry'

    Move-Item -LiteralPath (Join-Path $contractRoot 'sdk/wit/aura-runtime.wit') `
        -Destination (Join-Path $contractRoot 'sdk/wit/missing.wit')
    Assert-Fails {
        & $verifier -ArtifactManifest $manifestPath -PackageDirectory $temporary -ContractRoot $contractRoot
    } 'WIT contract'
} finally {
    if (Test-Path -LiteralPath $temporary) {
        Remove-Item -LiteralPath $temporary -Recurse -Force
    }
}

Write-Output 'Wasm Host artifact verifier tests passed'
