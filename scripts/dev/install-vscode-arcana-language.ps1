[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [ValidateSet("stable", "insiders")]
    [string]$Flavor = "stable",

    [switch]$Uninstall
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\\..")).Path
$extensionRoot = Join-Path $repoRoot "editor-support\\vscode\\arcana-language"
$packagePath = Join-Path $extensionRoot "package.json"

if (-not (Test-Path -LiteralPath $packagePath)) {
    throw "Arcana VS Code package was not found at $packagePath"
}

$package = Get-Content -LiteralPath $packagePath -Raw | ConvertFrom-Json
$extensionsHome = if ($Flavor -eq "insiders") {
    Join-Path $env:USERPROFILE ".vscode-insiders\\extensions"
} else {
    Join-Path $env:USERPROFILE ".vscode\\extensions"
}

$installName = "{0}.{1}-repo" -f $package.publisher, $package.name
$installPath = Join-Path $extensionsHome $installName
$resolvedExtensionsHome = [System.IO.Path]::GetFullPath($extensionsHome)
$resolvedInstallPath = [System.IO.Path]::GetFullPath($installPath)

if (-not $resolvedInstallPath.StartsWith($resolvedExtensionsHome, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to modify a path outside the VS Code extensions directory: $resolvedInstallPath"
}

if (-not (Test-Path -LiteralPath $extensionsHome)) {
    if ($PSCmdlet.ShouldProcess($extensionsHome, "Create VS Code extensions directory")) {
        New-Item -ItemType Directory -Path $extensionsHome -Force | Out-Null
    }
}

if ($Uninstall) {
    if (Test-Path -LiteralPath $installPath) {
        if ($PSCmdlet.ShouldProcess($installPath, "Remove Arcana repo-backed VS Code extension link")) {
            Remove-Item -LiteralPath $installPath -Recurse -Force
        }
        Write-Host "Removed $installPath"
    } else {
        Write-Host "No Arcana repo-backed VS Code extension link was installed at $installPath"
    }
    return
}

$linked = $false

if (Test-Path -LiteralPath $installPath) {
    $existing = Get-Item -LiteralPath $installPath -Force
    if (-not ($existing.Attributes -band [System.IO.FileAttributes]::ReparsePoint)) {
        throw "Refusing to replace a non-link directory at $installPath"
    }
    if ($PSCmdlet.ShouldProcess($installPath, "Replace existing Arcana repo-backed VS Code extension link")) {
        Remove-Item -LiteralPath $installPath -Recurse -Force
    }
}

if ($PSCmdlet.ShouldProcess($installPath, "Create junction to $extensionRoot")) {
    New-Item -ItemType Junction -Path $installPath -Target $extensionRoot | Out-Null
    $linked = $true
}

if ($linked) {
    Write-Host "Installed Arcana VS Code language support from repo:"
    Write-Host "  $installPath"
    Write-Host ""
    Write-Host "Reload VS Code to pick up the extension."
    Write-Host "Future edits under $extensionRoot update from this repo; reload the window to see changes."
} elseif ($WhatIfPreference) {
    Write-Host "Dry run complete. No files were changed."
}
