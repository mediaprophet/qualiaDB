"""Independent approved-schema6/7 vectors. No production codec/parser imports.
Uses the existing independent C04 BLAKE3/CRC oracle; writes only this fixture dir.
"""
from pathlib import Path
import sys,struct,json,hashlib
ROOT=Path(__file__).resolve().parents[7]
ORACLE=ROOT/'docs/plans/emf-qac-evolution-2026-11-13/vectors/tools/generator'
sys.path.insert(0,str(ORACLE))
from digests import blake3,crc32c,self_test
OUT=Path(__file__).resolve().parent
def s(fmt,*v):return struct.pack('<'+fmt,*v)
def field(k,t,data):
    if isinstance(data,str):data=data.encode()
    sizes={1:4,2:8,3:8,5:8,6:1,7:1,8:4,9:4}
    return s('HBBIII',k,t,1,len(data)//sizes[t],len(data),0)+data+bytes((-len(data))%8)
def block(i,schema,fields):
    body=b''.join(field(k,t,b) for k,t,b in fields)
    return s('IHHII',i,schema,1,len(body),1)+body
def typed_base(kind,payload,offset,length):
    return [(1,1,s('I',kind)),(2,9,s('I',1)),(3,7,blake3(payload)),(4,2,s('Q',offset)),(5,2,s('Q',length)),
        (6,2,s('Q',11)),(7,2,s('Q',12)),(8,1,s('I',0)),(9,2,s('Q',13))]
def tensor_fields(enc,shape,strides,axes):
    rank=len(shape)
    return [(10,1,s('I',enc)),(11,1,s('I',0)),(12,1,s('I',1 if enc==12 else 0)),(13,1,s('I',rank)),
      (14,1,s('I'*rank,*shape)),(15,2,s('Q'*rank,*strides)),(16,1,s('I'*rank,*axes)),(17,2,s('Q'*rank,*([0]*rank)))]
def time_fields():
    return [(20,8,s('I',4)),(21,2,s('Q',77)),(22,2,s('Q',4)),(23,3,s('q',100)),(24,2,s('Q',13)),(25,2,s('Q',7))]
def clock():
    return block(4,1,[(1,1,s('I',0)),(2,3,s('q',0)),(3,1,s('I',0)),(4,2,s('Q',1_000_000_000)),(5,2,s('Q',1))])
def descriptors(payload,derive=False):
    prov=block(1,4,[(1,1,s('I',2 if derive else 4))]+([(2,7,blake3(b'INPUT-original-bytes'))] if derive else []))
    fields=[(3,8,s('I',1)),(7,6,'original_bytes'),(8,6,'1'),(9,1,s('I',1)),(10,6,'byte'),(11,1,s('I',7)),(12,6,'urn:qualia:acquisition:original:1')]
    if derive:fields.append((17,9,s('I',2)))
    fields.append((18,8,s('II',3,5) if derive else s('I',3)))
    return prov+block(2,3,fields)
def container(family,payloads,metadata):
    align=lambda n:(n+63)//64*64
    n=len(payloads);mo=128+n*128;start=align(mo+len(metadata));length=start
    positions=[]
    for payload in payloads:positions.append(length);length=align(length+len(payload))
    b=bytearray(length);b[:8]=b'Q42EMF64' if family=='emf' else b'qac'+bytes(5)
    struct.pack_into('<HHIQQIHHQQ',b,8,4 if family=='emf' else 1,128,0,length,128,n,128,1,mo,len(metadata))
    b[mo:mo+len(metadata)]=metadata
    for i,payload in enumerate(payloads):
        row=bytearray(128)
        struct.pack_into('<HHIIIqQQQQQQIHH',row,0,0x104,1,0,i+1,0,0,0,0,positions[i],len(payload),len(payload),len(payload),1,0,0)
        row[80:112]=blake3(payload);struct.pack_into('<I',row,112,i*10+2)
        struct.pack_into('<I',row,124,crc32c(row))
        b[128+i*128:256+i*128]=row;b[positions[i]:positions[i]+len(payload)]=payload
    b[56:88]=blake3(b'QUALIA-FIELD-INDEX-v1\0'+b[:128]+b[128:mo]+metadata)
    struct.pack_into('<I',b,124,crc32c(b[:128]))
    return bytes(b)
def make(kind,variant=None):
    if kind=='tensor':
        payload=b'HDR'+s('QQ',0x8000000000000000,0xfff8000000001234)+b'ZZ'
        fields=typed_base(1,payload,3,16)+tensor_fields(2,[2],[8],[0])+[(32,1,s('I',0))]
        extra=b''
    elif kind=='events':
        raw=struct.pack('>qIQ',-5,1,2**64-1)+struct.pack('>qIQ',12,0,0)
        payload=b'HDR'+raw+b'ZZ'
        fields=typed_base(2,payload,3,40)+[(11,1,s('I',1))]+time_fields()+[
            (26,7,bytes([1])*32),(27,2,s('Q',101)),(28,7,bytes([2])*32),(29,2,s('Q',102)),(30,7,bytes([3])*32),(31,2,s('Q',103)),
            (32,1,s('I',2)),(33,5,s('d',2e6)),(34,5,s('dd',-1e5,1e5)),(35,2,s('Q',1000)),(36,2,s('Q',1)),
            (40,2,s('Q',2)),(41,1,s('I',2)),(42,1,s('I',1))]
        extra=clock()
    elif kind=='correlation':
        payload=b'HDR'+s('dddd',1,-2,3.5,-0.0)+b'ZZ'
        fields=typed_base(3,payload,3,32)+tensor_fields(12,[2,1,1],[16,16,16],[3,4,5])+time_fields()+[
            (32,1,s('I',2)),(33,5,s('d',1e9)),(37,1,s('I',1)),(38,7,bytes([4])*32),(39,2,s('Q',104)),
            (43,3,s('q',-1)),(44,3,s('q',2)),(45,1,s('I',0)),(46,1,s('I',1)),(47,2,s('Q',5))]
        extra=clock()
    elif kind=='lineage':
        payload=b'HDR'+s('HH',0x1234,0xabcd)+b'ZZ'
        fields=typed_base(1,payload,3,4)+tensor_fields(8,[2],[2],[0])+[(32,1,s('I',0))]
        extra=block(5,7,[(1,9,s('I',1)),(2,7,blake3(payload)),(3,7,bytes([0x11])*32),(4,2,s('Q',5)),
          (5,7,bytes([0x22])*32),(6,2,s('Q',6)),(7,9,s('I',2)),(8,7,blake3(b'INPUT-original-bytes')),
          (9,1,s('II',3,255)),(10,1,s('I',1)),(11,2,s('Q',7)),(12,7,bytes([0x33])*32)])
        extra+=block(11,4,[(1,1,s('I',4))])+block(12,3,[(3,8,s('I',11)),(7,6,'original_bytes'),(8,6,'1'),(9,1,s('I',1)),(10,6,'byte'),(11,1,s('I',7)),(12,6,'urn:qualia:acquisition:original:1'),(18,8,b'')])
    if variant=='unknown-critical':fields.append((50,1,s('I',9)))
    if variant=='unknown-optional':
        # Replace only the final field flag, preserving manual TLV layout.
        pass
    if variant=='bad-ref':fields=[(k,t,s('I',99) if k==2 else v) for k,t,v in fields]
    if variant=='bad-hash':fields=[(k,t,bytes(32) if k==3 else v) for k,t,v in fields]
    if variant=='overflow-stride':fields=[(k,t,s('Q',2**64-1) if k==15 else v) for k,t,v in fields]
    view=block(3,6,fields)
    if variant=='unknown-optional':
        addition=bytearray(field(100,7,b'UNINTERPRETED'));addition[3]=0
        body=view[16:]+addition;view=s('IHHII',3,6,1,len(body),1)+body
    metadata=descriptors(payload,kind=='lineage')+view+extra
    return [payload]+([b'INPUT-original-bytes'] if kind=='lineage' else []),metadata
def main():
    self_test();rows=[]
    for kind in ['tensor','events','correlation','lineage']:
      for family in ['qac','emf']:
        payloads,meta=make(kind);data=container(family,payloads,meta)
        name=kind+'.'+family; (OUT/name).write_bytes(data);rows.append(dict(path=name,expected='accept',sha256=hashlib.sha256(data).hexdigest()))
    for variant in ['unknown-critical','unknown-optional','bad-ref','bad-hash','overflow-stride']:
      payloads,meta=make('tensor',variant);data=container('qac',payloads,meta)
      name=variant+'.qac';(OUT/name).write_bytes(data);rows.append(dict(path=name,expected='accept' if variant=='unknown-optional' else 'reject',sha256=hashlib.sha256(data).hexdigest()))
    (OUT/'manifest.json').write_text(json.dumps(dict(oracle=str(ORACLE/'digests.py'),oracle_sha256=hashlib.sha256((ORACLE/'digests.py').read_bytes()).hexdigest(),fixtures=rows),indent=2)+'\n')
    print('generated',len(rows),'independent vectors')
if __name__=='__main__':main()
