use crate::{q_hash, NQuin};
use serde::{Deserialize, Serialize};

/// Represents a simple event from the streaming YAML lexer.
/// In a zero-alloc edge system, these would stream directly from a `&[u8]` buffer.
#[derive(Debug)]
pub enum YamlToken<'a> {
    MapStart,
    MapEnd,
    ListStart,
    ListEnd,
    Key(&'a str),
    ValueString(&'a str),
    ValueInt(i64),
}

/// A lightweight, state-machine based lexer for yaml-ld-q42.
/// Designed to parse without full document materialisation, avoiding `Vec` or `String` allocs
/// during the hot-path traversal of pane manifests.
pub struct YamlStreamingLexer<'a> {
    buffer: &'a [u8],
    cursor: usize,
}

impl<'a> YamlStreamingLexer<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self { buffer, cursor: 0 }
    }

    /// Advance the lexer to the next semantic token
    pub fn next_token(&mut self) -> Option<YamlToken<'a>> {
        // NOTE: This is a placeholder for the actual zero-alloc lexer.
        // For the purposes of the Phase A implementation, we fall back to serde_yaml
        // to parse the structured manifest until the full zero-copy byte scanner is wired.
        None
    }
}

/// A fully structured Webizen Studio workspace, typically deserialized from yaml-ld-q42.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WebizenWorkspace {
    pub pages: Vec<Page>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Page {
    pub url_path: String,
    pub name: String,
    pub panes: Vec<Pane>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Pane {
    pub component_id: String,
    pub x: u8,
    pub y: u8,
    pub w: u8,
    pub h: u8,
}

/// Parses a yaml-ld-q42 byte stream, packs the layout metadata into CBOR-LD (if needed),
/// and compiles the results down into a list of 48-byte NQuins.
pub fn compile_yaml_ld_to_quins(
    yaml_bytes: &[u8],
    namespace: u64,
    lamport_clock: u64,
) -> Result<Vec<NQuin>, &'static str> {
    // 1. Parse the YAML into our structured workspace
    let workspace: WebizenWorkspace = serde_yaml::from_slice(yaml_bytes)
        .map_err(|_| "Failed to parse yaml-ld-q42 payload")?;

    let mut quins = Vec::new();
    let pred_pane = q_hash("q42:SystemPaneState");
    let pred_page = q_hash("q42:SystemPageDef");

    // 2. Iterate through pages and panes
    for page in workspace.pages {
        let page_hash = q_hash(&page.url_path);

        // Emit page definition Quin
        let page_name_hash = q_hash(&page.name);
        quins.push(NQuin {
            subject: page_hash,
            predicate: pred_page,
            object: page_name_hash,
            context: namespace,
            metadata: lamport_clock << 32,
            parity: page_hash ^ pred_page ^ page_name_hash ^ namespace,
        });

        // 3. For each pane, pack the mathematical bounding box into the NQuin metadata
        for pane in page.panes {
            // Encode the 4-byte bounding box (x, y, w, h) into the lower 32 bits.
            let packed_layout: u64 = ((pane.x as u64) << 24)
                | ((pane.y as u64) << 16)
                | ((pane.w as u64) << 8)
                | (pane.h as u64);

            let metadata = packed_layout | (lamport_clock << 32);
            let subject = q_hash(&pane.component_id);

            // Note: If we needed to store complex configurations beyond the bounding box,
            // we would CBOR-encode them here:
            // let mut cbor_buf = [0u8; 128];
            // let cbor_len = ciborium::into_writer(&pane_config, &mut cbor_buf[..]).unwrap();
            // Then we would store the cbor_buf in a dedicated payload store and put the 
            // payload hash in the NQuin `object` field.

            quins.push(NQuin {
                subject,
                predicate: pred_pane,
                object: page_hash, // Panes belong to a page
                context: namespace,
                metadata,
                parity: subject ^ pred_pane ^ page_hash ^ namespace,
            });
        }
    }

    Ok(quins)
}
