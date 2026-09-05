# Verifies every native command guard in the CI tool-check step.
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$workflow = Get-Content -LiteralPath (Join-Path $PSScriptRoot '..\.github\workflows\ci.yml') -Raw
$step = [regex]::Match($workflow, '(?m)^      - name: Check Rust, Component, and packaging tools\r?\n        shell: pwsh\r?\n        run: \|\r?\n(?<body>(?:          [^\r\n]*(?:\r?\n|$))*)')
if (-not $step.Success) {
    throw 'The CI native tool-check step is missing'
}

$body = [regex]::Replace($step.Groups['body'].Value, '(?m)^          ', '')
$segment = [regex]::Match($body, '(?ms)\A(?<native>.*?)(?=^& \./tools/test-package-wasm-plugin\.ps1\r?$)')
if (-not $segment.Success) {
    throw 'The CI native tool-check segment is missing'
}

$shell = (Resolve-Path -LiteralPath (Get-Process -Id $PID).Path).Path
$node = (Get-Command node -ErrorAction Stop).Source
$temporaryBase = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$temporary = Join-Path $temporaryBase ('aura-ci-native-failure-' + [guid]::NewGuid().ToString('N'))

try {
    New-Item -ItemType Directory -Path $temporary | Out-Null
    foreach ($case in @(
        @{ Name = 'format'; FailureIndex = 1; Diagnostic = 'Rust formatting check failed' },
        @{ Name = 'component'; FailureIndex = 2; Diagnostic = 'Launch-hook component build failed' },
        @{ Name = 'clippy'; FailureIndex = 3; Diagnostic = 'Rust lint check failed' },
        @{ Name = 'workspace-test'; FailureIndex = 4; Diagnostic = 'Rust workspace tests failed' },
        @{ Name = 'guest-test'; FailureIndex = 5; Diagnostic = 'Rust guest SDK tests failed' }
    )) {
        $caseDirectory = Join-Path $temporary $case.Name
        New-Item -ItemType Directory -Path $caseDirectory | Out-Null
        $firstFailure = Join-Path $caseDirectory 'first-cargo-command-failed'
        $laterCommand = Join-Path $caseDirectory 'later-cargo-command-ran'
        $postSegment = Join-Path $caseDirectory 'post-native-segment-ran'
        $runner = Join-Path $caseDirectory 'run-ci-step.ps1'
        $preamble = @"
`$ErrorActionPreference = 'Stop'
`$script:cargoCalls = 0
function cargo {
    `$script:cargoCalls++
    if (`$script:cargoCalls -eq $($case.FailureIndex)) {
        New-Item -ItemType File -Path '$firstFailure' -Force | Out-Null
        & '$node' -e 'process.exit(23)'
        return
    }
    if (`$script:cargoCalls -gt $($case.FailureIndex)) {
        New-Item -ItemType File -Path '$laterCommand' -Force | Out-Null
    }
    & '$node' -e 'process.exit(0)'
}
"@
        [IO.File]::WriteAllText($runner, "$preamble`r`n$($segment.Groups['native'].Value)`r`nNew-Item -ItemType File -Path '$postSegment' -Force | Out-Null`r`n", [Text.UTF8Encoding]::new($false))

        $process = [Diagnostics.Process]::new()
        try {
            $process.StartInfo.FileName = $shell
            $process.StartInfo.Arguments = "-NoProfile -NonInteractive -ExecutionPolicy Bypass -File `"$runner`""
            $process.StartInfo.UseShellExecute = $false
            $process.StartInfo.CreateNoWindow = $true
            $process.StartInfo.RedirectStandardOutput = $true
            $process.StartInfo.RedirectStandardError = $true
            $process.Start() | Out-Null
            if (-not $process.WaitForExit(60000)) {
                $process.Kill()
                $process.WaitForExit()
                throw "$($case.Name): timed out after 60 seconds"
            }
            $output = $process.StandardOutput.ReadToEnd() + $process.StandardError.ReadToEnd()
            $exitCode = $process.ExitCode
        } finally {
            $process.Dispose()
        }

        if (-not (Test-Path -LiteralPath $firstFailure)) {
            throw "$($case.Name): the failing native child was not injected"
        }
        if ($exitCode -ne 1) {
            throw "$($case.Name): expected guarded child exit 1, got $exitCode"
        }
        if ($output -notlike "*$($case.Diagnostic)*") {
            throw "$($case.Name): expected guard diagnostic was not emitted"
        }
        if (Test-Path -LiteralPath $laterCommand) {
            throw "$($case.Name): a later cargo command ran after native failure"
        }
        if (Test-Path -LiteralPath $postSegment) {
            throw "$($case.Name): the native segment continued after failure"
        }
    }
} finally {
    if (Test-Path -LiteralPath $temporary) {
        $resolved = (Resolve-Path -LiteralPath $temporary).Path
        $expectedParent = $temporaryBase.TrimEnd([IO.Path]::DirectorySeparatorChar)
        if ([IO.Path]::GetDirectoryName($resolved) -cne $expectedParent -or
            -not [IO.Path]::GetFileName($resolved).StartsWith('aura-ci-native-failure-')) {
            throw 'Refusing cleanup outside the exact test temporary directory'
        }
        Remove-Item -LiteralPath $resolved -Recurse -Force
    }
}
Write-Output 'Native CI failure propagation: five guarded child-failure cases passed'
