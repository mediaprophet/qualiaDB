import os

replacements = {
    "Back to Matrix": "Back to Homepage",
    "Return to Matrix": "Return to Homepage",
    "RETURN TO MATRIX": "RETURN TO HOMEPAGE",
    "WebTorrent DHT Swarm": "WebTorrent DHT Community",
    "Sync from Swarm": "Sync from Community",
    "3D Spatial Binding Matrix": "3D Spatial Binding Architecture",
    "Live Telemetry Matrix": "Live Telemetry Dashboard",
    "GPU matrix math": "GPU dense math",
    "Swarm of independent": "Assembly of independent",
    "Sleep-Cycle Swarm": "Sleep-Cycle Assembly",
    "active vector matrix": "active vector space",
    "decentralized swarm": "decentralized community network",
    "resolution matrix": "resolution graph",
    "DHT swarm": "DHT community",
    "procedural matrix": "procedural path",
    "--matrix-green": "--neon-green",
    "override matrix": "override ruleset",
    "Intercept matrix": "Intercept ruleset",
    "Matrix": "Ecosystem",
    "matrix": "ecosystem"
}

def replace_in_file(filepath):
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
    
    new_content = content
    # Do exact phrase replacements first
    for old, new in list(replacements.items())[:-2]:
        new_content = new_content.replace(old, new)
    
    if new_content != content:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(new_content)
        print(f"Updated {filepath}")

for root, _, files in os.walk('.'):
    if 'node_modules' in root or '.git' in root or 'target' in root or 'build' in root:
        continue
    for file in files:
        if file.endswith('.html') or file.endswith('.md'):
            replace_in_file(os.path.join(root, file))
