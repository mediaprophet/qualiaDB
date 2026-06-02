import os
import struct

ico_dir = os.path.join("crates", "qualia-desktop", "icons")
os.makedirs(ico_dir, exist_ok=True)
ico_path = os.path.join(ico_dir, "icon.ico")

# A 1x1 pixel 32-bit ICO file (transparent)
# Format: ICONDIR, ICONDIRENTRY, BMP INFOHEADER, XOR mask, AND mask
ico_data = (
    b'\x00\x00\x01\x00\x01\x00' + # ICONDIR: reserved, type (1=ico), count (1)
    b'\x01\x01\x00\x00\x01\x00\x20\x00\x68\x00\x00\x00\x16\x00\x00\x00' + # ICONDIRENTRY: 1x1, 0 colors, reserved, 1 plane, 32 bpp, 104 bytes, offset 22
    # BITMAPINFOHEADER
    struct.pack('<LllHHLLllLL', 40, 1, 2, 1, 32, 0, 0, 0, 0, 0, 0) +
    # Image data (1 pixel, 4 bytes for XOR, 4 bytes padding for AND mask)
    b'\x00\x00\x00\x00' + b'\x00\x00\x00\x00' + b'\x00\x00\x00\x00' + b'\x00\x00\x00\x00'
)

with open(ico_path, 'wb') as f:
    f.write(ico_data)

print("Created", ico_path)
