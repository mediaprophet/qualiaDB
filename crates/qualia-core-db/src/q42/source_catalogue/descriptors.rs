//! Static source / connector descriptors for the Poet health asset programme.
//!
//! Conservative acquisition only: no downloaders, no bundled bytes, and no
//! redistribution claims without verified artifact terms (playbook AST-07).

use super::AcquisitionStatus;
use super::SourceDescriptor;

/// Complete static catalogue (FooDB … Cytoscape + ChEBI importer candidate).
pub(crate) static SOURCES: &[SourceDescriptor] = &[
    SourceDescriptor {
        id: "chebi",
        name: "ChEBI",
        official_url: "https://www.ebi.ac.uk/chebi/",
        status: AcquisitionStatus::Catalogue,
        licence_note: "Upstream release stated as CC BY 4.0; first production importer candidate. Catalogue entry only — do not bundle release bytes in-repo.",
        role: "Canonical chemical identity and ontology spine (Wave 1 identifier authority).",
    },
    SourceDescriptor {
        id: "foodb",
        name: "FooDB",
        official_url: "https://foodb.ca/",
        status: AcquisitionStatus::Restricted,
        licence_note: "Reported CC BY-NC 4.0 on source materials; non-commercial / research lane only. Do not place in a generally redistributable commercial bundle.",
        role: "Food composition, compounds, proteins, nutrients, and spectra.",
    },
    SourceDescriptor {
        id: "hmdb",
        name: "HMDB",
        official_url: "https://hmdb.ca/",
        status: AcquisitionStatus::Restricted,
        licence_note: "Commercial redistribution requires explicit permission. Support user-supplied / local import only until a written grant is recorded.",
        role: "Human metabolites, structures, spectra, and pathways.",
    },
    SourceDescriptor {
        id: "ctd",
        name: "CTD",
        official_url: "https://ctdbase.org/",
        status: AcquisitionStatus::Restricted,
        licence_note: "Non-commercial use commonly free; commercial licence required. Preserve terms and keep commercial distribution separate.",
        role: "Chemical–gene–disease–phenotype / exposure relationships.",
    },
    SourceDescriptor {
        id: "monarch",
        name: "Monarch Initiative",
        official_url: "https://monarchinitiative.org/",
        status: AcquisitionStatus::Unverified,
        licence_note: "Upstream edge licences vary by contributing source. Fail closed until each partition's obligations are recorded; do not treat the aggregate as redistributable.",
        role: "Disease, phenotype, gene, and treatment association graph.",
    },
    SourceDescriptor {
        id: "abckb",
        name: "ABCkb",
        official_url: "https://github.com/cbmi-group/ABCkb",
        status: AcquisitionStatus::Unverified,
        licence_note: "Software artefact commonly GPLv3; data-artefact licence is not confirmed from the software licence. Verify the data licence separately before any import or redistribution claim.",
        role: "Plant–chemical–human condition knowledge graph (Wave 2 discovery).",
    },
    SourceDescriptor {
        id: "foodatlas",
        name: "FoodAtlas",
        official_url: "https://www.foodatlas.ai/",
        status: AcquisitionStatus::Connector,
        licence_note: "Public Apache-2.0 code reported; API credentials by request. Connector / pipeline reference only — extracted claims must stay evidence-labelled; no bulk data redistributed here.",
        role: "Food–chemical–health literature graph and pipeline reference (Wave 2 connector).",
    },
    SourceDescriptor {
        id: "phenol-explorer",
        name: "Phenol-Explorer",
        official_url: "http://phenol-explorer.eu/",
        status: AcquisitionStatus::Unverified,
        licence_note: "Public web database and exports; licensing needs direct confirmation. Legal review required before any bundling or redistribution claim.",
        role: "Polyphenol food content, metabolism, and pharmacokinetics.",
    },
    SourceDescriptor {
        id: "foodball",
        name: "FOODBALL",
        official_url: "https://foodmetabolome.org/",
        status: AcquisitionStatus::Catalogue,
        licence_note: "Portal / directory of food-metabolome resources rather than one canonical bulk dataset. Catalogue guidance only — no monolithic asset to redistribute.",
        role: "Methods, guidelines, and directory of food-metabolome resources.",
    },
    SourceDescriptor {
        id: "phind",
        name: "PhInd",
        official_url: "https://phind.igfs.fraunhofer.de/",
        status: AcquisitionStatus::Unverified,
        licence_note: "Public-use statements appear in project material but redistribution terms are not independently verified here. Wave 2 after licence confirmation; preserve DOI and extraction method.",
        role: "Polyphenols in agri-food by-products with methods and DOI provenance.",
    },
    SourceDescriptor {
        id: "cytoscape",
        name: "Cytoscape",
        official_url: "https://cytoscape.org/",
        status: AcquisitionStatus::Connector,
        licence_note: "External desktop application (open-source app licence is separate from any health dataset). Integration / CyREST hand-off only — not a dataset to bundle.",
        role: "External network analysis and visualization connector (not a dataset).",
    },
];
