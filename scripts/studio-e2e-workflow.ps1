# Studio end-to-end workflow (Phase 5) — HTTP surface exercised without GUI automation.
#
# Simulates: cold portal -> edit workspace -> save (WAL) -> restore revision -> pane generate -> spatial shell.
# Prerequisites: Qualia desktop running (settings portal on 127.0.0.1:8080 by default).
param(
    [int]$Port = 8080,
    [string]$BindAddress = "127.0.0.1"
)

$ErrorActionPreference = "Stop"
$base = "http://${BindAddress}:${Port}"

function Assert-Status([int]$code, [string]$label) {
    if ($code -lt 200 -or $code -ge 300) {
        throw "$label failed with HTTP $code"
    }
}

Write-Host "Studio E2E workflow -> $base"

# 1. Cold start / health
$health = Invoke-RestMethod -Uri "$base/health" -Method Get
if ($health.status -ne "ok") { throw "health not ok" }
Write-Host "  [1/7] health ok"

# 2. Edit v1 — single N3 pane
$manifestV1 = @{
    pages = @(
        @{
            url_path = "/"
            name = "Home"
            panes = @(
                @{
                    component_id = "n3-logic-studio"
                    x = 0; y = 0; w = 56; h = 40
                    data_bindings = @("n3:rules")
                }
            )
            presentation_mode = "GridBound"
        }
    )
    theme_tokens = @{}
    themes = @()
    environment_theme = @{}
    app_theme = @{ theme_id = "fiduciary-dark" }
} | ConvertTo-Json -Depth 12 -Compress

$r1 = Invoke-WebRequest -Uri "$base/manifest" -Method Post -Body $manifestV1 -ContentType "application/json"
Assert-Status $r1.StatusCode "manifest POST v1"
Write-Host "  [2/7] saved manifest v1 (N3 pane)"

# 3. Edit v2 — health layout (simulates canvas edit + second save)
$manifestV2 = @{
    pages = @(
        @{
            url_path = "/"
            name = "Home"
            panes = @(
                @{
                    component_id = "health-monitor"
                    x = 0; y = 0; w = 48; h = 40
                    data_bindings = @("fhir:Patient")
                },
                @{
                    component_id = "sparql-explorer"
                    x = 50; y = 0; w = 44; h = 40
                    data_bindings = @("sparql:clinical")
                }
            )
            presentation_mode = "GridBound"
        }
    )
    theme_tokens = @{}
    themes = @()
    environment_theme = @{}
    app_theme = @{ theme_id = "fiduciary-dark" }
} | ConvertTo-Json -Depth 12 -Compress

$r2 = Invoke-WebRequest -Uri "$base/manifest" -Method Post -Body $manifestV2 -ContentType "application/json"
Assert-Status $r2.StatusCode "manifest POST v2"
Write-Host "  [3/7] saved manifest v2 (health layout)"

# 4. WAL history
$history = Invoke-RestMethod -Uri "$base/manifest/history" -Method Get
if ($history.Count -lt 1) { throw "expected WAL history entries" }
$firstRev = $history[0].revision
Write-Host "  [4/7] WAL history: $($history.Count) revisions (first=$firstRev)"

# 5. Replay first revision
$replay = Invoke-WebRequest -Uri "$base/manifest/replay/$firstRev" -Method Post
Assert-Status $replay.StatusCode "manifest replay"
$restored = $replay.Content | ConvertFrom-Json
$restoredJson = $replay.Content
if ($restoredJson -notmatch "n3-logic-studio") {
    throw "replay did not restore v1 n3-logic-studio pane"
}
if ($restoredJson -match "health-monitor") {
    throw "replay incorrectly contains v2 health-monitor pane"
}
Write-Host "  [5/7] replay revision $firstRev restored v1 layout"

# 6. Pane generation API
$genBody = @{
    prompt = "Health tracker with vitals chart"
    palette_ids = @("health-monitor", "sparql-explorer")
} | ConvertTo-Json
$plan = Invoke-RestMethod -Uri "$base/generate_pane" -Method Post -Body $genBody -ContentType "application/json"
if ($plan.panes.Count -lt 1) { throw "generate_pane empty" }
Write-Host "  [6/7] generate_pane: $($plan.panes.Count) panes"

# 7. Quin undo-chain
$undoBody = $manifestV1
Invoke-WebRequest -Uri "$base/manifest/undo-frame?stack_index=0" -Method Post -Body $undoBody -ContentType "application/json" | Out-Null
$chain = Invoke-RestMethod -Uri "$base/manifest/undo-chain" -Method Get
if ($chain.manifests.Count -lt 1) { throw "undo-chain empty" }
Write-Host "  [7/8] undo-chain: $($chain.manifests.Count) frames"

# 8. Offline spatial shell
$studio = Invoke-WebRequest -Uri "$base/design-studio.html" -Method Get
Assert-Status $studio.StatusCode "design-studio.html"
Write-Host "  [8/8] design-studio.html reachable (offline spatial)"

Write-Host ""
Write-Host "Studio E2E workflow: PASS"
Write-Host "  Simulated: edit -> save -> WAL -> replay -> generate -> spatial"
Write-Host "  GUI step (theme picker, drag panes) still manual; this script covers the portal contract."