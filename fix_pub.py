import re

path = r"crates\qualia-desktop\src\commands\mod.rs"
with open(path, "r", encoding="utf-8") as f:
    content = f.read()

content = content.replace("pub pub ", "pub ")
content = content.replace("pub async pub fn", "pub async fn")

with open(path, "w", encoding="utf-8") as f:
    f.write(content)
