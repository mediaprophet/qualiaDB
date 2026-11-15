param([string]$RepoRoot = 'C:\github\qualiaDB', [switch]$FinalRepair)
$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath $RepoRoot
$logRoot = $PSScriptRoot
$suffix = if ($FinalRepair) { '-repair' } else { '' }
$buildLog = Join-Path $logRoot ('core-build-handoff-linked' + $suffix + '.log')
& cargo test -p qualia-core-db --lib --no-run --locked --offline -j 1 *> $buildLog
if ($LASTEXITCODE -ne 0) { Write-Output "Core test build failed; see $buildLog"; exit $LASTEXITCODE }
$match = Select-String -LiteralPath $buildLog -Pattern 'Executable.*\((.+\.exe)\)' | Select-Object -Last 1
if (-not $match) { throw 'No current test executable was emitted by Cargo.' }
$binary = (Resolve-Path -LiteralPath $match.Matches[0].Groups[1].Value).Path
$results = @()
$selectors = @('bundle::stream', 'net::peer::replication::qbdl', 'net::qdnf::session::handshake::endpoint', 'net::qdnf::bearer::ipc', 'q42::emf_container', 'q42::m64_matter', 'container_10d::mesh_section', 'net::peer::runtime', 'specialized_libs::cryptographic_library::key_management::managed_object', 'gguf_bridge::gemma4::')
if ($FinalRepair) { $selectors = @('bundle::stream', 'gguf_bridge::gemma4::') }
$failed = $false
foreach ($selector in $selectors) {
    $log = Join-Path $logRoot ('handoff-test-' + $selector.Replace('::','-') + $suffix + '.log')
    & $binary $selector --test-threads=1 *> $log
    $code = $LASTEXITCODE
    $summary = @(Select-String -LiteralPath $log -Pattern '^test result:' | ForEach-Object { $_.Line })
    if ($code -ne 0 -or $summary.Count -eq 0 -or ($summary -join ' ') -match ' 0 passed;') { $failed = $true }
    $results += [pscustomobject]@{ selector=$selector; exit_code=$code; summary=$summary; log=$log }
    $results | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $logRoot ('build-handoff-test-results' + $suffix + '.json'))
    Write-Output "$selector : exit $code : $summary"
}
[pscustomobject]@{ binary=$binary; sha256=(Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash; completed=(Get-Date).ToString('o'); results=$results } | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $logRoot ('build-handoff-execution' + $suffix + '.json'))
if ($failed) { exit 1 }
exit 0
