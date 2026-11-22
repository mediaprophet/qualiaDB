//! Semantic segment addresses carried without repurposing packed HOLO lanes.

use super::{
    BundleReader, MODEL_ALIAS, RDF_TYPE, SEMANTIC_MODEL_KEY, invalid, open_semantic_model,
    valid_iri,
};
use crate::{
    NQuin,
    container_10d::{
        Container10dHeader, EulerianWavefieldRecord, FIELD_KIND_HOLOGRAPHIC_PHASE_SPACE,
        HOLOGRAPHIC_PHASE_HEADER_SIZE, HolographicPhaseHeader, MANIFOLD_IDENTITY_V2_MAX_FIELDS,
        MAX_HOLO_SEGMENTS, MultilingualLexicalTerm, SymbolicRdfCborLdPayload, TypedFieldRecord,
        TypedPayloadResolver, compute_whole_file_crc32c, decode_holographic_phase_section_into,
        holographic_phase_encoded_len,
    },
    q_hash,
    q42::m64_matter::M64HoloRecord,
};
use bytemuck::Zeroable;
use std::{collections::BTreeMap, io};

pub const HOLO_ADDRESS_CONTEXT: &str = "urn:qualia:10d:holo:addressability:v1";
const HAS_SEGMENT: &str = "https://qualiadb.org/ontology/hmc#hasHoloSegment";
const SEGMENT_CLASS: &str = "https://qualiadb.org/ontology/hmc#HoloSegment";

/// Stable IRI for a zero-based native HOLO segment ordinal in a root MID2 field.
/// The packed segment lanes remain native geometry/provenance data.
pub fn holo_segment_iri(root_hash: u64, field_id: u64, ordinal: u16) -> String {
    format!("urn:qualia:10d:holo:{root_hash:016x}:{field_id:016x}:{ordinal}")
}

/// Resolve a Q42 semantic hash to its canonical IRI and borrowed HOLO record.
/// The entry key selects the asset when multiple siblings share an authored root.
/// `lex_term` receives the canonical IRI from the mounted model.
pub fn open_hmc_holo_segment<'a, 'lex>(
    bundle: &BundleReader<'a>,
    key: &str,
    semantic_id: u64,
    lex_page: &mut [u8],
    lex_term: &'lex mut [u8],
) -> io::Result<(&'lex str, &'a [u8; 64])> {
    if bundle.entry(key).is_none_or(|entry| entry.kind != "10d") {
        return Err(invalid("HOLO segment requires a .10d bundle entry"));
    }
    let model_key = if bundle.entry(SEMANTIC_MODEL_KEY).is_some() {
        SEMANTIC_MODEL_KEY
    } else {
        MODEL_ALIAS
    };
    if !bundle.verify_entry(model_key) || !bundle.verify_entry(key) {
        return Err(invalid(
            "HMC semantic model or HOLO resource digest mismatch",
        ));
    }
    let model = open_semantic_model(bundle)?;
    let len = model
        .lookup_lexicon_hash_into(semantic_id, lex_page, lex_term)?
        .ok_or_else(|| invalid("HOLO semantic identity is absent from Q42LEX"))?;
    let iri = lex_term
        .get(..len)
        .and_then(|term| std::str::from_utf8(term).ok())
        .ok_or_else(|| invalid("invalid HOLO semantic IRI"))?;
    if semantic_id == 0 || q_hash(iri) != semantic_id {
        return Err(invalid("HOLO segment hash and lexical identity disagree"));
    }
    let (root, field_id, ordinal) = parse_holo_segment_iri(iri)
        .ok_or_else(|| invalid("expected a canonical HOLO segment IRI"))?;
    let bytes = bundle
        .get(key)
        .ok_or_else(|| invalid("missing .10d resource bytes"))?;
    let header = Container10dHeader::parse(bytes).map_err(|error| invalid(error.to_string()))?;
    if compute_whole_file_crc32c(bytes) != header.header_crc32c {
        return Err(invalid("HOLO resource whole-file checksum mismatch"));
    }
    let mut scratch = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
    let mut fields = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
    let view = TypedPayloadResolver::new(bytes, &[], &mut scratch, &mut fields)
        .map_err(|error| invalid(error.to_string()))?;
    let field = view
        .field(field_id)
        .map_err(|error| invalid(error.to_string()))?
        .filter(|field| field.record.kind == FIELD_KIND_HOLOGRAPHIC_PHASE_SPACE)
        .ok_or_else(|| invalid("HOLO segment field is absent from the selected resource"))?;
    let holo = validated_holo_header(field.bytes)?;
    if holo.nquin_subject_hash != root || ordinal >= holo.holo_segment_count {
        return Err(invalid(
            "HOLO semantic root or segment ordinal conflicts with the native payload",
        ));
    }
    let offset = HOLOGRAPHIC_PHASE_HEADER_SIZE + usize::from(ordinal) * 64;
    let record = field
        .bytes
        .get(offset..offset + 64)
        .and_then(|record| record.try_into().ok())
        .ok_or_else(|| invalid("HOLO segment extent exceeds the declared field"))?;
    Ok((iri, record))
}

fn parse_holo_segment_iri(iri: &str) -> Option<(u64, u64, u16)> {
    let mut parts = iri.strip_prefix("urn:qualia:10d:holo:")?.split(':');
    let parse_hash = |text: &str| {
        if text.len() != 16
            || !text
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return None;
        }
        u64::from_str_radix(text, 16)
            .ok()
            .filter(|&value| value != 0)
    };
    let root = parse_hash(parts.next()?)?;
    let field_id = parse_hash(parts.next()?)?;
    let ordinal = parts.next()?;
    if parts.next().is_some()
        || (ordinal != "0" && ordinal.starts_with('0'))
        || !ordinal.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    Some((root, field_id, ordinal.parse().ok()?))
}

pub(crate) fn validated_holo_header(bytes: &[u8]) -> io::Result<HolographicPhaseHeader> {
    let raw = bytes
        .get(..HOLOGRAPHIC_PHASE_HEADER_SIZE)
        .ok_or_else(|| invalid("missing HOLO header"))?;
    let header: HolographicPhaseHeader = bytemuck::pod_read_unaligned(raw);
    let segments = usize::from(header.holo_segment_count);
    let wavefields = usize::from(header.wavefield_count);
    if holographic_phase_encoded_len(segments, wavefields) != Some(bytes.len()) {
        return Err(invalid(
            "HOLO segment extent does not match its native directory",
        ));
    }
    let mut decoded = vec![M64HoloRecord::default(); segments];
    let mut waves = vec![EulerianWavefieldRecord::zeroed(); wavefields];
    let (header, _, _) = decode_holographic_phase_section_into(bytes, &mut decoded, &mut waves)
        .map_err(|error| invalid(format!("HOLO payload: {error:?}")))?;
    Ok(header)
}

pub(crate) fn holo_address_payload(
    root_iri: &str,
    field_id: u64,
    segment_count: u16,
) -> io::Result<SymbolicRdfCborLdPayload> {
    let root = q_hash(root_iri);
    let count = usize::from(segment_count);
    if root == 0 || !valid_iri(root_iri) || field_id == 0 || count > MAX_HOLO_SEGMENTS {
        return Err(invalid(
            "invalid HOLO root, field or segment count for semantic projection",
        ));
    }
    let term = |iri: String| MultilingualLexicalTerm {
        term_hash: q_hash(&iri),
        iri_utf8: iri,
        lang_bcp47: None,
        label_utf8: String::new(),
    };
    let graph = q_hash(HOLO_ADDRESS_CONTEXT);
    let fact = |s, p, o| {
        [
            s,
            p,
            o,
            graph,
            0,
            NQuin::calculate_parity(s, p, o, graph, 0),
        ]
    };
    let link = q_hash(HAS_SEGMENT);
    let kind = q_hash(RDF_TYPE);
    let class = q_hash(SEGMENT_CLASS);
    let mut payload = SymbolicRdfCborLdPayload {
        context_iri: HOLO_ADDRESS_CONTEXT.into(),
        lexicon: Vec::with_capacity(count + 5),
        nquins: Vec::with_capacity(count * 2),
        ..SymbolicRdfCborLdPayload::default()
    };
    payload.lexicon.extend(
        [
            root_iri,
            HOLO_ADDRESS_CONTEXT,
            HAS_SEGMENT,
            RDF_TYPE,
            SEGMENT_CLASS,
        ]
        .into_iter()
        .map(|iri| term(iri.to_owned())),
    );
    for ordinal in 0..segment_count {
        let entry = term(holo_segment_iri(root, field_id, ordinal));
        if entry.term_hash == 0 {
            return Err(invalid("HOLO segment IRI hashes to zero"));
        }
        payload.nquins.push(fact(root, link, entry.term_hash));
        payload.nquins.push(fact(entry.term_hash, kind, class));
        payload.lexicon.push(entry);
    }
    let mut spellings = BTreeMap::new();
    for term in &payload.lexicon {
        if spellings
            .insert(term.term_hash, term.iri_utf8.as_str())
            .is_some_and(|previous| previous != term.iri_utf8.as_str())
        {
            return Err(invalid(
                "HOLO address dictionary contains a q_hash collision",
            ));
        }
    }
    Ok(payload)
}
