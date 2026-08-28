[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string] $Source,

    [Parameter(Mandatory = $true)]
    [string] $Component,

    [Parameter(Mandatory = $true)]
    [string] $Output
)

$ErrorActionPreference = 'Stop'

function Assert-ExactProperties {
    param([object] $Value, [string[]] $Expected, [string] $Label)
    $actual = @($Value.PSObject.Properties.Name | Sort-Object)
    $expectedSorted = @($Expected | Sort-Object)
    if (Compare-Object $actual $expectedSorted -SyncWindow 0) {
        throw "$Label contains missing or unknown fields"
    }
}

function Assert-SafeRelativePath {
    param([string] $Value)
    if ([string]::IsNullOrWhiteSpace($Value) -or
        [System.IO.Path]::IsPathRooted($Value) -or
        $Value.Contains('\') -or
        $Value.Split('/') -contains '..' -or
        $Value.Split('/') -contains '.' -or
        $Value.Split('/') -contains '') {
        throw 'Component path must be a safe slash-separated relative path'
    }
}

$sourceRoot = (Resolve-Path -LiteralPath $Source).Path
$componentPath = (Resolve-Path -LiteralPath $Component).Path
if (-not (Test-Path -LiteralPath $sourceRoot -PathType Container)) {
    throw 'Source must be a directory'
}
if (-not (Test-Path -LiteralPath $componentPath -PathType Leaf)) {
    throw 'Component must be a file'
}

$pluginPath = Join-Path $sourceRoot 'plugin.json'
$descriptorPath = Join-Path $sourceRoot 'aura-wasm.json'
if (-not (Test-Path -LiteralPath $pluginPath -PathType Leaf) -or
    -not (Test-Path -LiteralPath $descriptorPath -PathType Leaf)) {
    throw 'Source must contain plugin.json and aura-wasm.json'
}

$plugin = Get-Content -LiteralPath $pluginPath -Raw | ConvertFrom-Json
$descriptor = Get-Content -LiteralPath $descriptorPath -Raw | ConvertFrom-Json
Assert-ExactProperties $descriptor @('schemaVersion', 'component') 'aura-wasm.json'
if ($descriptor.schemaVersion -ne 1 -or $descriptor.component -isnot [string] -or
    -not $descriptor.component.EndsWith('.wasm', [System.StringComparison]::Ordinal)) {
    throw 'aura-wasm.json must contain schemaVersion 1 and a .wasm component'
}
Assert-SafeRelativePath $descriptor.component

Assert-ExactProperties $plugin @(
    'schemaVersion', 'id', 'name', 'version', 'description', 'author', 'type', 'runtime', 'abi',
    'platforms', 'entrypoint', 'executionMode', 'runtimeProvider', 'dependencies', 'permissions',
    'requiredPermissions', 'hooks', 'patches', 'launcherVersion'
) 'plugin.json'
if ($plugin.schemaVersion -ne 5 -or $plugin.runtime -cne 'wasm' -or $plugin.abi -ne 1 -or
    $plugin.entrypoint -cne 'aura-wasm.json' -or $plugin.executionMode -cne 'isolated' -or
    $plugin.runtimeProvider -cne 'dev.hmclce.runtime.wasm-host') {
    throw 'plugin.json does not declare the exact isolated Wasm runtime contract'
}

$header = [System.IO.File]::ReadAllBytes($componentPath)
if ($header.Length -lt 8 -or $header[0] -ne 0 -or $header[1] -ne 97 -or
    $header[2] -ne 115 -or $header[3] -ne 109 -or $header[4] -ne 13 -or
    $header[5] -ne 0 -or $header[6] -ne 1 -or $header[7] -ne 0) {
    throw 'Component is not a WebAssembly Component Model binary'
}

$outputPath = [System.IO.Path]::GetFullPath($Output)
$outputDirectory = Split-Path -Parent $outputPath
if (-not [string]::IsNullOrEmpty($outputDirectory)) {
    New-Item -ItemType Directory -Force $outputDirectory | Out-Null
}
if (Test-Path -LiteralPath $outputPath) {
    Remove-Item -LiteralPath $outputPath -Force
}

$files = [ordered]@{
    'aura-wasm.json' = $descriptorPath
    'plugin.json' = $pluginPath
    $descriptor.component = $componentPath
}
if (Test-Path -LiteralPath (Join-Path $sourceRoot 'README.md') -PathType Leaf) {
    $files['README.md'] = Join-Path $sourceRoot 'README.md'
}

Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
$archive = [System.IO.Compression.ZipFile]::Open($outputPath, [System.IO.Compression.ZipArchiveMode]::Create)
try {
    $timestamp = [System.DateTimeOffset]::new(1980, 1, 1, 0, 0, 0, [System.TimeSpan]::Zero)
    foreach ($entryName in @($files.Keys | Sort-Object)) {
        $entry = $archive.CreateEntry($entryName, [System.IO.Compression.CompressionLevel]::Optimal)
        $entry.LastWriteTime = $timestamp
        $input = [System.IO.File]::OpenRead($files[$entryName])
        $outputStream = $entry.Open()
        try {
            $input.CopyTo($outputStream)
        } finally {
            $outputStream.Dispose()
            $input.Dispose()
        }
    }
} finally {
    $archive.Dispose()
}

Write-Output $outputPath
