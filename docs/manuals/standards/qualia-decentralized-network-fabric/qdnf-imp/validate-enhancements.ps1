# Read-only structural validation for the 0.0.37 enhancement supplements.
# Called by validate-plan.ps1; original registry checks remain unchanged.
function Test-EnhancementPlan {
    param([Parameter(Mandatory)][string]$Root)
    $problems = [Collections.Generic.List[string]]::new()
    $main = [IO.File]::ReadAllText((Join-Path $Root '0.0.37-enhancement-plan.md'))
    $recipe = [IO.File]::ReadAllText((Join-Path $Root 'advanced-algorithm-recipes.md'))
    $blueprint = [IO.File]::ReadAllText((Join-Path $Root 'sensitive-operations-blueprint.md'))
    $packages = @{}
    $childTotal = 0
    $sections = [regex]::Matches($main, '(?ms)^### (E[0-9]{2}) — [^\r\n]+\r?\n(.*?)(?=^### |^## |\z)')
    foreach ($section in $sections) {
        $id = $section.Groups[1].Value
        $body = $section.Groups[2].Value
        if ($packages.ContainsKey($id)) { $problems.Add("Duplicate enhancement package: $id"); continue }
        $depLine = [regex]::Match($body, '(?m)^\*\*Depends on:\*\* ([^.]+)\.')
        if (-not $depLine.Success) { $problems.Add("Missing dependency declaration: $id") }
        $deps = [Collections.Generic.List[string]]::new()
        $depText = $depLine.Groups[1].Value
        $range = [regex]::Match($depText, '^E([0-9]{2})–E([0-9]{2})$')
        if ($range.Success) {
            $first = [int]$range.Groups[1].Value
            $last = [int]$range.Groups[2].Value
            if ($first -gt $last) { $problems.Add("Reversed dependency range: $id") }
            else { foreach ($n in $first..$last) { $deps.Add(('E{0:D2}' -f $n)) } }
        } elseif ($depText -ne 'none') {
            foreach ($dep in ($depText -split ', ')) {
                if ($dep -notmatch '^E[0-9]{2}$') { $problems.Add("Malformed dependency on $id : $dep") }
                $deps.Add($dep)
            }
        }
        if (@($deps | Select-Object -Unique).Count -ne $deps.Count) { $problems.Add("Duplicate dependency on $id") }
        $packages[$id] = @($deps)
        if ($body -notmatch '\*\*Owner:\*\*' -or $body -notmatch '\*\*Accept:\*\*') {
            $problems.Add("Missing owner/acceptance: $id")
        }
        $checks = [regex]::Matches($body, '(?m)^- \[([ xX])\] (E[0-9]{2})\.([0-9]+) ')
        if ($checks.Count -lt 5) { $problems.Add("Fewer than five enhancement checks: $id") }
        for ($i = 0; $i -lt $checks.Count; $i++) {
            $check = $checks[$i]
            if ($check.Groups[2].Value -ne $id -or [int]$check.Groups[3].Value -ne $i + 1) {
                $problems.Add("Wrong parent/nonsequential child: $id")
            }
            if ($check.Groups[1].Value -ne ' ') { $problems.Add("Proposed enhancement marked complete: $id") }
        }
        $childTotal += $checks.Count
    }
    foreach ($n in 0..21) {
        $id = 'E{0:D2}' -f $n
        if (-not $packages.ContainsKey($id)) { $problems.Add("Missing enhancement package: $id") }
    }
    if ($packages.Count -ne 22) { $problems.Add('Expected exactly 22 enhancement packages') }
    foreach ($id in $packages.Keys) {
        foreach ($dep in $packages[$id]) {
            if ($dep -eq $id -or -not $packages.ContainsKey($dep)) { $problems.Add("Invalid enhancement dependency: $id -> $dep") }
        }
    }
    $visited = @{}
    while ($visited.Count -lt $packages.Count) {
        $ready = @($packages.Keys | Where-Object {
            -not $visited.ContainsKey($_) -and @($packages[$_] | Where-Object { -not $visited.ContainsKey($_) }).Count -eq 0
        })
        if ($ready.Count -eq 0) { $problems.Add('Enhancement dependency cycle/unresolved predecessor'); break }
        foreach ($id in $ready) { $visited[$id] = $true }
    }
    $finalChecks = [regex]::Matches($main, '(?m)^- \[ \] FINAL\.([0-9]+) ')
    if ($finalChecks.Count -ne 7) { $problems.Add('Expected seven final enhancement checks') }
    for ($i = 0; $i -lt $finalChecks.Count; $i++) {
        if ([int]$finalChecks[$i].Groups[1].Value -ne $i + 1) { $problems.Add('Nonsequential final checks') }
    }
    foreach ($line in ($main -split '\r?\n')) {
        if ($line -match '^- \[[ xX]\] ' -and $line -notmatch '^- \[ \] (E[0-9]{2}|FINAL)\.[0-9]+ ') {
            $problems.Add("Unnumbered enhancement check: $line")
        }
    }
    $recipeCounts = @{ QSR = 8; HANDOVER = 5; ROUTE = 6; TRANSPORT = 5; ECON = 6; EVIDENCE = 5; 'CRYPTO-ADV' = 5 }
    $recipeSeen = @{}
    $recipeTotal = 0
    foreach ($line in ($recipe -split '\r?\n')) {
        if ($line -notmatch '^- \[[ xX]\] ') { continue }
        if ($line -notmatch '^- \[ \] (QSR|HANDOVER|ROUTE|TRANSPORT|ECON|EVIDENCE|CRYPTO-ADV)-([A-Z]): ') {
            $problems.Add("Invalid recipe check: $line"); continue
        }
        $group = $Matches[1]
        $number = [int][char]$Matches[2] - [int][char]'A' + 1
        if (-not $recipeSeen.ContainsKey($group)) { $recipeSeen[$group] = 0 }
        $recipeSeen[$group]++
        $recipeTotal++
        if ($number -ne $recipeSeen[$group]) { $problems.Add("Nonsequential recipe child: $group") }
    }
    foreach ($group in $recipeCounts.Keys) {
        if (-not $recipeSeen.ContainsKey($group) -or $recipeSeen[$group] -ne $recipeCounts[$group]) {
            $problems.Add("Missing recipe assignments: $group")
        }
    }
    $scenarios = [regex]::Matches($blueprint, '(?m)^\| S([0-9]{2}) \|')
    if ($scenarios.Count -ne 40) { $problems.Add('Expected 40 sensitive-operation scenarios') }
    for ($i = 0; $i -lt $scenarios.Count; $i++) {
        if ([int]$scenarios[$i].Groups[1].Value -ne $i + 1) { $problems.Add('Nonsequential sensitive-operation scenarios') }
    }
    [pscustomobject]@{
        Issues = @($problems)
        Packages = $packages.Count
        Children = $childTotal
        FinalChecks = $finalChecks.Count
        Recipes = $recipeTotal
        Scenarios = $scenarios.Count
    }
}
