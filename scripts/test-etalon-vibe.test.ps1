$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
$tokens = $null; $parseErrors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot 'test-etalon-vibe.ps1'), [ref]$tokens, [ref]$parseErrors)
if ($parseErrors.Count) { throw ($parseErrors | Out-String) }
foreach ($name in @('Read-LogTail', 'Get-ProductTestSelection', 'Toml-String', 'Get-EtalonPatchBlock')) {
    $function = $ast.Find({ param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq $name }, $true)
    if (-not $function) { throw "Missing harness function: $name" }
    . ([scriptblock]::Create($function.Extent.Text))
}
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$packageNames = @('wgpu', 'naga', 'etalon', 'etalon-agent', 'etalon-wgsl-forge', 'etalon-nquin', 'etalon-tensor', 'etalon-render', 'etalon-wasm',
    'etalon-model', 'etalon-resources', 'etalon-diagnostics', 'etalon-inference', 'future-etalon-package')
$packages = @($packageNames | ForEach-Object { @{ name = $_; id = "path+file:///etalon/$_#0.0.2"; manifest_path = (Join-Path $root "target/fixture-etalon/$_/Cargo.toml") } })
$metadata = @{ packages = $packages; workspace_members = @($packages | ForEach-Object { $_.id }) }
$patchBlock = Get-EtalonPatchBlock $metadata
foreach ($name in $packageNames) {
    if ($patchBlock -notmatch "(?m)^$([regex]::Escape($name)) = \{ path = ") { throw "Missing local workspace override: $name" }
}
$metadata.packages = @($packages | Sort-Object name -Descending)
if ((Get-EtalonPatchBlock $metadata) -cne $patchBlock) { throw 'Local overrides must be deterministic' }
$metadata.workspace_members += 'missing-member'
try { $null = Get-EtalonPatchBlock $metadata; throw 'Incomplete workspace accepted' } catch {
    if ($_.Exception.Message -ne 'Etalon workspace metadata is incomplete') { throw }
}
$metadata.workspace_members = @($packages | ForEach-Object { $_.id })
$metadata.packages = $packages + @($packages[0])
$metadata.workspace_members += $packages[0].id
try { $null = Get-EtalonPatchBlock $metadata; throw 'Duplicate package accepted' } catch {
    if ($_.Exception.Message -notlike 'Duplicate Etalon workspace package:*') { throw }
}
$metadata.packages = @($packages | Where-Object name -NE 'etalon-agent')
$metadata.workspace_members = @($metadata.packages | ForEach-Object { $_.id })
try { $null = Get-EtalonPatchBlock $metadata; throw 'Required package omission accepted' } catch {
    if ($_.Exception.Message -ne 'Expected one local Etalon package named etalon-agent') { throw }
}
$reviewedRevision = (Get-Content (Join-Path $root 'tools/accelerator-harness/etalon-revision.txt') -Raw).Trim()
$harnessManifest = Get-Content (Join-Path $root 'tools/accelerator-harness/Cargo.toml') -Raw
if ($reviewedRevision -notmatch '^[0-9a-f]{40}$' -or $harnessManifest -notmatch ('etalon-agent = .*rev = "' + $reviewedRevision + '"')) {
    throw 'Harness agent dependency must match the reviewed checkout revision'
}
$fixture = Join-Path $root ('target/qualification/harness-cli-test-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $fixture | Out-Null
$log = Join-Path $fixture 'tail.log'
[IO.File]::WriteAllText($log, ('x' * 20000) + 'EXPECTED-END')
$tail = Read-LogTail $log
if ($tail.Length -ne 8192 -or -not $tail.EndsWith('EXPECTED-END')) { throw 'Log tail must retain only the final 8 KiB' }
foreach ($text in @('small', '')) {
    [IO.File]::WriteAllText($log, $text)
    if ((Read-LogTail $log) -cne $text) { throw 'Short log tail changed contents' }
}
if ((Read-LogTail ($log + '.missing')) -cne '') { throw 'Missing log must return an empty tail' }

$source = Join-Path $fixture 'selection.rs'
$binary = Join-Path $fixture $(if ($IsWindows -or $env:OS -eq 'Windows_NT') { 'selection.exe' } else { 'selection' })
[IO.File]::WriteAllText($source, @'
#[test]
fn cpu_graph_transaction_seals_qualia_quins() {}
#[test]
fn cpu_invalid_gpu_handle_is_rejected_through_vibe() {}
#[test]
fn gpu_compute_readback_matches_cpu_and_rejects_invalid_requests() {
    panic!("GPU execution must not run in CPU-only mode");
}
'@)
& rustc +1.96.1 --test --edition 2021 $source -o $binary
if ($LASTEXITCODE -ne 0) { throw 'Tiny libtest selection fixture failed to compile' }
$cpu = Get-ProductTestSelection $true
if ($cpu.expected.Count -ne 2 -or @($cpu.expected.Values | Where-Object { $_ -ne 'cpu' }).Count) { throw 'CPU-only selection must require two CPU receipts' }
& $binary @($cpu.arguments) *> (Join-Path $fixture 'cpu.log')
if ($LASTEXITCODE -ne 0) { throw 'CPU-only selection ran the GPU test or lost a CPU test' }
$cpuOutput = [IO.File]::ReadAllText((Join-Path $fixture 'cpu.log'))
if ($cpuOutput -notmatch '2 passed; 0 failed' -or $cpuOutput -notmatch '1 filtered out') { throw 'CPU-only selection must execute exactly two tests' }
$full = Get-ProductTestSelection $false
if ($full.expected.Count -ne 3) { throw 'Full selection must require all three receipts' }
& $binary @($full.arguments) *> (Join-Path $fixture 'full.log')
if ($LASTEXITCODE -eq 0) { throw 'Full selection lost mandatory GPU execution' }
$fullOutput = [IO.File]::ReadAllText((Join-Path $fixture 'full.log'))
if ($fullOutput -notmatch '2 passed; 1 failed' -or $fullOutput -notmatch '0 filtered out') { throw 'Full selection must execute the GPU test too' }
& $binary cpu_ *> (Join-Path $fixture 'old-filter.log')
if ($LASTEXITCODE -eq 0) { throw 'Regression fixture must reproduce the unsafe substring selection' }
Write-Host "Harness selection and bounded log-tail regressions passed; evidence: $fixture"
exit 0
