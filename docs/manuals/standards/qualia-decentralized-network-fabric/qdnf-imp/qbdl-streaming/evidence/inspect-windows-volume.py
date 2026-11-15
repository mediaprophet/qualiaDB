import ctypes, json, msvcrt
from pathlib import Path
class Status(ctypes.Union):
    _fields_=[('status',ctypes.c_int32),('pointer',ctypes.c_void_p)]
class IoStatus(ctypes.Structure):
    _fields_=[('value',Status),('information',ctypes.c_size_t)]
query=ctypes.WinDLL('ntdll').NtQueryVolumeInformationFile
query.argtypes=[ctypes.c_void_p,ctypes.POINTER(IoStatus),ctypes.c_void_p,ctypes.c_uint32,ctypes.c_int32]
query.restype=ctypes.c_int32
with open(r'C:\github\qualiaDB\AGENTS.md','rb') as f:
    handle=msvcrt.get_osfhandle(f.fileno())
    result=[]
    for kind,size in [(4,8),(5,76)]:
        status=IoStatus();status.value.status=-1
        buffer=ctypes.create_string_buffer(size)
        returned=query(handle,ctypes.byref(status),buffer,size,kind)
        raw=buffer.raw
        item={'class':kind,'status':returned,'completion':status.value.status,'bytes':status.information,'raw_hex':raw[:min(size,status.information)].hex()}
        if kind==4:item.update(device_type=int.from_bytes(raw[:4],'little'),characteristics=hex(int.from_bytes(raw[4:8],'little')))
        if kind==5:item.update(name_bytes=int.from_bytes(raw[8:12],'little'),name=raw[12:12+int.from_bytes(raw[8:12],'little')].decode('utf-16-le'))
        result.append(item)
    remote_query=ctypes.WinDLL('ntdll').NtQueryInformationFile
    remote_query.argtypes=query.argtypes
    remote_query.restype=ctypes.c_int32
    status=IoStatus();status.value.status=-1
    is_remote=ctypes.c_ubyte(255)
    returned=remote_query(handle,ctypes.byref(status),ctypes.byref(is_remote),1,51)
    result.append({'class':51,'status':returned,'completion':status.value.status,'bytes':status.information,'is_remote':is_remote.value})
    print(json.dumps(result,indent=2))
