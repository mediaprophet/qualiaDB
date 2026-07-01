# Studio settings-portal HTTP smoke test (Phase 5).
# Run while the Qualia desktop app is open (settings portal on 127.0.0.1:8080),
# or pass -Port if the portal bound to a different port.
param(
    [int]$Port = 8080,
    [string]$Host = "127.0.0.1"
)

$ErrorActionPreference = "Stop"
$base = "http://${Host}:${Port}"

function Assert-Ok($response, [string]$label) {
    if (-not $response) {
        throw "$label returned no response"
    }
}

Write-Host "Studio portal smoke -> $base"

# 1. Health
$health = Invoke-RestMethod -Uri "$base/health" -Method Get
Assert-Ok $health "health"
if ($health.status -ne "ok") { throw "health status not ok" }
Write-Host "  health: ok (port $($health.port))"

# 2. Generate pane
$genBody = @{
    prompt = "Health tracker with vitals and SPARQL"
    palette_ids = @("health-monitor", "sparql-explorer", "card-view")
} | ConvertTo-Json
$plan = Invoke-RestMethod -Uri "$base/generate_pane" -Method Post -Body $genBody -ContentType "application/json"
Assert-Ok $plan "generate_pane"
if ($plan.panes.Count -lt 1) { throw "generate_pane returned no panes" }
Write-Host "  generate_pane: $($plan.panes.Count) panes, presentation=$($plan.presentation)"

# 3. Manifest round-trip
$manifest = Invoke-RestMethod -Uri "$base/manifest" -Method Get
Assert-Ok $manifest "manifest GET"
$manifestJson = $manifest | ConvertTo-Json -Depth 12 -Compress
if ([string]::IsNullOrWhiteSpace($manifestJson)) {
    $manifestJson = '{"pages":[],"theme_tokens":{},"themes":[],"environment_theme":{},"app_theme":{}}'
}
Invoke-WebRequest -Uri "$base/manifest" -Method Post -Body $manifestJson -ContentType "application/json" | Out-Null
Write-Host "  manifest: GET + POST ok"

# 4. WAL history
$history = Invoke-RestMethod -Uri "$base/manifest/history" -Method Get
Assert-Ok $history "manifest/history"
Write-Host "  manifest/history: $($history.Count) revisions"

# 5. Offline spatial asset (portal WASM bundle)
$studioPage = Invoke-WebRequest -Uri "$base/design-studio.html" -Method Get
if ($studioPage.StatusCode -ne 200) { throw "design-studio.html not served" }
Write-Host "  design-studio.html: offline spatial shell reachable"

Write-Host "Studio portal smoke: PASS"