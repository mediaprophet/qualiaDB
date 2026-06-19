use std::fs::File;
use std::io::Read;
/// GLB Ingestion System for Anatomy Project
///
/// Zero-copy ingestion of heavy biomedical GLB assets using Binary IPC
/// Validates TensorBufferView performance with real-world 3D models
///
/// Zero-Heap Considerations:
/// - Memory-mapped file access (no full file read into RAM)
/// - Byte reference views (TensorBufferView pattern)
/// - Binary IPC transport (u64 indices instead of String IDs)
use std::path::Path;
use std::sync::Arc;

/// GLB asset metadata
#[derive(Debug, Clone)]
pub struct GLBMetadata {
    /// Asset identifier (binary index from BinaryNodeRegistry)
    pub asset_index: u64,
    /// Asset name (e.g., "blood-vasculature")
    pub asset_name: String,
    /// File path
    pub file_path: String,
    /// File size in bytes
    pub file_size: u64,
    /// Version (e.g., "v1.4")
    pub version: String,
}

/// GLB binary view for zero-copy access
///
/// Zero-Heap Consideration: Holds byte references, not heap allocations
#[derive(Debug, Clone)]
pub struct GLBView<'a> {
    /// Raw GLB byte data reference
    pub glb_data: &'a [u8],
    /// Asset metadata
    pub metadata: GLBMetadata,
}

impl<'a> GLBView<'a> {
    /// Create a new GLB view from byte data
    ///
    /// Zero-Heap Consideration: No allocation, just references
    pub fn new(glb_data: &'a [u8], metadata: GLBMetadata) -> Self {
        Self { glb_data, metadata }
    }

    /// Get GLB header (magic bytes + version + length)
    ///
    /// Zero-Heap Consideration: Returns byte slice reference
    pub fn header(&self) -> Option<&[u8]> {
        if self.glb_data.len() < 12 {
            return None;
        }
        Some(&self.glb_data[0..12])
    }

    /// Validate GLB magic bytes (glTF binary format)
    pub fn is_valid_glb(&self) -> bool {
        if let Some(header) = self.header() {
            // GLB magic bytes: "glTF" (0x676C5446)
            header[0..4] == [0x67, 0x6C, 0x54, 0x46]
        } else {
            false
        }
    }

    /// Get JSON chunk length (little-endian u32)
    pub fn json_chunk_length(&self) -> Option<u32> {
        if self.glb_data.len() < 16 {
            return None;
        }
        let bytes = &self.glb_data[12..16];
        Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// Get JSON chunk data
    ///
    /// Zero-Heap Consideration: Returns byte slice reference
    pub fn json_chunk(&self) -> Option<&[u8]> {
        let json_len = self.json_chunk_length()? as usize;
        if self.glb_data.len() < 20 + json_len {
            return None;
        }
        Some(&self.glb_data[20..20 + json_len])
    }

    /// Get binary chunk data (vertices, indices, normals)
    ///
    /// Zero-Heap Consideration: Returns byte slice reference for zero-copy transport
    pub fn binary_chunk(&self) -> Option<&[u8]> {
        let json_len = self.json_chunk_length()? as usize;
        let json_start = 20;
        let json_end = json_start + json_len;

        if self.glb_data.len() < json_end + 8 {
            return None;
        }

        // Binary chunk header
        let bin_len_bytes = &self.glb_data[json_end..json_end + 4];
        let bin_len = u32::from_le_bytes([
            bin_len_bytes[0],
            bin_len_bytes[1],
            bin_len_bytes[2],
            bin_len_bytes[3],
        ]) as usize;

        if self.glb_data.len() < json_end + 8 + bin_len {
            return None;
        }

        Some(&self.glb_data[json_end + 8..json_end + 8 + bin_len])
    }
}

/// GLB ingestion manager
///
/// Manages loading and zero-copy access to GLB assets
pub struct GLBIngestionManager {
    /// CCF base path
    ccf_base_path: String,
}

impl GLBIngestionManager {
    /// Create a new GLB ingestion manager
    pub fn new(ccf_base_path: String) -> Self {
        Self { ccf_base_path }
    }

    /// Load GLB file into memory (for validation)
    ///
    /// Zero-Heap Consideration: This loads the full file, but in production
    /// would use memory-mapped files for true zero-copy access
    pub fn load_glb(&self, relative_path: &str) -> Result<Vec<u8>, String> {
        let full_path = format!("{}/{}", self.ccf_base_path, relative_path);
        let path = Path::new(&full_path);

        if !path.exists() {
            return Err(format!("GLB file not found: {}", full_path));
        }

        let mut file = File::open(path).map_err(|e| format!("Failed to open GLB file: {}", e))?;

        let metadata =
            std::fs::metadata(path).map_err(|e| format!("Failed to get file metadata: {}", e))?;

        let mut buffer = Vec::with_capacity(metadata.len() as usize);
        file.read_to_end(&mut buffer)
            .map_err(|e| format!("Failed to read GLB file: {}", e))?;

        Ok(buffer)
    }

    /// Create GLB view from loaded data
    ///
    /// Zero-Heap Consideration: Returns view with byte references
    pub fn create_view<'a>(
        &self,
        glb_data: &'a [u8],
        asset_name: String,
        version: String,
    ) -> GLBView<'a> {
        let metadata = GLBMetadata {
            asset_index: 0, // Will be set by BinaryNodeRegistry
            asset_name,
            file_path: String::new(), // Set by caller
            file_size: glb_data.len() as u64,
            version,
        };

        GLBView::new(glb_data, metadata)
    }

    /// Get CCF VH_Male v1.4 asset list
    pub fn get_vh_male_v14_assets(&self) -> Vec<GLBMetadata> {
        vec![
            GLBMetadata {
                asset_index: 0,
                asset_name: "blood-vasculature".to_string(),
                file_path: "VH_Male/v1.4/3d-vh-m-blood-vasculature.glb".to_string(),
                file_size: 18_047_792, // ~18MB
                version: "v1.4".to_string(),
            },
            GLBMetadata {
                asset_index: 0,
                asset_name: "eye-left".to_string(),
                file_path: "VH_Male/v1.4/3d-vh-m-eye-l.glb".to_string(),
                file_size: 27_462_180, // ~27MB
                version: "v1.4".to_string(),
            },
            GLBMetadata {
                asset_index: 0,
                asset_name: "eye-right".to_string(),
                file_path: "VH_Male/v1.4/3d-vh-m-eye-r.glb".to_string(),
                file_size: 20_697_112, // ~20MB
                version: "v1.4".to_string(),
            },
            GLBMetadata {
                asset_index: 0,
                asset_name: "lung".to_string(),
                file_path: "VH_Male/v1.4/3d-vh-m-lung.glb".to_string(),
                file_size: 10_941_068, // ~10MB
                version: "v1.4".to_string(),
            },
            GLBMetadata {
                asset_index: 0,
                asset_name: "larynx".to_string(),
                file_path: "VH_Male/v1.4/3d-vh-m-larynx.glb".to_string(),
                file_size: 335_444, // ~335KB
                version: "v1.4".to_string(),
            },
        ]
    }
}

impl Default for GLBIngestionManager {
    fn default() -> Self {
        Self::new("C:\\Projects\\qualiaDB\\local\\ccf-3d-reference-object-library-main".to_string())
    }
}

/// Semantic identifier mapping for ontologies (FMA, SNOMED-CT)
///
/// Zero-Heap Consideration: Uses BinaryNodeRegistry to map string IDs to u64 indices
/// One-time heap allocation during parsing, zero-heap runtime access
#[derive(Debug, Clone)]
pub struct SemanticMapping {
    /// FMA (Foundational Model of Anatomy) identifier
    pub fma_id: Option<u64>,
    /// SNOMED-CT identifier
    pub snomed_id: Option<u64>,
    /// Custom ontology identifier
    pub custom_id: Option<u64>,
}

/// Semantic extractor for GLB JSON chunks
///
/// Extracts ontology identifiers from GLB metadata and maps to binary indices
pub struct SemanticExtractor;

impl SemanticExtractor {
    /// Extract semantic identifiers from GLB JSON chunk
    ///
    /// Zero-Heap Consideration: One-time heap allocation during parsing
    /// Returns binary indices for zero-heap runtime access
    pub fn extract_semantic_ids(
        json_chunk: &[u8],
        binary_registry: &crate::commands::binary_registry::BinaryNodeRegistry,
    ) -> Result<SemanticMapping, String> {
        // Parse JSON chunk (one-time heap allocation)
        let json_str = std::str::from_utf8(json_chunk)
            .map_err(|e| format!("Invalid UTF-8 in JSON chunk: {}", e))?;

        let json: serde_json::Value =
            serde_json::from_str(json_str).map_err(|e| format!("Failed to parse JSON: {}", e))?;

        // Extract semantic identifiers from GLTF asset metadata
        let fma_id = Self::extract_fma_id(&json, binary_registry)?;
        let snomed_id = Self::extract_snomed_id(&json, binary_registry)?;
        let custom_id = Self::extract_custom_id(&json, binary_registry)?;

        Ok(SemanticMapping {
            fma_id,
            snomed_id,
            custom_id,
        })
    }

    /// Extract FMA identifier from GLTF metadata
    fn extract_fma_id(
        json: &serde_json::Value,
        binary_registry: &crate::commands::binary_registry::BinaryNodeRegistry,
    ) -> Result<Option<u64>, String> {
        // Look for FMA ID in asset metadata or custom properties
        if let Some(asset) = json.get("asset") {
            if let Some(extras) = asset.get("extras") {
                if let Some(fma) = extras.get("fma_id") {
                    if let Some(fma_str) = fma.as_str() {
                        let index = binary_registry.register(fma_str);
                        return Ok(Some(index));
                    }
                }
            }
        }

        // Also check in node custom properties
        if let Some(nodes) = json.get("nodes") {
            if let Some(nodes_array) = nodes.as_array() {
                for node in nodes_array {
                    if let Some(extras) = node.get("extras") {
                        if let Some(fma) = extras.get("fma_id") {
                            if let Some(fma_str) = fma.as_str() {
                                let index = binary_registry.register(fma_str);
                                return Ok(Some(index));
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Extract SNOMED-CT identifier from GLTF metadata
    fn extract_snomed_id(
        json: &serde_json::Value,
        binary_registry: &crate::commands::binary_registry::BinaryNodeRegistry,
    ) -> Result<Option<u64>, String> {
        // Look for SNOMED ID in asset metadata or custom properties
        if let Some(asset) = json.get("asset") {
            if let Some(extras) = asset.get("extras") {
                if let Some(snomed) = extras.get("snomed_id") {
                    if let Some(snomed_str) = snomed.as_str() {
                        let index = binary_registry.register(snomed_str);
                        return Ok(Some(index));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Extract custom ontology identifier from GLTF metadata
    fn extract_custom_id(
        json: &serde_json::Value,
        binary_registry: &crate::commands::binary_registry::BinaryNodeRegistry,
    ) -> Result<Option<u64>, String> {
        // Look for custom ID in asset metadata
        if let Some(asset) = json.get("asset") {
            if let Some(extras) = asset.get("extras") {
                if let Some(custom) = extras.get("custom_id") {
                    if let Some(custom_str) = custom.as_str() {
                        let index = binary_registry.register(custom_str);
                        return Ok(Some(index));
                    }
                }
            }
        }

        Ok(None)
    }
}

/// 10D tensor mapping for GLB assets
///
/// Maps GLB geometry and semantics to 10D tensor structure [q, v, w, x, y, z, t, α, μ, σ]
#[derive(Debug, Clone)]
pub struct Tensor10DMapping {
    /// Spatial coordinates from binary chunk [x, y, z]
    pub spatial: [f64; 3],
    /// Topological class from semantic mapping [v]
    pub topological_class: u64,
    /// Manifold index from semantic mapping [w]
    pub manifold_index: u64,
    /// Spectral signature from semantic mapping [σ]
    pub spectral_signature: u64,
    /// Epistemic state [q] (default: Collapsed)
    pub epistemic_state: u8, // 0 = Collapsed, 1 = Pending, 2 = Sandbox
    /// Temporal slice [t] (default: current)
    pub temporal_slice: f64,
    /// Amplitude [α] (default: 1.0)
    pub amplitude: f64,
    /// Modulation [μ] (default: 0.0)
    pub modulation: f64,
}

impl Tensor10DMapping {
    /// Create tensor mapping from GLB view and semantic mapping
    ///
    /// Zero-Heap Consideration: Stack-allocated struct, no heap allocation
    pub fn from_glb_view(
        glb_view: &GLBView,
        semantic_mapping: &SemanticMapping,
        vertex_index: usize,
    ) -> Result<Self, String> {
        // Extract spatial coordinates from binary chunk
        let spatial = Self::extract_spatial_coordinates(glb_view, vertex_index)?;

        // Map semantic IDs to topological dimensions
        let topological_class = semantic_mapping.fma_id.unwrap_or(0);
        let manifold_index = semantic_mapping.snomed_id.unwrap_or(0);
        let spectral_signature = semantic_mapping.custom_id.unwrap_or(0);

        Ok(Self {
            spatial,
            topological_class,
            manifold_index,
            spectral_signature,
            epistemic_state: 0,  // Default: Collapsed
            temporal_slice: 0.0, // Default: current
            amplitude: 1.0,
            modulation: 0.0,
        })
    }

    /// Extract spatial coordinates [x, y, z] from GLB binary chunk
    ///
    /// Zero-Heap Consideration: Stack-allocated array operations
    fn extract_spatial_coordinates(
        glb_view: &GLBView,
        vertex_index: usize,
    ) -> Result<[f64; 3], String> {
        let binary_chunk = glb_view.binary_chunk().ok_or("No binary chunk found")?;

        // GLB binary chunk contains vertex data as f32 arrays
        // Each vertex is typically 3 floats (x, y, z) = 12 bytes
        let vertex_offset = vertex_index * 12;

        if binary_chunk.len() < vertex_offset + 12 {
            return Err("Vertex index out of bounds".to_string());
        }

        let x_bytes = &binary_chunk[vertex_offset..vertex_offset + 4];
        let y_bytes = &binary_chunk[vertex_offset + 4..vertex_offset + 8];
        let z_bytes = &binary_chunk[vertex_offset + 8..vertex_offset + 12];

        let x = f32::from_le_bytes([x_bytes[0], x_bytes[1], x_bytes[2], x_bytes[3]]) as f64;
        let y = f32::from_le_bytes([y_bytes[0], y_bytes[1], y_bytes[2], y_bytes[3]]) as f64;
        let z = f32::from_le_bytes([z_bytes[0], z_bytes[1], z_bytes[2], z_bytes[3]]) as f64;

        Ok([x, y, z])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glb_view_creation() {
        let glb_data = vec![
            0x67, 0x6C, 0x54, 0x46, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]; // Minimal GLB header
        let metadata = GLBMetadata {
            asset_index: 0,
            asset_name: "test".to_string(),
            file_path: "test.glb".to_string(),
            file_size: 12,
            version: "v1.0".to_string(),
        };

        let view = GLBView::new(&glb_data, metadata);
        assert_eq!(view.is_valid_glb(), true);
    }

    #[test]
    fn test_glb_manager_default() {
        let manager = GLBIngestionManager::default();
        assert!(manager
            .ccf_base_path
            .contains("ccf-3d-reference-object-library-main"));
    }

    #[test]
    fn test_vh_male_asset_list() {
        let manager = GLBIngestionManager::default();
        let assets = manager.get_vh_male_v14_assets();
        assert_eq!(assets.len(), 5);
        assert_eq!(assets[0].asset_name, "blood-vasculature");
        assert_eq!(assets[0].file_size, 18_047_792);
    }
}
