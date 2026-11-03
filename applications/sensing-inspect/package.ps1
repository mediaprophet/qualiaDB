[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$ExecutablePath,
    [Parameter(Mandatory)][string]$OutputParent,
    [Parameter(Mandatory)][ValidateSet('x86_64-pc-windows-msvc','aarch64-pc-windows-msvc','x86_64-pc-windows-gnu')][string]$Target,
    [ValidateRange(16384,1073741824)][long]$ByteBudget = 268435456
)
$ErrorActionPreference = 'Stop'
if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) { throw 'Windows packaging host required' }

function Get-LocalPath([string]$Path) {
    if ($Path -notmatch '^[A-Za-z]:[\\/]' -or $Path.StartsWith('\\') -or $Path.Substring(2).Contains(':')) { throw 'Absolute local drive path required' }
    $parts = $Path.Substring(3).Split([char[]]'\/')
    foreach ($part in $parts) {
        if ($part -eq '..' -or $part -eq '.' -or $part.TrimEnd(' ','.') -ne $part -or
            $part -match '^(?i:CON|PRN|AUX|NUL|CLOCK\$|CONIN\$|CONOUT\$|COM[1-9¹²³]|LPT[1-9¹²³])(?:\.|$)') { throw 'Unsafe path component' }
    }
    $full = [IO.Path]::GetFullPath($Path)
    if ([IO.DriveInfo]::new([IO.Path]::GetPathRoot($full)).DriveType -ne [IO.DriveType]::Fixed) { throw 'Fixed local drive required' }
    $item = Get-Item -LiteralPath $full -Force
    $cursor = $item
    while ($null -ne $cursor) {
        if (($cursor.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'Reparse-point path refused' }
        $cursor = if ($cursor -is [IO.FileInfo]) { $cursor.Directory } else { $cursor.Parent }
    }
    return $item
}

function Copy-BoundedFile([string]$Source, [string]$Destination, [long]$Length) {
    $inputStream = $null
    $outputStream = $null
    try {
        $inputStream = [IO.File]::Open($Source, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
        $outputStream = [IO.File]::Open($Destination, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
        $buffer = [byte[]]::new(16384)
        [long]$remaining = $Length
        while ($remaining -gt 0) {
            $count = $inputStream.Read($buffer, 0, [int][Math]::Min($remaining, $buffer.Length))
            if ($count -eq 0) { throw 'Source shortened during packaging' }
            $outputStream.Write($buffer, 0, $count)
            $remaining -= $count
        }
        if ($inputStream.ReadByte() -ne -1) { throw 'Source grew during packaging' }
        $outputStream.Flush($true)
    } finally {
        if ($null -ne $outputStream) { $outputStream.Dispose() }
        if ($null -ne $inputStream) { $inputStream.Dispose() }
    }
}

$executable = Get-LocalPath $ExecutablePath
$parent = Get-LocalPath $OutputParent
if ($executable -isnot [IO.FileInfo] -or $parent -isnot [IO.DirectoryInfo] -or $executable.Length -eq 0) { throw 'Executable file and existing output directory required' }
$readme = Get-LocalPath (Join-Path $PSScriptRoot 'README.md')
$distribution = Get-LocalPath (Join-Path $PSScriptRoot 'distribution.json')
$verifier = Get-LocalPath (Join-Path $PSScriptRoot 'verify-package.ps1')
$license = Get-LocalPath ([IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../LICENSE')))
$files = @(
    @{ Source = $executable; Name = 'qualia-sensing-inspect.exe' },
    @{ Source = $readme; Name = 'README.md' },
    @{ Source = $license; Name = 'LICENSE' },
    @{ Source = $distribution; Name = 'distribution.json' },
    @{ Source = $verifier; Name = 'verify-package.ps1' }
)
[long]$total = 16384 # bounded manifest and staging marker reserve
foreach ($file in $files) {
    if ($file.Source -isnot [IO.FileInfo] -or $file.Source.Length -gt ($ByteBudget - $total)) { throw 'Package byte budget exceeded' }
    $total += $file.Source.Length
}
$packageName = "qualia-sensing-inspect-0.1.0-$Target"
$destination = Join-Path $parent.FullName $packageName
if (Test-Path -LiteralPath $destination) { throw 'Destination already exists' }
$owner = [Guid]::NewGuid().ToString('N')
$stage = Join-Path $parent.FullName ('.qualia-package-' + $owner)
$marker = Join-Path $stage '.qualia-package-owner'
$published = $false
try {
    [IO.Directory]::CreateDirectory($stage) | Out-Null
    [IO.File]::WriteAllText($marker, $owner, [Text.UTF8Encoding]::new($false))
    $entries = @([ordered]@{ path = '.qualia-package-owner'; bytes = 32; sha256 = (Get-FileHash -LiteralPath $marker -Algorithm SHA256).Hash.ToLowerInvariant() })
    foreach ($file in $files) {
        $stagedFile = Join-Path $stage $file.Name
        Copy-BoundedFile $file.Source.FullName $stagedFile $file.Source.Length
        $entries += [ordered]@{ path = $file.Name; bytes = $file.Source.Length; sha256 = (Get-FileHash -LiteralPath $stagedFile -Algorithm SHA256).Hash.ToLowerInvariant() }
    }
    $manifest = [ordered]@{
        schema = 'qualia.sensing-inspect.package.v1'; source_version = '0.1.0'
        target_declaration = $Target; entrypoint = 'qualia-sensing-inspect.exe'
        supplied_binary_qualified = $false; build_reproduced = $false
        integrity_is_authenticity = $false; files = $entries
    } | ConvertTo-Json -Depth 6
    $manifestBytes = [Text.UTF8Encoding]::new($false).GetBytes($manifest)
    if ($manifestBytes.Length + 32 -gt 16384) { throw 'Manifest budget exceeded' }
    [IO.File]::WriteAllBytes((Join-Path $stage 'package.json'), $manifestBytes)
    [IO.Directory]::Move($stage, $destination)
    $published = $true
    Write-Output $destination
} catch {
    Write-Warning "Package publication failed or is uncertain. Preserve any final destination: $destination"
    throw
} finally {
    if (-not $published -and [IO.Directory]::Exists($stage)) {
        $resolvedStage = [IO.Path]::GetFullPath($stage)
        $expected = [IO.Path]::Combine($parent.FullName, '.qualia-package-' + $owner)
        try {
            if ($resolvedStage.Equals($expected, [StringComparison]::OrdinalIgnoreCase) -and
                (Get-LocalPath $resolvedStage) -is [IO.DirectoryInfo] -and
                [IO.File]::ReadAllText($marker) -eq $owner) {
                [IO.Directory]::Delete($resolvedStage, $true)
            }
        } catch { Write-Warning "Staging cleanup incomplete; inspect exact directory: $stage" }
    }
}
