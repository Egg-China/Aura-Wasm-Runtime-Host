[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$repositoryRoot = Split-Path -Parent $PSScriptRoot
$component = Join-Path $repositoryRoot 'target/wasm32-wasip1/release/launch_hook.wasm'
if (-not (Test-Path -LiteralPath $component -PathType Leaf)) {
    throw "Build the launch-hook component before running packaging tests: $component"
}

$temporary = Join-Path ([System.IO.Path]::GetTempPath()) ('aura-wasm-package-test-' + [guid]::NewGuid().ToString('N'))
$source = Join-Path $temporary 'source'
$first = Join-Path $temporary 'first.npl'
$second = Join-Path $temporary 'second.npl'

try {
    New-Item -ItemType Directory -Force $source | Out-Null
    Copy-Item -LiteralPath $component -Destination (Join-Path $source 'plugin.wasm')
    Set-Content -LiteralPath (Join-Path $source 'aura-wasm.json') -NoNewline -Value '{"schemaVersion":1,"component":"plugin.wasm"}'
    Set-Content -LiteralPath (Join-Path $source 'plugin.json') -NoNewline -Value '{"schemaVersion":5,"id":"dev.hmclce.test.wasm","name":"Test","version":"1.0.0","description":"Test","author":"Test","type":"java","runtime":"wasm","abi":1,"platforms":["windows-x64"],"entrypoint":"aura-wasm.json","executionMode":"isolated","runtimeProvider":"dev.hmclce.runtime.wasm-host","dependencies":[],"permissions":[],"requiredPermissions":[],"hooks":[],"patches":[],"launcherVersion":">=27.1-0-next"}'

    & (Join-Path $PSScriptRoot 'package-wasm-plugin.ps1') -Source $source -Component (Join-Path $source 'plugin.wasm') -Output $first
    & (Join-Path $PSScriptRoot 'package-wasm-plugin.ps1') -Source $source -Component (Join-Path $source 'plugin.wasm') -Output $second
    $firstHash = (Get-FileHash -LiteralPath $first -Algorithm SHA256).Hash
    $secondHash = (Get-FileHash -LiteralPath $second -Algorithm SHA256).Hash
    if ($firstHash -cne $secondHash) {
        throw 'Equivalent Wasm payload packages were not deterministic'
    }

    Add-Type -AssemblyName System.IO.Compression
    $archive = [System.IO.Compression.ZipFile]::OpenRead($first)
    try {
        $entries = @($archive.Entries | ForEach-Object FullName)
    } finally {
        $archive.Dispose()
    }
    if (($entries -join ',') -cne 'aura-wasm.json,plugin.json,plugin.wasm') {
        throw "Unexpected deterministic archive entries: $($entries -join ',')"
    }

    [System.IO.File]::WriteAllBytes((Join-Path $source 'plugin.wasm'), [byte[]](0, 97, 115, 109, 1, 0, 0, 0))
    $rejected = $false
    try {
        & (Join-Path $PSScriptRoot 'package-wasm-plugin.ps1') -Source $source -Component (Join-Path $source 'plugin.wasm') -Output (Join-Path $temporary 'core.npl') 2>$null
    } catch {
        $rejected = $true
    }
    if (-not $rejected) {
        throw 'Core WebAssembly module was accepted as an Aura Component payload'
    }
} finally {
    if (Test-Path -LiteralPath $temporary) {
        Remove-Item -LiteralPath $temporary -Recurse -Force
    }
}

Write-Output 'Wasm payload packaging tests passed'
