// Hand-specified normative DICOM RLE packets, not produced by a codec encoder.
// PS3.5 2026c AnnexG/A.4.2. Synthetic only; not a full SOP conformance corpus.
import {writeFileSync} from 'node:fs';
const dir=new URL('.',import.meta.url), cat=(...x)=>Buffer.concat(x), u16=n=>{const b=Buffer.alloc(2);b.writeUInt16LE(n);return b}, u32=n=>{const b=Buffer.alloc(4);b.writeUInt32LE(n);return b},tag=t=>cat(u16(t>>>16),u16(t&65535));
function element(t,vr,val){let b=Buffer.isBuffer(val)?val:Buffer.from(val);if(b.length%2)b=cat(b,Buffer.from([vr==='UI'?0:32]));return cat(tag(t),Buffer.from(vr),['OB','OW'].includes(vr)?cat(u16(0),u32(b.length)):u16(b.length),b)}
const us=(t,n)=>element(t,'US',u16(n));
function header(offsets){const h=Buffer.alloc(64);h.writeUInt32LE(offsets.length,0);offsets.forEach((n,i)=>h.writeUInt32LE(n,(i+1)*4));return h}
// 2x2 12-bit signed stored codes: -2048,-1,0,2047. Two row packets per plane.
const first=cat(header([64,70]),Buffer.from([1,8,15,1,0,7]),Buffer.from([1,0,255,1,0,255]));
// 1,1,2047,-2048. Replicate first row, literal second row, final zero padding.
const second=cat(header([64,70]),Buffer.from([255,0,1,7,8,0]),Buffer.from([255,1,1,255,0,0]));
// 2x4 8-bit: NOOP, repeat four7s, then literal1,2,3,4.
const gray=cat(header([64]),Buffer.from([128,253,7,3,1,2,3,4]));
function file({frames,rows,columns,allocated,stored,signed,rle,emptyBot=false,raw,photo='MONOCHROME2'}){const cls='1.2.840.10008.5.1.4.1.1.7.'+(allocated===8?'2':'3'),instance='1.2.826.0.1.3680043.10.101.1';const e=[element(0x00080016,'UI',cls),element(0x00080018,'UI',instance),element(0x00111010,'LO','ORIGINAL RLE TEST'),element(0x0020000d,'UI','1.2.826.0.1.3680043.10.101.10'),element(0x0020000e,'UI','1.2.826.0.1.3680043.10.101.20'),us(0x00280002,1),element(0x00280004,'CS',photo),element(0x00280008,'IS',String(frames)),us(0x00280010,rows),us(0x00280011,columns),us(0x00280100,allocated),us(0x00280101,stored),us(0x00280102,stored-1),us(0x00280103,signed),element(0x00281052,'DS','-1000'),element(0x00281053,'DS','2'),element(0x00281054,'LO','HU')];let pixels;if(rle){const offsets=[];let n=0;for(const f of rle){offsets.push(n);n+=8+f.length;}const bot=emptyBot?Buffer.alloc(0):cat(...offsets.map(u32));pixels=cat(tag(0x7fe00010),Buffer.from('OB'),u16(0),u32(0xffffffff),tag(0xfffee000),u32(bot.length),bot,...rle.map(f=>cat(tag(0xfffee000),u32(f.length),f)),tag(0xfffee0dd),u32(0));}else{pixels=element(0x7fe00010,allocated===8?'OB':'OW',raw);}return cat(Buffer.alloc(128,0xb3),Buffer.from('DICM'),element(0x00020002,'UI',cls),element(0x00020003,'UI',instance),element(0x00020010,'UI',rle?'1.2.840.10008.1.2.5':'1.2.840.10008.1.2.1'),...e,pixels)}
const spec16={frames:2,rows:2,columns:2,allocated:16,stored:12,signed:1},spec8={frames:1,rows:2,columns:4,allocated:8,stored:8,signed:0,photo:'MONOCHROME1'};
const native16=cat(...[0x0800,0x0fff,0,0x07ff,1,1,0x07ff,0x0800].map(u16));
const signed8=cat(header([64]),Buffer.from([3,128,255,0,127,0]));
const unsigned16=cat(header([64,70]),Buffer.from([3,0,255,128,0,0]),Buffer.from([3,0,255,0,1,0]));
const files={'rle-i8.dcm':file({frames:1,rows:1,columns:4,allocated:8,stored:8,signed:1,rle:[signed8]}),'native-i8.dcm':file({frames:1,rows:1,columns:4,allocated:8,stored:8,signed:1,raw:Buffer.from([128,255,0,127])}),'rle-u16.dcm':file({frames:1,rows:1,columns:4,allocated:16,stored:16,signed:0,rle:[unsigned16]}),'native-u16.dcm':file({frames:1,rows:1,columns:4,allocated:16,stored:16,signed:0,raw:cat(...[0,65535,32768,1].map(u16))}),'rle-i12-two-frames-bot.dcm':file({...spec16,rle:[first,second]}),'rle-i12-two-frames-empty-bot.dcm':file({...spec16,rle:[first,second],emptyBot:true}),'native-i12-two-frames.dcm':file({...spec16,raw:native16}),'rle-u8-noop-repeat.dcm':file({...spec8,rle:[gray],emptyBot:true}),'native-u8.dcm':file({...spec8,raw:Buffer.from([7,7,7,7,1,2,3,4])})};
for(const [name,bytes] of Object.entries(files))writeFileSync(new URL(name,dir),bytes);
writeFileSync(new URL('frame-i12-first.rle',dir),first);writeFileSync(new URL('frame-u8.rle',dir),gray);
