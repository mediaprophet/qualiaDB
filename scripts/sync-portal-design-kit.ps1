# Copy Design Studio + Qualia Portal WASM kit into :8080 static portal (webizen-desktop).
param(
    [string]$DocsRoot = "$PSScriptRoot\..\docs",
    [string]$PortalRoot = "$PSScriptRoot\..\crates\webizen-desktop\static\portal"
)

$ErrorActionPreference = "Stop"

function Copy-Tree($src, $dst) {
    if (-not (Test-Path $src)) {
        Write-Warning "Skip missing: $src"
        return
    }
    New-Item -ItemType Directory -Force -Path $dst | Out-Null
    Copy-Item -Path (Join-Path $src "*") -Destination $dst -Recurse -Force
}

$jsFiles = @(
    "design-studio-app.js",
    "asset-recommendations.js",
    "qualia-shell.js",
    "qualia-wasm-runtime.js",
    "ambient-viz.js",
    "qualia-coi.js",
    "coi-serviceworker.js",
    "menu-loader.js"
)

New-Item -ItemType Directory -Force -Path (Join-Path $PortalRoot "js") | Out-Null
foreach ($f in $jsFiles) {
    $from = Join-Path $DocsRoot "js\$f"
    if (Test-Path $from) {
        Copy-Item $from (Join-Path $PortalRoot "js\$f") -Force
    }
}

Copy-Tree (Join-Path $DocsRoot "pkg\qualia") (Join-Path $PortalRoot "pkg\qualia")
Copy-Tree (Join-Path $DocsRoot "resources") (Join-Path $PortalRoot "resources")
New-Item -ItemType Directory -Force -Path (Join-Path $PortalRoot "css") | Out-Null
Copy-Item (Join-Path $DocsRoot "css\design-studio.css") (Join-Path $PortalRoot "css\design-studio.css") -Force
Copy-Item (Join-Path $DocsRoot "design-studio.html") (Join-Path $PortalRoot "design-studio.html") -Force
Copy-Item (Join-Path $DocsRoot "menu.json") (Join-Path $PortalRoot "menu.json") -Force -ErrorAction SilentlyContinue

Write-Host "Portal design kit synced -> $PortalRoot"
Write-Host "  pkg/qualia, js/*, resources/, design-studio.html"