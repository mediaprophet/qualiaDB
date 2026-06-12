import re

with open('crates/qualia-cli/src/main.rs', 'r', encoding='utf-8') as f:
    content = f.read()

if 'pub mod bench;' not in content:
    content = content.replace('pub mod telemetry_server;', 'pub mod telemetry_server;\npub mod bench;')
    
with open('crates/qualia-cli/src/main.rs', 'w', encoding='utf-8') as f:
    f.write(content)
