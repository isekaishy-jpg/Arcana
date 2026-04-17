param(
    [switch]$Check
)

$ErrorActionPreference = "Stop"
$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\\..")
Push-Location $repoRoot
try {
    $mode = if ($Check) { "--check" } else { "--write" }
    cargo run -q -p arcana-winapi-gen -- $mode
}
finally {
    Pop-Location
}
