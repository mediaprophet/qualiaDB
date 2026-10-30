param(
    [string]$EtalonPath = (Join-Path $PSScriptRoot '../../etalon'),
    [ValidateSet('dx12', 'vulkan', 'metal', 'gl')][string]$Backend,
    [switch]$DiscoveryOnly,
    [switch]$LanguageOnly,
    [switch]$CpuOnly,
    [switch]$NpuOpenVino,
    [switch]$RequireNpu,
    [string]$OrtLibraryPath,
    [string]$OpenVinoEpPath,
    [string]$ReportPath,
    [switch]$Offline,
    [ValidateRange(1, 3600)][int]$TimeoutSeconds = 120,
    [ValidateRange(1, 7200)][int]$BuildTimeoutSeconds = 1800,
    [ValidateRange(1, 16)][int]$BuildJobs = 2,
    [ValidateRange(0.25, 1024)][double]$MinimumFreeGiB = 4
)

$ErrorActionPreference = 'Stop'
$qualiaRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$harnessRoot = Join-Path $qualiaRoot 'target/etalon-vibe-harness'
$runDir = Join-Path $harnessRoot ("runs/" + [DateTime]::UtcNow.ToString('yyyyMMddTHHmmssZ') + '-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $runDir | Out-Null
$localReport = Join-Path $runDir 'report.json'
$summary = [ordered]@{
    schema_version = 2; qualification = 'failed'; started_at_utc = [DateTime]::UtcNow.ToString('o')
    requested = @{ discovery_only = [bool]$DiscoveryOnly; language_only = [bool]$LanguageOnly; cpu_only = [bool]$CpuOnly
        npu_openvino = [bool]$NpuOpenVino; require_npu = [bool]$RequireNpu; backend = $Backend }
    sources = @{}; phases = [Collections.Generic.List[object]]::new()
    lightweight_manifest = (Join-Path $harnessRoot 'inventory/Cargo.toml')
    target_directory = (Join-Path $harnessRoot 'target')
    execution_counts = @{ cpu = @{ passed = 0; failed = 0 }; gpu = @{ passed = 0; failed = 0 }; npu = @{ passed = 0; failed = 0 } }
    count_scope = 'passed requires verified evidence; failed includes failures and missing required product events'
    free_space_checks = [Collections.Generic.List[object]]::new()
    generated_lockfiles = @{}; product_events = @(); failure = $null
    artifacts = @{ run_directory = $runDir; lightweight_manifest = (Join-Path $harnessRoot 'inventory/Cargo.toml')
        product_manifest = (Join-Path $harnessRoot 'Cargo.toml'); report = $localReport }
}
[IO.File]::WriteAllText((Join-Path $runDir '.qualia-etalon-harness-run.json'), '{"kind":"qualia-etalon-vibe-run","schema_version":1}')
function Save-Summary {
    $text = $summary | ConvertTo-Json -Depth 32
    if ([Text.Encoding]::UTF8.GetByteCount($text) -gt 1MB) {
        $summary.qualification = 'failed'; $summary.failure = 'Combined receipt exceeded its 1 MiB budget'
        $summary.product_events = @(); $summary.Remove('lightweight_receipt')
        foreach ($name in @('qualia', 'etalon')) {
            if ($summary.sources.ContainsKey($name)) { $summary.sources[$name].Remove('changed_files') }
            if ($summary.sources_after -and $summary.sources_after.ContainsKey($name)) { $summary.sources_after[$name].Remove('changed_files') }
        }
        $summary.source_file_details_omitted = $true
        $text = $summary | ConvertTo-Json -Depth 32
    }
    [IO.File]::WriteAllText($localReport, $text, [Text.UTF8Encoding]::new($false))
}
Save-Summary

if (-not ('QualiaHarnessBoundedLogV1' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.IO;
public sealed class QualiaHarnessBoundedLogV1 : Stream {
    private readonly FileStream file;
    private readonly long limit;
    private long stored;
    public volatile bool Exceeded;
    public QualiaHarnessBoundedLogV1(string path, long limit) {
        file = new FileStream(path, FileMode.CreateNew, FileAccess.Write, FileShare.Read);
        this.limit = limit;
    }
    public override bool CanRead { get { return false; } }
    public override bool CanSeek { get { return false; } }
    public override bool CanWrite { get { return true; } }
    public override long Length { get { return stored; } }
    public override long Position { get { return stored; } set { throw new NotSupportedException(); } }
    public override void Flush() { file.Flush(); }
    public override int Read(byte[] buffer, int offset, int count) { throw new NotSupportedException(); }
    public override long Seek(long offset, SeekOrigin origin) { throw new NotSupportedException(); }
    public override void SetLength(long value) { throw new NotSupportedException(); }
    public override void Write(byte[] buffer, int offset, int count) {
        int allowed = (int)Math.Min(count, Math.Max(0, limit - stored));
        if (allowed > 0) { file.Write(buffer, offset, allowed); stored += allowed; }
        if (allowed < count) Exceeded = true;
    }
    protected override void Dispose(bool disposing) {
        if (disposing) file.Dispose();
        base.Dispose(disposing);
    }
}
'@
}

function Invoke-OwnedProcess([string]$Name, [string]$Executable, [string[]]$Arguments, [int]$Deadline, [hashtable]$Environment = @{}) {
    $prefix = '{0:D2}-{1}' -f $summary.phases.Count, $Name
    $phase = [ordered]@{ name = $Name; exit_code = -1; timed_out = $false; log_limit_exceeded = $false; free_space_exhausted = $false; cleanup_incomplete = $false
        stdout = (Join-Path $runDir "$prefix.stdout.log"); stderr = (Join-Path $runDir "$prefix.stderr.log") }
    $summary.phases.Add($phase)
    $start = [Diagnostics.ProcessStartInfo]::new()
    $start.FileName = $Executable; $start.UseShellExecute = $false; $start.CreateNoWindow = $true
    $start.RedirectStandardOutput = $true; $start.RedirectStandardError = $true
    $start.WorkingDirectory = $script:etalonRoot
    foreach ($argument in $Arguments) { $start.ArgumentList.Add($argument) }
    foreach ($nameToClear in @('ORT_DYLIB_PATH', 'QUALIA_OPENVINO_EP_PATH', 'QUALIA_ETALON_HARNESS_EVENTS', 'QUALIA_WGPU_BACKEND')) {
        $start.Environment.Remove($nameToClear) | Out-Null
    }
    foreach ($entry in $script:buildEnvironment.GetEnumerator()) { $start.Environment[$entry.Key] = [string]$entry.Value }
    foreach ($entry in $Environment.GetEnumerator()) { $start.Environment[$entry.Key] = [string]$entry.Value }
    $stdout = [QualiaHarnessBoundedLogV1]::new($phase.stdout, 16MB)
    $stderr = [QualiaHarnessBoundedLogV1]::new($phase.stderr, 16MB)
    $process = $null
    $copyCancellation = [Threading.CancellationTokenSource]::new()
    $watch = [Diagnostics.Stopwatch]::StartNew()
    try {
        $process = [Diagnostics.Process]::Start($start)
        $outTask = $process.StandardOutput.BaseStream.CopyToAsync($stdout, 81920, $copyCancellation.Token)
        $errTask = $process.StandardError.BaseStream.CopyToAsync($stderr, 81920, $copyCancellation.Token)
        while (-not $process.WaitForExit(200)) {
            $phase.free_space_exhausted = $Name.EndsWith('-build') -and $script:buildDrive.AvailableFreeSpace -lt 512MB
            if ($stdout.Exceeded -or $stderr.Exceeded -or $watch.Elapsed.TotalSeconds -ge $Deadline -or $phase.free_space_exhausted) {
                $phase.timed_out = $watch.Elapsed.TotalSeconds -ge $Deadline
                $phase.log_limit_exceeded = $stdout.Exceeded -or $stderr.Exceeded
                $process.Kill($true); $phase.cleanup_incomplete = -not $process.WaitForExit(1000); break
            }
        }
        $drains = [Threading.Tasks.Task]::WhenAll([Threading.Tasks.Task[]]@($outTask, $errTask))
        $remaining = [int][Math]::Max(0, [Math]::Ceiling($Deadline * 1000 - $watch.Elapsed.TotalMilliseconds))
        if (-not $drains.Wait($remaining)) {
            $phase.timed_out = $true
            $phase.cleanup_incomplete = $true
            $phase.error = 'Output pipes remained open after the child deadline'
            $copyCancellation.Cancel()
            $process.StandardOutput.Close(); $process.StandardError.Close()
            try { $null = $drains.Wait(1000) } catch { }
        } else { $drains.GetAwaiter().GetResult() }
        $phase.log_limit_exceeded = $stdout.Exceeded -or $stderr.Exceeded
        $phase.exit_code = $process.ExitCode
    } catch {
        $phase.error = $_.Exception.Message
    } finally {
        $copyCancellation.Cancel()
        if ($process) {
            try {
                if (-not $process.HasExited) { $process.Kill($true); $phase.cleanup_incomplete = -not $process.WaitForExit(1000) }
            } catch { $phase.cleanup_incomplete = $true }
            try { $process.StandardOutput.Close(); $process.StandardError.Close() } catch { $phase.cleanup_incomplete = $true }
            $process.Dispose()
        }
        $copyCancellation.Dispose()
        $stdout.Dispose(); $stderr.Dispose()
    }
    return $phase
}
function Assert-Success($Phase) {
    if ($Phase.exit_code -ne 0 -or $Phase.timed_out -or $Phase.log_limit_exceeded -or $Phase.free_space_exhausted -or $Phase.cleanup_incomplete -or $Phase.error) {
        foreach ($streamName in @('stdout', 'stderr')) {
            $tail = Read-LogTail $Phase[$streamName]
            $Phase["${streamName}_tail"] = $tail
            if ($tail) { Write-Host "$($Phase.name) ${streamName} tail (at most 8 KiB):`n$tail" }
        }
        throw "$($Phase.name) failed; inspect $($Phase.stderr) and $($Phase.stdout)"
    }
}
function Read-LogTail([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return '' }
    $file = [IO.File]::Open($Path, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::ReadWrite)
    try {
        $length = [int][Math]::Min(8192, $file.Length)
        [void]$file.Seek(-$length, [IO.SeekOrigin]::End)
        $bytes = [byte[]]::new($length)
        $offset = 0
        while ($offset -lt $length) {
            $read = $file.Read($bytes, $offset, $length - $offset)
            if ($read -eq 0) { break }
            $offset += $read
        }
        return [Text.Encoding]::UTF8.GetString($bytes, 0, $offset)
    } finally { $file.Dispose() }
}
function Get-ProductTestSelection([bool]$CpuOnly) {
    $expected = @{ cpu_graph_transaction_seals_qualia_quins = 'cpu'; cpu_invalid_gpu_handle_is_rejected_through_vibe = 'cpu' }
    if (-not $CpuOnly) { $expected.gpu_compute_readback_matches_cpu_and_rejects_invalid_requests = 'gpu' }
    return @{
        expected = $expected
        arguments = @('--exact', '--nocapture', '--test-threads=1') + @($expected.Keys | Sort-Object)
    }
}
function Read-BoundedJson([string]$Path, [long]$Limit) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw "Required receipt missing: $Path" }
    if ((Get-Item -LiteralPath $Path).Length -gt $Limit) { throw "Receipt exceeds its byte budget: $Path" }
    return [IO.File]::ReadAllText($Path) | ConvertFrom-Json -AsHashtable
}
function Hash-Text([string]$Text) {
    $hash = [Security.Cryptography.SHA256]::Create()
    try { return [BitConverter]::ToString($hash.ComputeHash([Text.Encoding]::UTF8.GetBytes($Text))).Replace('-', '').ToLowerInvariant() }
    finally { $hash.Dispose() }
}
function File-Hash([string]$Path) {
    if (Test-Path -LiteralPath $Path -PathType Leaf) { return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() }
    return $null
}
function Library-Provenance([string]$Path) {
    $file = [IO.File]::OpenRead($Path)
    $hash = [Security.Cryptography.IncrementalHash]::CreateHash([Security.Cryptography.HashAlgorithmName]::SHA256)
    try {
        if ($file.Length -eq 0 -or $file.Length -gt 1GB) { throw 'NPU library must be nonempty and at most 1 GiB' }
        $buffer = [byte[]]::new(64KB); $total = 0L; $watch = [Diagnostics.Stopwatch]::StartNew()
        while (($read = $file.Read($buffer, 0, $buffer.Length)) -gt 0) {
            $total += $read
            if ($total -gt 1GB -or $watch.Elapsed.TotalSeconds -gt 60) { throw 'NPU library fingerprint budget exceeded' }
            $hash.AppendData($buffer, 0, $read)
        }
        return @{ path = $Path; bytes = $total; sha256 = [BitConverter]::ToString($hash.GetHashAndReset()).Replace('-', '').ToLowerInvariant() }
    } finally { $hash.Dispose(); $file.Dispose() }
}
function Source-Identity([string]$Name, [string]$Root) {
    $headPhase = Invoke-OwnedProcess "$Name-head" 'git' @('-C', $Root, 'rev-parse', 'HEAD') 30
    Assert-Success $headPhase
    $head = [IO.File]::ReadAllText($headPhase.stdout).Trim()
    $statusPhase = Invoke-OwnedProcess "$Name-status" 'git' @('-C', $Root, 'status', '--porcelain=v1', '-z', '--untracked-files=all') 30
    Assert-Success $statusPhase
    $status = [IO.File]::ReadAllText($statusPhase.stdout)
    $entries = $status.Split([char[]]@([char]0), [StringSplitOptions]::RemoveEmptyEntries)
    $files = [Collections.Generic.List[object]]::new()
    $bytes = 0L; $exact = $true
    for ($index = 0; $index -lt $entries.Count; $index++) {
        $entry = $entries[$index]
        if ($entry.Length -lt 4) { throw "Invalid Git status record in $Root" }
        $relative = $entry.Substring(3)
        $original = $null
        if ($entry.Substring(0, 2).Contains('R') -or $entry.Substring(0, 2).Contains('C')) { $index++; $original = $entries[$index] }
        if ($files.Count -ge 4096) { $exact = $false; break }
        $path = [IO.Path]::GetFullPath((Join-Path $Root $relative))
        if (-not $path.StartsWith($Root + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Source path escaped its repository' }
        $digest = 'deleted'
        if (Test-Path -LiteralPath $path) {
            $item = Get-Item -LiteralPath $path -Force
            if ($item.LinkType) { $digest = Hash-Text ([string]$item.LinkTarget) }
            elseif (-not $item.PSIsContainer) {
                $bytes += $item.Length
                if ($bytes -gt 256MB) { $exact = $false; break }
                $digest = File-Hash $path
            } else { $digest = 'directory-or-submodule'; $exact = $false }
        }
        $files.Add([ordered]@{ path = $relative; original_path = $original; status = $entry.Substring(0, 2); sha256 = $digest })
    }
    $workingHash = if ($exact) { Hash-Text ($head + ($files | ConvertTo-Json -Depth 4 -Compress)) } else { $null }
    return @{ path = $Root; head = $head; working_tree = $(if ($status.Length) { 'dirty' } else { 'clean' })
        working_hash = $workingHash; contents_fully_fingerprinted = $exact; changed_files = $files.ToArray() }
}
function Toml-String([string]$Value) { return '"' + $Value.Replace('\', '/').Replace('"', '\"') + '"' }
function Write-Manifest([string]$Path, [string]$Text) {
    New-Item -ItemType Directory -Force -Path (Split-Path $Path) | Out-Null
    if (-not (Test-Path -LiteralPath $Path) -or [IO.File]::ReadAllText($Path) -cne $Text) { [IO.File]::WriteAllText($Path, $Text, [Text.UTF8Encoding]::new($false)) }
}
function Assert-FreeSpace([string]$Stage) {
    $available = $script:buildDrive.AvailableFreeSpace
    $summary.free_space_checks.Add(@{ stage = $Stage; available_bytes = $available; required_bytes = $MinimumFreeGiB * 1GB })
    if ($available -lt $MinimumFreeGiB * 1GB) { throw "$Stage requires at least $MinimumFreeGiB GiB free; no build was started" }
}
function Find-Artifact($Phase, [string]$Name) {
    Assert-Success $Phase
    $artifact = $null
    foreach ($line in [IO.File]::ReadLines($Phase.stdout)) {
        try { $item = $line | ConvertFrom-Json -AsHashtable } catch { continue }
        if ($item.reason -eq 'compiler-artifact' -and $item.target.name -eq $Name -and $item.executable) { $artifact = $item.executable }
    }
    if (-not $artifact -or -not (Test-Path -LiteralPath $artifact -PathType Leaf)) { throw "Cargo did not produce $Name" }
    return $artifact
}

try {
    if (($DiscoveryOnly -and ($LanguageOnly -or $CpuOnly -or $NpuOpenVino -or $RequireNpu)) -or ($LanguageOnly -and $CpuOnly) -or ($CpuOnly -and ($NpuOpenVino -or $RequireNpu))) {
        throw 'DiscoveryOnly, LanguageOnly and CpuOnly select distinct scopes; NPU execution requires a non-CPU-only scope'
    }
    $script:etalonRoot = (Resolve-Path -LiteralPath $EtalonPath).Path
    $script:buildEnvironment = @{ CARGO_TARGET_DIR = (Join-Path $harnessRoot 'target'); CARGO_BUILD_JOBS = $BuildJobs
        CARGO_PROFILE_DEV_DEBUG = '0'; CARGO_INCREMENTAL = '0'; CARGO_TERM_COLOR = 'never' }
    $script:buildDrive = [IO.DriveInfo]::GetDrives() | Where-Object { $_.IsReady -and $runDir.StartsWith($_.RootDirectory.FullName, [StringComparison]::OrdinalIgnoreCase) } |
        Sort-Object { $_.RootDirectory.FullName.Length } -Descending | Select-Object -First 1
    if (-not $script:buildDrive) { throw 'Cannot establish free space on the build volume' }
    $summary.free_space_before_bytes = $script:buildDrive.AvailableFreeSpace
    Assert-FreeSpace 'preflight'
    if ($NpuOpenVino) {
        foreach ($providerPath in @($OrtLibraryPath, $OpenVinoEpPath)) {
            if (-not $providerPath -or -not (Test-Path -LiteralPath $providerPath -PathType Leaf)) { throw 'NpuOpenVino requires explicit existing OrtLibraryPath and OpenVinoEpPath files' }
        }
        $OrtLibraryPath = (Resolve-Path -LiteralPath $OrtLibraryPath).Path
        $OpenVinoEpPath = (Resolve-Path -LiteralPath $OpenVinoEpPath).Path
        $summary.npu_libraries = @{ runtime = Library-Provenance $OrtLibraryPath; provider = Library-Provenance $OpenVinoEpPath }
    } elseif ($OrtLibraryPath -or $OpenVinoEpPath) { throw 'Provider paths require NpuOpenVino' }
    $summary.sources.qualia = Source-Identity 'qualia-start' $qualiaRoot
    $summary.sources.etalon = Source-Identity 'etalon-start' $etalonRoot
    $productionLocks = @{ qualia = File-Hash (Join-Path $qualiaRoot 'Cargo.lock'); etalon = File-Hash (Join-Path $etalonRoot 'Cargo.lock') }
    $metadataArgs = @('metadata', '--manifest-path', (Join-Path $etalonRoot 'Cargo.toml'), '--no-deps', '--locked', '--format-version', '1')
    if ($Offline) { $metadataArgs += '--offline' }
    $metadataPhase = Invoke-OwnedProcess 'etalon-metadata' 'cargo' $metadataArgs 120
    Assert-Success $metadataPhase
    $metadata = Read-BoundedJson $metadataPhase.stdout 16MB
    $patches = foreach ($name in @('wgpu', 'naga', 'etalon', 'etalon-agent', 'etalon-wgsl-forge', 'etalon-nquin', 'etalon-tensor', 'etalon-render', 'etalon-wasm')) {
        $package = @($metadata.packages | Where-Object name -EQ $name)
        if ($package.Count -ne 1) { throw "Expected one local Etalon package named $name" }
        "$name = { path = $(Toml-String (Split-Path $package[0].manifest_path)) }"
    }
    $patchBlock = '[patch."https://github.com/mediaprophet/etalon.git"]' + [Environment]::NewLine + ($patches -join [Environment]::NewLine)
    $template = [IO.File]::ReadAllText((Join-Path $qualiaRoot 'tools/accelerator-harness/Cargo.toml'))
    $oldVibe = 'vibe = { path = "../../crates/vibe" }'
    if (-not $template.Contains($oldVibe)) { throw 'Lightweight manifest Vibe path contract changed' }
    $lightManifest = $template.Replace($oldVibe, "vibe = { path = $(Toml-String (Join-Path $qualiaRoot 'crates/vibe')) }")
    $lightManifest += [Environment]::NewLine + $patchBlock + [Environment]::NewLine + '[[bin]]' + [Environment]::NewLine +
        'name = "qualia-accelerator-harness"' + [Environment]::NewLine +
        "path = $(Toml-String (Join-Path $qualiaRoot 'tools/accelerator-harness/src/main.rs'))" + [Environment]::NewLine
    Write-Manifest $summary.artifacts.lightweight_manifest $lightManifest
    if (($IsWindows -or $env:OS -eq 'Windows_NT') -and -not $env:AR) {
        $rustPhase = Invoke-OwnedProcess 'rust-sysroot' 'rustc' @('--print', 'sysroot') 30
        Assert-Success $rustPhase
        $versionPhase = Invoke-OwnedProcess 'rust-version' 'rustc' @('-vV') 30
        Assert-Success $versionPhase
        $sysroot = [IO.File]::ReadAllText($rustPhase.stdout).Trim()
        $hostLine = [IO.File]::ReadLines($versionPhase.stdout) | Where-Object { $_ -like 'host: *' }
        $hostTriple = $hostLine.Substring(6)
        foreach ($candidate in @((Join-Path $sysroot "lib/rustlib/$hostTriple/bin/llvm-ar.exe"), (Join-Path (Split-Path $sysroot) "stable-$hostTriple/lib/rustlib/$hostTriple/bin/llvm-ar.exe"))) {
            if (Test-Path -LiteralPath $candidate -PathType Leaf) { $script:buildEnvironment.AR = $candidate; break }
        }
    }
    $lightArgs = @('build', '--manifest-path', $summary.artifacts.lightweight_manifest, '--bin', 'qualia-accelerator-harness', '--message-format=json')
    if ($Offline) { $lightArgs += '--offline' }; if ($NpuOpenVino) { $lightArgs += @('--features', 'npu-openvino') }
    Write-Host "Building lightweight Vibe/discovery harness against $etalonRoot"
    Assert-FreeSpace 'lightweight-build'
    $lightBuild = Invoke-OwnedProcess 'lightweight-build' 'cargo' $lightArgs $BuildTimeoutSeconds
    $lightBinary = Find-Artifact $lightBuild 'qualia-accelerator-harness'
    $lightReceiptPath = Join-Path $runDir 'lightweight.json'
    $summary.artifacts.lightweight_receipt = $lightReceiptPath
    $runArgs = @('--output', $lightReceiptPath)
    if ($DiscoveryOnly) { $runArgs += '--discovery-only' }; if ($RequireNpu) { $runArgs += '--require-npu' }; if ($NpuOpenVino) { $runArgs += '--npu-openvino' }
    $runtimeEnv = @{}
    if ($NpuOpenVino) { $runtimeEnv.ORT_DYLIB_PATH = $OrtLibraryPath; $runtimeEnv.QUALIA_OPENVINO_EP_PATH = $OpenVinoEpPath }
    $lightRun = Invoke-OwnedProcess 'lightweight-execution' $lightBinary $runArgs $TimeoutSeconds $runtimeEnv
    $lightReceipt = $null
    if (Test-Path -LiteralPath $lightReceiptPath) {
        $lightReceipt = Read-BoundedJson $lightReceiptPath 64KB
        $summary.lightweight_receipt = $lightReceipt
        if ($lightReceipt.language.execution -eq 'verified') { $summary.execution_counts.cpu.passed += [int]$lightReceipt.language.passed }
        if ($lightReceipt.language.execution -eq 'failed') { $summary.execution_counts.cpu.failed += 1 }
        if ($lightReceipt.npu_execution.execution -eq 'verified' -and $NpuOpenVino) { $summary.execution_counts.npu.passed += 1 }
        if ($lightReceipt.npu_execution.execution -eq 'failed') { $summary.execution_counts.npu.failed += 1 }
    }
    Assert-Success $lightRun
    if (-not $lightReceipt -or $lightReceipt.qualification -ne 'requested_checks_complete') { throw 'Lightweight execution receipt is missing or unsuccessful' }
    if (-not ($DiscoveryOnly -or $LanguageOnly)) {
        $fullManifest = @"
[workspace]
[package]
name = "qualia-etalon-vibe-harness"
version = "0.0.0"
edition = "2021"
publish = false
[features]
default = ["gpu-runtime"]
gpu-runtime = []
dx12 = ["qualia-core-db/gpu-native-dx12"]
[dependencies]
qualia-core-db = { path = $(Toml-String (Join-Path $qualiaRoot 'crates/qualia-core-db')), default-features = false, features = ["gpu-runtime", "wgsl-forge"] }
vibe = { path = $(Toml-String (Join-Path $qualiaRoot 'crates/vibe')) }
serde_json = "1"
[[test]]
name = "etalon_vibe_harness"
path = $(Toml-String (Join-Path $qualiaRoot 'crates/qualia-core-db/tests/etalon_vibe_harness.rs'))
$patchBlock
[profile.dev]
debug = 0
incremental = false
"@
        Write-Manifest $summary.artifacts.product_manifest $fullManifest
        $fullArgs = @('test', '--manifest-path', $summary.artifacts.product_manifest, '--test', 'etalon_vibe_harness', '--no-run', '--message-format=json')
        if ($IsWindows -or $env:OS -eq 'Windows_NT') { $fullArgs += @('--features', 'dx12') }; if ($Offline) { $fullArgs += '--offline' }
        Assert-FreeSpace 'product-build'
        $fullBuild = Invoke-OwnedProcess 'product-build' 'cargo' $fullArgs $BuildTimeoutSeconds
        $fullBinary = Find-Artifact $fullBuild 'etalon_vibe_harness'
        $eventsPath = Join-Path $runDir 'product-events.jsonl'; $summary.artifacts.product_events = $eventsPath
        $selection = Get-ProductTestSelection ([bool]$CpuOnly)
        $expected = $selection.expected
        $fullRunArgs = $selection.arguments
        $fullEnv = @{ QUALIA_ETALON_HARNESS_EVENTS = $eventsPath }; if ($Backend) { $fullEnv.QUALIA_WGPU_BACKEND = $Backend }
        $fullRun = Invoke-OwnedProcess 'product-execution' $fullBinary $fullRunArgs $TimeoutSeconds $fullEnv
        $seen = @{}; $events = [Collections.Generic.List[object]]::new()
        if (Test-Path -LiteralPath $eventsPath) {
            if ((Get-Item -LiteralPath $eventsPath).Length -gt 1MB) { throw 'Product event receipt exceeds 1 MiB' }
            try {
                foreach ($line in [IO.File]::ReadLines($eventsPath)) {
                    if (-not $line.Trim()) { continue }; $event = $line | ConvertFrom-Json -AsHashtable
                    if (-not $expected.ContainsKey($event.name) -or $seen.ContainsKey($event.name) -or $event.execution_class -ne $expected[$event.name] -or $event.outcome -ne 'passed') {
                        throw 'Unexpected, duplicate or unsuccessful product execution event'
                    }
                    $seen[$event.name] = $true; $events.Add($event); $summary.execution_counts[$event.execution_class].passed += 1
                }
            } finally {
                $summary.product_events = $events.ToArray()
            }
        }
        $summary.product_events = $events.ToArray()
        $missing = @($expected.Keys | Where-Object { -not $seen.ContainsKey($_) })
        foreach ($name in $missing) { $summary.execution_counts[$expected[$name]].failed += 1 }
        if ($missing.Count) { $summary.missing_required_events = $missing }
        Assert-Success $fullRun
        if ($missing.Count) { throw "Missing required product execution events: $($missing -join ', ')" }
    }
    $summary.qualification = 'requested_checks_complete'
} catch {
    $summary.qualification = 'failed'; $summary.failure = $_.Exception.Message
} finally {
    foreach ($pair in @(@('lightweight', $summary.artifacts.lightweight_manifest), @('product', $summary.artifacts.product_manifest))) {
        $summary.generated_lockfiles[$pair[0]] = File-Hash (Join-Path (Split-Path $pair[1]) 'Cargo.lock')
    }
    if ($summary.sources.Count -eq 2) {
        try {
            $finalQualia = Source-Identity 'qualia-end' $qualiaRoot; $finalEtalon = Source-Identity 'etalon-end' $script:etalonRoot
            $summary.sources_after = @{ qualia = $finalQualia; etalon = $finalEtalon }
            foreach ($name in @('qualia', 'etalon')) {
                if ($summary.sources[$name].head -ne $summary.sources_after[$name].head -or $summary.sources[$name].working_hash -ne $summary.sources_after[$name].working_hash) {
                    $summary.qualification = 'failed'; $summary.failure = 'Source identity changed during qualification'
                }
            }
            $summary.production_locks_unchanged = ($productionLocks.qualia -eq (File-Hash (Join-Path $qualiaRoot 'Cargo.lock'))) -and ($productionLocks.etalon -eq (File-Hash (Join-Path $script:etalonRoot 'Cargo.lock')))
            if (-not $summary.production_locks_unchanged) { $summary.qualification = 'failed'; $summary.failure = 'Production lockfile changed during qualification' }
        } catch { $summary.qualification = 'failed'; $summary.failure = $_.Exception.Message }
    }
    if ($summary.npu_libraries) {
        try {
            $summary.npu_libraries_after = @{ runtime = Library-Provenance $OrtLibraryPath; provider = Library-Provenance $OpenVinoEpPath }
            foreach ($name in @('runtime', 'provider')) {
                if ($summary.npu_libraries[$name].sha256 -ne $summary.npu_libraries_after[$name].sha256) {
                    $summary.qualification = 'failed'; $summary.failure = 'NPU runtime or provider library changed during qualification'
                }
            }
        } catch { $summary.qualification = 'failed'; $summary.failure = $_.Exception.Message }
    }
    $summary.finished_at_utc = [DateTime]::UtcNow.ToString('o')
    Save-Summary
    if ($ReportPath) {
        try {
            $export = [IO.Path]::GetFullPath($ReportPath)
            New-Item -ItemType Directory -Force -Path (Split-Path $export) | Out-Null
            if ($export -cne $localReport) { [IO.File]::WriteAllText($export, [IO.File]::ReadAllText($localReport), [Text.UTF8Encoding]::new($false)) }
        } catch { Write-Warning "Report export failed; retained receipt: $localReport"; throw }
    }
    Write-Host "Qualification: $($summary.qualification); receipt: $localReport"
}
if ($summary.qualification -ne 'requested_checks_complete') { throw $summary.failure }
