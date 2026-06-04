import os
import re
import shutil

def replace_in_file(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
    except Exception as e:
        return # Skip binary or unreadable files

    new_content = content.replace("Sentinel", "Webizen")
    new_content = new_content.replace("sentinel", "webizen")
    new_content = new_content.replace("SENTINEL", "WEBIZEN")

    if new_content != content:
        with open(filepath, 'w', encoding='utf-8', newline='\n') as f:
            f.write(new_content)
        print(f"Updated content in {filepath}")

def main():
    base_dir = r"C:\Projects\qualiaDB"
    exclude_dirs = {".git", "target", "node_modules", ".gemini"}
    
    renames = []

    for root, dirs, files in os.walk(base_dir):
        dirs[:] = [d for d in dirs if d not in exclude_dirs]
        
        for name in dirs:
            if "sentinel" in name.lower():
                new_name = name.replace("Sentinel", "Webizen").replace("sentinel", "webizen").replace("SENTINEL", "WEBIZEN")
                renames.append((os.path.join(root, name), os.path.join(root, new_name)))

        for file in files:
            filepath = os.path.join(root, file)
            if not file.endswith((".exe", ".gguf", ".bidx", ".png", ".jpg", ".svg", ".zip", ".tar", ".gz", ".py")):
                replace_in_file(filepath)
            
            if "sentinel" in file.lower():
                new_name = file.replace("Sentinel", "Webizen").replace("sentinel", "webizen").replace("SENTINEL", "WEBIZEN")
                renames.append((filepath, os.path.join(root, new_name)))

    # Sort renames by length descending so children are renamed before parents
    renames.sort(key=lambda x: len(x[0]), reverse=True)

    for old_path, new_path in renames:
        os.rename(old_path, new_path)
        print(f"Renamed {old_path} to {new_path}")

if __name__ == "__main__":
    main()
