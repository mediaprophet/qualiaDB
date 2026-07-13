param(
  [string]$ArtifactsDir = "release-assets",
  [string]$Version = "",
  [string]$Tag = "",
  [string]$Repo = "mediaprophet/qualiaDB",
  [string]$Output = "webizen-latest.json",
  [string]$Notes = "Webizen Desktop update"
)

$ErrorActionPreference = "Stop"

function Read-WebizenVersion {
  $configPath = Join-Path $PSScriptRoot "..\crates\webizen-desktop\tauri.conf.json"
  $config = Get-Content $configPath -Raw | ConvertFrom-Json
  return [string]$config.version
}

function Escape-ReleaseAssetName([string]$Name) {
  return [Uri]::EscapeDataString($Name).Replace("%2F", "/")
}

function Find-SignedArtifact {
  param(
    [array]$Files,
    [string]$Platform,
    [string[]]$NamePatterns,
    [string[]]$PreferredSuffixes
  )

  $candidates = @()
  foreach ($file in $Files) {
    if ($file.Name.EndsWith(".sig")) {
      continue
    }
    $sigPath = "$($file.FullName).sig"
    if (-not (Test-Path $sigPath)) {
      continue
    }
    $name = $file.Name.ToLowerInvariant()
    $haystack = "$($file.FullName) $($file.Name)".ToLowerInvariant()
    $matchedPlatform = $false
    foreach ($pattern in $NamePatterns) {
      if ($haystack -match $pattern) {
        $matchedPlatform = $true
        break
      }
    }
    if (-not $matchedPlatform) {
      continue
    }

    $rank = 100
    for ($i = 0; $i -lt $PreferredSuffixes.Length; $i++) {
      if ($name.EndsWith($PreferredSuffixes[$i])) {
        $rank = $i
        break
      }
    }
    $candidates += [pscustomobject]@{
      File = $file
      SignaturePath = $sigPath
      Rank = $rank
      Length = $file.Length
      Platform = $Platform
    }
  }

  return $candidates | Sort-Object Rank, Length -Descending | Select-Object -First 1
}

if (-not $Version) {
  $Version = Read-WebizenVersion
}
if (-not $Tag) {
  $Tag = "v$Version"
}

$root = Resolve-Path $ArtifactsDir
$files = @(Get-ChildItem $root -Recurse -File)

$platforms = [ordered]@{}
$selected = @()
$windowsArtifact = Find-SignedArtifact `
  -Files $files `
  -Platform "windows-x86_64" `
  -NamePatterns @("windows", "win32", "win64", "x64", "x86_64", "setup", "nsis") `
  -PreferredSuffixes @(".nsis.zip", "-setup.exe", ".exe", ".zip")
if ($null -ne $windowsArtifact) {
  $selected += $windowsArtifact
}

$macArtifact = Find-SignedArtifact `
  -Files $files `
  -Platform "darwin-aarch64" `
  -NamePatterns @("darwin", "macos", "aarch64", "apple", "metal", "app\.tar\.gz") `
  -PreferredSuffixes @(".app.tar.gz", ".tar.gz", ".zip", ".dmg")
if ($null -ne $macArtifact) {
  $selected += $macArtifact
}

foreach ($entry in $selected) {
  $assetName = $entry.File.Name
  $signature = (Get-Content $entry.SignaturePath -Raw).Trim()
  $platforms[$entry.Platform] = [ordered]@{
    signature = $signature
    url = "https://github.com/$Repo/releases/download/$Tag/$(Escape-ReleaseAssetName $assetName)"
  }
}

if ($platforms.Count -eq 0) {
  throw "No signed updater artifacts found under $root"
}

$manifest = [ordered]@{
  version = $Version
  notes = $Notes
  pub_date = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
  platforms = $platforms
}

$json = $manifest | ConvertTo-Json -Depth 8
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
$outputPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Output)
[System.IO.File]::WriteAllText($outputPath, $json, $utf8NoBom)
Write-Host "Wrote updater manifest: $Output"
$json
