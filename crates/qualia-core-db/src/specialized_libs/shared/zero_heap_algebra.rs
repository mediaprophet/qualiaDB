//! Zero-Heap Linear Algebra
//!
//! Provides statically-sized mathematical primitives that comply with the system's strict zero-allocation boundaries.
//! Designed to serve as the structural backbone for Category-theoretic morphisms.

use core::ops::{Add, Mul};

/// A fixed-size zero-heap matrix.
#[derive(Debug, Clone, Copy)]
pub struct ZeroHeapMatrix<T, const ROWS: usize, const COLS: usize> {
    pub data: [[T; COLS]; ROWS],
}

impl<T, const ROWS: usize, const COLS: usize> ZeroHeapMatrix<T, ROWS, COLS>
where
    T: Copy + Default,
{
    pub fn new(data: [[T; COLS]; ROWS]) -> Self {
        Self { data }
    }

    pub fn zeros() -> Self {
        Self {
            data: [[T::default(); COLS]; ROWS],
        }
    }

    pub fn get(&self, row: usize, col: usize) -> T {
        self.data[row][col]
    }

    pub fn set(&mut self, row: usize, col: usize, value: T) {
        self.data[row][col] = value;
    }
}

// Implement matrix multiplication
impl<T, const R1: usize, const C1: usize, const C2: usize> Mul<ZeroHeapMatrix<T, C1, C2>>
    for ZeroHeapMatrix<T, R1, C1>
where
    T: Copy + Default + Add<Output = T> + Mul<Output = T>,
{
    type Output = ZeroHeapMatrix<T, R1, C2>;

    fn mul(self, rhs: ZeroHeapMatrix<T, C1, C2>) -> Self::Output {
        // Safe because the arrays are fully initialized with T::Default
        let mut result_data = [[T::default(); C2]; R1];

        for i in 0..R1 {
            for j in 0..C2 {
                let mut sum = T::default();
                for k in 0..C1 {
                    sum = sum + (self.data[i][k] * rhs.data[k][j]);
                }
                result_data[i][j] = sum;
            }
        }

        ZeroHeapMatrix::new(result_data)
    }
}
