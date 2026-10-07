//! Consent-gated Terrarium DEM tile fetching, decoding, mosaicking, and terrain compilation.

use std::collections::HashMap;
use std::io::Cursor;

use crate::domains::geospatial::adapters::AdapterHttpRequest;
use crate::domains::geospatial::terrain_pipeline::{
    compile_and_encode_10d_tile_with_media_type, compile_terrain_tile, GeodeticTerrainTile,
};
use crate::net::disclosure::NetworkDisclosureRegistry;
use crate::NQuin;

pub const DEFAULT_TERRARIUM_ENDPOINT: &str =
    "https://s3.amazonaws.com/elevation-tiles-prod/terrarium/{z}/{x}/{y}.png";
pub const TERRAIN_ATTRIBUTION_URL: &str =
    "https://github.com/tilezen/joerd/blob/master/docs/attribution.md";
pub const TERRAIN_ATTRIBUTION: &str = "Terrain data: Mapzen Terrain Tiles. ArcticDEM terrain data DEM(s) were created from DigitalGlobe, Inc., imagery and funded under National Science Foundation awards 1043681, 1559691, and 1542736; Australia terrain data © Commonwealth of Australia (Geoscience Australia) 2017; Austria terrain data © offene Daten Österreichs – Digitales Geländemodell (DGM) Österreich; Canada terrain data contains information licensed under the Open Government Licence – Canada; Europe terrain data produced using Copernicus data and information funded by the European Union - EU-DEM layers; Global ETOPO1 terrain data U.S. National Oceanic and Atmospheric Administration; Mexico terrain data source: INEGI, Continental relief, 2016; New Zealand terrain data Copyright 2011 Crown copyright (c) Land Information New Zealand and the New Zealand Government (All rights reserved); Norway terrain data © Kartverket; United Kingdom terrain data © Environment Agency copyright and/or database right 2015. All rights reserved; United States 3DEP (formerly NED) and global GMTED2010 and SRTM terrain data courtesy of the U.S. Geological Survey. See the Tilezen terrain attribution page.";
const MAX_ZOOM: u8 = 15;
const MAX_TILES: usize = 16;
const MAX_HEIGHTFIELD_SAMPLES: usize = 1_048_576;
const MAX_SOURCE_BUNDLE_BYTES: usize = 16 * 1024 * 1024;
const TILE_SIZE: usize = 256;
const EARTH_CIRCUMFERENCE_M: f64 = 40_075_016.685_578_49;
const MERCATOR_MAX_LAT: f64 = 85.051_128_779_806_6;

/// Region height samples decoded from Terrarium PNG tiles.
#[derive(Debug, Clone)]
pub struct DemHeightfield {
    pub values_m: Vec<f32>,
    pub width: usize,
    pub height: usize,
    /// Requested WGS84 extent `(west, south, east, north)`.
    pub bbox: (f64, f64, f64, f64),
    pub zoom: u8,
    pub source_template: String,
    pub cell_size_m: f64,
    pub centre_lat: f64,
    pub centre_lon: f64,
    pub centre_alt_m: f64,
    /// Original source PNGs, including tile coordinates, for `.10d` provenance.
    pub source_bundle: Vec<u8>,
}

/// Fetched DEM and its compiled in-memory and `.10d` terrain representations.
#[derive(Debug, Clone)]
pub struct DemTerrainAsset {
    pub heightfield: DemHeightfield,
    pub terrain: GeodeticTerrainTile,
    pub encoded_10d: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct TileKey {
    x: u32,
    y: u32,
}

/// Public, global elevation tiles in Terrarium's 24-bit RGB elevation encoding.
/// `endpoint` is an XYZ template (`{z}`, `{x}`, `{y}`); a bare base URL passed to
/// `new` gets `/terrarium/{z}/{x}/{y}.png` appended for compatibility.
pub struct DemAdapter {
    pub id: &'static str,
    pub endpoint: String,
    pub zoom: u8,
}

impl DemAdapter {
    pub fn new(id: &'static str, endpoint: &str) -> Self {
        let endpoint =
            if endpoint.contains("{z}") && endpoint.contains("{x}") && endpoint.contains("{y}") {
                endpoint.to_string()
            } else {
                let base = endpoint.trim_end_matches('/');
                let base = base.strip_suffix("/terrarium").unwrap_or(base);
                format!("{}/terrarium/{{z}}/{{x}}/{{y}}.png", base)
            };
        Self {
            id,
            endpoint,
            zoom: 12,
        }
    }

    pub fn terrarium(id: &'static str) -> Self {
        Self::new(id, DEFAULT_TERRARIUM_ENDPOINT)
    }

    pub fn with_zoom(mut self, zoom: u8) -> Result<Self, String> {
        if zoom > MAX_ZOOM {
            return Err(format!("Terrarium zoom must be in 0..={MAX_ZOOM}"));
        }
        self.zoom = zoom;
        Ok(self)
    }

    pub fn adapter_id(&self) -> &'static str {
        self.id
    }

    pub fn estimate_tile_count(&self, bbox: (f64, f64, f64, f64)) -> u32 {
        estimate_tile_count(bbox, self.zoom).unwrap_or(0)
    }

    /// Build every tile request needed to cover the requested geographic extent.
    /// Consent is checked against each expanded URL before requests are returned.
    pub fn build_fetch_requests(
        &self,
        bbox: (f64, f64, f64, f64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<Vec<AdapterHttpRequest>, String> {
        validate_bbox(bbox)?;
        if self.zoom > MAX_ZOOM {
            return Err(format!("Terrarium zoom must be in 0..={MAX_ZOOM}"));
        }
        let (west, south, east, north) = bbox;
        if south < -MERCATOR_MAX_LAT || north > MERCATOR_MAX_LAT {
            return Err(format!(
                "Terrarium tiles cover latitudes between -{MERCATOR_MAX_LAT} and {MERCATOR_MAX_LAT}"
            ));
        }
        let min = lon_lat_to_global_pixel(west, north, self.zoom);
        let max = lon_lat_to_global_pixel(east, south, self.zoom);
        let first_x = (min.0.floor() as usize / TILE_SIZE) as u32;
        let first_y = (min.1.floor() as usize / TILE_SIZE) as u32;
        let last_x = (max.0.floor() as usize / TILE_SIZE) as u32;
        let last_y = (max.1.floor() as usize / TILE_SIZE) as u32;
        let n = 1u32 << self.zoom;
        let mut requests = Vec::new();
        for y in first_y.min(n - 1)..=last_y.min(n - 1) {
            for x in first_x.min(n - 1)..=last_x.min(n - 1) {
                let url = self
                    .endpoint
                    .replace("{z}", &self.zoom.to_string())
                    .replace("{x}", &x.to_string())
                    .replace("{y}", &y.to_string());
                if !registry.check_egress_consent(self.adapter_id(), &url) {
                    return Err(format!(
                        "Consent denied or unregistered for endpoint {url} by adapter {}",
                        self.adapter_id()
                    ));
                }
                requests.push(AdapterHttpRequest::get(url, "Terrarium elevation"));
                if requests.len() > MAX_TILES {
                    return Err(format!(
                        "requested area spans more than {MAX_TILES} terrain tiles at zoom {}",
                        self.zoom
                    ));
                }
            }
        }
        if requests.is_empty() {
            return Err("requested area does not intersect a terrain tile".into());
        }
        Ok(requests)
    }

    /// Compatibility with the single-request adapter trait. Use
    /// `build_fetch_requests` for the complete extent; a bounding box can cross
    /// tile boundaries and therefore require several requests.
    pub fn build_fetch_request(
        &self,
        bbox: (f64, f64, f64, f64),
        _time_range: (u64, u64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<AdapterHttpRequest, String> {
        self.build_fetch_requests(bbox, registry)?
            .into_iter()
            .next()
            .ok_or_else(|| "requested area does not intersect a terrain tile".to_string())
    }

    /// Fetch, decode, mosaic, georeference, and compile a DEM region on native targets.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn fetch_region_dem(
        &self,
        bbox: (f64, f64, f64, f64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<DemTerrainAsset, String> {
        let requests = self.build_fetch_requests(bbox, registry)?;
        let mut tiles = HashMap::with_capacity(requests.len());
        for request in &requests {
            let bytes = super::execute_http_request_bytes(request)?;
            let key = tile_key_from_url(&request.url)?;
            tiles.insert(key, decode_terrarium_png(&bytes)?);
        }
        self.compile_region(bbox, tiles)
    }

    /// Browser-safe asynchronous fetch, decode, mosaic, and terrain compilation.
    #[cfg(target_arch = "wasm32")]
    pub async fn fetch_region_dem_async(
        &self,
        bbox: (f64, f64, f64, f64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<DemTerrainAsset, String> {
        let requests = self.build_fetch_requests(bbox, registry)?;
        let mut tiles = HashMap::with_capacity(requests.len());
        for request in &requests {
            let bytes = super::execute_http_request_bytes_async(request).await?;
            let key = tile_key_from_url(&request.url)?;
            tiles.insert(key, decode_terrarium_png(&bytes)?);
        }
        self.compile_region(bbox, tiles)
    }

    fn compile_region(
        &self,
        bbox: (f64, f64, f64, f64),
        tiles: HashMap<TileKey, DecodedTerrainTile>,
    ) -> Result<DemTerrainAsset, String> {
        let heightfield = assemble_heightfield(bbox, self.zoom, &self.endpoint, tiles)?;
        let terrain = compile_terrain_tile(
            &heightfield.values_m,
            heightfield.width,
            heightfield.height,
            heightfield.cell_size_m,
            heightfield.centre_lat,
            heightfield.centre_lon,
            heightfield.centre_alt_m,
            heightfield.centre_lat,
            heightfield.centre_lon,
            heightfield.centre_alt_m,
        );
        let metadata = serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Dataset",
            "name": "Terrarium terrain tile mosaic",
            "provider": "Mapzen Terrain Tiles on AWS",
            "url": self.endpoint,
            "license": TERRAIN_ATTRIBUTION_URL,
            "creditText": TERRAIN_ATTRIBUTION,
            "spatialCoverage": {
                "southwest": {"latitude": heightfield.bbox.1, "longitude": heightfield.bbox.0},
                "northeast": {"latitude": heightfield.bbox.3, "longitude": heightfield.bbox.2}
            },
            "additionalProperty": [
                {"name": "encoding", "value": "Terrarium RGB: (R*256 + G + B/256) - 32768 metres"},
                {"name": "zoom", "value": heightfield.zoom},
                {"name": "width", "value": heightfield.width},
                {"name": "height", "value": heightfield.height},
                {"name": "cellSizeMetres", "value": heightfield.cell_size_m},
                {"name": "centreLatitude", "value": heightfield.centre_lat},
                {"name": "centreLongitude", "value": heightfield.centre_lon},
                {"name": "centreElevationMetres", "value": heightfield.centre_alt_m}
            ]
        });
        let mut metadata_cbor = Vec::new();
        ciborium::ser::into_writer(&metadata, &mut metadata_cbor)
            .map_err(|error| format!("failed to encode DEM provenance metadata: {error}"))?;
        let encoded_10d = compile_and_encode_10d_tile_with_media_type(
            &heightfield.values_m,
            heightfield.width,
            heightfield.height,
            heightfield.cell_size_m,
            heightfield.source_bundle.clone(),
            "application/vnd.tilezen.terrarium-tile-bundle".into(),
            TERRAIN_ATTRIBUTION_URL.into(),
            metadata_cbor,
        )?;
        Ok(DemTerrainAsset {
            heightfield,
            terrain,
            encoded_10d,
        })
    }
}

#[derive(Debug, Clone)]
struct DecodedTerrainTile {
    values_m: Vec<f32>,
    width: usize,
    height: usize,
    source_png: Vec<u8>,
}

fn decode_terrarium_png(bytes: &[u8]) -> Result<DecodedTerrainTile, String> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder
        .read_info()
        .map_err(|error| format!("invalid Terrarium PNG header: {error}"))?;
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buffer)
        .map_err(|error| format!("invalid Terrarium PNG frame: {error}"))?;
    if info.width as usize != TILE_SIZE || info.height as usize != TILE_SIZE {
        return Err(format!(
            "Terrarium tile must be {TILE_SIZE}x{TILE_SIZE}, received {}x{}",
            info.width, info.height
        ));
    }
    let channels = match info.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        other => {
            return Err(format!(
                "Terrarium PNG must use RGB or RGBA pixels, received {other:?}"
            ));
        }
    };
    let data = &buffer[..info.buffer_size()];
    let mut values_m = Vec::with_capacity(TILE_SIZE * TILE_SIZE);
    for pixel in data.chunks_exact(channels) {
        if channels == 4 && pixel[3] == 0 {
            return Err("Terrarium PNG contains transparent elevation samples".into());
        }
        let metres =
            (f64::from(pixel[0]) * 256.0 + f64::from(pixel[1]) + f64::from(pixel[2]) / 256.0)
                - 32_768.0;
        values_m.push(metres as f32);
    }
    Ok(DecodedTerrainTile {
        values_m,
        width: TILE_SIZE,
        height: TILE_SIZE,
        source_png: bytes.to_vec(),
    })
}

fn assemble_heightfield(
    bbox: (f64, f64, f64, f64),
    zoom: u8,
    source_template: &str,
    tiles: HashMap<TileKey, DecodedTerrainTile>,
) -> Result<DemHeightfield, String> {
    validate_bbox(bbox)?;
    let (west, south, east, north) = bbox;
    if south < -MERCATOR_MAX_LAT || north > MERCATOR_MAX_LAT {
        return Err("requested DEM area exceeds the Web Mercator latitude limit".into());
    }
    let start = lon_lat_to_global_pixel(west, north, zoom);
    let end = lon_lat_to_global_pixel(east, south, zoom);
    let span_x = end.0 - start.0;
    let span_y = end.1 - start.1;
    let width = span_x.ceil() as usize + 1;
    let height = span_y.ceil() as usize + 1;
    if width < 2 || height < 2 {
        return Err("DEM extent must cover at least two samples in each dimension".into());
    }
    if width
        .checked_mul(height)
        .is_none_or(|samples| samples > MAX_HEIGHTFIELD_SAMPLES)
    {
        return Err(format!(
            "requested DEM heightfield exceeds the {MAX_HEIGHTFIELD_SAMPLES} sample limit"
        ));
    }

    let mut values_m = Vec::with_capacity(width * height);
    // Heightfield rows increase northward so the terrain mesh's +Y axis points north.
    for row in 0..height {
        let global_y = end.1 - span_y * row as f64 / (height - 1) as f64;
        let pixel_y = global_pixel_index(global_y);
        let tile_y = (pixel_y / TILE_SIZE) as u32;
        let local_y = pixel_y % TILE_SIZE;
        for column in 0..width {
            let global_x = start.0 + span_x * column as f64 / (width - 1) as f64;
            let pixel_x = global_pixel_index(global_x);
            let tile_x = (pixel_x / TILE_SIZE) as u32;
            let local_x = pixel_x % TILE_SIZE;
            let key = TileKey {
                x: tile_x,
                y: tile_y,
            };
            let tile = tiles
                .get(&key)
                .ok_or_else(|| format!("missing Terrarium tile {},{}", key.x, key.y))?;
            if tile.width != TILE_SIZE || tile.height != TILE_SIZE {
                return Err("decoded Terrarium tile dimensions are inconsistent".into());
            }
            values_m.push(tile.values_m[local_y * TILE_SIZE + local_x]);
        }
    }

    let centre_lat = (north + south) * 0.5;
    let centre_lon = (west + east) * 0.5;
    let centre_alt_m = values_m[(height / 2) * width + width / 2] as f64;
    let cell_size_m = EARTH_CIRCUMFERENCE_M * centre_lat.to_radians().cos()
        / (TILE_SIZE as f64 * (1u32 << zoom) as f64);
    let source_bundle = encode_source_bundle(zoom, &tiles)?;
    if source_bundle.len() > MAX_SOURCE_BUNDLE_BYTES {
        return Err(format!(
            "DEM source provenance exceeds the {} MiB `.10d` sidecar limit",
            MAX_SOURCE_BUNDLE_BYTES / (1024 * 1024)
        ));
    }
    Ok(DemHeightfield {
        values_m,
        width,
        height,
        bbox,
        zoom,
        source_template: source_template.to_string(),
        cell_size_m,
        centre_lat,
        centre_lon,
        centre_alt_m,
        source_bundle,
    })
}

fn encode_source_bundle(
    zoom: u8,
    tiles: &HashMap<TileKey, DecodedTerrainTile>,
) -> Result<Vec<u8>, String> {
    let mut bundle = b"QDEMTR1\0".to_vec();
    let mut keys: Vec<_> = tiles.keys().copied().collect();
    keys.sort_unstable();
    for key in keys {
        let tile = tiles
            .get(&key)
            .ok_or_else(|| "internal error while encoding DEM provenance".to_string())?;
        bundle.push(zoom);
        bundle.extend_from_slice(&key.x.to_be_bytes());
        bundle.extend_from_slice(&key.y.to_be_bytes());
        let png_len = u32::try_from(tile.source_png.len())
            .map_err(|_| "Terrarium source PNG exceeds the bundle size limit".to_string())?;
        bundle.extend_from_slice(&png_len.to_be_bytes());
        bundle.extend_from_slice(&tile.source_png);
    }
    Ok(bundle)
}

impl DemTerrainAsset {
    /// Create compact graph metadata facts; raster elevations remain in the mesh asset.
    pub fn to_quins(&self) -> Vec<NQuin> {
        let subject = crate::lexicon::generate_60bit_token(
            format!(
                "urn:qualiadb:dem:{:.7}:{:.7}:{:.7}:{:.7}:z{}",
                self.heightfield.bbox.0,
                self.heightfield.bbox.1,
                self.heightfield.bbox.2,
                self.heightfield.bbox.3,
                self.heightfield.zoom
            )
            .as_bytes(),
        );
        let predicates = vec![
            (
                "http://purl.org/dc/terms/source".to_string(),
                self.heightfield.source_template.clone(),
            ),
            (
                "http://purl.org/dc/terms/license".to_string(),
                TERRAIN_ATTRIBUTION_URL.to_string(),
            ),
            (
                "http://purl.org/dc/terms/description".to_string(),
                TERRAIN_ATTRIBUTION.to_string(),
            ),
            (
                "http://www.opengis.net/ont/geosparql#asWKT".to_string(),
                format!(
                    "POLYGON(({} {}, {} {}, {} {}, {} {}, {} {}))",
                    self.heightfield.bbox.0,
                    self.heightfield.bbox.1,
                    self.heightfield.bbox.2,
                    self.heightfield.bbox.1,
                    self.heightfield.bbox.2,
                    self.heightfield.bbox.3,
                    self.heightfield.bbox.0,
                    self.heightfield.bbox.3,
                    self.heightfield.bbox.0,
                    self.heightfield.bbox.1
                ),
            ),
        ];
        predicates
            .into_iter()
            .map(|(predicate, object)| {
                let predicate = crate::lexicon::generate_60bit_token(predicate.as_bytes());
                let object = crate::lexicon::generate_60bit_token(object.as_bytes());
                NQuin {
                    subject,
                    predicate,
                    object,
                    context: 0,
                    metadata: 0,
                    parity: subject ^ predicate ^ object,
                }
            })
            .collect()
    }
}

fn tile_key_from_url(url: &str) -> Result<TileKey, String> {
    let path = url.split('?').next().unwrap_or(url);
    let mut parts = path.rsplit('/');
    let y = parts
        .next()
        .and_then(|part| part.strip_suffix(".png"))
        .and_then(|part| part.parse().ok())
        .ok_or_else(|| format!("cannot read tile y coordinate from {url}"))?;
    let x = parts
        .next()
        .and_then(|part| part.parse().ok())
        .ok_or_else(|| format!("cannot read tile x coordinate from {url}"))?;
    Ok(TileKey { x, y })
}

fn lon_lat_to_global_pixel(lon: f64, lat: f64, zoom: u8) -> (f64, f64) {
    let world_size = TILE_SIZE as f64 * (1u32 << zoom) as f64;
    let x = ((lon + 180.0) / 360.0 * world_size).clamp(0.0, world_size - f64::EPSILON);
    let sin_lat = lat.to_radians().sin();
    let y = ((0.5
        - (0.25 * (1.0 + sin_lat) / (1.0 - sin_lat)).ln() / (4.0 * std::f64::consts::PI))
        * world_size)
        .clamp(0.0, world_size - f64::EPSILON);
    (x, y)
}

fn global_pixel_index(pixel: f64) -> usize {
    pixel.floor().max(0.0) as usize
}

fn validate_bbox(bbox: (f64, f64, f64, f64)) -> Result<(), String> {
    let (west, south, east, north) = bbox;
    if ![west, south, east, north]
        .iter()
        .all(|value| value.is_finite())
        || west < -180.0
        || east > 180.0
        || south < -90.0
        || north > 90.0
        || west >= east
        || south >= north
    {
        return Err("invalid WGS84 bounding box; expected west < east and south < north".into());
    }
    Ok(())
}

fn estimate_tile_count(bbox: (f64, f64, f64, f64), zoom: u8) -> Result<u32, String> {
    validate_bbox(bbox)?;
    let (west, south, east, north) = bbox;
    if south < -MERCATOR_MAX_LAT || north > MERCATOR_MAX_LAT {
        return Err("requested area exceeds the Web Mercator latitude limit".into());
    }
    let min = lon_lat_to_global_pixel(west, north, zoom);
    let max = lon_lat_to_global_pixel(east, south, zoom);
    let x_count =
        ((max.0.floor() as usize / TILE_SIZE) - (min.0.floor() as usize / TILE_SIZE) + 1) as u32;
    let y_count =
        ((max.1.floor() as usize / TILE_SIZE) - (min.1.floor() as usize / TILE_SIZE) + 1) as u32;
    x_count
        .checked_mul(y_count)
        .ok_or_else(|| "terrain tile count overflow".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::geospatial::adapters::DataAdapter;

    fn solid_png(rgb: [u8; 3]) -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, TILE_SIZE as u32, TILE_SIZE as u32);
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            let mut pixels = vec![0u8; TILE_SIZE * TILE_SIZE * 3];
            for pixel in pixels.chunks_exact_mut(3) {
                pixel.copy_from_slice(&rgb);
            }
            writer.write_image_data(&pixels).unwrap();
        }
        bytes
    }

    #[test]
    fn terrarium_rgb_decodes_metres_and_rejects_bad_tiles() {
        let tile = decode_terrarium_png(&solid_png([128, 0, 0])).unwrap();
        assert_eq!(tile.width, TILE_SIZE);
        assert_eq!(tile.height, TILE_SIZE);
        assert_eq!(tile.values_m[0], 0.0);
        assert!(decode_terrarium_png(b"not a png").is_err());
    }

    #[test]
    fn requests_are_consent_gated_and_cover_all_xyz_tiles() {
        let adapter = DemAdapter::terrarium("dem_adapter").with_zoom(12).unwrap();
        let bbox = (151.20, -33.88, 151.22, -33.86);
        let mut registry = NetworkDisclosureRegistry::new();
        assert!(adapter.build_fetch_requests(bbox, &registry).is_err());
        registry.register_egress(
            adapter.adapter_id(),
            "https://s3.amazonaws.com/elevation-tiles-prod/terrarium/",
            "Fetch Terrarium elevation tiles",
            "User opens a real-world terrain region",
        );
        let requests = adapter.build_fetch_requests(bbox, &registry).unwrap();
        assert!(requests.len() > 1);
        assert!(requests.iter().all(|request| request.url.ends_with(".png")));
    }

    #[test]
    fn assembled_region_compiles_mesh_and_provenance_container() {
        let adapter = DemAdapter::terrarium("dem_adapter").with_zoom(12).unwrap();
        let bbox = (151.20, -33.88, 151.201, -33.879);
        let start = lon_lat_to_global_pixel(bbox.0, bbox.3, adapter.zoom);
        let end = lon_lat_to_global_pixel(bbox.2, bbox.1, adapter.zoom);
        let first_x = (start.0.floor() as usize / TILE_SIZE) as u32;
        let first_y = (start.1.floor() as usize / TILE_SIZE) as u32;
        let last_x = (end.0.floor() as usize / TILE_SIZE) as u32;
        let last_y = (end.1.floor() as usize / TILE_SIZE) as u32;
        let tile = decode_terrarium_png(&solid_png([128, 0, 0])).unwrap();
        let mut tiles = HashMap::new();
        for y in first_y..=last_y {
            for x in first_x..=last_x {
                tiles.insert(TileKey { x, y }, tile.clone());
            }
        }
        let asset = adapter.compile_region(bbox, tiles).unwrap();
        assert!(asset.heightfield.width > 1);
        assert_eq!(
            asset.terrain.mesh.vertices.len(),
            asset.heightfield.values_m.len()
        );
        assert!(!asset.encoded_10d.is_empty());
        assert_eq!(asset.heightfield.centre_alt_m, 0.0);
    }

    #[test]
    fn rejects_invalid_extent_and_excessive_heightfields() {
        assert!(validate_bbox((2.0, 1.0, 1.0, 2.0)).is_err());
        assert!(estimate_tile_count((-180.0, -80.0, 180.0, 80.0), MAX_ZOOM).is_ok());
        let adapter = DemAdapter::terrarium("dem_adapter")
            .with_zoom(MAX_ZOOM)
            .unwrap();
        assert!(adapter
            .build_fetch_requests(
                (-180.0, -80.0, 180.0, 80.0),
                &NetworkDisclosureRegistry::new()
            )
            .is_err());
    }

    #[test]
    fn generic_trait_describes_binary_fetches_and_estimates_tiles() {
        let adapter = DemAdapter::terrarium("dem_adapter");
        assert!(adapter.estimate_tile_count((2.0, 1.0, 1.0, 2.0)) == 0);
        assert!(adapter.needs_fetch_body());
        assert!(adapter
            .handle_fetch_body("not a binary png")
            .unwrap_err()
            .contains("binary PNG"));
    }
}
