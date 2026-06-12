# Convert RDF-Star annotated Turtle to standard Turtle
# This strips the << >> annotations and keeps the base triples

$sourceFile = "C:\Projects\qualiaDB\local\ontology\yago\yago-meta-facts.ntx"
$destFile = "C:\Projects\qualiaDB\local\ontology\yago\yago-meta-facts-standard.ttl"

Write-Host "Converting RDF-Star to standard Turtle format..."
Write-Host "Source: $sourceFile"
Write-Host "Destination: $destFile"

$lineCount = 0
$tripleCount = 0
$annotationCount = 0

Get-Content $sourceFile | ForEach-Object {
    $line = $_
    $lineCount++
    
    # Skip prefix declarations
    if ($line -match '^@prefix') {
        $line | Out-File -FilePath $destFile -Append -Encoding utf8
        return
    }
    
    # Check for RDF-Star annotations (<< >>)
    if ($line -match '<<\s*(.+?)\s*>>\s*(.+?)\s*\.\s*$') {
        $annotationCount++
        
        # Extract the base triple inside << >>
        $innerTriple = $matches[1]
        
        # Parse the inner triple: subject predicate object
        if ($innerTriple -match '^\s*(\S+)\s+(\S+)\s+(.+?)\s*$') {
            $subject = $matches[1]
            $predicate = $matches[2]
            $object = $matches[3]
            
            # Write as standard triple
            "$subject $predicate $object ." | Out-File -FilePath $destFile -Append -Encoding utf8
            $tripleCount++
        }
    } else {
        # Regular Turtle line - write as-is
        if ($line.Trim() -ne '') {
            $line | Out-File -FilePath $destFile -Append -Encoding utf8
            if ($line -match '\.\s*$') {
                $tripleCount++
            }
        }
    }
    
    if ($lineCount % 100000 -eq 0) {
        Write-Host "Processed $lineCount lines, $tripleCount triples, $annotationCount annotations"
    }
}

Write-Host "Conversion complete!"
Write-Host "Lines processed: $lineCount"
Write-Host "Triples extracted: $tripleCount"
Write-Host "Annotations processed: $annotationCount"
Write-Host "Output file: $destFile"