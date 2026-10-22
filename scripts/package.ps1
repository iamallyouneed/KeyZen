<#
.SYNOPSIS
Build and package KeyZen as a portable Windows zip.

.DESCRIPTION
The script detects the KeyZen version from crates/keyzen-win/Cargo.toml,
runs the release checks, creates dist/KeyZen-<version>-windows-<arch>.zip,
writes dist/SHA256SUMS.txt, and verifies the packaged executable.

.EXAMPLE
.\scripts\package.ps1

.EXAMPLE
.\scripts\package.ps1 -SkipTests

.EXAMPLE
.\scripts\package.ps1 -StopRunningKeyZen
#>

[CmdletBinding()]
param(
    [string]$Version,
    [string]$PackageName,
    [switch]$SkipFmt,
    [switch]$SkipTests,
    [switch]$SkipBuild,
    [switch]$SkipPackageVerify,
    [switch]$StopRunningKeyZen
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Get-WorkspaceRoot {
    $scriptDir = Split-Path -Parent $PSCommandPath
    return (Resolve-Path -LiteralPath (Join-Path $scriptDir "..")).Path
}

function Invoke-Step {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][scriptblock]$Body
    )

    Write-Host ""
    Write-Host "==> $Name"
    & $Body
}

function Invoke-CheckedCommand {
    param(
        [Parameter(Mandatory = $true)][string]$FilePath,
        [Parameter(Mandatory = $true)][string[]]$Arguments
    )

    & $FilePath @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "Command failed with exit code ${LASTEXITCODE}: $FilePath $($Arguments -join ' ')"
    }
}

function Get-KeyZenVersion {
    param([Parameter(Mandatory = $true)][string]$Workspace)

    $manifestPath = Join-Path $Workspace "crates\keyzen-win\Cargo.toml"
    $manifest = Get-Content -LiteralPath $manifestPath -Raw
    if ($manifest -notmatch '(?m)^version\s*=\s*"([^"]+)"') {
        throw "Could not find package version in $manifestPath"
    }

    return $Matches[1]
}

function Get-HostPackageArch {
    switch ([System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture) {
        "X64" { return "x64" }
        "Arm64" { return "arm64" }
        "X86" { return "x86" }
        default { return $_.ToString().ToLowerInvariant() }
    }
}

function Assert-UnderWorkspace {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Workspace
    )

    $workspaceFull = [System.IO.Path]::GetFullPath($Workspace).TrimEnd('\') + '\'
    $pathFull = [System.IO.Path]::GetFullPath($Path)
    if (-not $pathFull.StartsWith($workspaceFull, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to touch path outside workspace: $pathFull"
    }

    return $pathFull
}

function Remove-WorkspaceItem {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Workspace,
        [switch]$Recurse
    )

    $pathFull = Assert-UnderWorkspace -Path $Path -Workspace $Workspace
    if (Test-Path -LiteralPath $pathFull) {
        if ($Recurse) {
            Remove-Item -LiteralPath $pathFull -Recurse -Force
        } else {
            Remove-Item -LiteralPath $pathFull -Force
        }
    }
}

function Copy-RequiredFile {
    param(
        [Parameter(Mandatory = $true)][string]$Source,
        [Parameter(Mandatory = $true)][string]$Destination
    )

    if (-not (Test-Path -LiteralPath $Source -PathType Leaf)) {
        throw "Required file is missing: $Source"
    }

    Copy-Item -LiteralPath $Source -Destination $Destination
}

function Copy-RequiredDirectory {
    param(
        [Parameter(Mandatory = $true)][string]$Source,
        [Parameter(Mandatory = $true)][string]$Destination
    )

    if (-not (Test-Path -LiteralPath $Source -PathType Container)) {
        throw "Required directory is missing: $Source"
    }

    Copy-Item -LiteralPath $Source -Destination $Destination -Recurse
}

function Get-PeSubsystem {
    param([Parameter(Mandatory = $true)][string]$Path)

    $bytes = [System.IO.File]::ReadAllBytes($Path)
    if ($bytes.Length -lt 0x40) {
        throw "Not a PE file: $Path"
    }

    $lfanew = [System.BitConverter]::ToInt32($bytes, 0x3C)
    $subsystemOffset = $lfanew + 92
    if ($lfanew -le 0 -or ($subsystemOffset + 2) -gt $bytes.Length) {
        throw "Invalid PE header: $Path"
    }
    if ($bytes[$lfanew] -ne 0x50 -or $bytes[$lfanew + 1] -ne 0x45) {
        throw "Missing PE signature: $Path"
    }

    return [System.BitConverter]::ToUInt16($bytes, $subsystemOffset)
}

function Assert-GuiExecutable {
    param([Parameter(Mandatory = $true)][string]$Path)

    $subsystem = Get-PeSubsystem -Path $Path
    if ($subsystem -ne 2) {
        throw "Expected a Windows GUI subsystem executable (starts without a console): $Path (subsystem $subsystem)"
    }
}

function Assert-SingleExecutable {
    param(
        [Parameter(Mandatory = $true)][string]$Directory,
        [Parameter(Mandatory = $true)][string]$Name
    )

    $executables = @(Get-ChildItem -LiteralPath $Directory -Filter "*.exe" -File)
    $names = ($executables | Select-Object -ExpandProperty Name) -join ", "
    if ($executables.Count -ne 1 -or $executables[0].Name -ne $Name) {
        throw "Expected exactly one executable named $Name in $Directory, found: $names"
    }
}

function Stop-LockingKeyZenProcesses {
    param(
        [Parameter(Mandatory = $true)][string]$Workspace,
        [switch]$AllowStop
    )

    $releaseDir = [System.IO.Path]::GetFullPath((Join-Path $Workspace "target\release")).TrimEnd('\') + '\'
    $processes = @(
        Get-Process -Name "keyzen" -ErrorAction SilentlyContinue |
            Where-Object {
                $_.Path -and
                [System.IO.Path]::GetFullPath($_.Path).StartsWith($releaseDir, [System.StringComparison]::OrdinalIgnoreCase)
            }
    )

    if ($processes.Count -eq 0) {
        return
    }

    if (-not $AllowStop) {
        $details = ($processes | ForEach-Object { "PID $($_.Id): $($_.Path)" }) -join [Environment]::NewLine
        throw "KeyZen release executable is already running and may lock target\release. Quit KeyZen or rerun with -StopRunningKeyZen.$([Environment]::NewLine)$details"
    }

    foreach ($process in $processes) {
        Stop-Process -Id $process.Id
    }
}

$workspace = Get-WorkspaceRoot
Set-Location $workspace

$isWindowsHost = [System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
    [System.Runtime.InteropServices.OSPlatform]::Windows
)
if (-not $isWindowsHost) {
    throw "KeyZen package generation currently targets Windows."
}

if ([string]::IsNullOrWhiteSpace($Version)) {
    $Version = Get-KeyZenVersion -Workspace $workspace
}

if ([string]::IsNullOrWhiteSpace($PackageName)) {
    $PackageName = "KeyZen-$Version-windows-$(Get-HostPackageArch)"
}

$distDir = Join-Path $workspace "dist"
$stageDir = Join-Path $distDir $PackageName
$zipPath = Join-Path $distDir "$PackageName.zip"
$hashPath = Join-Path $distDir "SHA256SUMS.txt"
$verifyDir = Join-Path $distDir "_verify-$PackageName"
$releaseDir = Join-Path $workspace "target\release"
$keyzenExe = Join-Path $releaseDir "keyzen.exe"

Write-Host "Packaging KeyZen $Version"
Write-Host "Workspace: $workspace"
Write-Host "Package:   $zipPath"

Get-Command cargo -ErrorAction Stop | Out-Null

Invoke-Step "Checking for running release executables" {
    Stop-LockingKeyZenProcesses -Workspace $workspace -AllowStop:$StopRunningKeyZen
}

if (-not $SkipFmt) {
    Invoke-Step "Formatting" {
        Invoke-CheckedCommand -FilePath "cargo" -Arguments @("fmt", "--all")
    }
}

if (-not $SkipTests) {
    Invoke-Step "Testing" {
        Invoke-CheckedCommand -FilePath "cargo" -Arguments @("test", "--workspace")
    }
}

if (-not $SkipBuild) {
    Invoke-Step "Building release binaries" {
        Invoke-CheckedCommand -FilePath "cargo" -Arguments @("build", "--workspace", "--release")
    }
}

Invoke-Step "Verifying release executable" {
    if (-not (Test-Path -LiteralPath $keyzenExe -PathType Leaf)) {
        throw "Release executable is missing: $keyzenExe"
    }

    Assert-GuiExecutable -Path $keyzenExe
}

Invoke-Step "Staging package files" {
    New-Item -ItemType Directory -Path $distDir -Force | Out-Null
    Remove-WorkspaceItem -Path $stageDir -Workspace $workspace -Recurse
    Remove-WorkspaceItem -Path $zipPath -Workspace $workspace
    Remove-WorkspaceItem -Path $verifyDir -Workspace $workspace -Recurse

    New-Item -ItemType Directory -Path $stageDir | Out-Null
    Copy-RequiredFile -Source $keyzenExe -Destination $stageDir
    Copy-RequiredFile -Source (Join-Path $workspace "README.md") -Destination $stageDir
    Copy-RequiredFile -Source (Join-Path $workspace "CHANGELOG.md") -Destination $stageDir
    Copy-RequiredFile -Source (Join-Path $workspace "LICENSE") -Destination $stageDir
    Copy-RequiredDirectory -Source (Join-Path $workspace "docs") -Destination (Join-Path $stageDir "docs")
    Copy-RequiredDirectory -Source (Join-Path $workspace "examples") -Destination (Join-Path $stageDir "examples")
}

Invoke-Step "Creating zip and SHA-256" {
    Compress-Archive -Path (Join-Path $stageDir "*") -DestinationPath $zipPath -Force
    $hash = Get-FileHash -LiteralPath $zipPath -Algorithm SHA256
    "$($hash.Hash)  $(Split-Path -Leaf $zipPath)" | Set-Content -LiteralPath $hashPath -Encoding ASCII
}

if (-not $SkipPackageVerify) {
    Invoke-Step "Verifying zip contents" {
        New-Item -ItemType Directory -Path $verifyDir | Out-Null
        Expand-Archive -LiteralPath $zipPath -DestinationPath $verifyDir -Force
        Assert-SingleExecutable -Directory $verifyDir -Name "keyzen.exe"
        Assert-GuiExecutable -Path (Join-Path $verifyDir "keyzen.exe")
        Remove-WorkspaceItem -Path $verifyDir -Workspace $workspace -Recurse
    }
}

$zipItem = Get-Item -LiteralPath $zipPath
$hashText = Get-Content -LiteralPath $hashPath -Raw

Write-Host ""
Write-Host "Package complete"
Write-Host "Zip:    $($zipItem.FullName)"
Write-Host "Size:   $($zipItem.Length) bytes"
Write-Host "SHA256: $hashText"
