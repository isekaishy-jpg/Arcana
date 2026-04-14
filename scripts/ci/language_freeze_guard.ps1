param(
    [string]$Base = "",
    [string]$Head = "HEAD"
)

$exceptionFile = ".ci/language_freeze_exceptions.txt"

if ($env:ARCANA_ALLOW_LANGUAGE_FREEZE_EXCEPTION -eq "1") {
    Write-Host "language freeze exception enabled"
    exit 0
}

$protected = @(
    "POLICY.md",
    "docs/arcana-v0.md",
    "conformance/selfhost_language_matrix.toml",
    "crates/arcana-syntax/src/freeze.rs",
    "crates/arcana-hir/src/freeze.rs",
    "docs/specs/spec-status.md",
    "docs/specs/page-rollups/page-rollups/v1-scope.md",
    "docs/specs/tuples/tuples/v1-scope.md",
    "docs/specs/backend/anybox-policy.md",
    "docs/specs/callables/callables/v1-status.md"
)

$changed = @()
if ($Base) {
    $changed = git diff --name-only $Base $Head 2>$null
} else {
    $changed = git status --porcelain=v1 --untracked-files=all | ForEach-Object {
        if ($_.Length -ge 4) { $_.Substring(3) }
    }
}

if (-not $changed) {
    Write-Host "no protected language files changed"
    exit 0
}

$normalized = $changed | ForEach-Object { $_.Replace("\", "/").Trim() } | Where-Object { $_ }
$hits = $normalized | Where-Object { $protected -contains $_ }

$allowed = @()
if (Test-Path $exceptionFile) {
    $allowed = Get-Content $exceptionFile |
        ForEach-Object { $_.Trim() } |
        Where-Object { $_ -and -not $_.StartsWith("#") } |
        ForEach-Object { $_.Replace("\", "/") }
}

$remaining = $hits | Where-Object { $allowed -notcontains $_ }

if ($remaining.Count -gt 0) {
    $message = "language freeze violation: changed protected files without ARCANA_ALLOW_LANGUAGE_FREEZE_EXCEPTION=1"
    if ($allowed.Count -gt 0) {
        $message += " or an entry in $exceptionFile"
    }
    $message += ":`n" + ($remaining -join "`n")
    Write-Error $message
    exit 1
}

Write-Host "language freeze guard passed"
