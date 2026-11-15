//! Explicit full-cell dimensions and brick addressing; local records are XYZ
//! row-major inside each clipped brick, independently of directory curve order.
use super::*;
use crate::q42::m64_matter::brick3d::{decode_hilbert3d_u64, encode_hilbert3d_u64};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridIndexing {
    Linear,
    Morton { order: u32 },
    Hilbert { order: u32 },
}
impl GridIndexing {
    pub fn name(self) -> &'static str {
        match self {
            Self::Linear => "linear",
            Self::Morton { .. } => "morton3",
            Self::Hilbert { .. } => "hilbert3",
        }
    }
    pub fn order(self) -> u32 {
        match self {
            Self::Linear => 0,
            Self::Morton { order } | Self::Hilbert { order } => order,
        }
    }
    pub fn common_order(self) -> Option<u32> {
        match self {
            Self::Linear => Some(0),
            Self::Morton { .. } => Some(1),
            Self::Hilbert { .. } => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridSpec {
    pub dimensions: [u32; 3],
    pub brick_dimensions: [u32; 3],
    pub spacing_m: [f64; 3],
    pub indexing: GridIndexing,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridLayout {
    spec: GridSpec,
    brick_grid: [u32; 3],
    brick_count: u64,
}
impl GridSpec {
    pub fn validate(self) -> Result<GridLayout, Error> {
        let mut brick_grid = [0; 3];
        let mut cells = 1u64;
        let mut brick_count = 1u64;
        for axis in 0..3 {
            let n = self.dimensions[axis];
            let b = self.brick_dimensions[axis];
            let spacing = self.spacing_m[axis];
            if n == 0 || b == 0 || !spacing.is_finite() || spacing <= 0.0 {
                return Err(Error::Shape);
            }
            brick_grid[axis] = n / b + u32::from(n % b != 0);
            cells = cells.checked_mul(u64::from(n)).ok_or(Error::Overflow)?;
            brick_count = brick_count
                .checked_mul(u64::from(brick_grid[axis]))
                .ok_or(Error::Overflow)?;
        }
        if self.indexing != GridIndexing::Linear {
            let order = self.indexing.order();
            if order > 21 {
                return Err(Error::Shape);
            }
            let side = 1u32 << order;
            if brick_grid.iter().any(|&n| n > side) {
                return Err(Error::Shape);
            }
        }
        let _ = cells;
        Ok(GridLayout {
            spec: self,
            brick_grid,
            brick_count,
        })
    }
}
impl GridLayout {
    pub fn spec(self) -> GridSpec {
        self.spec
    }
    pub fn brick_grid_dimensions(self) -> [u32; 3] {
        self.brick_grid
    }
    pub fn brick_count(self) -> u64 {
        self.brick_count
    }
    pub fn encode_key(self, coordinate: [u32; 3]) -> Result<u64, Error> {
        if (0..3).any(|i| coordinate[i] >= self.brick_grid[i]) {
            return Err(Error::Index);
        }
        Ok(match self.spec.indexing {
            GridIndexing::Linear => {
                (u64::from(coordinate[2]) * u64::from(self.brick_grid[1])
                    + u64::from(coordinate[1]))
                    * u64::from(self.brick_grid[0])
                    + u64::from(coordinate[0])
            }
            GridIndexing::Morton { order } => {
                let mut key = 0;
                for bit in 0..order {
                    for axis in 0..3 {
                        key |= u64::from((coordinate[axis] >> bit) & 1) << (3 * bit + axis as u32);
                    }
                }
                key
            }
            GridIndexing::Hilbert { order } => {
                encode_hilbert3d_u64(coordinate[0], coordinate[1], coordinate[2], order)
            }
        })
    }
    pub fn decode_key(self, key: u64) -> Result<[u32; 3], Error> {
        let coordinate = match self.spec.indexing {
            GridIndexing::Linear => {
                if key >= self.brick_count {
                    return Err(Error::Index);
                }
                let x = key % u64::from(self.brick_grid[0]);
                let yz = key / u64::from(self.brick_grid[0]);
                [
                    x as u32,
                    (yz % u64::from(self.brick_grid[1])) as u32,
                    (yz / u64::from(self.brick_grid[1])) as u32,
                ]
            }
            GridIndexing::Morton { order } => {
                check_curve_key(key, order)?;
                let mut coordinate = [0; 3];
                for bit in 0..order {
                    for axis in 0..3 {
                        coordinate[axis] |= (((key >> (3 * bit + axis as u32)) & 1) as u32) << bit;
                    }
                }
                coordinate
            }
            GridIndexing::Hilbert { order } => {
                check_curve_key(key, order)?;
                let (x, y, z) = decode_hilbert3d_u64(key, order);
                [x, y, z]
            }
        };
        if (0..3).any(|i| coordinate[i] >= self.brick_grid[i])
            || self.encode_key(coordinate)? != key
        {
            return Err(Error::Index);
        }
        Ok(coordinate)
    }
    pub fn brick(self, key: u64) -> Result<BrickLayout, Error> {
        let coordinate = self.decode_key(key)?;
        let mut origin = [0; 3];
        let mut shape = [0; 3];
        let mut cells = 1u64;
        for axis in 0..3 {
            origin[axis] = coordinate[axis]
                .checked_mul(self.spec.brick_dimensions[axis])
                .ok_or(Error::Overflow)?;
            shape[axis] =
                self.spec.brick_dimensions[axis].min(self.spec.dimensions[axis] - origin[axis]);
            cells = cells
                .checked_mul(u64::from(shape[axis]))
                .ok_or(Error::Overflow)?;
        }
        Ok(BrickLayout {
            grid: self,
            origin,
            shape,
            cells,
        })
    }
}
fn check_curve_key(key: u64, order: u32) -> Result<(), Error> {
    if order > 21 || key >> (3 * order) != 0 {
        return Err(Error::Index);
    }
    Ok(())
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrickLayout {
    grid: GridLayout,
    origin: [u32; 3],
    shape: [u32; 3],
    cells: u64,
}
impl BrickLayout {
    /// The immutable validated grid used to decode this brick.
    pub fn grid_layout(self) -> GridLayout {
        self.grid
    }
    pub fn origin_cells(self) -> [u32; 3] {
        self.origin
    }
    pub fn shape(self) -> [u32; 3] {
        self.shape
    }
    pub fn cell_count(self) -> u64 {
        self.cells
    }
    /// X fastest. Callers apply the time index separately, never Morton/Hilbert
    /// permute these local rows just because the directory is curve-addressed.
    pub fn global_cell(self, index: u64) -> Result<[u32; 3], Error> {
        if index >= self.cells {
            return Err(Error::Index);
        }
        let x = index % u64::from(self.shape[0]);
        let yz = index / u64::from(self.shape[0]);
        Ok([
            self.origin[0] + x as u32,
            self.origin[1] + (yz % u64::from(self.shape[1])) as u32,
            self.origin[2] + (yz / u64::from(self.shape[1])) as u32,
        ])
    }
    pub fn local_position_m(self, index: u64) -> Result<[f64; 3], Error> {
        let cell = self.global_cell(index)?;
        let mut p = [0.0; 3];
        for axis in 0..3 {
            p[axis] = f64::from(cell[axis]) * self.grid.spec.spacing_m[axis];
            if !p[axis].is_finite() {
                return Err(Error::NonFinite);
            }
        }
        Ok(p)
    }
}
pub(super) fn from_metadata(frame: Block<'_>, ext: Block<'_>) -> Result<GridLayout, Error> {
    semantics::validate_fields(ext, super::extension_rules()).map_err(Error::Shared)?;
    let dimensions = typed(frame, 6, 1, 3)?;
    let spacing = typed(frame, 5, 5, 3)?;
    let full = typed(ext, 8, 2, 3)?;
    let brick = typed(ext, 9, 1, 3)?;
    let mut dims = [0; 3];
    let mut bricks = [0; 3];
    let mut spacings = [0.0; 3];
    for axis in 0..3 {
        dims[axis] = u32_at(dimensions, axis * 4);
        if u64_at(full, axis * 8) != u64::from(dims[axis]) {
            return Err(Error::Frame);
        }
        bricks[axis] = u32_at(brick, axis * 4);
        spacings[axis] = f64_at(spacing, axis * 8);
    }
    let order = scalar32(ext, 10)?;
    let indexing = match text(required(ext, 7)?)? {
        "linear" if order == 0 => GridIndexing::Linear,
        "morton3" => GridIndexing::Morton { order },
        "hilbert3" => GridIndexing::Hilbert { order },
        _ => return Err(Error::UnsupportedProfile),
    };
    match (indexing.common_order(), field(frame, 7)) {
        (Some(order), Some(value)) => {
            if value.type_tag() != 1
                || value.element_count() != 1
                || value.flags() & 1 == 0
                || u32_at(value.value(), 0) != order
            {
                return Err(Error::Frame);
            }
        }
        (None, None) => {}
        _ => return Err(Error::Frame),
    }
    GridSpec {
        dimensions: dims,
        brick_dimensions: bricks,
        spacing_m: spacings,
        indexing,
    }
    .validate()
}
fn typed<'a>(block: Block<'a>, key: u16, tag: u8, count: u32) -> Result<&'a [u8], Error> {
    let f = required(block, key)?;
    if f.type_tag() != tag || f.element_count() != count || f.flags() & 1 == 0 {
        return Err(Error::Frame);
    }
    Ok(f.value())
}
