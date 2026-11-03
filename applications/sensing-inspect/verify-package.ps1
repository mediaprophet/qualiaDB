#Requires -Version 7.4
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$PackageDirectory,
    [ValidateRange(16384,1073741824)][long]$ByteBudget = 268435456
)
$ErrorActionPreference = 'Stop'
if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) { throw 'Windows verification host required' }

function Get-LocalItem([string]$Path) {
    if ($Path -notmatch '^[A-Za-z]:[\\/]' -or $Path.StartsWith('\\') -or $Path.Substring(2).Contains(':')) { throw 'Absolute local drive path required' }
    foreach ($part in $Path.Substring(3).Split([char[]]'\/')) {
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

function Get-ExactHash([IO.FileInfo]$File, [long]$Length) {
    $stream = $null
    $hash = $null
    try {
        $stream = [IO.File]::Open($File.FullName, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
        if ($stream.Length -ne $Length) { throw 'Package file length mismatch' }
        $hash = [Security.Cryptography.IncrementalHash]::CreateHash([Security.Cryptography.HashAlgorithmName]::SHA256)
        $buffer = [byte[]]::new(16384)
        [long]$remaining = $Length
        while ($remaining -gt 0) {
            $count = $stream.Read($buffer, 0, [int][Math]::Min($remaining, $buffer.Length))
            if ($count -eq 0) { throw 'Package file shortened while reading' }
            $hash.AppendData($buffer, 0, $count)
            $remaining -= $count
        }
        if ($stream.ReadByte() -ne -1 -or $stream.Length -ne $Length) { throw 'Package file changed while reading' }
        $digest = $hash.GetHashAndReset()
        return [Convert]::ToHexString($digest).ToLowerInvariant()
    } finally {
        if ($null -ne $hash) { $hash.Dispose() }
        if ($null -ne $stream) { $stream.Dispose() }
    }
}

function Assert-ExactProperties([Text.Json.JsonElement]$Object, [string[]]$Names) {
    if ($Object.ValueKind -ne [Text.Json.JsonValueKind]::Object) { throw 'JSON object required' }
    $properties = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    foreach ($property in $Object.EnumerateObject()) {
        if ($property.Name -cnotin $Names -or -not $properties.Add($property.Name)) {
            throw 'Unknown or duplicate JSON property'
        }
    }
    if ($properties.Count -ne $Names.Count) { throw 'Missing JSON property' }
}

function Get-JsonString([Text.Json.JsonElement]$Object, [string]$Name) {
    $value = $Object.GetProperty($Name)
    if ($value.ValueKind -ne [Text.Json.JsonValueKind]::String) { throw 'JSON string required' }
    return $value.GetString()
}

$root = Get-LocalItem $PackageDirectory
if ($root -isnot [IO.DirectoryInfo]) { throw 'Existing package directory required' }
$expected = @('.qualia-package-owner','qualia-sensing-inspect.exe','README.md','LICENSE','distribution.json','verify-package.ps1')
$allowed = $expected + @('package.json')
$seen = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
foreach ($path in [IO.Directory]::EnumerateFileSystemEntries($root.FullName)) {
    if ($seen.Count -ge $allowed.Count) { throw 'Unexpected package entries' }
    $name = [IO.Path]::GetFileName($path)
    if ($name -cnotin $allowed -or -not $seen.Add($name)) { throw 'Unexpected or duplicate package entry' }
    if ((Get-LocalItem $path) -isnot [IO.FileInfo]) { throw 'Regular package files required' }
}
if ($seen.Count -ne $allowed.Count) { throw 'Incomplete package' }
$manifestFile = Get-LocalItem (Join-Path $root.FullName 'package.json')
if ($manifestFile.Length -eq 0 -or $manifestFile.Length -gt 16384 -or $manifestFile.Length -gt $ByteBudget) { throw 'Manifest byte budget exceeded' }
$manifestBytes = [byte[]]::new([int]$manifestFile.Length)
$manifestStream = $null
try {
    $manifestStream = [IO.File]::OpenRead($manifestFile.FullName)
    $manifestStream.ReadExactly($manifestBytes)
    if ($manifestStream.ReadByte() -ne -1) { throw 'Manifest grew while reading' }
} finally { if ($null -ne $manifestStream) { $manifestStream.Dispose() } }
$document = $null
try {
    # Preserve every property, including duplicates; ConvertFrom-Json can hide ambiguity.
    $options = [Text.Json.JsonDocumentOptions]::new()
    $options.MaxDepth = 8
    $document = [Text.Json.JsonDocument]::Parse([Text.UTF8Encoding]::new($false, $true).GetString($manifestBytes), $options)
    $manifest = $document.RootElement
    Assert-ExactProperties $manifest @('schema','source_version','entrypoint','target_declaration',
        'supplied_binary_qualified','build_reproduced','integrity_is_authenticity','files')
    $target = Get-JsonString $manifest 'target_declaration'
    if ((Get-JsonString $manifest 'schema') -cne 'qualia.sensing-inspect.package.v1' -or
        (Get-JsonString $manifest 'source_version') -cne '0.1.0' -or
        (Get-JsonString $manifest 'entrypoint') -cne 'qualia-sensing-inspect.exe' -or
        $target -cnotin @('x86_64-pc-windows-msvc','aarch64-pc-windows-msvc','x86_64-pc-windows-gnu')) { throw 'Unsupported package manifest' }
    foreach ($flag in @('supplied_binary_qualified','build_reproduced','integrity_is_authenticity')) {
        if ($manifest.GetProperty($flag).ValueKind -ne [Text.Json.JsonValueKind]::False) { throw 'Unsupported qualification flag' }
    }
    $files = $manifest.GetProperty('files')
    if ($files.ValueKind -ne [Text.Json.JsonValueKind]::Array -or $files.GetArrayLength() -ne $expected.Count) {
        throw 'Incorrect manifest entry array'
    }
    $seen.Clear()
    [long]$total = $manifestBytes.Length
    # Admit the complete aggregate before reading any payload files.
    foreach ($entry in $files.EnumerateArray()) {
        Assert-ExactProperties $entry @('path','bytes','sha256')
        $name = Get-JsonString $entry 'path'
        $digest = Get-JsonString $entry 'sha256'
        [long]$length = 0
        $size = $entry.GetProperty('bytes')
        if ($name -cnotin $expected -or -not $seen.Add($name) -or
            $size.ValueKind -ne [Text.Json.JsonValueKind]::Number -or -not $size.TryGetInt64([ref]$length) -or
            $length -lt 0 -or $length -gt ($ByteBudget - $total) -or
            ($name -ceq '.qualia-package-owner' -and $length -ne 32) -or
            ($name -ceq 'qualia-sensing-inspect.exe' -and $length -eq 0) -or
            $digest -cnotmatch '^[0-9a-f]{64}$') { throw 'Invalid manifest file entry' }
        $total += $length
    }
    foreach ($entry in $files.EnumerateArray()) {
        $name = Get-JsonString $entry 'path'
        $length = $entry.GetProperty('bytes').GetInt64()
        $file = Get-LocalItem (Join-Path $root.FullName $name)
        if ($file -isnot [IO.FileInfo] -or $file.Length -ne $length -or
            (Get-ExactHash $file $length) -cne (Get-JsonString $entry 'sha256')) { throw 'Package integrity mismatch' }
    }
    [ordered]@{ schema = 'qualia.sensing-inspect.package-verification.v1'; integrity_matches_manifest = $true;
        signer_authenticity_established = $false; executable_qualified = $false; encoded_bytes = $total;
        target_declaration = $target } | ConvertTo-Json
} finally { if ($null -ne $document) { $document.Dispose() } }
