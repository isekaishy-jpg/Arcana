param(
    [string]$Base = "",
    [string]$Head = "HEAD"
)

if ($env:ARCANA_ALLOW_LANGUAGE_FREEZE_EXCEPTION -eq "1") {
    Write-Host "language freeze exception enabled"
    exit 0
}

$protected = @(
    "POLICY.md",
    "docs/arcana-v0.md",
    "conformance/selfhost_language_matrix.toml",
    "crates/arcana-syntax/src/freeze.rs",
    "crates/arcana-hir/src/freeze.rs"
)

$changed = @()
if ($Base) {
    $changed = git diff --name-only $Base $Head 2>$null
} else {
    $changed = git status --porcelain | ForEach-Object {
        if ($_.Length -ge 4) { $_.Substring(3) }
    }
}

if (-not $changed) {
    Write-Host "no protected language files changed"
    exit 0
}

$normalized = $changed | ForEach-Object { $_.Replace("\", "/").Trim() } | Where-Object { $_ }
$hits = $normalized | Where-Object { $protected -contains $_ }

if ($hits.Count -gt 0) {
    Write-Error ("language freeze violation: changed protected files without ARCANA_ALLOW_LANGUAGE_FREEZE_EXCEPTION=1:`n" + ($hits -join "`n"))
    exit 1
}

Write-Host "language freeze guard passed"
