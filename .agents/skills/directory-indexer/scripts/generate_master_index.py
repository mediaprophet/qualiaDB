import os
from pathlib import Path
from datetime import datetime

def generate_master_index(root_dir="."):
    root_path = Path(root_dir).resolve()
    master_index_path = root_path / "MASTER_INDEX.md"
    
    # Exclude typical build/cache directories
    excludes = {".git", ".github", "target", "vendor", "node_modules", ".agents", ".cargo", ".claude"}
    
    # Store found indices
    indices = []
    
    print(f"Scanning {root_path} for DIRECTORY_INDEX.md files...")
    
    for dirpath, dirnames, filenames in os.walk(root_path):
        # Modify dirnames in-place to skip excluded directories
        dirnames[:] = [d for d in dirnames if d not in excludes]
        
        if "DIRECTORY_INDEX.md" in filenames:
            rel_path = Path(dirpath).relative_to(root_path)
            indices.append(rel_path)
    
    indices.sort()
    
    print(f"Found {len(indices)} index files. Generating MASTER_INDEX.md...")
    
    with open(master_index_path, "w", encoding="utf-8") as f:
        f.write("# QualiaDB Master Directory Index\n\n")
        f.write(f"**Generated/Updated On:** {datetime.now().strftime('%Y-%m-%d')}\n\n")
        f.write("This document provides a comprehensive list of all directory indexes across the repository. Click on any link to navigate to the detailed index for that specific component.\n\n")
        f.write("## Component Indexes\n\n")
        
        if not indices:
            f.write("*No `DIRECTORY_INDEX.md` files found. Have the sub-agents run yet?*\n")
        else:
            for rel_path in indices:
                # Format the display name (e.g. `crates/qualia-core-db` or `Root`)
                display_name = str(rel_path) if str(rel_path) != "." else "Repository Root"
                link_path = (rel_path / "DIRECTORY_INDEX.md").as_posix()
                
                f.write(f"- [{display_name}]({link_path})\n")
    
    print(f"Success! MASTER_INDEX.md generated at {master_index_path}")

if __name__ == "__main__":
    generate_master_index()
