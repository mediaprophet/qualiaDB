<#
.SYNOPSIS
Read-only consistency checks for the NLP implementation plan's Markdown tracker.
.DESCRIPTION
No dependencies or filesystem writes. -SelfTest checks malformed plans in memory.
Does not verify that referenced test results or completion claims are true.
#>
[CmdletBinding()]
param(
    [string]$PlanPath = '',
    [switch]$SelfTest
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

if (-not $PlanPath) {
    $scriptDir = if ($PSScriptRoot) { $PSScriptRoot } else { Split-Path -Parent $MyInvocation.MyCommand.Definition }
    $PlanPath = Join-Path $scriptDir '../docs/work-in-progress/NLP_EXCEPTIONAL_IMPLEMENTATION_PLAN_2026-09-14.md'
}

function Test-NlpPlanText {
    param([string]$Content)

    $errors = [System.Collections.Generic.List[string]]::new()
    $tasks = @{}
    $milestones = @{}
    $dashboard = @{}
    $current = ''
    $total = $null
    $states = @('DONE', 'REVIEW', 'IN_PROGRESS', 'READY', 'BLOCKED', 'PLANNED', 'DEFERRED')
    $lineNumber = 0
    foreach ($line in ($Content -split '\r?\n')) {
        $lineNumber++
        if ($line -match '^#### (M\d+)\s+[—-]') {
            $current = $Matches[1]
            if ($milestones.ContainsKey($current)) {
                $errors.Add("Duplicate milestone heading $current at line $lineNumber")
            } else {
                $milestones[$current] = [System.Collections.Generic.List[string]]::new()
            }
        }
        if ($line -notmatch '^\|') { continue }
        $cells = @($line.Trim().Trim('|').Split('|') | ForEach-Object { $_.Trim() })
        if ($cells[0] -match '^NLP-\d{3}$') {
            $id = $cells[0]
            if ($cells.Count -ne 7) {
                $errors.Add("$id at line $lineNumber has $($cells.Count) columns; expected 7")
                continue
            }
            if (-not $current) {
                $errors.Add("$id has no milestone heading")
                continue
            }
            if ($tasks.ContainsKey($id)) {
                $errors.Add("Duplicate package $id")
                continue
            }
            $state = $cells[1].Trim('`')
            if ($state -notin $states) { $errors.Add("$id has invalid status $state") }
            if ($cells[2] -notin @('P0', 'P1', 'P2')) { $errors.Add("$id has invalid priority") }
            if (-not $cells[4]) { $errors.Add("$id needs an acceptance summary") }
            if ($state -in @('IN_PROGRESS', 'REVIEW', 'DONE')) {
                if ($cells[5] -in @('', '—', '-')) { $errors.Add("$id needs an owner") }
            }
            if ($state -in @('REVIEW', 'DONE')) {
                if ($cells[6] -in @('', '—', '-')) { $errors.Add("$id needs evidence") }
            }
            if ($state -eq 'BLOCKED' -and $cells[6] -in @('', '—', '-')) {
                $errors.Add("$id needs a blocker/next action in Evidence")
            }
            $tasks[$id] = [pscustomobject]@{
                Id = $id; State = $state; Milestone = $current; DependsText = $cells[3]
                Dependencies = [System.Collections.Generic.List[string]]::new()
            }
            $milestones[$current].Add($id)
        } elseif ($cells[0] -match '^M\d+$' -and $cells.Count -eq 5) {
            $key = $cells[0]
            if ($dashboard.ContainsKey($key)) { $errors.Add("Duplicate dashboard row $key") }
            if ($cells[2] -match '^(\d+)\s*/\s*(\d+)$') {
                $dashboard[$key] = @([int]$Matches[1], [int]$Matches[2])
            } else { $errors.Add("Invalid dashboard count for $key") }
            if ($cells[3].Trim('`') -notin $states) { $errors.Add("Invalid dashboard status for $key") }
        } elseif ($cells[0] -eq '**Total**') {
            if ($null -ne $total) { $errors.Add('Duplicate total dashboard row') }
            if ($cells.Count -eq 5 -and $cells[2].Trim('*') -match '^(\d+)\s*/\s*(\d+)$') {
                $total = @([int]$Matches[1], [int]$Matches[2])
            } else { $errors.Add('Invalid total dashboard count') }
        }
    }

    if ($tasks.Count -eq 0) { $errors.Add('No work packages found') }
    foreach ($id in @($tasks.Keys | Sort-Object)) {
        $task = $tasks[$id]
        foreach ($part in ($task.DependsText -split ',')) {
            $part = $part.Trim()
            if ($part -in @('—', '-')) { continue }
            $expanded = [System.Collections.Generic.List[string]]::new()
            if ($part -match '^NLP-(\d{3})(?:\.\.(\d{3}))?$') {
                $first = [int]$Matches[1]
                $last = $first
                if ($Matches.ContainsKey(2)) { $last = [int]$Matches[2] }
                if ($last -lt $first) { $errors.Add("$id has reversed range $part"); continue }
                for ($number = $first; $number -le $last; $number++) {
                    $expanded.Add(('NLP-{0:000}' -f $number))
                }
            } elseif ($part -match '^M(\d+)(?:\.\.M(\d+))?$') {
                $first = [int]$Matches[1]
                $last = $first
                if ($Matches.ContainsKey(2)) { $last = [int]$Matches[2] }
                if ($last -lt $first -or $last - $first -gt 100) {
                    $errors.Add("$id has invalid milestone range $part"); continue
                }
                for ($number = $first; $number -le $last; $number++) {
                    $key = "M$number"
                    if (-not $milestones.ContainsKey($key)) {
                        $errors.Add("$id references unknown milestone $key")
                    } else {
                        foreach ($dependency in $milestones[$key]) { $expanded.Add($dependency) }
                    }
                }
            } else { $errors.Add("$id has unparseable dependency '$part'"); continue }
            foreach ($dependency in $expanded) {
                if (-not $tasks.ContainsKey($dependency)) {
                    $errors.Add("$id references unknown package $dependency")
                } elseif (-not $task.Dependencies.Contains($dependency)) {
                    $task.Dependencies.Add($dependency)
                    if ($id -eq $dependency) { $errors.Add("$id depends on itself") }
                    if ($task.State -in @('READY', 'IN_PROGRESS', 'REVIEW', 'DONE') -and
                        $tasks[$dependency].State -ne 'DONE') {
                        $errors.Add("$id is $($task.State) but prerequisite $dependency is $($tasks[$dependency].State)")
                    }
                }
            }
        }
    }

    # Kahn's algorithm, avoiding recursive traversal and ambiguous dependency phrases.
    $remaining = @{}
    foreach ($id in $tasks.Keys) { $remaining[$id] = $tasks[$id].Dependencies.Count }
    $order = [System.Collections.Generic.List[string]]::new()
    while ($remaining.Count -gt 0) {
        $ready = @($remaining.Keys | Where-Object { $remaining[$_] -eq 0 } | Sort-Object)
        if ($ready.Count -eq 0) {
            $errors.Add("Dependency cycle; unresolved packages: $((@($remaining.Keys | Sort-Object)) -join ', ')")
            break
        }
        foreach ($id in $ready) {
            $remaining.Remove($id)
            $order.Add($id)
            foreach ($other in @($remaining.Keys)) {
                if ($tasks[$other].Dependencies.Contains($id)) { $remaining[$other]-- }
            }
        }
    }
    $done = 0
    foreach ($key in @($milestones.Keys | Sort-Object)) {
        $ids = @($milestones[$key])
        $accepted = @($ids | Where-Object { $tasks[$_].State -eq 'DONE' }).Count
        $done += $accepted
        if (-not $dashboard.ContainsKey($key)) { $errors.Add("Missing dashboard row $key") }
        elseif ($dashboard[$key][0] -ne $accepted -or $dashboard[$key][1] -ne $ids.Count) {
            $errors.Add("$key dashboard must be $accepted / $($ids.Count)")
        }
    }
    foreach ($key in $dashboard.Keys) {
        if (-not $milestones.ContainsKey($key)) { $errors.Add("Dashboard $key has no package section") }
    }
    if ($null -eq $total) { $errors.Add('Missing total dashboard row') }
    elseif ($total[0] -ne $done -or $total[1] -ne $tasks.Count) {
        $errors.Add("Total dashboard must be $done / $($tasks.Count)")
    }
    return [pscustomobject]@{
        Errors = $errors.ToArray(); Packages = $tasks.Count; Done = $done
        Milestones = $milestones.Count; Order = $order.ToArray()
    }
}

if ($SelfTest) {
    $sample = @'
| M0 | Foundation | 1 / 2 | `IN_PROGRESS` | test |
| **Total** | | **1 / 2** | `IN_PROGRESS` | |
#### M0 — Foundation
| NLP-000 | `DONE` | P0 | — | Baseline | Reviewer/date | Receipt |
| NLP-001 | `READY` | P0 | NLP-000 | Repair | — | — |
'@
    $badSamples = @(
        $sample.Replace('1 / 2', '0 / 2'),
        $sample.Replace('NLP-000 | Repair', 'NLP-999 | Repair'),
        $sample.Replace('P0 | — | Baseline', 'P0 | NLP-001 | Baseline'),
        $sample.Replace('`READY`', '`MAGIC`'),
        $sample.Replace('Reviewer/date', '—'),
        $sample.Replace('Receipt', '—'),
        $sample.Replace('NLP-000 | Repair', 'all release tasks | Repair'),
        ($sample + "`n| NLP-001 | ``READY`` | P0 | NLP-000 | Duplicate | — | — |"),
        $sample.Replace('| Repair | — | — |', '| Repair | — |'),
        $sample.Replace('NLP-000 | Repair', 'NLP-001..000 | Repair')
    )
    $result = Test-NlpPlanText $sample
    if ($result.Errors.Count) { throw "Self-test rejected valid fixture: $($result.Errors -join '; ')" }
    foreach ($bad in $badSamples) {
        if ((Test-NlpPlanText $bad).Errors.Count -eq 0) { throw 'Self-test accepted an invalid fixture' }
    }
    $ranges = @'
| M0 | Foundation | 2 / 2 | `DONE` | test |
| M1 | Next | 1 / 1 | `DONE` | test |
| M2 | Release | 0 / 1 | `READY` | test |
| **Total** | | **3 / 4** | `IN_PROGRESS` | |
#### M0 — Foundation
| NLP-000 | `DONE` | P0 | — | Baseline | Reviewer/date | Receipt |
| NLP-001 | `DONE` | P0 | NLP-000 | Repair | Reviewer/date | Receipt |
#### M1 — Next
| NLP-100 | `DONE` | P0 | NLP-000..001 | Contract | Reviewer/date | Receipt |
#### M2 — Release
| NLP-200 | `READY` | P0 | M0..M1 | Release | — | — |
'@
    $result = Test-NlpPlanText $ranges
    if ($result.Errors.Count) { throw "Range self-test failed: $($result.Errors -join '; ')" }
    Write-Output "Self-test passed: 2 valid fixtures and $($badSamples.Count) rejected malformed fixtures."
}

$resolved = (Resolve-Path -LiteralPath $PlanPath).Path
$result = Test-NlpPlanText (Get-Content -Raw -Encoding UTF8 -LiteralPath $resolved)
if ($result.Errors.Count -gt 0) {
    foreach ($issue in $result.Errors) { Write-Output "ERROR: $issue" }
    exit 1
}
Write-Output "NLP tracker valid: $($result.Packages) unique packages, $($result.Done) DONE, $($result.Milestones) milestones; dependency graph acyclic."
