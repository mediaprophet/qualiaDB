import re

path = r"crates\qualia-core-db\src\webizen.rs"
with open(path, "r", encoding="utf-8") as f:
    content = f.read()

# First replace all vm_log! back to println! so we start fresh
content = content.replace("vm_log!(", "println!(")
# Remove the old macro block (it has "macro_rules! vm_log { ... }")
content = re.sub(r"macro_rules! vm_log \{.*?\n\}\n", "", content, flags=re.DOTALL)

macro_def = """
macro_rules! vm_log {
    ($($arg:tt)*) => {
        if cfg!(feature = "vm_tracing") {
            println!($($arg)*);
        }
    };
}
"""

parts = content.split("use ", 1)
if len(parts) == 2:
    parts2 = parts[1].split("\n\n", 1)
    if len(parts2) == 2:
        content = parts[0] + "use " + parts2[0] + "\n\n" + macro_def + "\n" + parts2[1]
    else:
        content = macro_def + "\n" + content
else:
    content = macro_def + "\n" + content

content = content.replace("println!(", "vm_log!(")
# Put println! back inside the macro definition
content = content.replace("vm_log!($($arg)*);", "println!($($arg)*);")

with open(path, "w", encoding="utf-8") as f:
    f.write(content)

print("Fixed vm_log!")
