//! Append-only experiment CSV (plan §5.1).

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

pub const CSV_HEADER: &str = "run_id,utc,git_sha,host_passport_key,adapter,backend,model_id,model_hash,layout,mode,profile,toggles_json,qualia_decode_tok_s,ollama_decode_tok_s,a_gap,prefill_tok_s,phase_ns_json,n_ulp_max,c_score,notes";

#[derive(Debug, Clone, Default)]
pub struct ExperimentRun {
    pub run_id: String,
    pub utc: String,
    pub git_sha: String,
    pub host_passport_key: String,
    pub adapter: String,
    pub backend: String,
    pub model_id: String,
    pub model_hash: String,
    pub layout: String,
    pub mode: String,
    pub profile: String,
    pub toggles_json: String,
    pub qualia_decode_tok_s: Option<f64>,
    pub ollama_decode_tok_s: Option<f64>,
    pub a_gap: Option<f64>,
    pub prefill_tok_s: Option<f64>,
    pub phase_ns_json: String,
    pub n_ulp_max: Option<u64>,
    pub c_score: Option<f64>,
    pub notes: String,
}

impl ExperimentRun {
    pub fn new_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let t = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        format!("run-{t}")
    }

    pub fn utc_now() -> String {
        // ISO-ish without chrono dep: unix ms is enough for lab correlation.
        use std::time::{SystemTime, UNIX_EPOCH};
        let t = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        format!("{t}")
    }

    pub fn compute_a_gap(&mut self) {
        if let (Some(q), Some(o)) = (self.qualia_decode_tok_s, self.ollama_decode_tok_s) {
            if q > 0.0 {
                self.a_gap = Some(o / q);
            }
        }
    }

    fn csv_escape(s: &str) -> String {
        if s.contains(',') || s.contains('"') || s.contains('\n') {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_string()
        }
    }

    pub fn to_csv_line(&self) -> String {
        let fopt = |o: Option<f64>| o.map(|v| format!("{v:.6}")).unwrap_or_default();
        let uopt = |o: Option<u64>| o.map(|v| v.to_string()).unwrap_or_default();
        [
            Self::csv_escape(&self.run_id),
            Self::csv_escape(&self.utc),
            Self::csv_escape(&self.git_sha),
            Self::csv_escape(&self.host_passport_key),
            Self::csv_escape(&self.adapter),
            Self::csv_escape(&self.backend),
            Self::csv_escape(&self.model_id),
            Self::csv_escape(&self.model_hash),
            Self::csv_escape(&self.layout),
            Self::csv_escape(&self.mode),
            Self::csv_escape(&self.profile),
            Self::csv_escape(&self.toggles_json),
            fopt(self.qualia_decode_tok_s),
            fopt(self.ollama_decode_tok_s),
            fopt(self.a_gap),
            fopt(self.prefill_tok_s),
            Self::csv_escape(&self.phase_ns_json),
            uopt(self.n_ulp_max),
            fopt(self.c_score),
            Self::csv_escape(&self.notes),
        ]
        .join(",")
    }
}

/// Append one run; create file with header if missing.
pub fn append_run_csv(path: &Path, run: &ExperimentRun) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let need_header = !path.exists() || path.metadata().map(|m| m.len() == 0).unwrap_or(true);
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("open {}: {e}", path.display()))?;
    if need_header {
        writeln!(f, "{CSV_HEADER}").map_err(|e| e.to_string())?;
    }
    writeln!(f, "{}", run.to_csv_line()).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_line_roundtrip_fields() {
        let mut r = ExperimentRun {
            run_id: "r1".into(),
            utc: "1".into(),
            git_sha: "abc".into(),
            host_passport_key: "k".into(),
            adapter: "A2000".into(),
            backend: "vulkan".into(),
            model_id: "m".into(),
            model_hash: "0".into(),
            layout: "soa".into(),
            mode: "portable".into(),
            profile: "interactive".into(),
            toggles_json: "{}".into(),
            qualia_decode_tok_s: Some(2.0),
            ollama_decode_tok_s: Some(80.0),
            a_gap: None,
            prefill_tok_s: None,
            phase_ns_json: "{}".into(),
            n_ulp_max: Some(0),
            c_score: None,
            notes: "test".into(),
        };
        r.compute_a_gap();
        assert!((r.a_gap.unwrap() - 40.0).abs() < 1e-9);
        let line = r.to_csv_line();
        assert!(line.contains("80.000000"));
        assert!(line.starts_with("r1,"));
    }
}
