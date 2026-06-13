import os
import re

directories = [
    r"C:\Projects\qualiaDB\init-draft-standards-wip-main",
    r"C:\Users\Admin\.gemini\antigravity\brain\b3f2d2b7-a02d-49ec-aa95-eb441c5d5d7a"
]

replacements = {
    re.compile(r'\bXanadu QApp\b', re.IGNORECASE): "Contextual Workspace QApp",
    re.compile(r'\bXanadu-style\b', re.IGNORECASE): "deep bidirectional",
    re.compile(r"Ted Nelson's Project Xanadu", re.IGNORECASE): "early multidimensional hypermedia theories",
    re.compile(r'\bXanadu\b', re.IGNORECASE): "Contextual Workspace"
}

for d in directories:
    for root, dirs, files in os.walk(d):
        for file in files:
            if file.endswith('.md') or file.endswith('.py') or file.endswith('.html'):
                filepath = os.path.join(root, file)
                
                # skip this script itself
                if file == "remove_trademarks.py":
                    continue
                    
                with open(filepath, 'r', encoding='utf-8') as f:
                    content = f.read()
                
                new_content = content
                for pattern, repl in replacements.items():
                    new_content = pattern.sub(repl, new_content)
                    
                if new_content != content:
                    with open(filepath, 'w', encoding='utf-8') as f:
                        f.write(new_content)
                    print(f"Updated {filepath}")
