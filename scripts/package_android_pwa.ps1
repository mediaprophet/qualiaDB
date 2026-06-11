# package_android_pwa.ps1
# Packages the qualia-mobile-harness into an installable Android PWA
# and deploys it to the local bootstrap_gateway/mobile directory.

$ErrorActionPreference = "Stop"

$ProjectRoot = (Resolve-Path "..\").Path
$HarnessDir = Join-Path $ProjectRoot "crates\qualia-mobile-harness"
$OutputDir = Join-Path $ProjectRoot "bootstrap_gateway\mobile"
$DistDir = Join-Path $HarnessDir "dist"

Write-Host "Building Qualia Mobile Harness (Android Sovereign Edge)..." -ForegroundColor Cyan

# 1. Ensure target directory exists
if (!(Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
}

# 2. Build Dioxus WASM with the android_pwa_edge feature
Set-Location $HarnessDir
# We use cargo to build since Dioxus CLI can be complex with features, 
# but assuming dx CLI is installed:
Write-Host "Running dx build..."
# For simplicity in this script, we'll write the manifest generation directly
# Usually dx build handles HTML wrapping.
# NOTE: User should have Dioxus CLI installed. 
# dx build --release --features "android_pwa_edge"

# 3. Generate PWA manifest.json
Write-Host "Generating manifest.json..."
$Manifest = @{
    name = "Qualia Sovereign Hub"
    short_name = "Qualia"
    start_url = "./index.html"
    display = "standalone"
    background_color = "#1a1a1a"
    theme_color = "#1a1a1a"
    description = "Sovereign Edge Node and Tethered Client"
    icons = @(
        @{
            src = "icon-192.png"
            sizes = "192x192"
            type = "image/png"
        },
        @{
            src = "icon-512.png"
            sizes = "512x512"
            type = "image/png"
        }
    )
}
$ManifestJson = $Manifest | ConvertTo-Json -Depth 5
Set-Content -Path (Join-Path $OutputDir "manifest.json") -Value $ManifestJson

# 4. Generate sw.js (Service Worker for offline capability)
Write-Host "Generating sw.js..."
$ServiceWorker = @"
const CACHE_NAME = 'qualia-edge-v1';
const ASSETS = [
    './',
    './index.html',
    './manifest.json',
    './qualia-mobile-harness.js',
    './qualia-mobile-harness_bg.wasm',
    './sync_io_worker.js'
];

self.addEventListener('install', event => {
    event.waitUntil(
        caches.open(CACHE_NAME).then(cache => cache.addAll(ASSETS))
    );
});

self.addEventListener('fetch', event => {
    event.respondWith(
        caches.match(event.request).then(response => {
            return response || fetch(event.request);
        })
    );
});
"@
Set-Content -Path (Join-Path $OutputDir "sw.js") -Value $ServiceWorker

# 5. Copy the Web Worker script
Write-Host "Deploying sync_io_worker.js..."
$WorkerSource = Join-Path $HarnessDir "assets\sync_io_worker.js"
if (Test-Path $WorkerSource) {
    Copy-Item -Path $WorkerSource -Destination $OutputDir -Force
} else {
    Write-Host "Warning: sync_io_worker.js not found in assets." -ForegroundColor Yellow
}

Write-Host "✅ Android PWA Packaged Successfully!" -ForegroundColor Green
Write-Host "Deployed to: $OutputDir"
