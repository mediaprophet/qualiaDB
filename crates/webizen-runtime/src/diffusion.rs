#[derive(Debug, Clone, Copy)]
pub struct DiffusionConfig {
    pub width: u32,
    pub height: u32,
    pub diffusion_rate: f32,
}

impl DiffusionConfig {
    pub fn cell_count(&self) -> usize {
        self.width as usize * self.height as usize
    }

    pub fn raw_byte_len(&self) -> u64 {
        (self.cell_count() * std::mem::size_of::<f32>()) as u64
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DiffusionField {
    pub dimensions: (u32, u32),
    pub epoch: u64,
}

impl DiffusionField {
    pub fn new(dimensions: (u32, u32)) -> Self {
        Self {
            dimensions,
            epoch: 0,
        }
    }
}
