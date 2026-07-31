# build_frontend.ps1
# Script to build webizen-studio as WASM and stage it for the daemon

$ErrorActionPreference = "Stop"

$repositoryRoot = "$PSScriptRoot/.."
# Release builds must not depend on a machine-specific global excludes file.
# In restricted agent environments Git may be unable to read that file, and
# PowerShell promotes the resulting native stderr warning to a terminating
# error before the actual frontend build can start.
$repositoryExcludeFile = Join-Path (Resolve-Path $repositoryRoot) ".git\info\exclude"
$releaseGitArgs = @("-c", "core.excludesFile=$repositoryExcludeFile", "-C", $repositoryRoot)
$initialStatus = git @releaseGitArgs status --porcelain
if ($LASTEXITCODE -ne 0) {
    throw "Could not inspect the source tree before the frontend build."
}
$sourceTreeDirty = -not [string]::IsNullOrWhiteSpace(($initialStatus -join "`n"))

# Keep in lockstep with crates/webizen-studio/Cargo.toml wasm-bindgen pin.
$WasmBindgenCliVersion = if ($env:WASM_BINDGEN_CLI_VERSION) { $env:WASM_BINDGEN_CLI_VERSION } else { "0.2.125" }
# Keep in lockstep with dioxus-cli 0.8.0-alpha.0/src/esbuild.rs.
$EsbuildVersion = if ($env:ESBUILD_VERSION) { $env:ESBUILD_VERSION } else { "0.27.3" }

Write-Host "Ensuring dioxus-cli is installed..."
if (!(Get-Command "dx" -ErrorAction SilentlyContinue)) {
    Write-Host "Installing dioxus-cli..."
    cargo install dioxus-cli --version 0.8.0-alpha.0 --locked
}

# dx uses whatever wasm-bindgen is on PATH. CLI/crate mismatch fails with:
#   failed to find the `__wbindgen_externref_table_dealloc` function
$needWbInstall = $true
if (Get-Command "wasm-bindgen" -ErrorAction SilentlyContinue) {
    $ver = (& wasm-bindgen --version 2>$null | Out-String).Trim()
    if ($ver -match [regex]::Escape("wasm-bindgen $WasmBindgenCliVersion")) {
        $needWbInstall = $false
    }
}
if ($needWbInstall) {
    Write-Host "Installing wasm-bindgen-cli $WasmBindgenCliVersion (must match crate pin)..."
    cargo install wasm-bindgen-cli --version $WasmBindgenCliVersion --locked --force
}
$cargoBin = if ($env:CARGO_HOME) { Join-Path $env:CARGO_HOME "bin" } else { Join-Path $env:USERPROFILE ".cargo\bin" }
$managedToolDirs = @()
$dxTools = Join-Path $env:USERPROFILE ".dx\tools"
if (Test-Path -LiteralPath $dxTools) {
    $managedEsbuild = Join-Path $dxTools "esbuild-$EsbuildVersion"
    if (Test-Path -LiteralPath $managedEsbuild) {
        $managedToolDirs += $managedEsbuild
    }
    $managedToolDirs += Get-ChildItem -LiteralPath $dxTools -Directory -Filter "binaryen-*" -ErrorAction SilentlyContinue |
        Sort-Object Name -Descending |
        Select-Object -First 1 |
        ForEach-Object { Join-Path $_.FullName "bin" }
}
$toolPath = (@($cargoBin) + @($managedToolDirs) + @($env:Path)) -join ";"
$env:Path = $toolPath
Write-Host "Using wasm-bindgen: $((Get-Command wasm-bindgen).Source) $(wasm-bindgen --version)"

# NO_DOWNLOADS prevents Dioxus from replacing the pinned wasm-bindgen CLI, but
# it also requires esbuild to exist before dx starts. Fresh Windows runners do
# not carry Dioxus' ~/.dx/tools cache, so provision the exact pinned binary.
$esbuildOk = $false
if (Get-Command "esbuild" -ErrorAction SilentlyContinue) {
    $foundEsbuildVersion = (& esbuild --version 2>$null | Out-String).Trim()
    $esbuildOk = $foundEsbuildVersion -eq $EsbuildVersion
}
if (-not $esbuildOk) {
    $npm = Get-Command "npm.cmd" -ErrorAction SilentlyContinue
    if (-not $npm) {
        throw "esbuild $EsbuildVersion is required, but npm.cmd is unavailable."
    }
    Write-Host "Installing esbuild $EsbuildVersion (must match dioxus-cli pin)..."
    & $npm.Source install --global "esbuild@$EsbuildVersion"
    if ($LASTEXITCODE -ne 0) {
        throw "Could not install esbuild $EsbuildVersion."
    }
}
$resolvedEsbuild = Get-Command "esbuild" -ErrorAction SilentlyContinue
if (-not $resolvedEsbuild) {
    throw "esbuild $EsbuildVersion was installed but is not available on PATH."
}
$resolvedEsbuildVersion = (& esbuild --version 2>$null | Out-String).Trim()
if ($resolvedEsbuildVersion -ne $EsbuildVersion) {
    throw "esbuild version $resolvedEsbuildVersion does not match required $EsbuildVersion."
}
Write-Host "Using esbuild: $($resolvedEsbuild.Source) $resolvedEsbuildVersion"

# NO_DOWNLOADS also blocks Dioxus from fetching Binaryen. Fresh Windows runners
# then fail at the end of `dx build` with:
#   Failed to create wasm-opt instance / Missing wasm-opt
# Provision wasm-opt into ~/.dx/tools (or PATH) before NO_DOWNLOADS is set.
$BinaryenVersion = if ($env:BINARYEN_VERSION) { $env:BINARYEN_VERSION } else { "123" }
$wasmOptOk = $false
if (Get-Command "wasm-opt" -ErrorAction SilentlyContinue) {
    $wasmOptOk = $true
    Write-Host "Using wasm-opt on PATH: $((Get-Command wasm-opt).Source)"
}
if (-not $wasmOptOk) {
    $dxToolsRoot = Join-Path $env:USERPROFILE ".dx\tools"
    $binaryenHome = Join-Path $dxToolsRoot "binaryen-version_$BinaryenVersion"
    $wasmOptCandidate = Join-Path $binaryenHome "bin\wasm-opt.exe"
    if (-not (Test-Path -LiteralPath $wasmOptCandidate)) {
        $wasmOptCandidate = Join-Path $binaryenHome "wasm-opt.exe"
    }
    if (-not (Test-Path -LiteralPath $wasmOptCandidate)) {
        New-Item -ItemType Directory -Force -Path $dxToolsRoot | Out-Null
        $archTag = if ($env:PROCESSOR_ARCHITECTURE -match 'ARM64|aarch64') { 'arm64' } else { 'x86_64' }
        $isWindows = $env:OS -eq 'Windows_NT' -or $IsWindows
        $isMac = $IsMacOS -or ($PSVersionTable.OS -match 'Darwin')
        if ($isWindows) {
            $asset = "binaryen-version_$BinaryenVersion-$archTag-windows.tar.gz"
        } elseif ($isMac) {
            $asset = "binaryen-version_$BinaryenVersion-$archTag-macos.tar.gz"
        } else {
            $asset = "binaryen-version_$BinaryenVersion-$archTag-linux.tar.gz"
        }
        $url = "https://github.com/WebAssembly/binaryen/releases/download/version_$BinaryenVersion/$asset"
        $archive = Join-Path $dxToolsRoot $asset
        Write-Host "Downloading Binaryen version_$BinaryenVersion ($asset) for wasm-opt..."
        try {
            Invoke-WebRequest -Uri $url -OutFile $archive -UseBasicParsing
        } catch {
            throw "Could not download Binaryen wasm-opt from $url : $($_.Exception.Message)"
        }
        Write-Host "Extracting Binaryen to $binaryenHome..."
        if (Test-Path -LiteralPath $binaryenHome) {
            Remove-Item -LiteralPath $binaryenHome -Recurse -Force
        }
        # tar is available on modern Windows runners and handles .tar.gz.
        New-Item -ItemType Directory -Force -Path $binaryenHome | Out-Null
        tar -xzf $archive -C $dxToolsRoot
        if ($LASTEXITCODE -ne 0) {
            throw "Failed to extract Binaryen archive $archive"
        }
        # Release tarballs unpack as binaryen-version_N/; normalize if nested.
        if (-not (Test-Path -LiteralPath (Join-Path $binaryenHome "bin")) -and
            -not (Test-Path -LiteralPath (Join-Path $binaryenHome "wasm-opt.exe"))) {
            $unpacked = Get-ChildItem -LiteralPath $dxToolsRoot -Directory -Filter "binaryen-version_*" |
                Sort-Object LastWriteTime -Descending |
                Select-Object -First 1
            if ($unpacked -and $unpacked.FullName -ne $binaryenHome) {
                if (Test-Path -LiteralPath $binaryenHome) {
                    Remove-Item -LiteralPath $binaryenHome -Recurse -Force
                }
                Move-Item -LiteralPath $unpacked.FullName -Destination $binaryenHome
            }
        }
        Remove-Item -LiteralPath $archive -Force -ErrorAction SilentlyContinue
        $wasmOptCandidate = Join-Path $binaryenHome "bin\wasm-opt.exe"
        if (-not (Test-Path -LiteralPath $wasmOptCandidate)) {
            $wasmOptCandidate = Join-Path $binaryenHome "bin\wasm-opt"
        }
        if (-not (Test-Path -LiteralPath $wasmOptCandidate)) {
            $wasmOptCandidate = Join-Path $binaryenHome "wasm-opt.exe"
        }
        if (-not (Test-Path -LiteralPath $wasmOptCandidate)) {
            $wasmOptCandidate = Join-Path $binaryenHome "wasm-opt"
        }
    }
    if (Test-Path -LiteralPath $wasmOptCandidate) {
        $binDir = Split-Path -Parent $wasmOptCandidate
        $env:Path = "$binDir;$env:Path"
        $managedToolDirs += $binDir
        $toolPath = (@($cargoBin) + @($managedToolDirs) + @($env:Path)) -join ";"
        $env:Path = $toolPath
        $wasmOptOk = $true
        Write-Host "Using wasm-opt: $wasmOptCandidate"
    }
}
if (-not $wasmOptOk -or -not (Get-Command "wasm-opt" -ErrorAction SilentlyContinue)) {
    throw "wasm-opt is required for dx build (Binaryen version_$BinaryenVersion). Install Binaryen or set BINARYEN_VERSION."
}
Write-Host "Using wasm-opt: $((Get-Command wasm-opt).Source)"

# Dioxus 0.8 otherwise ignores the verified PATH binary and attempts to
# redownload a managed wasm-bindgen on every invocation. Agents and offline
# builds must use the exact pinned local CLI established above.
$env:NO_DOWNLOADS = "1"
# Host-cpu RUSTFLAGS break wasm32 + wasm-bindgen; clear for frontend only.
if ($env:RUSTFLAGS_WASM) { $env:RUSTFLAGS = $env:RUSTFLAGS_WASM } else { Remove-Item Env:RUSTFLAGS -ErrorAction SilentlyContinue }
Remove-Item Env:CARGO_ENCODED_RUSTFLAGS -ErrorAction SilentlyContinue

Write-Host "Building webizen-studio..."
$publicAssets = "$PSScriptRoot/../target/dx/webizen-studio/release/web/public/assets"
if (Test-Path $publicAssets) {
    Get-ChildItem -LiteralPath $publicAssets -Filter "webizen-studio*" -File |
        Remove-Item -Force
}
$wasmOut = "$PSScriptRoot/../target/dx/webizen-studio/release/web/public/wasm"
if (Test-Path $wasmOut) {
    Remove-Item -LiteralPath $wasmOut -Recurse -Force -ErrorAction SilentlyContinue
}
Push-Location "$PSScriptRoot/../crates/webizen-studio"
$dxBuildCommand = "dx build --web --release 2>&1"
cmd.exe /d /s /c $dxBuildCommand
$dxBuildExitCode = $LASTEXITCODE
if ($dxBuildExitCode -ne 0) {
    Write-Error "Dioxus build failed."
    Pop-Location
    exit $dxBuildExitCode
}
Pop-Location

$public = Resolve-Path "$PSScriptRoot/../target/dx/webizen-studio/release/web/public"
$dist = Resolve-Path "$PSScriptRoot/../crates/webizen-studio/dist"
$assets = Join-Path $dist "assets"
$browserSource = Resolve-Path "$PSScriptRoot/../crates/webizen-desktop/src/browser"

if (-not (Test-Path (Join-Path $public "index.html"))) {
    throw "Dioxus output is missing index.html: $public"
}

New-Item -ItemType Directory -Path $assets -Force | Out-Null
Get-ChildItem -LiteralPath $assets -Filter "webizen-studio*" -File |
    Remove-Item -Force
Copy-Item -LiteralPath (Join-Path $public "index.html") -Destination (Join-Path $dist "index.html") -Force
Copy-Item -Path (Join-Path $public "assets\*") -Destination $assets -Force
Copy-Item -LiteralPath (Join-Path $browserSource "chrome.html") -Destination (Join-Path $dist "browser-chrome.html") -Force
Copy-Item -LiteralPath (Join-Path $browserSource "universe.html") -Destination (Join-Path $dist "chora-universe.html") -Force
$sourceRevision = (git @releaseGitArgs rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($sourceRevision)) {
    throw "Could not determine the source revision for the frontend build."
}
if ($sourceTreeDirty) {
    $sourceRevision = "$sourceRevision-dirty"
}
Set-Content -LiteralPath (Join-Path $dist "source-revision.txt") -Value $sourceRevision -NoNewline

Write-Host "Build complete. Staged fresh desktop assets in crates/webizen-studio/dist."
