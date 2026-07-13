import os
from pathlib import Path
from datetime import datetime

def generate_index_skeletons(root_dir="."):
    root_path = Path(root_dir).resolve()
    
    # Exclude typical build/cache directories
    excludes = {".git", ".github", "target", "vendor", "node_modules", ".agents", ".cargo", ".claude", "benches", "tests"}
    
    print(f"Generating DIRECTORY_INDEX.md skeletons in {root_path}...")
    
    today = datetime.now().strftime('%Y-%m-%d')
    count = 0
    
    for dirpath, dirnames, filenames in os.walk(root_path):
        # Modify dirnames in-place to skip excluded directories
        dirnames[:] = [d for d in dirnames if d not in excludes]
        
        index_path = Path(dirpath) / "DIRECTORY_INDEX.md"
        
        # Only create if it doesn't exist
        if not index_path.exists():
            dir_name = Path(dirpath).name
            if str(Path(dirpath)) == str(root_path):
                dir_name = "Root"
            
            with open(index_path, "w", encoding="utf-8") as f:
                f.write("---\n")
                f.write(f"created: {today}\n")
                f.write(f"updated: {today}\n")
                f.write("update_scope: Comprehensive\n")
                f.write("---\n\n")
                f.write(f"# {dir_name} Index\n\n")
                f.write("## Functionality Overview\n")
                f.write("[A concise description of the primary purpose and functionality of this directory.]\n\n")
                f.write("## File & Subdirectory Manifest\n")
                
                # List immediate children that aren't excluded
                children = [p.name for p in Path(dirpath).iterdir() if p.name not in excludes and p.name != "DIRECTORY_INDEX.md"]
                children.sort()
                
                if children:
                    for child in children:
                        f.write(f"- `{child}`: [Brief explanation of what this does.]\n")
                else:
                    f.write("*(No files or subdirectories)*\n")
                
                f.write("\n## Changelog\n")
                f.write(f"- **{today}**: Initial index skeleton generation.\n")
            
            count += 1
            print(f"Created {index_path.relative_to(root_path)}")
            
    print(f"\nDone! Created {count} skeleton DIRECTORY_INDEX.md files.")
    print("Subagents can now traverse these files and fill in the bracketed placeholder descriptions.")

if __name__ == "__main__":
    generate_index_skeletons()
