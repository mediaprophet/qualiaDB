#![allow(dead_code,unused_imports)]
//! Actual Qualia compiler/transcode/GGUF/safetensor/quant source. Unused platform
//! and Q4_AWQ helpers panic; all exercised F32 and raw serialization is real.
#[path="@QUALIA_ROOT@/crates/qualia-core-db/src/q42/p64_weight/mod.rs"] pub mod p64_weight;
#[path="@QUALIA_ROOT@/crates/qualia-core-db/src/inference/gguf_sharder/mod.rs"] pub mod gguf_sharder;
#[path="@QUALIA_ROOT@/crates/qualia-core-db/src/inference/safetensor.rs"] pub mod safetensor;
#[path="@QUALIA_ROOT@/crates/qualia-core-db/src/inference/ggml_quants.rs"] pub mod ggml_quants;
#[path="@QUALIA_ROOT@/crates/qualia-core-db/src/inference/ternary.rs"] pub mod ternary;
#[path="@QUALIA_ROOT@/crates/qualia-core-db/src/inference/tensor_roles.rs"] pub mod tensor_roles;
pub mod inference {pub use crate::tensor_roles;pub mod moe {pub mod nvfp4 {pub fn fp8_e4m3_to_f32(_:u8)->f32{panic!("FP8 not exercised")}}}}
mod container_10d {#[path="@QUALIA_ROOT@/crates/qualia-core-db/src/container_10d/crc32c.rs"] pub mod crc32c;}
mod modalities {pub mod manifold {include!("manifold_coordinate.rs");}}
mod llm_kernel_parity {
    pub fn q4_0_bytes(_:usize)->usize{panic!("Q4 AWQ not exercised")}
    pub fn quantize_q4_0_from_f32(_: &[f32],_:&mut[u8])->bool{panic!("Q4 AWQ not exercised")}
}
pub struct NQuin{pub subject:u64,pub predicate:u64,pub object:u64,pub context:u64,pub metadata:u64,pub parity:u64}
pub struct QualiaSuperBlock;
pub const MODALITY_FLAG_VISION_TENSOR:u8=0;
pub const MODALITY_FLAG_LLM_TENSOR:u8=0;
pub const MODALITY_FLAG_DENSE_PHYSICS:u8=0;
include!("q_hash.rs");
fn str64(out:&mut Vec<u8>,s:&str){out.extend_from_slice(&(s.len() as u64).to_le_bytes());out.extend_from_slice(s.as_bytes());}
fn kv(out:&mut Vec<u8>,name:&str,value:u32){str64(out,name);out.extend_from_slice(&4u32.to_le_bytes());out.extend_from_slice(&value.to_le_bytes());}
fn gguf()->Vec<u8>{
    let mut b=Vec::new();b.extend_from_slice(b"GGUF");b.extend_from_slice(&3u32.to_le_bytes());b.extend_from_slice(&5u64.to_le_bytes());b.extend_from_slice(&6u64.to_le_bytes());
    kv(&mut b,"llama.block_count",2);kv(&mut b,"llama.embedding_length",2);kv(&mut b,"llama.attention.head_count",1);kv(&mut b,"tokenizer.ggml.bos_token_id",0);kv(&mut b,"tokenizer.ggml.eos_token_id",1);
    str64(&mut b,"tokenizer.ggml.tokens");b.extend_from_slice(&9u32.to_le_bytes());b.extend_from_slice(&8u32.to_le_bytes());b.extend_from_slice(&2u64.to_le_bytes());str64(&mut b,"a");str64(&mut b,"b");
    for (name,offset) in [("extra.b",128u64),("blk.0.attn_k.weight",0),("token_embd.weight",64),("extra.a",96),("blk.0.attn_q.weight",32)]{
        str64(&mut b,name);b.extend_from_slice(&2u32.to_le_bytes());b.extend_from_slice(&2u64.to_le_bytes());b.extend_from_slice(&1u64.to_le_bytes());b.extend_from_slice(&0u32.to_le_bytes());b.extend_from_slice(&offset.to_le_bytes());
    }
    b.resize((b.len()+31)&!31,0);let start=b.len();b.resize(start+136,0);
    for (index,offset) in [0,32,64,96,128].into_iter().enumerate(){for (j,value) in [index as f32+0.25,-(index as f32+0.5)].into_iter().enumerate(){b[start+offset+j*4..start+offset+j*4+4].copy_from_slice(&value.to_le_bytes());}}
    b
}
fn raw()->Vec<u8>{
    let header=br#"{"z.unknown":{"dtype":"F32","shape":[2],"data_offsets":[16,24]},"blk.0.attn_q.weight":{"dtype":"F32","shape":[2],"data_offsets":[0,8]},"blk.0.ffn_up.weight":{"dtype":"F32","shape":[2],"data_offsets":[8,16]}}"#;
    let mut b=Vec::new();b.extend_from_slice(&(header.len()as u64).to_le_bytes());b.extend_from_slice(header);
    for v in [1f32,-2.,3.,-4.,5.,-6.]{b.extend_from_slice(&v.to_le_bytes());}b
}
fn main(){
    let output = std::env::args().nth(1).expect("caller-selected output directory"); let parent=std::path::Path::new(&output);
    let mut total=0;
    for page in [8,9,12]{
        let g=p64_weight::compile_gguf_to_p64(&gguf(),page).unwrap();
        let mut r=Vec::new();p64_weight::transcode_safetensor_to_p64(&raw(),page,&mut r).unwrap();
        let mut v=Vec::new();p64_weight::transcode_vision_tensors_to_p64(&[
            p64_weight::RawVisionTensor{name:"vision.z".into(),shape:vec![2],data:vec![1.,-2.],role_id:0x4444},
            p64_weight::RawVisionTensor{name:"vision.a".into(),shape:vec![1,2],data:vec![3.,4.],role_id:0x5555},
        ],page,&mut v).unwrap();
        for (profile,bytes) in [("gguf",g),("raw",r),("vision",v)]{
            let parsed=p64_weight::P64TensorIndex::from_p64_with_integrity(&bytes,p64_weight::IntegrityMode::Full).unwrap();
            assert!(!parsed.entries.is_empty());total+=bytes.len();assert!(total<262144);
            std::fs::write(parent.join(format!("{profile}-page{page}.p64")),&bytes).unwrap();
            println!("{profile} page_log2={page} bytes={} tensors={}",bytes.len(),parsed.entries.len());
        }
    }
}
