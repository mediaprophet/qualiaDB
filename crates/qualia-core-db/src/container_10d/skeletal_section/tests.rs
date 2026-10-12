use super::*;
use crate::render::skeletal::{Motor,Joint,Key,Track,SkinWeight,sample_pose_into,compose_palette_into,skin_vertices_into};

struct Fixture {joints:[Joint;2],ids:[u64;2],skin:[SkinWeight;3],clips:[ClipRecord;1],tracks:[Track;2],keys:[Key;4]}
impl Fixture {
    fn new()->Self {
        let angle=std::f64::consts::FRAC_PI_4;
        let identity=Motor::identity();let up=Motor::from_translation(0.0,1.0,0.0);
        Self{joints:[Joint{parent:None,rest_local:identity,inverse_bind:identity},
                Joint{parent:Some(0),rest_local:up,inverse_bind:Motor::from_translation(0.0,-1.0,0.0)}],
            ids:[crate::q_hash("urn:test:root"),crate::q_hash("urn:test:arm")],
            skin:[SkinWeight{joints:[1,0,0,0],weights:[1.0,0.0,0.0,0.0]};3],
            clips:[ClipRecord{id:crate::q_hash("urn:test:reach"),duration:1.0,first_track:0,track_count:2,looping:false}],
            tracks:[Track{joint:0,first_key:0,key_count:2},Track{joint:1,first_key:2,key_count:2}],
            keys:[Key{time:0.0,local:identity},Key{time:1.0,local:Motor::from_translation(2.0,0.0,0.0)},
                Key{time:0.0,local:up},Key{time:1.0,local:Motor::from_rotation_translation([angle.cos(),0.0,0.0,angle.sin()],[0.0,1.0,0.0])}]}
    }
    fn data(&self)->SkeletalData<'_> {SkeletalData{joints:&self.joints,joint_ids:&self.ids,skin:&self.skin,clips:&self.clips,tracks:&self.tracks,keys:&self.keys}}
}

#[test]
fn canonical_round_trip_drives_native_skinning() {
    let mut fixture=Fixture::new();let mut bytes=[0;728];
    assert_eq!(encode_skeletal_section_into(&fixture.data(),&mut bytes).unwrap(),bytes.len());
    let section=SkeletalSection::parse(&bytes,3).unwrap();
    assert_eq!(section.joint_id(1),Some(crate::q_hash("urn:test:arm")));
    section.decode_into(&mut fixture.joints,&mut fixture.ids,&mut fixture.skin,&mut fixture.clips,&mut fixture.tracks,&mut fixture.keys).unwrap();
    let mut second=[0;728];encode_skeletal_section_into(&fixture.data(),&mut second).unwrap();
    assert_eq!(bytes,second);
    let mut local=[Motor::identity();2];let mut global=local;let mut palette=local;
    sample_pose_into(&fixture.joints,clip_view(fixture.clips[0],&fixture.tracks,&fixture.keys).unwrap(),1.0,None,1.0,&mut local).unwrap();
    compose_palette_into(&fixture.joints,&local,Motor::identity(),&mut global,&mut palette).unwrap();
    let mut p=[[0.0;3];3];let mut n=p;
    skin_vertices_into(&[[0.0,2.0,0.0];3],&[[0.0,1.0,0.0];3],&fixture.skin,&palette,&mut p,&mut n).unwrap();
    for point in p {for (a,b) in point.iter().zip([1.0,1.0,0.0]) {assert!((a-b).abs()<1e-5);}}
    for normal in n {for (a,b) in normal.iter().zip([-1.0,0.0,0.0]) {assert!((a-b).abs()<1e-5);}}
}

#[test]
fn bad_bind_skin_ranges_and_loop_seams_are_rejected_without_output_writes() {
    let mut fixture=Fixture::new();let mut out=[99;728];
    fixture.joints[1].inverse_bind=Motor::identity();
    assert_eq!(encode_skeletal_section_into(&fixture.data(),&mut out),Err(SkeletalSectionError::InvalidData));
    assert_eq!(out,[99;728]);
    fixture=Fixture::new();fixture.skin[2].weights=[0.5,0.0,0.0,0.0];
    assert_eq!(encode_skeletal_section_into(&fixture.data(),&mut out),Err(SkeletalSectionError::InvalidData));
    fixture=Fixture::new();fixture.tracks[1].first_key=u32::MAX;
    assert_eq!(encode_skeletal_section_into(&fixture.data(),&mut out),Err(SkeletalSectionError::InvalidData));
    fixture=Fixture::new();fixture.clips[0].looping=true;
    assert_eq!(encode_skeletal_section_into(&fixture.data(),&mut out),Err(SkeletalSectionError::InvalidData));
    assert_eq!(out,[99;728]);
}

#[test]
fn malformed_binary_and_vertex_mismatch_fail_closed() {
    let fixture=Fixture::new();let mut bytes=[0;728];
    encode_skeletal_section_into(&fixture.data(),&mut bytes).unwrap();
    assert!(matches!(SkeletalSection::parse(&bytes,4),Err(SkeletalSectionError::Length)));
    bytes[HEADER_BYTES+2]=1;
    assert!(matches!(SkeletalSection::parse(&bytes,3),Err(SkeletalSectionError::Header)));
    bytes[HEADER_BYTES+2]=0;bytes[HEADER_BYTES..HEADER_BYTES+2].copy_from_slice(&0u16.to_le_bytes());
    assert!(matches!(SkeletalSection::parse(&bytes,3),Err(SkeletalSectionError::InvalidData)));
}

#[test]
fn short_decode_outputs_leave_every_stream_untouched() {
    let mut fixture=Fixture::new();let mut bytes=[0;728];
    encode_skeletal_section_into(&fixture.data(),&mut bytes).unwrap();
    let before=fixture.ids;
    let section=SkeletalSection::parse(&bytes,3).unwrap();
    assert_eq!(section.decode_into(&mut fixture.joints,&mut fixture.ids,&mut fixture.skin,&mut [],&mut fixture.tracks,&mut fixture.keys),
        Err(SkeletalSectionError::OutputTooSmall));
    assert_eq!(fixture.ids,before);
}

#[test]
fn sealed_container_attachment_retains_mesh_and_validates_integrity() {
    use crate::container_10d::skeletal_container::*;
    use crate::render::{assets::Mesh,compile_10d::{compile_mesh_to_10d_with_nodes,decode_10d_mesh}};
    let mesh=Mesh{positions:vec![[0.0,0.0,0.0],[1.0,0.0,0.0],[0.0,1.0,0.0]],triangles:vec![[0,1,2]],min:[0.0;3],max:[1.0,1.0,0.0]};
    let original=compile_mesh_to_10d_with_nodes(&mesh,&[]).unwrap();
    let fixture=Fixture::new();let mut payload=[0;728];encode_skeletal_section_into(&fixture.data(),&mut payload).unwrap();
    let mut output=[0;8192];let len=attach_skeletal_section_into(&original,&payload,&mut output).unwrap();
    assert_eq!(read_skeletal_section(&output[..len]).unwrap().counts().vertices,3);
    assert_eq!(decode_10d_mesh(&output[..len]).unwrap().triangles,mesh.triangles);
    assert!(matches!(attach_skeletal_section_into(&output[..len],&payload,&mut [0;8192]),Err(SkeletalContainerError::AlreadyPresent)));
    output[len-1]^=1;
    assert!(matches!(read_skeletal_section(&output[..len]),Err(SkeletalContainerError::Integrity)));
}

#[cfg(not(target_arch="wasm32"))]
#[test]
fn section_codec_allocates_no_heap() {
    use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;
    let source=Fixture::new();let mut decoded=Fixture::new();let mut bytes=[0;728];
    assert_zero_alloc("skeletal_section_codec",||{
        encode_skeletal_section_into(&source.data(),&mut bytes).unwrap();
        let section=SkeletalSection::parse(&bytes,3).unwrap();
        section.decode_into(&mut decoded.joints,&mut decoded.ids,&mut decoded.skin,&mut decoded.clips,&mut decoded.tracks,&mut decoded.keys).unwrap();
    });
}

fn player_fixture()->crate::render::skeletal::SkeletalPlayer {
    let fixture=Fixture::new();let mut bytes=[0;728];
    encode_skeletal_section_into(&fixture.data(),&mut bytes).unwrap();
    let section=SkeletalSection::parse(&bytes,3).unwrap();
    crate::render::skeletal::SkeletalPlayer::from_section(&section,
        &[[0.0,2.0,0.0],[0.1,2.0,0.0],[0.0,2.1,0.0]],&[[0.0,1.0,0.0];3],
        &[[0.0,0.0],[1.0,0.0],[0.0,1.0]],&[0,1,2]).unwrap()
}

#[test]
fn resident_playback_seek_does_not_accumulate_deformation_and_keeps_attachment() {
    let mut player=player_fixture();
    player.seek(1.0).unwrap();let receipt=player.advance(0.0,false).unwrap();
    assert_eq!(receipt.vertices,3);assert_eq!(receipt.joints,2);
    let first=player.positions().to_vec();
    for _ in 0..10 {player.seek(1.0).unwrap();player.advance(0.0,false).unwrap();assert_eq!(player.positions(),first);}
    let hand=player.attachment(crate::q_hash("urn:test:arm")).unwrap().transform_point([0.0;3]);
    assert_eq!(hand,[2.0,1.0,0.0]);
    player.set_output_frame([1.0,1.0,0.0],0.5).unwrap();player.advance(0.0,false).unwrap();
    assert!(player.positions()[0].iter().all(|v|v.abs()<1e-5));
    assert!(player.tangents().iter().flatten().all(|v|v.is_finite()));
}

#[test]
fn reduced_motion_uses_first_pose_and_bad_commands_do_not_replace_clip() {
    use crate::render::skeletal::PlayerError;
    let mut player=player_fixture();player.seek(1.0).unwrap();
    let receipt=player.advance(0.0,true).unwrap();assert!(receipt.reduced_motion);
    assert_eq!(player.positions()[0],[0.0,2.0,0.0]);
    assert_eq!(player.select_clip(12345,0.2),Err(PlayerError::UnknownClip));
    assert_eq!(player.advance(f64::NAN,false),Err(PlayerError::InvalidInput));
    assert_eq!(player.advance(0.0,false).unwrap().clip_id,crate::q_hash("urn:test:reach"));
}

#[cfg(not(target_arch="wasm32"))]
#[test]
fn resident_pose_normal_and_tangent_playback_allocates_no_heap() {
    use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;
    let mut player=player_fixture();
    assert_zero_alloc("resident_skinning_frame",||{
        for _ in 0..30 {player.advance(1.0/60.0,false).unwrap();}
    });
}

#[test]
fn clip_change_blends_from_last_sampled_pose() {
    use crate::render::skeletal::SkeletalPlayer;
    let fixture=Fixture::new();
    let clips=[fixture.clips[0],ClipRecord{id:crate::q_hash("urn:test:idle"),duration:1.0,first_track:2,track_count:1,looping:true}];
    let tracks=[fixture.tracks[0],fixture.tracks[1],Track{joint:0,first_key:4,key_count:2}];
    let keys=[fixture.keys[0],fixture.keys[1],fixture.keys[2],fixture.keys[3],
        Key{time:0.0,local:Motor::identity()},Key{time:1.0,local:Motor::identity()}];
    let data=SkeletalData{joints:&fixture.joints,joint_ids:&fixture.ids,skin:&fixture.skin,clips:&clips,tracks:&tracks,keys:&keys};
    let mut bytes=vec![0;data.counts().encoded_len().unwrap()];encode_skeletal_section_into(&data,&mut bytes).unwrap();
    let section=SkeletalSection::parse(&bytes,3).unwrap();
    let mut player=SkeletalPlayer::from_section(&section,&[[0.0,2.0,0.0];3],&[[0.0,1.0,0.0];3],
        &[[0.0,0.0];3],&[0,1,2]).unwrap();
    player.seek(1.0).unwrap();player.advance(0.0,false).unwrap();
    player.select_clip(clips[1].id,1.0).unwrap();player.advance(0.5,false).unwrap();
    let root=player.attachment(fixture.ids[0]).unwrap().transform_point([0.0;3]);
    assert!((root[0]-1.0).abs()<1e-6);
}
