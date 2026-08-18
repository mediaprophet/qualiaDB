#!/usr/bin/env python3
"""Quick WASM memory section parser."""
import sys

def read_leb128(data, idx):
    result = 0
    shift = 0
    while True:
        b = data[idx]
        idx += 1
        result |= (b & 0x7f) << shift
        if not (b & 0x80):
            break
        shift += 7
    return result, idx

def main(path):
    with open(path, 'rb') as f:
        data = f.read()
    
    assert data[:4] == b'\x00asm', f"Not a WASM file: {data[:4].hex()}"
    version = int.from_bytes(data[4:8], 'little')
    print(f"WASM version: {version}")
    
    idx = 8
    section_names = {
        0: "Custom", 1: "Type", 2: "Import", 3: "Function",
        4: "Table", 5: "Memory", 6: "Global", 7: "Export",
        8: "Start", 9: "Element", 10: "Code", 11: "Data",
        12: "DataCount"
    }
    
    while idx < len(data):
        sid = data[idx]
        idx += 1
        slen, idx = read_leb128(data, idx)
        
        sname = section_names.get(sid, f"Unknown({sid})")
        
        if sid == 5:  # Memory section
            end = idx + slen
            count, idx2 = read_leb128(data, idx)
            print(f"Memory section: {count} memory/ies")
            for i in range(count):
                flags = data[idx2]
                idx2 += 1
                initial, idx2 = read_leb128(data, idx2)
                if flags & 1:
                    maximum, idx2 = read_leb128(data, idx2)
                    print(f"  Memory {i}: initial={initial} pages ({initial*65536} bytes = {initial*65536/1024/1024:.1f} MB), max={maximum} pages ({maximum*65536} bytes = {maximum*65536/1024/1024:.1f} MB)")
                else:
                    print(f"  Memory {i}: initial={initial} pages ({initial*65536} bytes = {initial*65536/1024/1024:.1f} MB), no max")
            idx = end
        elif sid == 0:  # Custom section - check for "name" or stack info
            end = idx + slen
            if data[idx:idx+4] == b'name':
                print(f"Custom 'name' section: {slen} bytes")
            elif data[idx:idx+7] == b'producers':
                print(f"Custom 'producers' section: {slen} bytes")
            else:
                print(f"Custom section: {slen} bytes (starts with {data[idx:idx+20]})")
            idx = end
        else:
            idx += slen
    
    print(f"File size: {len(data)} bytes ({len(data)/1024/1024:.1f} MB)")

if __name__ == '__main__':
    main(sys.argv[1] if len(sys.argv) > 1 else r'c:\Projects\qualia-27062026\docs\playground\qualia_core_db_bg.wasm')
