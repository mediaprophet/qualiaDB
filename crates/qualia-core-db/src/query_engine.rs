use std::fs::File;
use std::path::Path;
use std::time::Instant;
use memmap2::MmapOptions;
use crate::QualiaQuin;

/// Memory-maps a large `.q42` file and performs a lightning-fast scan for a subject ID
pub fn mmap_query_subject(file_path: &str, subject_id: u64) -> Result<Vec<QualiaQuin>, Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    let path = Path::new(file_path);
    
    // Open the file and memory-map it directly from the OS page cache
    let file = File::open(path)?;
    let mmap = unsafe { MmapOptions::new().map(&file)? };
    
    let quin_size = std::mem::size_of::<QualiaQuin>();
    let mut results = Vec::new();
    
    // Safety check for corruption
    if mmap.len() % quin_size != 0 {
        return Err("WARNING: File size is not aligned to 48-byte QualiaQuin structures. Corrupt dataset.".into());
    }

    let total_quins = mmap.len() / quin_size;
    println!("🔍 Memory-Mapped {} Quins ({:.2} MB) into virtual memory.", 
        total_quins, 
        mmap.len() as f64 / 1_048_576.0
    );

    // Scan through the virtual memory pages
    for chunk in mmap.chunks_exact(quin_size) {
        let quin: QualiaQuin = unsafe { std::ptr::read_unaligned(chunk.as_ptr() as *const QualiaQuin) };
        if quin.subject == subject_id {
            results.push(quin);
        }
    }

    let duration = start_time.elapsed();
    println!("⚡ Query Complete! Scanned {} triples in {:?}", total_quins, duration);
    println!("🎯 Found {} relationships for subject {}", results.len(), subject_id);
    
    Ok(results)
}
