//! Native Q42 → Solid LDP directory export (desktop / CLI leave path).

use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::io::{self, Write};
use std::path::Path;

use crate::q42::q42_lexicon::Q42Lexicon;
use crate::q42_volume::Q42Volume;
use crate::NQuin;

use super::bundle::{build_migration_bundle, SolidMigrationOptions};
use super::consent::{
    plan_sanctuary_migration_notice, SanctuaryExportChoice, SanctuaryMigrationNotice,
};

/// Translates a Qualia `.q42` volume into a Solid-portable RDF directory.
///
/// Writes:
/// - `data.ttl` — Turtle triples (lexicon-resolved when Q42LEX present)
/// - `data.nq` — N-Quads (named graphs / context preserved)
/// - `data.jsonld` — compact JSON-LD
/// - `data.ttl.acl` — WAC (owner-only unless `grant_public_read`)
/// - `manifest.jsonld` — leave/migrate receipt for Solid-Databox import
/// - `SANCTUARY_NOTICE.txt` — user-facing choice text (always written)
pub struct SolidExporter;

impl SolidExporter {
    /// Pre-flight: notice the desktop/CLI must show before asking for a choice.
    pub fn plan_notice(input_q42_path: &str) -> io::Result<SanctuaryMigrationNotice> {
        let (quins, _) = load_q42_quins_and_lex(Path::new(input_q42_path))?;
        Ok(plan_sanctuary_migration_notice(&quins))
    }

    /// Export with an **explicit** sanctuary choice. Use
    /// [`plan_notice`](Self::plan_notice) first when `requires_choice` is true.
    ///
    /// `export_to_solid_pod` without a choice uses `Unset` and fails closed if
    /// sanctuary data is present — desktop must collect omit vs reclassify.
    pub fn export_to_solid_pod(input_q42_path: &str, output_dir_path: &str) -> io::Result<()> {
        Self::export_with_choice(
            input_q42_path,
            output_dir_path,
            SanctuaryExportChoice::Unset,
        )
    }

    pub fn export_with_choice(
        input_q42_path: &str,
        output_dir_path: &str,
        choice: SanctuaryExportChoice,
    ) -> io::Result<()> {
        Self::export_with_options(
            input_q42_path,
            output_dir_path,
            &SolidMigrationOptions {
                sanctuary_choice: choice,
                grant_public_read: false,
                ..SolidMigrationOptions::default()
            },
        )
    }

    pub fn export_with_options(
        input_q42_path: &str,
        output_dir_path: &str,
        opts: &SolidMigrationOptions,
    ) -> io::Result<()> {
        let out_dir = Path::new(output_dir_path);
        create_dir_all(out_dir)?;

        let (quins, lex_map) = load_q42_quins_and_lex(Path::new(input_q42_path))?;
        let notice = plan_sanctuary_migration_notice(&quins);
        write_file(
            out_dir.join("SANCTUARY_NOTICE.txt"),
            format!(
                "{}\n\n{}\n\nOmit: {}\nReclassify: {}\n{}\n",
                notice.title,
                notice.body,
                notice.choice_omit_label,
                notice.choice_reclassify_label,
                notice.classified_note
            )
            .as_bytes(),
        )?;

        let bundle = build_migration_bundle(&quins, lex_map.as_ref(), opts)
            .map_err(|e| io::Error::new(io::ErrorKind::PermissionDenied, e))?;

        if let Some(ttl) = &bundle.turtle {
            write_file(out_dir.join("data.ttl"), ttl.as_bytes())?;
        }
        if let Some(nq) = &bundle.nquads {
            write_file(out_dir.join("data.nq"), nq.as_bytes())?;
        }
        if let Some(jl) = &bundle.jsonld {
            write_file(out_dir.join("data.jsonld"), jl.as_bytes())?;
        }
        if let Some(acl) = &bundle.acl {
            write_file(out_dir.join("data.ttl.acl"), acl.as_bytes())?;
        }
        write_file(
            out_dir.join("manifest.jsonld"),
            bundle.manifest_jsonld.as_bytes(),
        )?;

        println!(
            "Solid leave/migrate: {} → {}",
            bundle.outcome_summary, output_dir_path
        );
        Ok(())
    }
}

fn write_file(path: impl AsRef<Path>, bytes: &[u8]) -> io::Result<()> {
    let mut f = File::create(path)?;
    f.write_all(bytes)?;
    f.sync_all()
}

fn load_q42_quins_and_lex(path: &Path) -> io::Result<(Vec<NQuin>, Option<HashMap<u64, String>>)> {
    let quins = crate::q42_reader::read_q42_quins(path)?;
    let lex_map = Q42Volume::open(path)
        .ok()
        .and_then(|vol| Q42Lexicon::from_volume(&vol).ok())
        .map(|lex| lex.reverse);
    Ok((quins, lex_map))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q42_volume::UnifiedVolumeBuilder;
    use crate::q_hash;
    use tempfile::tempdir;

    #[test]
    fn q42_export_writes_turtle_and_manifest() {
        let dir = tempdir().unwrap();
        let q42 = dir.path().join("graph.q42");
        let out = dir.path().join("solid-out");

        let mut lex = HashMap::new();
        lex.insert(q_hash("Alice"), "https://example.org/Alice".into());
        lex.insert(q_hash("Bob"), "https://example.org/Bob".into());
        lex.insert(q_hash("knows"), "http://schema.org/knows".into());

        let mut quin = NQuin {
            subject: q_hash("Alice"),
            predicate: q_hash("knows"),
            object: q_hash("Bob"),
            context: 0,
            metadata: 0,
            parity: 0,
        };
        quin.recalculate_parity();

        let mut builder = UnifiedVolumeBuilder::with_lex_map(&lex).unwrap();
        builder.push_block(0, &[quin]).unwrap();
        builder.finish(&q42).unwrap();

        SolidExporter::export_with_choice(
            q42.to_str().unwrap(),
            out.to_str().unwrap(),
            SanctuaryExportChoice::OmitSanctuary,
        )
        .unwrap();

        let ttl = std::fs::read_to_string(out.join("data.ttl")).unwrap();
        assert!(ttl.contains("https://example.org/Alice"));
        assert!(ttl.contains("schema.org/knows"));
        let manifest = std::fs::read_to_string(out.join("manifest.jsonld")).unwrap();
        assert!(manifest.contains("SolidMigrationBundle"));
        assert!(out.join("SANCTUARY_NOTICE.txt").exists());
        assert!(out.join("data.nq").exists());
        assert!(out.join("data.ttl.acl").exists());
    }

    #[test]
    fn sanctuary_without_choice_fails() {
        let dir = tempdir().unwrap();
        let q42 = dir.path().join("graph.q42");
        let out = dir.path().join("solid-out");

        let mut lex = HashMap::new();
        lex.insert(q_hash("Alice"), "https://example.org/Alice".into());
        lex.insert(q_hash("secret"), "https://example.org/secret".into());
        lex.insert(q_hash("Bob"), "https://example.org/Bob".into());

        let mut quin = NQuin {
            subject: q_hash("Alice"),
            predicate: q_hash("secret"),
            object: q_hash("Bob"),
            context: 0,
            metadata: 0,
            parity: 0,
        };
        quin.set_sensitivity_byte(NQuin::SENSITIVITY_RESTRICTED);
        quin.recalculate_parity();

        let mut builder = UnifiedVolumeBuilder::with_lex_map(&lex).unwrap();
        builder.push_block(0, &[quin]).unwrap();
        builder.finish(&q42).unwrap();

        let err = SolidExporter::export_to_solid_pod(q42.to_str().unwrap(), out.to_str().unwrap())
            .unwrap_err();
        assert!(
            err.to_string().contains("SANCTUARY_CHOICE_REQUIRED"),
            "{err}"
        );
        let notice = SolidExporter::plan_notice(q42.to_str().unwrap()).unwrap();
        assert!(notice.requires_choice);
    }
}
