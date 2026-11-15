//! Satellite Imagery & Visual Map Drape Streamer (WMS / WMTS Display Path).
//!
//! Manages tile-matrix addressing ($z, x, y$) and caches $256 \times 256$ RGBA8 textures
//! into slices of a GPU 2D Texture Array without scraping physical truth from pixels.

pub const TILE_DIMENSION_PX: usize = 256;
pub const TILE_BYTES: usize = TILE_DIMENSION_PX * TILE_DIMENSION_PX * 4; // 256 KB per slice
pub const MAX_CACHE_LAYERS: usize = 32; // 32-layer 2D Texture Array (8 MB VRAM footprint)

/// Tile coordinate in standard Slippy Map / WMTS tile-matrix convention ($z, x, y$).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileCoordinate {
    pub zoom: u8,
    pub x: u32,
    pub y: u32,
}

/// A cached texture slice entry in the GPU 2D Texture Array.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextureArraySlot {
    pub coord: TileCoordinate,
    pub layer_index: u32,
    pub last_accessed_frame: u64,
    pub is_valid: bool,
}

/// WMS / WMTS Tile Array Cache Manager.
pub struct WmsTileStreamer {
    pub slots: [TextureArraySlot; MAX_CACHE_LAYERS],
    pub current_frame: u64,
}

impl Default for WmsTileStreamer {
    fn default() -> Self {
        let dummy = TextureArraySlot {
            coord: TileCoordinate { zoom: 0, x: 0, y: 0 },
            layer_index: 0,
            last_accessed_frame: 0,
            is_valid: false,
        };
        let mut slots = [dummy; MAX_CACHE_LAYERS];
        for i in 0..MAX_CACHE_LAYERS {
            slots[i].layer_index = i as u32;
        }
        Self {
            slots,
            current_frame: 0,
        }
    }
}

impl WmsTileStreamer {
    /// Advances frame counter for LRU eviction.
    #[inline]
    pub fn advance_frame(&mut self) {
        self.current_frame = self.current_frame.wrapping_add(1);
    }

    /// Looks up a tile in the cache; returns layer slice index in the GPU 2D Texture Array.
    pub fn lookup_tile(&mut self, coord: &TileCoordinate) -> Option<u32> {
        for slot in &mut self.slots {
            if slot.is_valid && slot.coord == *coord {
                slot.last_accessed_frame = self.current_frame;
                return Some(slot.layer_index);
            }
        }
        None
    }

    /// Allocates or evicts a slot for a newly fetched tile coordinate.
    pub fn allocate_slot(&mut self, coord: &TileCoordinate) -> u32 {
        // 1. Check if already present
        if let Some(idx) = self.lookup_tile(coord) {
            return idx;
        }

        // 2. Find unused slot
        for slot in &mut self.slots {
            if !slot.is_valid {
                slot.coord = *coord;
                slot.last_accessed_frame = self.current_frame;
                slot.is_valid = true;
                return slot.layer_index;
            }
        }

        // 3. Evict least-recently-used slot
        let mut lru_idx = 0;
        let mut oldest_frame = u64::MAX;
        for (i, slot) in self.slots.iter().enumerate() {
            if slot.last_accessed_frame < oldest_frame {
                oldest_frame = slot.last_accessed_frame;
                lru_idx = i;
            }
        }

        self.slots[lru_idx].coord = *coord;
        self.slots[lru_idx].last_accessed_frame = self.current_frame;
        self.slots[lru_idx].is_valid = true;
        self.slots[lru_idx].layer_index
    }
}
