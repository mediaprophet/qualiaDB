import os
import re
from pathlib import Path
from datetime import datetime

def extract_symbols(filepath):
    symbols = []
    ext = filepath.suffix.lower()
    
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
            
            if ext == '.rs':
                # Match pub fn, pub struct, pub enum, pub trait, impl
                matches = re.findall(r'^\s*(?:pub\s+)?(?:async\s+)?(fn|struct|enum|trait|impl(?:\s+<[^>]+>)?)\s+([A-Za-z0-9_]+)', content, re.MULTILINE)
                symbols = [f"{m[0]} {m[1]}" for m in matches]
                
            elif ext in ('.py',):
                matches = re.findall(r'^\s*(class|def)\s+([A-Za-z0-9_]+)', content, re.MULTILINE)
                symbols = [f"{m[0]} {m[1]}" for m in matches]
                
            elif ext in ('.ts', '.js', '.mjs'):
                matches = re.findall(r'^\s*(?:export\s+)?(?:async\s+)?(class|function|interface|const)\s+([A-Za-z0-9_]+)', content, re.MULTILINE)
                symbols = [f"{m[0]} {m[1]}" for m in matches]
                
    except Exception as e:
        pass
        
    # Deduplicate while preserving order
    seen = set()
    result = []
    for s in symbols:
        if s not in seen:
            seen.add(s)
            result.append(s)
            
    return result

def generate_full_indexes(root_dir="."):
    root_path = Path(root_dir).resolve()
    
    # Exclude typical build/cache directories
    excludes = {".git", ".github", "target", "vendor", "node_modules", ".agents", ".cargo", ".claude", "benches", "tests"}
    
    print(f"Generating COMPREHENSIVE DIRECTORY_INDEX.md files in {root_path}...")
    
    today = datetime.now().strftime('%Y-%m-%d')
    count = 0
    
    for dirpath, dirnames, filenames in os.walk(root_path):
        dirnames[:] = [d for d in dirnames if d not in excludes]
        
        index_path = Path(dirpath) / "DIRECTORY_INDEX.md"
        dir_name = Path(dirpath).name if str(Path(dirpath)) != str(root_path) else "Root"
        
        with open(index_path, "w", encoding="utf-8") as f:
            f.write("---\n")
            f.write(f"created: {today}\n")
            f.write(f"updated: {today}\n")
            f.write("update_scope: Comprehensive\n")
            f.write("---\n\n")
            f.write(f"# {dir_name} Index\n\n")
            f.write("## Functionality Overview\n")
            f.write(f"Comprehensive index of functionality for `{dir_name}`. This document serves as the ground truth for bots regarding implemented components and dependencies.\n\n")
            f.write("## File & Subdirectory Manifest\n")
            
            # Subdirectories first
            children_dirs = [p.name for p in Path(dirpath).iterdir() if p.is_dir() and p.name not in excludes]
            children_dirs.sort()
            
            if children_dirs:
                f.write("### Subdirectories\n")
                for child in children_dirs:
                    f.write(f"- 📁 `[{child}]({child}/DIRECTORY_INDEX.md)`\n")
                f.write("\n")
                
            # Files
            children_files = [p for p in Path(dirpath).iterdir() if p.is_file() and p.name != "DIRECTORY_INDEX.md"]
            children_files.sort(key=lambda x: x.name)
            
            if children_files:
                f.write("### Files & Exported Functionality\n")
                for child_file in children_files:
                    f.write(f"- 📄 `{child_file.name}`\n")
                    symbols = extract_symbols(child_file)
                    if symbols:
                        for sym in symbols[:15]: # Cap at 15 to prevent massive files, but enough for coverage
                            f.write(f"  - `{sym}`\n")
                        if len(symbols) > 15:
                            f.write(f"  - *(...and {len(symbols) - 15} more)*\n")
            
            if not children_dirs and not children_files:
                f.write("*(Empty directory)*\n")
            
            f.write("\n## Changelog\n")
            f.write(f"- **{today}**: Automated full index generation, extracting code definitions.\n")
        
        count += 1
        
    print(f"\nDone! Created/Updated {count} comprehensive DIRECTORY_INDEX.md files.")

if __name__ == "__main__":
    generate_full_indexes()
