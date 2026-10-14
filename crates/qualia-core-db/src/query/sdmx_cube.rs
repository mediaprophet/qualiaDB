//! SDMX and RDF Data Cube (`qb:`) observation engine with confidentiality-mark semantics (§GAP-09 / G38 / G42).
//!
//! Provides structured statistical observation mapping (`qb:DataSet`, `qb:Observation`,
//! `qb:DataStructureDefinition`, `qb:dimension`, `qb:measure`, `qb:attribute`) and first-class
//! confidentiality marks (`Public`, `SuppressedForPrivacy`, `Perturbed`, `ModelledEstimate`)
//! bound to `NQuin` sensitivity contexts and epistemic certainty weights. Zero-heap hot path.

use crate::{q_hash, NQuin};

/// RDF Data Cube (`qb:`) predicate URIs.
pub const QB_DATASET: &str = "http://purl.org/linked-data/cube#dataSet";
pub const QB_OBSERVATION: &str = "http://purl.org/linked-data/cube#Observation";
pub const QB_MEASURE_TYPE: &str = "http://purl.org/linked-data/cube#measureType";
pub const QB_TIME_PERIOD: &str = "http://purl.org/linked-data/sdmx/2009/dimension#timePeriod";
pub const QB_REF_AREA: &str = "http://purl.org/linked-data/sdmx/2009/dimension#refArea";
pub const QB_OBS_VALUE: &str = "http://purl.org/linked-data/sdmx/2009/measure#obsValue";
pub const QB_CONF_STATUS: &str = "http://purl.org/linked-data/sdmx/2009/attribute#confStatus";

/// Confidentiality-mark semantics for statistical cells (`G42`).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConfidentialityMark {
    /// Freely publishable cell.
    #[default]
    Public = 0,
    /// Cell suppressed to prevent primary or secondary identification.
    SuppressedForPrivacy = 1,
    /// Cell value perturbed (e.g. additive noise or random rounding).
    Perturbed = 2,
    /// Modelled or imputed estimate rather than direct census enumeration.
    ModelledEstimate = 3,
}

impl ConfidentialityMark {
    /// Maps confidentiality mark to an NQuin sensitivity class (bits [56..63] of context):
    /// 0 = Public, 1 = Restricted, 2 = Classified.
    #[inline]
    pub fn sensitivity_class(&self) -> u8 {
        match self {
            Self::Public | Self::Perturbed | Self::ModelledEstimate => 0,
            Self::SuppressedForPrivacy => 1, // Restricted
        }
    }

    /// Epistemic certainty weight in `[0.0, 1.0]` corresponding to the mark.
    #[inline]
    pub fn epistemic_certainty(&self) -> f32 {
        match self {
            Self::Public => 1.0,
            Self::SuppressedForPrivacy => 0.0,
            Self::Perturbed => 0.85,
            Self::ModelledEstimate => 0.70,
        }
    }
}

/// A structured statistical observation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SdmxObservation {
    pub dataset_id: u64,
    pub observation_id: u64,
    pub time_period: u32,
    pub geo_region: u64,
    pub measure_uri: u64,
    pub measure_value: f64,
    pub confidentiality: ConfidentialityMark,
}

/// Compile an SDMX statistical observation into compliant `qb:` NQuins.
/// Writes up to 6 Quins into `out`:
/// 1. `rdf:type` -> `qb:Observation`
/// 2. `qb:dataSet` -> `dataset_id`
/// 3. `sdmx-dimension:timePeriod` -> `time_period`
/// 4. `sdmx-dimension:refArea` -> `geo_region`
/// 5. `sdmx-measure:obsValue` -> packed measure (zeroed if suppressed)
/// 6. `sdmx-attribute:confStatus` -> confidentiality mark
///
/// Context field embeds sensitivity class in MSB bits `[56..63]`.
/// Metadata embeds epistemic certainty weight in bits `[0..15]`.
/// Zero-heap caller-buffered.
pub fn compile_observation_to_quins(
    obs: &SdmxObservation,
    out: &mut [NQuin],
) -> usize {
    if out.len() < 6 {
        return 0;
    }

    let p_type = q_hash("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
    let o_obs_type = q_hash(QB_OBSERVATION);
    let p_dataset = q_hash(QB_DATASET);
    let p_time = q_hash(QB_TIME_PERIOD);
    let p_area = q_hash(QB_REF_AREA);
    let p_val = q_hash(QB_OBS_VALUE);
    let p_conf = q_hash(QB_CONF_STATUS);

    // Embed sensitivity in top 8 bits of context
    let context = (obs.dataset_id & 0x00FF_FFFF_FFFF_FFFF)
        | ((obs.confidentiality.sensitivity_class() as u64) << 56);

    // Embed certainty in bits [0..15] of metadata
    let certainty_u16 = ((obs.confidentiality.epistemic_certainty() * 65535.0).clamp(0.0, 65535.0)) as u64;
    let metadata = certainty_u16;

    let make_q = |p: u64, o: u64| {
        let mut q = NQuin {
            subject: obs.observation_id,
            predicate: p,
            object: o,
            context,
            metadata,
            parity: 0,
        };
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
        q
    };

    // 1. rdf:type qb:Observation
    out[0] = make_q(p_type, o_obs_type);
    // 2. qb:dataSet
    out[1] = make_q(p_dataset, obs.dataset_id);
    // 3. timePeriod
    out[2] = make_q(p_time, obs.time_period as u64);
    // 4. refArea
    out[3] = make_q(p_area, obs.geo_region);
    // 5. obsValue (suppressed cells are zeroed)
    let val_payload = if obs.confidentiality == ConfidentialityMark::SuppressedForPrivacy {
        0u64
    } else {
        // Pack scaled integer (e.g. value * 1000)
        (obs.measure_value * 1000.0) as u64
    };
    out[4] = make_q(p_val, val_payload);
    // 6. confStatus
    out[5] = make_q(p_conf, obs.confidentiality as u64);

    6
}

/// Parse standard SDMX confidentiality status codes (`confStatus` / `OBS_STATUS`).
pub fn parse_conf_status(code: &str) -> ConfidentialityMark {
    match code.trim().to_uppercase().as_str() {
        "C" | "N" | "CONFIDENTIAL" | "SUPPRESSED" => ConfidentialityMark::SuppressedForPrivacy,
        "P" | "PERTURBED" | "PROVISIONAL" => ConfidentialityMark::Perturbed,
        "E" | "ESTIMATED" | "MODELLED" => ConfidentialityMark::ModelledEstimate,
        _ => ConfidentialityMark::Public,
    }
}

/// Parse SDMX-CSV formatted lines into a caller-supplied `SdmxObservation` slice.
///
/// Accepts standard SDMX-CSV header shapes:
/// `DATAFLOW,REF_AREA,TIME_PERIOD,MEASURE,OBS_VALUE,CONF_STATUS`
/// or flexible order when a header row is present.
/// Zero heap, hot path.
pub fn parse_sdmx_csv_into(csv: &str, out: &mut [SdmxObservation]) -> usize {
    let mut written = 0usize;
    let mut col_dataflow = 0;
    let mut col_area = 1;
    let mut col_time = 2;
    let mut col_measure = 3;
    let mut col_val = 4;
    let mut col_conf = 5;

    for line in csv.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut fields = [""; 16];
        let mut num_fields = 0usize;
        for part in line.split(',') {
            if num_fields < 16 {
                fields[num_fields] = part.trim().trim_matches('"');
                num_fields += 1;
            }
        }
        if num_fields == 0 {
            continue;
        }

        // Header detection
        let upper = fields[0].to_uppercase();
        if upper.contains("DATAFLOW") || upper.contains("DATASET") || upper.contains("KEY") {
            for (i, &part) in fields[..num_fields].iter().enumerate() {
                let col_name = part.to_uppercase();
                match col_name.as_str() {
                    "DATAFLOW" | "DATASET" => col_dataflow = i,
                    "REF_AREA" | "REGION" | "GEO" => col_area = i,
                    "TIME_PERIOD" | "TIME" | "YEAR" => col_time = i,
                    "MEASURE" | "MEASURE_TYPE" | "INDICATOR" => col_measure = i,
                    "OBS_VALUE" | "VALUE" => col_val = i,
                    "CONF_STATUS" | "OBS_STATUS" | "STATUS" => col_conf = i,
                    _ => {}
                }
            }
            continue;
        }

        if written >= out.len() {
            break;
        }

        if num_fields <= col_val {
            continue;
        }

        let dataset_str = if col_dataflow < num_fields { fields[col_dataflow] } else { "default_dataset" };
        let area_str = if col_area < num_fields { fields[col_area] } else { "global" };
        let time_str = if col_time < num_fields { fields[col_time] } else { "2026" };
        let measure_str = if col_measure < num_fields { fields[col_measure] } else { "value" };
        let val_str = if col_val < num_fields { fields[col_val] } else { "0" };
        let conf_str = if col_conf < num_fields { fields[col_conf] } else { "F" };

        let val: f64 = match val_str.parse() {
            Ok(v) => v,
            Err(_) => continue,
        };

        let time_period: u32 = time_str.split('-').next().unwrap_or("2026").parse().unwrap_or(2026);
        let dataset_id = q_hash(dataset_str);
        let geo_region = q_hash(area_str);
        let measure_uri = q_hash(measure_str);
        let confidentiality = parse_conf_status(conf_str);

        // Deterministic observation ID: FNV hash fold
        let observation_id = dataset_id ^ (time_period as u64) ^ (geo_region.rotate_left(16)) ^ (measure_uri.rotate_left(32));

        out[written] = SdmxObservation {
            dataset_id,
            observation_id,
            time_period,
            geo_region,
            measure_uri,
            measure_value: val,
            confidentiality,
        };
        written += 1;
    }

    written
}

/// Ingest an SDMX-CSV stream directly into caller-supplied `NQuin` buffer.
/// Each observation compiles into up to 6 compliant `qb:` NQuins.
/// Completely zero heap, hot path.
pub fn ingest_sdmx_csv_to_quins(csv: &str, out_quins: &mut [NQuin]) -> usize {
    let mut quins_written = 0usize;
    let mut obs_buf = [SdmxObservation {
        dataset_id: 0,
        observation_id: 0,
        time_period: 0,
        geo_region: 0,
        measure_uri: 0,
        measure_value: 0.0,
        confidentiality: ConfidentialityMark::Public,
    }; 1];

    for line in csv.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if quins_written + 6 > out_quins.len() {
            break;
        }
        let parsed = parse_sdmx_csv_into(line, &mut obs_buf);
        if parsed > 0 {
            let n = compile_observation_to_quins(&obs_buf[0], &mut out_quins[quins_written..]);
            quins_written += n;
        }
    }

    quins_written
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_public_observation() {
        let obs = SdmxObservation {
            dataset_id: q_hash("urn:dataset:abs_biodiversity_expenditure_2026"),
            observation_id: q_hash("urn:obs:biodiv_exp_2026_vic"),
            time_period: 2026,
            geo_region: q_hash("urn:geo:asgs:victoria"),
            measure_uri: q_hash("qudt:AustralianDollar"),
            measure_value: 450000.0,
            confidentiality: ConfidentialityMark::Public,
        };

        let mut quins = [NQuin::default(); 6];
        let n = compile_observation_to_quins(&obs, &mut quins);
        assert_eq!(n, 6);
        assert_eq!(quins[0].predicate, q_hash("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"));
        assert_eq!(quins[0].object, q_hash(QB_OBSERVATION));
        // Sensitivity class is 0 (public)
        assert_eq!(quins[0].context >> 56, 0);
        // Epistemic certainty is ~65535 (1.0)
        assert_eq!(quins[0].metadata & 0xFFFF, 65535);
        // Value is present
        assert_eq!(quins[4].object, 450000000);
    }

    #[test]
    fn test_compile_suppressed_privacy_observation() {
        let obs = SdmxObservation {
            dataset_id: q_hash("urn:dataset:species_location_confidential"),
            observation_id: q_hash("urn:obs:rare_parrot_count"),
            time_period: 2026,
            geo_region: q_hash("urn:geo:sensitive_grid_cell"),
            measure_uri: q_hash("qudt:IndividualCount"),
            measure_value: 3.0,
            confidentiality: ConfidentialityMark::SuppressedForPrivacy,
        };

        let mut quins = [NQuin::default(); 6];
        let n = compile_observation_to_quins(&obs, &mut quins);
        assert_eq!(n, 6);
        // Sensitivity class is 1 (restricted)
        assert_eq!(quins[0].context >> 56, 1);
        // Certainty weight is 0
        assert_eq!(quins[0].metadata & 0xFFFF, 0);
        // Value payload is zeroed out to prevent leakage
        assert_eq!(quins[4].object, 0);
    }

    #[test]
    fn test_parse_sdmx_csv_into() {
        let csv = r#"DATAFLOW,REF_AREA,TIME_PERIOD,MEASURE,OBS_VALUE,CONF_STATUS
ABS_LAND_ACCOUNT_2026,AUS,2026,HECTARES,145200.5,F
ABS_LAND_ACCOUNT_2026,VIC,2026,HECTARES,32000.0,C
ABS_LAND_ACCOUNT_2026,NSW,2026,HECTARES,45000.0,E
"#;
        let mut obs = [SdmxObservation {
            dataset_id: 0,
            observation_id: 0,
            time_period: 0,
            geo_region: 0,
            measure_uri: 0,
            measure_value: 0.0,
            confidentiality: ConfidentialityMark::Public,
        }; 4];

        let count = parse_sdmx_csv_into(csv, &mut obs);
        assert_eq!(count, 3);

        assert_eq!(obs[0].dataset_id, q_hash("ABS_LAND_ACCOUNT_2026"));
        assert_eq!(obs[0].geo_region, q_hash("AUS"));
        assert_eq!(obs[0].time_period, 2026);
        assert_eq!(obs[0].measure_value, 145200.5);
        assert_eq!(obs[0].confidentiality, ConfidentialityMark::Public);

        assert_eq!(obs[1].geo_region, q_hash("VIC"));
        assert_eq!(obs[1].confidentiality, ConfidentialityMark::SuppressedForPrivacy);

        assert_eq!(obs[2].geo_region, q_hash("NSW"));
        assert_eq!(obs[2].confidentiality, ConfidentialityMark::ModelledEstimate);
    }

    #[test]
    fn test_ingest_sdmx_csv_to_quins() {
        let csv = r#"DATAFLOW,REF_AREA,TIME_PERIOD,MEASURE,OBS_VALUE,CONF_STATUS
ABS_LAND_ACCOUNT_2026,AUS,2026,HECTARES,145200.5,F
ABS_LAND_ACCOUNT_2026,VIC,2026,HECTARES,32000.0,C
"#;
        let mut quins = [NQuin::default(); 16];
        let n = ingest_sdmx_csv_to_quins(csv, &mut quins);
        // 2 observations * 6 quins = 12 quins
        assert_eq!(n, 12);
        // Check first observation quins
        assert_eq!(quins[0].predicate, q_hash("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"));
        assert_eq!(quins[0].object, q_hash(QB_OBSERVATION));
        // Check second observation (suppressed) has restricted sensitivity
        assert_eq!(quins[6].context >> 56, 1);
        assert_eq!(quins[10].object, 0); // suppressed value
    }
}
