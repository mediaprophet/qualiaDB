# Copy resources/ YAML catalog into a desktop dist folder.
# Usage: .\scripts\copy-bundled-resources.ps1 -OutDir dist\qualia-flutter-windows-x64

param(
    [Parameter(Mandatory = $true)]
    [string]$OutDir
)

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $false
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$Src = Join-Path $Root "resources"
$Dest = Join-Path $OutDir "bundled\resources"

if (-not (Test-Path $Src)) {
    Write-Warning "resources/ not found at $Src - skipping bundled resources copy."
    return
}

Write-Host "Copying resource catalog to $Dest ..."
New-Item -ItemType Directory -Force -Path $Dest | Out-Null
robocopy $Src $Dest /E /NFL /NDL /NJH /NJS /nc /ns /np | Out-Null
if ($LASTEXITCODE -ge 8) { throw "robocopy resources failed with exit code $LASTEXITCODE" }
$global:LASTEXITCODE = 0
Write-Host "Bundled resources staged under $Dest"
