import re

path = r"crates\qualia-desktop\src\commands\mod.rs"
with open(path, "r", encoding="utf-8") as f:
    content = f.read()

# Add missing imports
content = content.replace(
    "use futures_util::StreamExt;",
    "use futures_util::StreamExt;\nuse std::time::Duration;\nuse tokio::time::sleep;"
)

# Make all functions pub
# Match `fn ` that is not preceded by `pub `
content = re.sub(r'(?<!pub\s)(async\s+fn\s)', r'pub \1', content)
content = re.sub(r'(?<!pub\s)(fn\s)', r'pub \1', content)

with open(path, "w", encoding="utf-8") as f:
    f.write(content)

print("Fixed visibility and imports")
