param()

$roots = @("crates", "grimoires", "std", "examples", "conformance")
$patterns = @("\bAnyBox\b", "HandleKind::Any")

$rg = Get-Command rg -ErrorAction SilentlyContinue
if ($null -ne $rg) {
    $hits = & rg --line-number --with-filename --color never -e $patterns[0] -e $patterns[1] @roots 2>$null
} else {
    $files = foreach ($root in $roots) {
        if (Test-Path $root) {
            Get-ChildItem -Path $root -Recurse -File | Select-Object -ExpandProperty FullName
        }
    }
    $hits = @()
    if ($files) {
        $hits = Select-String -Path $files -Pattern $patterns
        $hits = $hits | ForEach-Object { "{0}:{1}:{2}" -f $_.Path, $_.LineNumber, $_.Line.Trim() }
    }
}

if ($hits) {
    Write-Error ("AnyBox policy violation: found forbidden erased-value patterns:`n" + ($hits -join "`n"))
    exit 1
}

Write-Host "anybox policy guard passed"
