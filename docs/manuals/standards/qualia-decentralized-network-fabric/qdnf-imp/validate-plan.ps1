[CmdletBinding()]
param()
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$planRoot = [IO.Path]::GetFullPath($PSScriptRoot)
$issues = [Collections.Generic.List[string]]::new()
function Add-Issue([string]$message) { $issues.Add($message) }
function Resolve-PlanPath([string]$relative) {
    if ([IO.Path]::IsPathRooted($relative)) { throw "Expected relative plan path: $relative" }
    $path = [IO.Path]::GetFullPath((Join-Path $planRoot $relative))
    $prefix = $planRoot + [IO.Path]::DirectorySeparatorChar
    if (-not $path.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Path escapes plan: $relative"
    }
    return $path
}
function Get-Anchors([string]$path) {
    $anchors = @{}
    $counts = @{}
    $fence = ''
    foreach ($line in [IO.File]::ReadAllLines($path)) {
        if ($line -match '^[ \t]*(\x60{3,}|~{3,})') {
            $mark = $Matches[1].Substring(0, 1)
            if (-not $fence) { $fence = $mark } elseif ($fence -eq $mark) { $fence = '' }
            continue
        }
        if ($fence) { continue }
        if ($line -match '^#{1,6}[ \t]+(.+?)[ \t]*#*[ \t]*$') {
            $slug = $Matches[1].ToLowerInvariant() -replace '<[^>]+>', ''
            $slug = ($slug -replace '[^\p{L}\p{Nd}_ -]', '').Trim() -replace ' ', '-'
            if ($counts.ContainsKey($slug)) { $counts[$slug]++ } else { $counts[$slug] = 0 }
            $anchor = $slug
            if ($counts[$slug] -gt 0) { $anchor += '-' + $counts[$slug] }
            $anchors[$anchor] = $true
        }
        foreach ($m in [regex]::Matches($line, '<a[ \t]+(?:id|name)="([^"]+)"')) {
            $anchors[$m.Groups[1].Value] = $true
        }
    }
    return $anchors
}
try {
    $registry = Get-Content -LiteralPath (Join-Path $planRoot 'task-registry.json') -Raw | ConvertFrom-Json
    if ($registry.schema_version -ne 1) { Add-Issue 'Unsupported registry schema version' }
    $expected = @('FND-01','FND-02','FND-03','CORE-01','CORE-02','CORE-03','CORE-04',
        'CRY-01','CRY-02','NET-01','NET-02','NET-03','NET-04','NET-05','RT-01','RT-02','RT-03',
        'SEM-01','SVC-01','SVC-02','SVC-03','ECO-01','ECO-02','EVD-01','EVD-02',
        'OPS-01','OPS-02','QA-01','QA-02','REL-01')
    $states = @('pending','claimed','in_progress','review','blocked','complete')
    $tasks = @{}
    $coverage = @{}
    $edgeCount = 0
    foreach ($task in $registry.tasks) {
        if ($task.id -notin $expected) { Add-Issue "Unexpected package: $($task.id)" }
        if ($tasks.ContainsKey($task.id)) { Add-Issue "Duplicate package: $($task.id)"; continue }
        $tasks[$task.id] = $task
        if ($task.status -notin $states) { Add-Issue "Unknown status: $($task.id)" }
        if (-not $task.title -or -not $task.owner_role) { Add-Issue "Missing title/owner: $($task.id)" }
        $path = Resolve-PlanPath $task.checklist
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { Add-Issue "Missing checklist: $path" }
        foreach ($package in $task.source_packages) {
            if ($package -notmatch '^P([0-9]|1[0-9]|2[01])$') { Add-Issue "Invalid source package: $package" }
            if ($task.owner_role -ne 'verification') { $coverage[$package] = $true }
        }
        if ($task.status -in @('claimed','in_progress','review')) {
            if (-not $task.assignee -or -not $task.claim) { Add-Issue "Missing active claim: $($task.id)" }
        }
        if ($task.claim) {
            foreach ($name in @('owner','source_state','allowed_paths','started_at','review_at','interface_versions')) {
                if ($name -notin $task.claim.PSObject.Properties.Name) { Add-Issue "Claim lacks $name on $($task.id)" }
            }
            if ($task.claim.owner -ne $task.assignee) { Add-Issue "Claim owner mismatch: $($task.id)" }
        }
        foreach ($evidence in $task.evidence) {
            if ($evidence -isnot [string] -or -not $evidence) {
                Add-Issue "Evidence entry must be a relative manifest path: $($task.id)"; continue
            }
            $evidencePath = Resolve-PlanPath $evidence
            if (-not (Test-Path -LiteralPath $evidencePath -PathType Leaf)) { Add-Issue "Missing evidence manifest: $evidence" }
        }
        if ($task.status -eq 'complete') {
            if (-not $task.assignee -or -not $task.reviewer -or $task.assignee -eq $task.reviewer) {
                Add-Issue "Independent assignee/reviewer required: $($task.id)"
            }
            if (@($task.evidence).Count -eq 0) { Add-Issue "No completion evidence: $($task.id)" }
        }
    }
    foreach ($id in $expected) { if (-not $tasks.ContainsKey($id)) { Add-Issue "Missing package: $id" } }
    foreach ($n in 0..21) { if (-not $coverage.ContainsKey("P$n")) { Add-Issue "No domain owner for P$n" } }
    foreach ($task in $registry.tasks) {
        $seen = @{}
        foreach ($dep in $task.depends_on) {
            $edgeCount++
            if ($seen.ContainsKey($dep)) { Add-Issue "Duplicate dependency: $($task.id) -> $dep" }
            $seen[$dep] = $true
            if ($dep -eq $task.id -or -not $tasks.ContainsKey($dep)) {
                Add-Issue "Invalid dependency: $($task.id) -> $dep"
            } elseif ($task.status -eq 'complete' -and $tasks[$dep].status -ne 'complete') {
                Add-Issue "Incomplete predecessor: $($task.id) -> $dep"
            }
        }
    }
    $visited = @{}
    while ($visited.Count -lt $tasks.Count) {
        $ready = @($tasks.Values | Where-Object {
            -not $visited.ContainsKey($_.id) -and @($_.depends_on | Where-Object {
                -not $visited.ContainsKey($_)
            }).Count -eq 0
        })
        if ($ready.Count -eq 0) { Add-Issue 'Dependency cycle or unresolved predecessor'; break }
        foreach ($task in $ready) { $visited[$task.id] = $true }
    }
    $documents = @(Get-ChildItem -LiteralPath $planRoot -Filter '*.md' -Recurse -File)
    $headings = @{}
    $children = @{}
    $checked = @{}
    $links = 0
    $anchorsCache = @{}
    foreach ($doc in $documents) {
        $fence = ''
        foreach ($line in [IO.File]::ReadAllLines($doc.FullName)) {
            if ($line -match '^[ \t]*(\x60{3,}|~{3,})') {
                $mark = $Matches[1].Substring(0, 1)
                if (-not $fence) { $fence = $mark } elseif ($fence -eq $mark) { $fence = '' }
                continue
            }
            if ($fence) { continue }
            if ($line -match '^##[ \t]+([A-Z]+-[0-9]{2})[ \t]+') {
                $id = $Matches[1]
                if ($headings.ContainsKey($id)) { Add-Issue "Duplicate task heading: $id" }
                $headings[$id] = $doc.FullName
            }
            if ($line -match '^- \[([ xX])\] ') {
                if ($line -notmatch '^- \[([ xX])\] ([A-Z]+-[0-9]{2})\.([0-9]{2})[ \t]+') {
                    Add-Issue "Unnumbered checklist item: $($doc.Name)"; continue
                }
                $isChecked = $Matches[1] -ne ' '
                $id = $Matches[2]
                $number = [int]$Matches[3]
                if (-not $tasks.ContainsKey($id)) { Add-Issue "Unknown child parent: $id"; continue }
                if ((Resolve-PlanPath $tasks[$id].checklist) -ne $doc.FullName) { Add-Issue "Child in wrong file: $id" }
                if (-not $children.ContainsKey($id)) { $children[$id] = 0; $checked[$id] = 0 }
                $children[$id]++
                if ($number -ne $children[$id]) { Add-Issue "Nonsequential/duplicate child: $id.$number" }
                if ($isChecked) { $checked[$id]++ }
            }
            foreach ($m in [regex]::Matches($line, '\[[^\]]+\]\(([^)]+)\)')) {
                $target = $m.Groups[1].Value.Trim()
                if ($target.StartsWith('<')) { $target = $target.Trim('<','>') }
                if ($target -match '^[a-zA-Z][a-zA-Z0-9+.-]*:') { continue }
                $target = [Uri]::UnescapeDataString($target)
                $parts = $target.Split('#', 2)
                $dest = $doc.FullName
                if ($parts[0]) { $dest = [IO.Path]::GetFullPath((Join-Path $doc.DirectoryName $parts[0])) }
                $links++
                if (-not (Test-Path -LiteralPath $dest)) { Add-Issue "Broken link in $($doc.Name): $target"; continue }
                if ($parts.Count -gt 1 -and $parts[1] -and [IO.Path]::GetExtension($dest) -eq '.md') {
                    if (-not $anchorsCache.ContainsKey($dest)) { $anchorsCache[$dest] = Get-Anchors $dest }
                    if (-not $anchorsCache[$dest].ContainsKey($parts[1])) { Add-Issue "Broken anchor in $($doc.Name): $target" }
                }
            }
        }
        if ($fence) { Add-Issue "Unclosed code fence: $($doc.Name)" }
    }
    foreach ($id in $tasks.Keys) {
        if (-not $headings.ContainsKey($id)) { Add-Issue "Missing task heading: $id" }
        elseif ($headings[$id] -ne (Resolve-PlanPath $tasks[$id].checklist)) { Add-Issue "Wrong heading file: $id" }
        if (-not $children.ContainsKey($id) -or $children[$id] -lt 10) {
            Add-Issue "Fewer than ten detailed checks: $id"; continue
        }
        if ($tasks[$id].status -eq 'pending' -and $checked[$id] -gt 0) { Add-Issue "Pending task has completed checks: $id" }
        if ($tasks[$id].status -eq 'complete' -and $checked[$id] -ne $children[$id]) { Add-Issue "Completed task has unchecked work: $id" }
    }
    if ($issues.Count -gt 0) {
        foreach ($issue in $issues) { Write-Output "FAIL: $issue" }
        exit 1
    }
    $total = ($children.Values | Measure-Object -Sum).Sum
    Write-Output "PASS: $($tasks.Count) packages; $total child checks; $edgeCount dependency edges; $($documents.Count) Markdown files; $links local links; P0-P21 covered by domain owners."
    exit 0
} catch {
    Write-Output "FAIL: $($_.Exception.Message)"
    exit 1
}
