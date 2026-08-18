#!/usr/bin/env python3
"""Parse WASM globals and data sections to find stack pointer and stack size."""
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

def read_leb128_signed(data, idx):
    result = 0
    shift = 0
    while True:
        b = data[idx]
        idx += 1
        result |= (b & 0x7f) << shift
        shift += 7
        if not (b & 0x80):
            if b & 0x40:
                result |= -(1 << shift)
            break
    return result, idx

def main(path):
    with open(path, 'rb') as f:
        data = f.read()
    
    assert data[:4] == b'\x00asm'
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
        end = idx + slen
        
        if sid == 6:  # Global section
            count, idx2 = read_leb128(data, idx)
            print(f"Global section: {count} globals")
            for i in range(count):
                # valtype
                vt = data[idx2]; idx2 += 1
                # mutability
                mut = data[idx2]; idx2 += 1
                # init expr
                # Read opcode(s) until 0x0b (end)
                opcodes = []
                while data[idx2] != 0x0b:
                    op = data[idx2]; idx2 += 1
                    opcodes.append(op)
                    if op == 0x41:  # i32.const
                        val, idx2 = read_leb128_signed(data, idx2)
                        opcodes.append(val)
                    elif op == 0x42:  # i64.const
                        val, idx2 = read_leb128_signed(data, idx2)
                        opcodes.append(val)
                idx2 += 1  # skip 0x0b
                vt_name = {0x7f: "i32", 0x7e: "i64", 0x7d: "f32", 0x7c: "f64"}.get(vt, f"0x{vt:02x}")
                print(f"  Global {i}: type={vt_name}, mut={mut}, init_ops={opcodes}")
        elif sid == 7:  # Export section
            count, idx2 = read_leb128(data, idx)
            for i in range(count):
                nlen, idx2 = read_leb128(data, idx2)
                name = data[idx2:idx2+nlen].decode('utf-8', errors='replace')
                idx2 += nlen
                ekind = data[idx2]; idx2 += 1
                eidx, idx2 = read_leb128(data, idx2)
                if 'stack' in name.lower() or 'memory' in name.lower():
                    print(f"Export: {name} (kind={ekind}, idx={eidx})")
        elif sid == 11:  # Data section
            count, idx2 = read_leb128(data, idx)
            print(f"Data section: {count} segments")
            for i in range(count):
                flag = data[idx2]; idx2 += 1
                if flag == 0:  # active
                    # init expr
                    if data[idx2] == 0x41:  # i32.const
                        idx2 += 1
                        offset, idx2 = read_leb128_signed(data, idx2)
                        assert data[idx2] == 0x0b
                        idx2 += 1
                    else:
                        offset = "?"
                        idx2 += 1
                    seg_len, idx2 = read_leb128(data, idx2)
                    print(f"  Segment {i}: offset={offset} (0x{offset:x}), len={seg_len} ({seg_len/1024:.1f} KB)")
                    idx2 += seg_len
                elif flag == 1:  # passive
                    seg_len, idx2 = read_leb128(data, idx2)
                    print(f"  Segment {i}: passive, len={seg_len}")
                    idx2 += seg_len
                elif flag == 2:  # active with memory index
                    memidx, idx2 = read_leb128(data, idx2)
                    if data[idx2] == 0x41:
                        idx2 += 1
                        offset, idx2 = read_leb128_signed(data, idx2)
                        assert data[idx2] == 0x0b
                        idx2 += 1
                    else:
                        offset = "?"
                        idx2 += 1
                    seg_len, idx2 = read_leb128(data, idx2)
                    print(f"  Segment {i}: memidx={memidx}, offset={offset} (0x{offset:x}), len={seg_len} ({seg_len/1024:.1f} KB)")
                    idx2 += seg_len
        
        idx = end

if __name__ == '__main__':
    main(sys.argv[1] if len(sys.argv) > 1 else r'c:\Projects\qualia-27062026\docs\playground\qualia_core_db_bg.wasm')
