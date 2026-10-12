//! Caller-buffered SKL1 attachment and integrity-checked readback.
use super::header::{Container10dHeader,HeaderParseError,MAX_SECTION_COUNT};
use super::integrity::{compute_whole_file_crc32c,seal_whole_file_crc32c};
use super::mesh_section::{parse_mesh_header,MeshSectionError};
use super::section::{encode_container,parse_section_table,AlignmentTier,SectionInput,SectionTableError,SectionType};
use super::skeletal_section::{SkeletalSection,SkeletalSectionError};

#[derive(Debug,PartialEq,Eq)]
pub enum SkeletalContainerError {
    Header(HeaderParseError),Sections(SectionTableError),Mesh(MeshSectionError),
    Payload(SkeletalSectionError),Integrity,AlreadyPresent,MissingMesh,Missing,
    TooManySections,CountLimit,InvalidDescriptor,
}

fn checked_header(bytes:&[u8])->Result<Container10dHeader,SkeletalContainerError> {
    if bytes.len()>42*1024*1024 {return Err(SkeletalContainerError::CountLimit);}
    let header=Container10dHeader::parse(bytes).map_err(SkeletalContainerError::Header)?;
    if compute_whole_file_crc32c(bytes)!=header.header_crc32c {return Err(SkeletalContainerError::Integrity);}
    Ok(header)
}

/// Read the optional section after validating the existing container envelope,
/// whole-file seal, per-section seals and actual QuantizedMesh vertex count.
pub fn read_skeletal_section(bytes:&[u8])->Result<SkeletalSection<'_>,SkeletalContainerError> {
    let header=checked_header(bytes)?;
    let sections=parse_section_table(bytes,&header).map_err(SkeletalContainerError::Sections)?;
    let mesh=sections.iter().find(|s|s.typ()==Some(SectionType::QuantizedMesh)).ok_or(SkeletalContainerError::MissingMesh)?;
    let (mesh_header,_)=parse_mesh_header(&bytes[mesh.byte_offset as usize..(mesh.byte_offset+mesh.byte_length) as usize])
        .map_err(SkeletalContainerError::Mesh)?;
    let section=sections.iter().find(|s|s.typ()==Some(SectionType::SkeletalAnimation)).ok_or(SkeletalContainerError::Missing)?;
    if section.stride!=0||section.element_count!=0 {return Err(SkeletalContainerError::InvalidDescriptor);}
    SkeletalSection::parse(&bytes[section.byte_offset as usize..(section.byte_offset+section.byte_length) as usize],
        mesh_header.vertex_count as usize).map_err(SkeletalContainerError::Payload)
}

/// Attach an already encoded SKL1 payload to a sealed container, retaining all
/// existing fields. Output remains untouched on validation or capacity failure.
/// The section writer reports the required capacity in its structured error.
pub fn attach_skeletal_section_into(bytes:&[u8],payload:&[u8],out:&mut [u8])->Result<usize,SkeletalContainerError> {
    let header=checked_header(bytes)?;
    let sections=parse_section_table(bytes,&header).map_err(SkeletalContainerError::Sections)?;
    if sections.len()>=MAX_SECTION_COUNT as usize {return Err(SkeletalContainerError::TooManySections);}
    if sections.iter().any(|s|s.typ()==Some(SectionType::SkeletalAnimation)) {return Err(SkeletalContainerError::AlreadyPresent);}
    let mesh=sections.iter().find(|s|s.typ()==Some(SectionType::QuantizedMesh)).ok_or(SkeletalContainerError::MissingMesh)?;
    let (mesh_header,_)=parse_mesh_header(&bytes[mesh.byte_offset as usize..(mesh.byte_offset+mesh.byte_length) as usize])
        .map_err(SkeletalContainerError::Mesh)?;
    SkeletalSection::parse(payload,mesh_header.vertex_count as usize).map_err(SkeletalContainerError::Payload)?;
    let empty=SectionInput{section_type:SectionType::Undefined,alignment_tier:AlignmentTier::Byte,stride:0,element_count:0,payload:&[]};
    let mut inputs=[empty;MAX_SECTION_COUNT as usize];
    for (i,section) in sections.iter().enumerate() {
        inputs[i]=SectionInput{section_type:section.typ().ok_or(SkeletalContainerError::InvalidDescriptor)?,
            alignment_tier:section.tier().ok_or(SkeletalContainerError::InvalidDescriptor)?,
            stride:section.stride,element_count:section.element_count,
            payload:&bytes[section.byte_offset as usize..(section.byte_offset+section.byte_length) as usize]};
    }
    inputs[sections.len()]=SectionInput{section_type:SectionType::SkeletalAnimation,
        alignment_tier:AlignmentTier::Word,stride:0,element_count:0,payload};
    // Admission before writes also enforces the enclosing execution budget.
    let needed=match encode_container(&header,&inputs[..sections.len()+1],&mut []) {
        Err(SectionTableError::OutputBufferTooSmall{needed,..})=>needed,
        Err(error)=>return Err(SkeletalContainerError::Sections(error)),Ok(needed)=>needed,
    };
    if needed>42*1024*1024 {return Err(SkeletalContainerError::CountLimit);}
    let written=encode_container(&header,&inputs[..sections.len()+1],out).map_err(SkeletalContainerError::Sections)?;
    seal_whole_file_crc32c(&mut out[..written]);
    Ok(written)
}
