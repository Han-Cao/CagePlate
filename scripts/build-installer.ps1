#requires -Version 5.1
<#
.SYNOPSIS
Build the Windows x64 GUI and package it with Inno Setup 6.3 or newer.
.PARAMETER Offline
Use only cached Cargo dependencies (no network access).
.PARAMETER IsccPath
Path to ISCC.exe. Otherwise search PATH and standard Inno Setup locations.
.OUTPUTS
An object with Version, Executable, and Installer properties.
#>
[CmdletBinding()]
param(
    [switch]$Offline,
    [string]$IsccPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
    throw 'Run this script in native Windows PowerShell, not a Linux/WSL PowerShell session.'
}

# Find the packaging dependency before spending time on a release build.
if ($IsccPath) {
    if (-not (Test-Path -LiteralPath $IsccPath -PathType Leaf)) {
        throw "Inno Setup compiler not found: $IsccPath"
    }
    $compiler = (Resolve-Path -LiteralPath $IsccPath).Path
}
else {
    $compiler = $null
    $command = Get-Command ISCC.exe -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($command) { $compiler = $command.Source }
    $installRoots = @(${env:ProgramFiles(x86)}, $env:ProgramFiles)
    if ($env:LOCALAPPDATA) {
        $installRoots += Join-Path $env:LOCALAPPDATA 'Programs'
    }
    foreach ($root in $installRoots) {
        if (-not $root) { continue }
        foreach ($directory in @('Inno Setup 7', 'Inno Setup 6')) {
            $candidate = Join-Path $root "$directory\ISCC.exe"
            if (-not $compiler -and (Test-Path -LiteralPath $candidate -PathType Leaf)) {
                $compiler = $candidate
            }
        }
    }
    if (-not $compiler) {
        throw 'Install Inno Setup 6.3+ from https://jrsoftware.org/isinfo.php or pass -IsccPath C:\path\to\ISCC.exe.'
    }
}

$build = & (Join-Path $PSScriptRoot 'build-windows.ps1') -Offline:$Offline
$outputDirectory = Join-Path $build.TargetDirectory 'installer'
$installer = Join-Path $outputDirectory "CagePlate-$($build.Version)-Setup.exe"
# Windows file versions are numeric; keep any SemVer suffix in AppVersion/filename.
$numericVersion = [regex]::Match($build.Version, '^\d+\.\d+\.\d+').Value
$compilerArgs = @(
    '/Qp',
    "/DAppVersion=$($build.Version)",
    "/DAppVersionNumeric=$numericVersion",
    "/DAppBinary=$($build.Executable)",
    "/DInstallerOutput=$outputDirectory",
    (Join-Path $PSScriptRoot 'CagePlate.iss')
)
Write-Host "Packaging CagePlate $($build.Version) with $compiler..."
& $compiler @compilerArgs | Out-Host
if ($LASTEXITCODE -ne 0) {
    throw "Inno Setup compilation failed (exit $LASTEXITCODE)."
}
if (-not (Test-Path -LiteralPath $installer -PathType Leaf)) {
    throw "Inno Setup succeeded but the expected installer is missing: $installer"
}
Write-Host "Windows installer: $installer"
[pscustomobject]@{
    Version = $build.Version
    Executable = $build.Executable
    Installer = $installer
}
