use super::*;
use crate::q42::emf_container::*;
use crate::q42::chunk_integrity::blake3_trailer;
use crate::container_10d::crc32c::crc32c;
use crate::q42::m64_matter::brick3d::encode_hilbert3d_u64;

fn field()->EmfFieldInterpretation {
    EmfFieldInterpretation{colour:EmfColourInterpretation::SpectralOnly,
        timebase:EmfTimebase{epoch:EmfEpoch{scale:EmfEpochScale::Tai,seconds:10,nanoseconds:250_000_000},
            ticks_per_second_num:48_000,ticks_per_second_den:1,slice_start_tick:100,slice_step_ticks:480},
        hilbert_order:2,spectral_semantics:EmfSpectralSemantics::AuthoredWavelengthLegacySigma}
}
fn spec(id:&str,channels:u16)->EmfAudioTrackSpec {
    EmfAudioTrackSpec{id:id.into(),purpose:EmfAudioPurpose::Recorded,channels,
        channel_layout:if channels==1{EmfChannelLayout::Mono}else{EmfChannelLayout::Stereo},
        sample_rate_num:1_000_000,sample_rate_den:1,start_tick:-24,
        units:EmfSignalUnits::Pascal,units_per_sample:0.1}
}
fn writer(field:&EmfFieldInterpretation)->EmfContainerWriter {
    let mut writer=EmfContainerWriter::new([0.;3],0.5,[1;3],field.timebase.epoch.header_seconds());
    let mut cells=[EmfCellRecord64::default();512];cells[0].wavelength_nm=1_000_000.;cells[0].spectral_sigma=0.5;
    writer.add_brick_chunk(0,&cells,0,1,0).unwrap();writer
}
fn sealed()->Vec<u8>{
    let field=field();let samples=[0.,-0.,2.5,-3.75];
    writer(&field).to_bytes_v3(&field,&[EmfAudioTrack{spec:spec("pressure",1),samples:&samples}],EmfExtensionLimits::conservative()).unwrap()
}
fn u64_at(bytes:&[u8],at:usize)->usize{u64::from_le_bytes(bytes[at..at+8].try_into().unwrap()) as usize}
fn rehash_extension(bytes:&mut[u8]){
    let footer=bytes.len()-64;let start=u64_at(bytes,footer+8);let length=u64_at(bytes,footer+32);
    let trailer=blake3_trailer(&bytes[start..start+length]);bytes[start+length..start+length+64].copy_from_slice(&trailer);
}
/// Rebuild an attacker-authored envelope with valid outer checksum/digest, leaving
/// independent PCM digests and semantic validation to catch inconsistent metadata.
fn rewrite_metadata(bytes:&[u8],change:impl FnOnce(&mut serde_json::Value))->Vec<u8>{
    let footer=&bytes[bytes.len()-64..];let start=u64_at(footer,8);let old_json=u64_at(footer,16);
    let old_aligned=(old_json+63)&!63;let pcm_length=u64_at(footer,24);
    let mut value:serde_json::Value=serde_json::from_slice(&bytes[start..start+old_json]).unwrap();change(&mut value);
    let json=serde_json::to_vec(&value).unwrap();let aligned=(json.len()+63)&!63;
    let mut out=bytes[..start].to_vec();out.extend_from_slice(&json);out.resize(start+aligned,0);
    out.extend_from_slice(&bytes[start+old_aligned..start+old_aligned+pcm_length]);
    let trailer=blake3_trailer(&out[start..]);out.extend_from_slice(&trailer);
    let mut locator=[0;64];locator.copy_from_slice(footer);
    locator[16..24].copy_from_slice(&(json.len() as u64).to_le_bytes());
    locator[32..40].copy_from_slice(&((aligned+pcm_length) as u64).to_le_bytes());
    let crc=crc32c(&locator[..60]);locator[60..].copy_from_slice(&crc.to_le_bytes());out.extend_from_slice(&locator);out
}

#[test]
fn v3_roundtrip_preserves_v2_cells_exact_pcm_bits_time_and_interpretation(){
    let field=field();let source=[0.,-0.,2.5,-3.75];let stereo=[0.1,-0.1,0.2,-0.2];
    let writer=writer(&field);let v2=writer.to_bytes().unwrap();
    let tracks=[EmfAudioTrack{spec:spec("pressure",1),samples:&source},EmfAudioTrack{spec:spec("stereo",2),samples:&stereo}];
    let v3=writer.to_bytes_v3(&field,&tracks,EmfExtensionLimits::conservative()).unwrap();
    assert_eq!(&v3[64..v2.len()],&v2[64..],"directory, cells and repaired full digests do not change");
    assert_eq!(writer.to_bytes().unwrap(),v2,"opt-in does not change default v2 serialization");
    let parsed=EmfContainerReader::parse(&v3).unwrap();assert_eq!(parsed.header().version,3);
    let ext=parsed.extension().unwrap();assert_eq!(ext.metadata().field,field);
    assert_eq!(ext.metadata().audio_tracks[0].codec,EmfAudioCodec::PcmF32Le);
    assert_eq!(ext.metadata().audio_tracks.len(),2);assert_eq!(ext.metadata().audio_tracks[0].spec.sample_rate_num,1_000_000);
    for (frame,sample) in source.iter().enumerate(){assert_eq!(ext.audio_sample(0,frame as u64,0).unwrap().to_bits(),sample.to_bits());}
    assert_eq!(ext.audio_sample(1,1,1).unwrap().to_bits(),stereo[3].to_bits());
    assert_eq!(parsed.chunk_payload(0).unwrap().len(),512*64);
    assert_eq!(parsed.cell(0,0).unwrap().wavelength_nm,1_000_000.,"IR/radio wavelength is not visible sigma");
    assert!(EmfContainerReader::parse(&v2).unwrap().extension().is_none());
}

#[test]
fn v3_dense_decode_uses_declared_hilbert_order(){
    let mut field=field();field.hilbert_order=3;
    let mut writer=EmfContainerWriter::new([0.;3],1.,[8,1,1],field.timebase.epoch.header_seconds());
    let mut cells=[EmfCellRecord64::default();512];cells[emf_brick_voxel_linear_index(2,0,0)].e_field=[123.,0.,0.];
    writer.add_brick_chunk(encode_hilbert3d_u64(5,0,0,3),&cells,0,1,0).unwrap();
    let bytes=writer.to_bytes_v3(&field,&[],EmfExtensionLimits::conservative()).unwrap();
    let dense=EmfContainerReader::parse(&bytes).unwrap().dense_volume().unwrap();
    assert_eq!(dense.cells[42].e_field,[123.,0.,0.]);
}

#[test]
fn v3_same_spatial_brick_selects_exact_time_and_legacy_lookup_defaults_to_zero(){
    let field=field();
    let mut writer=EmfContainerWriter::new([0.;3],1.,[1;3],field.timebase.epoch.header_seconds());
    let mut first=[EmfCellRecord64::default();512];
    let mut second=first;
    first[0].wavelength_nm=450.;first[0].e_field=[100.,0.,0.];
    second[0].wavelength_nm=650.;second[0].e_field=[200.,0.,0.];
    // Reverse insertion makes an accidental first-match or binary equality fail.
    writer.add_brick_chunk(0,&second,1,1,0).unwrap();
    writer.add_brick_chunk(0,&first,0,1,0).unwrap();
    let bytes=writer.to_bytes_v3(&field,&[],EmfExtensionLimits::conservative()).unwrap();
    let reader=EmfContainerReader::parse(&bytes).unwrap();
    let default=reader.find_chunk_by_hilbert(0).unwrap();
    assert_eq!(reader.chunk_entry(default).unwrap().time_slice_index,0);
    assert_eq!(reader.cell(default,0).unwrap().wavelength_nm,450.);
    assert_eq!(reader.lookup_brick_cell(0,0,0,0,0,0,2).unwrap().wavelength_nm,450.);
    assert_eq!(reader.lookup_brick_cell_at_time(0,0,0,0,0,0,2,1).unwrap().wavelength_nm,650.);
    assert!(reader.find_chunk_at_time(0,2).is_none());
    assert!(reader.lookup_brick_cell_at_time(0,0,0,0,0,0,2,2).is_none());
    let dense=reader.dense_volume().unwrap();
    assert_eq!(dense.cells[0].e_field,[100.,0.,0.]);
    assert_eq!(dense.cells[512].e_field,[200.,0.,0.]);
}

#[test]
fn v3_detects_all_extension_digest_bytes_and_pcm_tampering(){
    let bytes=sealed();let footer=bytes.len()-64;let start=u64_at(&bytes,footer+8);let payload=u64_at(&bytes,footer+32);
    for byte in 0..64{
        let mut bad=bytes.clone();bad[start+payload+byte]^=1;
        assert_eq!(EmfContainerReader::parse(&bad).err(),Some(EmfError::ExtensionHashMismatch));
    }
    let mut bad=bytes.clone();let json=u64_at(&bytes,footer+16);let pcm=start+((json+63)&!63);bad[pcm]^=1;
    assert_eq!(EmfContainerReader::parse(&bad).err(),Some(EmfError::ExtensionHashMismatch));
    rehash_extension(&mut bad);
    assert_eq!(EmfContainerReader::parse(&bad).err(),Some(EmfError::AudioHashMismatch{track_index:0}));
    assert_eq!(EmfContainerReader::parse_legacy_with_limits(&bad,EmfReadLimits::conservative()).err(),Some(EmfError::AudioHashMismatch{track_index:0}),"migration API never falls back after v3 failure");
}

#[test]
fn v3_refuses_unknown_metadata_overlap_nonfinite_padding_and_limits(){
    let bytes=sealed();
    for bad in [rewrite_metadata(&bytes,|v|v["field"]["colour"]["model"]=serde_json::json!("unknown")),
        rewrite_metadata(&bytes,|v|v["audio_tracks"][0]["pcm_offset"]=serde_json::json!(64)),
        rewrite_metadata(&bytes,|v|v["audio_tracks"][0]["spec"]["channels"]=serde_json::json!(2)),
        rewrite_metadata(&bytes,|v|v["field"]["timebase"]["ticks_per_second_den"]=serde_json::json!(0)),
        rewrite_metadata(&bytes,|v|v["audio_tracks"][0]["codec"]=serde_json::json!("unsupported")),
        rewrite_metadata(&bytes,|v|v["undeclared"]=serde_json::json!(true))]{assert!(EmfContainerReader::parse(&bad).is_err());}
    let footer=bytes.len()-64;let start=u64_at(&bytes,footer+8);let json=u64_at(&bytes,footer+16);let pcm=start+((json+63)&!63);
    let mut padded=bytes.clone();padded[pcm+16]=1;rehash_extension(&mut padded);assert!(EmfContainerReader::parse(&padded).is_err());
    let field=field();let nan=[f32::NAN];assert!(writer(&field).to_bytes_v3(&field,&[EmfAudioTrack{spec:spec("bad",1),samples:&nan}],EmfExtensionLimits::conservative()).is_err());
    let mut limits=EmfExtensionLimits::conservative();limits.max_file_bytes=bytes.len()-1;
    assert!(writer(&field).to_bytes_v3(&field,&[],limits).is_ok(),"a smaller different file remains admissible");
    assert!(EmfContainerReader::parse_with_extension_limits(&bytes,EmfReadLimits::conservative(),limits).is_err());
    let samples=[0.,1.];limits.max_audio_bytes=1;
    assert!(writer(&field).to_bytes_v3(&field,&[EmfAudioTrack{spec:spec("limit",1),samples:&samples}],limits).is_err());
}

#[test]
fn v3_checks_metadata_field_boundaries_and_preserves_exact_large_epoch(){
    let mut field=field();field.timebase.epoch.seconds=(1i64<<54)+1;field.timebase.epoch.nanoseconds=123_456_789;
    let bytes=writer(&field).to_bytes_v3(&field,&[],EmfExtensionLimits::conservative()).unwrap();
    let reader=EmfContainerReader::parse(&bytes).unwrap();assert_eq!(reader.extension().unwrap().metadata().field.timebase.epoch,field.timebase.epoch);
    let mut alias=bytes.clone();let mut entry:EmfChunkDirectoryEntry=bytemuck::pod_read_unaligned(&alias[64..128]);
    let footer=alias.len()-64;entry.byte_offset=u64_at(&alias,footer+8) as u64;entry.update_crc();alias[64..128].copy_from_slice(bytemuck::bytes_of(&entry));
    assert!(EmfContainerReader::parse(&alias).is_err());
    let mut wrong=bytes.clone();let mut header:EmfContainerHeader=bytemuck::pod_read_unaligned(&wrong[..64]);header.reference_epoch_s+=4096.;header.update_crc();wrong[..64].copy_from_slice(bytemuck::bytes_of(&header));
    assert!(EmfContainerReader::parse(&wrong).is_err());
}
