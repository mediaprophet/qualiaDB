import os
import re

target_dir = r"C:\Projects\qualiaDB\init-draft-standards-wip-main"

replacements = {
    re.compile(r'\bdigital identity\b', re.IGNORECASE): "digital human agency",
    re.compile(r'\bidentities\b', re.IGNORECASE): "identifiers",
    re.compile(r'\bidentity management\b', re.IGNORECASE): "credential management",
    re.compile(r'\bdecentralized identity\b', re.IGNORECASE): "decentralized identifiers and verifiable claims",
    re.compile(r'\bIdentity Verification\b'): "Human Agency Verification",
    re.compile(r'\bidentity credentials\b', re.IGNORECASE): "verifiable credentials",
    # Specific fallback for "identity" alone, careful with "identity credentials" which we don't want to double replace
    # We will just replace standalone "identity" with "verifiable claims" or "human agency" based on heuristics.
    re.compile(r'\bidentity and rights\b', re.IGNORECASE): "human agency and rights",
    re.compile(r'\bidentity record\b', re.IGNORECASE): "verifiable claim record",
    re.compile(r'\bidentity\b(?!\s+(management|credentials|verification|and rights|record))', re.IGNORECASE): "human agency",
}

for root, dirs, files in os.walk(target_dir):
    for file in files:
        if file.endswith('.md') or file.endswith('.html') or file.endswith('.txt'):
            filepath = os.path.join(root, file)
            with open(filepath, 'r', encoding='utf-8') as f:
                content = f.read()
            
            new_content = content
            for pattern, repl in replacements.items():
                new_content = pattern.sub(repl, new_content)
                
            if new_content != content:
                with open(filepath, 'w', encoding='utf-8') as f:
                    f.write(new_content)
                print(f"Updated {filepath}")
