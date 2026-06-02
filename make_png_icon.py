import os
import base64

png_base64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII="

png_path = os.path.join("crates", "qualia-desktop", "app-icon.png")

with open(png_path, 'wb') as f:
    f.write(base64.b64decode(png_base64))

print("Created app-icon.png")
