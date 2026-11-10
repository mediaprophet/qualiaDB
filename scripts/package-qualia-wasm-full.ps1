# Build the browser LLM/anatomy bundle consumed by docs/online-llm-demo.html.
param(
    [string]$CrateDir = "$PSScriptRoot\..\crates\qualia-core-db",
    [string]$OutputDir = "$PSScriptRoot\..\docs\playground",
    [string]$Features = "wasm-full",
    [string]$OutputName = "qualia_core_db",
    [ValidateSet("Full", "Portal")]
    [string]$FlagProfile = "Full",
    [string]$Toolchain = "1.96.1",
    # Bounds generated output eligible for promotion; Cargo's managed build cache
    # and compiler memory are separate from this deployment artifact budget.
    [long]$ArtifactBudgetBytes = 256MB
)

$ErrorActionPreference = "Stop"

# Windows PowerShell 5.1 (.NET Framework) has no [IO.Path]::GetRelativePath.
function Get-PathRelativeTo([string]$Base, [string]$Target) {
    $baseFull = [IO.Path]::GetFullPath($Base).TrimEnd('\', '/')
    $targetFull = [IO.Path]::GetFullPath($Target)
    $prefix = $baseFull + [IO.Path]::DirectorySeparatorChar
    if ($targetFull.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        return $targetFull.Substring($prefix.Length)
    }
    if ($targetFull.Equals($baseFull, [System.StringComparison]::OrdinalIgnoreCase)) {
        return "."
    }
    throw "Path '$Target' is not under '$Base'"
}

if (-not (Test-Path (Join-Path $CrateDir "Cargo.toml"))) {
    throw "qualia-core-db crate not found at $CrateDir"
}
if (-not (Get-Command wasm-pack -ErrorAction SilentlyContinue)) {
    throw "wasm-pack not found. Install it with: cargo install wasm-pack"
}

$previousRustFlags = $env:RUSTFLAGS
$previousToolchain = $env:RUSTUP_TOOLCHAIN
$scratchParent = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "../target"))
$scratch = Join-Path $scratchParent ("qualia-browser-package-" + [Guid]::NewGuid().ToString("N"))
if ($ArtifactBudgetBytes -le 0) { throw "ArtifactBudgetBytes must be positive" }
New-Item -ItemType Directory -Path $scratch -Force | Out-Null
$marker = Join-Path $scratch ".qualia-browser-package"
Set-Content -LiteralPath $marker -Value "qualia-browser-package" -NoNewline
try {
    $env:RUSTUP_TOOLCHAIN = $Toolchain
    $memoryFlags = if ($FlagProfile -eq "Full") {
        "-C link-arg=-zstack-size=8388608 -C link-arg=--max-memory=4294967296 "
    } else {
        "-C link-arg=-zstack-size=8388608 -C link-arg=--max-memory=1073741824 "
    }
    $env:RUSTFLAGS = "-C target-feature=+simd128 ${memoryFlags}--cfg getrandom_backend=""wasm_js"""
    Push-Location $CrateDir
    try {
        wasm-pack build `
            --target web `
            --release `
            --out-dir $scratch `
            --out-name $OutputName `
            -- `
            --locked `
            --no-default-features `
            --features $Features
        if ($LASTEXITCODE -ne 0) {
            throw "wasm-pack full browser build failed with exit code $LASTEXITCODE"
        }
    } finally {
        Pop-Location
    }
    $files = @(Get-ChildItem -LiteralPath $scratch -Recurse -File | Where-Object {
        $_.Name -match '\.(js|wasm|d\.ts)$' -or
        ($_.Name -eq 'package.json' -and -not (Test-Path -LiteralPath (Join-Path $OutputDir 'package.json')))
    })
    $total = ($files | Measure-Object -Property Length -Sum).Sum
    if ($total -gt $ArtifactBudgetBytes) { throw "Generated browser artifacts exceed the caller-selected byte budget" }
    foreach ($required in @("$OutputName.js", "${OutputName}_bg.wasm", "$OutputName.d.ts", "${OutputName}_bg.wasm.d.ts")) {
        if (-not (Test-Path -LiteralPath (Join-Path $scratch $required))) { throw "Missing generated artifact: $required" }
    }
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
    foreach ($file in $files) {
        $relative = Get-PathRelativeTo $scratch $file.FullName
        $destination = Join-Path $OutputDir $relative
        New-Item -ItemType Directory -Path (Split-Path $destination -Parent) -Force | Out-Null
        Copy-Item -LiteralPath $file.FullName -Destination $destination -Force
    }
} finally {
    $env:RUSTFLAGS = $previousRustFlags
    $env:RUSTUP_TOOLCHAIN = $previousToolchain
    $resolvedScratch = [IO.Path]::GetFullPath($scratch)
    if ($resolvedScratch.StartsWith($scratchParent + [IO.Path]::DirectorySeparatorChar) -and
        (Test-Path -LiteralPath $marker) -and
        (Get-Content -LiteralPath $marker -Raw) -eq "qualia-browser-package") {
        Remove-Item -LiteralPath $resolvedScratch -Recurse -Force
    }
}

$glue = Join-Path $OutputDir "$OutputName.js"
$wasm = Join-Path $OutputDir "${OutputName}_bg.wasm"
if (-not (Test-Path $glue) -or -not (Test-Path $wasm)) {
    throw "Full browser artifacts were not produced in $OutputDir"
}

Write-Host "Qualia $Features WASM bundle built with SIMD128 -> $OutputDir"
