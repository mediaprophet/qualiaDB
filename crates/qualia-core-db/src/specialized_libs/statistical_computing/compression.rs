use super::*;

/// Data compression engine
pub struct DataCompressionEngine {
    compression_algorithms: Vec<CompressionAlgorithm>,
    compression_statistics: CompressionStatistics,
}

/// Compression algorithms
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    Gzip,
    LZ4,
    ZSTD,
    Snappy,
    Custom(String),
}

/// Compression statistics
///
/// Tracks cumulative metrics across all compression/decompression operations
/// performed by a [`DataCompressionEngine`]. The `compression_ratio` field
/// records the ratio of the *most recent* compression operation, while
/// [`CompressionStatistics::compression_ratio`] computes the *overall* ratio
/// from the cumulative byte totals.
#[derive(Debug, Clone)]
pub struct CompressionStatistics {
    /// Total original (uncompressed) bytes processed across all compress ops.
    pub original_size: u64,
    /// Total compressed bytes produced across all compress ops.
    pub compressed_size: u64,
    /// Ratio of the most recent compression operation (compressed / original).
    pub compression_ratio: f64,
    /// Total time spent compressing, in nanoseconds.
    pub compression_time: u64,
    /// Total time spent decompressing, in nanoseconds.
    pub decompression_time: u64,
    /// Number of compression operations performed.
    pub compression_count: u64,
    /// Number of decompression operations performed.
    pub decompression_count: u64,
}

impl DataCompressionEngine {
    pub fn new() -> Self {
        Self {
            compression_algorithms: vec![CompressionAlgorithm::LZ4, CompressionAlgorithm::ZSTD],
            compression_statistics: CompressionStatistics::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        Ok(())
    }

    /// Compress `data` using a simple run-length encoding and record the
    /// operation's statistics (original size, compressed size, ratio, time).
    pub fn compress(&mut self, data: &[u8]) -> Result<Vec<u8>, StatisticalError> {
        let start = Instant::now();
        let compressed = rle_compress(data);
        let elapsed = start.elapsed().as_nanos() as u64;
        self.compression_statistics.record_compression(
            data.len() as u64,
            compressed.len() as u64,
            elapsed,
        );
        Ok(compressed)
    }

    /// Decompress data previously produced by [`compress`](Self::compress) and
    /// record the decompression statistics.
    pub fn decompress(&mut self, data: &[u8]) -> Result<Vec<u8>, StatisticalError> {
        let start = Instant::now();
        let decompressed = rle_decompress(data)?;
        let elapsed = start.elapsed().as_nanos() as u64;
        self.compression_statistics.record_decompression(elapsed);
        Ok(decompressed)
    }

    /// Returns a reference to the cumulative compression statistics.
    pub fn get_statistics(&self) -> &CompressionStatistics {
        &self.compression_statistics
    }

    /// Resets all accumulated compression statistics to zero.
    pub fn reset_statistics(&mut self) {
        self.compression_statistics = CompressionStatistics::new();
    }

    /// Returns the list of compression algorithms available to this engine.
    pub fn compression_algorithms(&self) -> &[CompressionAlgorithm] {
        &self.compression_algorithms
    }

    /// Register an additional compression algorithm.
    pub fn add_compression_algorithm(&mut self, algorithm: CompressionAlgorithm) {
        if !self.compression_algorithms.contains(&algorithm) {
            self.compression_algorithms.push(algorithm);
        }
    }

    /// Returns `true` when the given algorithm is registered.
    pub fn supports_algorithm(&self, algorithm: &CompressionAlgorithm) -> bool {
        self.compression_algorithms.contains(algorithm)
    }
}

impl CompressionStatistics {
    /// Create a fresh, zeroed statistics record.
    pub fn new() -> Self {
        Self {
            original_size: 0,
            compressed_size: 0,
            compression_ratio: 0.0,
            compression_time: 0,
            decompression_time: 0,
            compression_count: 0,
            decompression_count: 0,
        }
    }

    /// Record a single compression operation.
    pub fn record_compression(&mut self, original: u64, compressed: u64, elapsed_ns: u64) {
        self.original_size += original;
        self.compressed_size += compressed;
        self.compression_time += elapsed_ns;
        self.compression_count += 1;
        self.compression_ratio = if original == 0 {
            0.0
        } else {
            compressed as f64 / original as f64
        };
    }

    /// Record a single decompression operation.
    pub fn record_decompression(&mut self, elapsed_ns: u64) {
        self.decompression_time += elapsed_ns;
        self.decompression_count += 1;
    }

    /// Overall compression ratio across all operations
    /// (`compressed_size / original_size`). Returns `0.0` when no data has been
    /// compressed yet.
    pub fn compression_ratio(&self) -> f64 {
        if self.original_size == 0 {
            0.0
        } else {
            self.compressed_size as f64 / self.original_size as f64
        }
    }

    /// Human-readable summary of the accumulated statistics.
    pub fn summary(&self) -> String {
        format!(
            "CompressionStatistics: {} compress op(s), {} decompress op(s), \
             original={} bytes, compressed={} bytes, overall ratio={:.4}, \
             last-op ratio={:.4}, compress_time={} ns, decompress_time={} ns",
            self.compression_count,
            self.decompression_count,
            self.original_size,
            self.compressed_size,
            self.compression_ratio(),
            self.compression_ratio,
            self.compression_time,
            self.decompression_time,
        )
    }
}

/// Simple run-length encoding over bytes. Each run is emitted as
/// `(count: u8, byte: u8)`; runs longer than 255 are split. Incompressible
/// data expands by ~2x, but repetitive data (the common statistical-dataset
/// case for constant columns) compresses well.
fn rle_compress(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut i = 0;
    while i < data.len() {
        let byte = data[i];
        let mut count: usize = 1;
        while i + count < data.len() && data[i + count] == byte && count < 255 {
            count += 1;
        }
        out.push(count as u8);
        out.push(byte);
        i += count;
    }
    out
}

/// Inverse of [`rle_compress`].
fn rle_decompress(data: &[u8]) -> Result<Vec<u8>, StatisticalError> {
    if data.len() % 2 != 0 {
        return Err(StatisticalError::InvalidData(
            "Corrupted RLE stream (odd length)".to_string(),
        ));
    }
    let mut out = Vec::with_capacity(data.len());
    let mut i = 0;
    while i < data.len() {
        let count = data[i] as usize;
        let byte = data[i + 1];
        out.resize(out.len() + count, byte);
        i += 2;
    }
    Ok(out)
}
