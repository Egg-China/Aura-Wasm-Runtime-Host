[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
& node (Join-Path $PSScriptRoot 'validate-ci-workflows.mjs')
if ($LASTEXITCODE -ne 0) {
    throw 'Wasm CI workflow contract validation failed'
}
& node (Join-Path $PSScriptRoot 'test-ci-workflows.mjs')
if ($LASTEXITCODE -ne 0) {
    throw 'Wasm CI workflow behavior tests failed'
}
& (Join-Path $PSScriptRoot 'test-ci-native-failure.ps1')
if ($LASTEXITCODE -ne 0) {
    throw 'Wasm CI native child-failure regression failed'
}
