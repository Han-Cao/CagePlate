#requires -Version 5.1
<#
.SYNOPSIS
Build the Windows x64 GUI release with locked dependencies and a static MSVC runtime.
.PARAMETER Offline
Use only cached Cargo dependencies (no network access).
.OUTPUTS
An object with Version, Executable, and TargetDirectory properties.
#>
[CmdletBinding()]
param(
    [switch]$Offline
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
    throw 'Run this script in native Windows PowerShell, not a Linux/WSL PowerShell session.'
}

$cargo = Get-Command cargo -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $cargo) {
    throw 'Cargo was not found. Install Rust 1.95+ with the MSVC toolchain and Visual Studio C++ Build Tools.'
}

$projectRoot = Split-Path -Parent $PSScriptRoot
$target = 'x86_64-pc-windows-msvc'
$originalRustFlags = [Environment]::GetEnvironmentVariable('RUSTFLAGS')
$originalEncodedFlags = [Environment]::GetEnvironmentVariable('CARGO_ENCODED_RUSTFLAGS')
Push-Location -LiteralPath $projectRoot
try {
    $metadataArgs = @('metadata', '--format-version', '1', '--no-deps', '--locked')
    if ($Offline) { $metadataArgs += '--offline' }
    $metadataJson = & $cargo.Source @metadataArgs
    if ($LASTEXITCODE -ne 0) {
        throw "cargo metadata failed (exit $LASTEXITCODE)."
    }
    $metadata = ($metadataJson -join "`n") | ConvertFrom-Json
    $package = @($metadata.packages | Where-Object { $_.name -eq 'cageplate' })
    if ($package.Count -ne 1) {
        throw 'Expected exactly one cageplate package in cargo metadata.'
    }

    $buildArgs = @('build', '--release', '--locked', '--target', $target, '--features', 'gui', '--bin', 'cageplate')
    if ($Offline) { $buildArgs += '--offline' }
    # Avoid a VCRUNTIME140.dll prerequisite on clean Windows installations.
    # Cargo gives encoded flags precedence over RUSTFLAGS; preserve either form.
    if ($originalEncodedFlags) {
        $separator = [char]31
        $env:CARGO_ENCODED_RUSTFLAGS = $originalEncodedFlags + $separator + '-C' + $separator + 'target-feature=+crt-static'
    }
    else {
        $env:RUSTFLAGS = ("$originalRustFlags -C target-feature=+crt-static").Trim()
    }
    Write-Host "Building CagePlate $($package[0].version) for $target (static MSVC runtime)..."
    & $cargo.Source @buildArgs | Out-Host
    if ($LASTEXITCODE -ne 0) {
        throw "Windows release build failed (exit $LASTEXITCODE). Close any running copy of the output executable and check the MSVC tools/Windows SDK."
    }

    # Cargo metadata honors CARGO_TARGET_DIR and .cargo/config.toml overrides.
    $executable = Join-Path $metadata.target_directory "$target\release\cageplate.exe"
    if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
        throw "Cargo succeeded but the expected executable is missing: $executable"
    }
    Write-Host "Windows executable: $executable"
    [pscustomobject]@{
        Version = $package[0].version
        Executable = $executable
        TargetDirectory = $metadata.target_directory
    }
}
finally {
    [Environment]::SetEnvironmentVariable('RUSTFLAGS', $originalRustFlags)
    [Environment]::SetEnvironmentVariable('CARGO_ENCODED_RUSTFLAGS', $originalEncodedFlags)
    Pop-Location
}
