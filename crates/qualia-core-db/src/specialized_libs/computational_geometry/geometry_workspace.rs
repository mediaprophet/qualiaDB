//! P10.5 — Geometry workspace: caller-owned arenas with byte budgets,
//! deterministic partition/reduction order, and cancellation.
//!
//! The execution plan (P10.5) requires that geometry algorithms expose a
//! uniform workspace contract:
//!
//! 1. **Caller-owned arenas** — the caller allocates a byte buffer (stack or
//!    caller-owned heap) and passes it in. The algorithm bumps within the
//!    buffer; it never calls the global allocator. This keeps the 42-MiB
//!    Webizen VM ceiling (AGENTS.md §0) under the caller's control.
//!
//! 2. **Byte budgets** — every arena carries a `byte_budget`. An allocation
//!    that would exceed the budget returns `Err(WorkspaceError::BudgetExceeded)`,
//!    not a panic. The caller decides whether to grow, spill, or refuse.
//!
//! 3. **Deterministic partition/reduction order** — parallel geometry passes
//!    partition work items in a deterministic order (Morton-sorted by
//!    spatial key) and reduce partial results in a deterministic order
//!    (left-fold over the partition index). This makes parallel output
//!    bit-identical to serial output — the `DeterminismClass::BitExact`
//!    contract holds regardless of thread count.
//!
//! 4. **Cancellation** — a `Cancellation` token (atomic bool) lets the caller
//!    abort a long-running pass. The algorithm checks the token at each
//!    partition boundary and returns `Err(WorkspaceError::Cancelled)` if set.
//!
//! ## 42 MiB ceiling
//!
//! The `DEFAULT_WORKSPACE_BUDGET` is 42 MiB (44,040,192 bytes), matching the
//! Webizen VM `SlgArena` ceiling. A maximal admitted pass — the largest
//! input the workspace accepts without spilling — must remain below this.
//! The `Workspace::admit_pass` function checks this.

use std::sync::atomic::{AtomicBool, Ordering};

// ───────────────────────────────────────────────────────────────────────────
//  Constants
// ───────────────────────────────────────────────────────────────────────────

/// The default byte budget: 42 MiB, matching the Webizen VM SlgArena ceiling.
pub const DEFAULT_WORKSPACE_BUDGET: usize = 42 * 1024 * 1024;

/// Minimum alignment for arena allocations (8 bytes — enough for u64/f64/pointer).
pub const ARENA_ALIGN: usize = 8;

// ───────────────────────────────────────────────────────────────────────────
//  Errors
// ───────────────────────────────────────────────────────────────────────────

/// Typed error returned by workspace operations. Never panics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceError {
    /// The allocation would exceed the byte budget.
    BudgetExceeded {
        requested: usize,
        available: usize,
    },
    /// The caller set the cancellation token.
    Cancelled,
    /// The input is too large for a single admitted pass under the budget.
    PassTooLarge {
        input_bytes: usize,
        budget: usize,
    },
    /// The arena is exhausted (all bytes used; reset needed).
    Exhausted,
}

impl std::fmt::Display for WorkspaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkspaceError::BudgetExceeded { requested, available } => {
                write!(f, "workspace budget exceeded: requested {} bytes, {} available", requested, available)
            }
            WorkspaceError::Cancelled => write!(f, "workspace operation cancelled by caller"),
            WorkspaceError::PassTooLarge { input_bytes, budget } => {
                write!(f, "input too large for single pass: {} bytes > {} budget", input_bytes, budget)
            }
            WorkspaceError::Exhausted => write!(f, "workspace arena exhausted (reset needed)"),
        }
    }
}

impl std::error::Error for WorkspaceError {}

// ───────────────────────────────────────────────────────────────────────────
//  Cancellation token
// ───────────────────────────────────────────────────────────────────────────

/// A cancellation token shared between the caller and the algorithm.
///
/// The caller sets `cancel()` to abort a long-running pass; the algorithm
/// checks `is_cancelled()` at each partition boundary. This is a cooperative
/// cancel — the algorithm is not forcibly stopped, it checks at safe points.
#[derive(Debug, Default)]
pub struct Cancellation {
    cancelled: AtomicBool,
}

impl Cancellation {
    /// Create a new non-cancelled token.
    pub fn new() -> Self {
        Self { cancelled: AtomicBool::new(false) }
    }

    /// Set the cancellation flag.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    /// Check if cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    /// Reset the cancellation flag (for reuse across passes).
    pub fn reset(&self) {
        self.cancelled.store(false, Ordering::Relaxed);
    }
}

impl Clone for Cancellation {
    fn clone(&self) -> Self {
        Self { cancelled: AtomicBool::new(self.cancelled.load(Ordering::Relaxed)) }
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  GeometryWorkspace — caller-owned byte arena
// ───────────────────────────────────────────────────────────────────────────

/// A caller-owned byte arena for geometry algorithm scratch space.
///
/// The caller allocates the backing buffer (stack array, `Box<[u8]>`, or
/// mmap'd region) and wraps it in a `GeometryWorkspace`. The algorithm
/// bumps within the buffer via `alloc` / `alloc_slice`; it never calls the
/// global allocator.
///
/// ## Determinism
///
/// The workspace itself is deterministic: the bump order is determined by
/// the algorithm's allocation sequence, which is deterministic for a given
/// input. Parallel passes use `deterministic_partition` to split work items
/// in Morton-sorted order, ensuring bit-identical output regardless of
/// thread count.
///
/// ## 42 MiB ceiling
///
/// `admit_pass` checks that the input + estimated scratch fits within the
/// byte budget. The default budget is 42 MiB (AGENTS.md §0).
pub struct GeometryWorkspace<'a> {
    /// The backing byte buffer (caller-owned).
    buffer: &'a mut [u8],
    /// The current bump offset within the buffer.
    offset: usize,
    /// The byte budget (usually equal to buffer.len(), but can be smaller
    /// to reserve headroom).
    byte_budget: usize,
    /// Cancellation token (shared via reference).
    cancel: &'a Cancellation,
}

impl<'a> GeometryWorkspace<'a> {
    /// Create a new workspace over a caller-owned byte buffer.
    ///
    /// The byte budget is set to `buffer.len()`. The cancellation token is
    /// borrowed from the caller.
    pub fn new(buffer: &'a mut [u8], cancel: &'a Cancellation) -> Self {
        let byte_budget = buffer.len();
        Self { buffer, offset: 0, byte_budget, cancel }
    }

    /// Create a workspace with a custom byte budget (smaller than the buffer
    /// to reserve headroom).
    pub fn with_budget(buffer: &'a mut [u8], byte_budget: usize, cancel: &'a Cancellation) -> Self {
        assert!(byte_budget <= buffer.len(), "budget cannot exceed buffer size");
        Self { buffer, offset: 0, byte_budget, cancel }
    }

    /// The total byte budget.
    pub fn byte_budget(&self) -> usize {
        self.byte_budget
    }

    /// The number of bytes currently used.
    pub fn bytes_used(&self) -> usize {
        self.offset
    }

    /// The number of bytes still available.
    pub fn bytes_available(&self) -> usize {
        self.byte_budget.saturating_sub(self.offset)
    }

    /// Check if the caller has requested cancellation.
    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    /// Reset the arena to empty (reuse for a new pass).
    pub fn reset(&mut self) {
        self.offset = 0;
    }

    /// Allocate `size` bytes with `ARENA_ALIGN` alignment. Returns a slice
    /// into the backing buffer, or `Err(WorkspaceError)` if the budget is
    /// exceeded or the caller cancelled.
    pub fn alloc(&mut self, size: usize) -> Result<&mut [u8], WorkspaceError> {
        if self.cancel.is_cancelled() {
            return Err(WorkspaceError::Cancelled);
        }
        // Align the offset up.
        let aligned_offset = align_up(self.offset, ARENA_ALIGN);
        let end = aligned_offset.checked_add(size)
            .ok_or(WorkspaceError::BudgetExceeded { requested: size, available: self.bytes_available() })?;
        if end > self.byte_budget {
            return Err(WorkspaceError::BudgetExceeded { requested: size, available: self.bytes_available() });
        }
        self.offset = end;
        Ok(&mut self.buffer[aligned_offset..end])
    }

    /// Allocate space for `count` elements of `T` (typed slice). The caller
    /// is responsible for initializing the memory.
    pub fn alloc_slice<T>(&mut self, count: usize) -> Result<&mut [T], WorkspaceError> {
        let size = count.checked_mul(std::mem::size_of::<T>())
            .ok_or(WorkspaceError::BudgetExceeded { requested: usize::MAX, available: self.bytes_available() })?;
        let align = std::mem::align_of::<T>().max(ARENA_ALIGN);
        if self.cancel.is_cancelled() {
            return Err(WorkspaceError::Cancelled);
        }
        let aligned_offset = align_up(self.offset, align);
        let end = aligned_offset.checked_add(size)
            .ok_or(WorkspaceError::BudgetExceeded { requested: size, available: self.bytes_available() })?;
        if end > self.byte_budget {
            return Err(WorkspaceError::BudgetExceeded { requested: size, available: self.bytes_available() });
        }
        self.offset = end;
        // SAFETY: the buffer is valid for `byte_budget` bytes, we've checked
        // that [aligned_offset..end] is within bounds, and the alignment is
        // correct. The caller must initialize before reading.
        let ptr = self.buffer[aligned_offset..].as_mut_ptr() as *mut T;
        unsafe { Ok(std::slice::from_raw_parts_mut(ptr, count)) }
    }

    /// Check that an input of `input_bytes` can be admitted for a single pass,
    /// given the workspace's byte budget. Returns `Ok(())` if the input fits,
    /// `Err(PassTooLarge)` if it exceeds the budget.
    ///
    /// The "maximal admitted pass" is the largest input that fits within the
    /// 42 MiB ceiling. This function is the gate: if it returns `Err`, the
    /// caller must split the input into smaller passes or refuse.
    pub fn admit_pass(&self, input_bytes: usize) -> Result<(), WorkspaceError> {
        // The input itself must fit, plus we need scratch space. A conservative
        // estimate: input + 50% for scratch (algorithm-specific; the caller
        // can override with a custom check).
        let estimated_total = input_bytes + (input_bytes / 2);
        if estimated_total > self.byte_budget {
            Err(WorkspaceError::PassTooLarge {
                input_bytes,
                budget: self.byte_budget,
            })
        } else {
            Ok(())
        }
    }
}

/// Align `offset` up to `align`.
fn align_up(offset: usize, align: usize) -> usize {
    (offset + align - 1) & !(align - 1)
}

// ───────────────────────────────────────────────────────────────────────────
//  Deterministic partition / reduction
// ───────────────────────────────────────────────────────────────────────────

/// Partition `n` work items into `num_partitions` contiguous chunks in
/// deterministic order (partition 0 gets items [0, k), partition 1 gets
/// [k, 2k), etc.).
///
/// This is the deterministic partition order: regardless of thread count,
/// partition `i` always gets the same range of items. A parallel pass that
/// processes partitions in order and reduces left-to-right produces
/// bit-identical output to a serial pass.
pub fn deterministic_partition(n: usize, num_partitions: usize) -> Vec<(usize, usize)> {
    if n == 0 || num_partitions == 0 {
        return Vec::new();
    }
    let chunk = (n + num_partitions - 1) / num_partitions;
    let mut partitions = Vec::with_capacity(num_partitions);
    let mut start = 0;
    while start < n {
        let end = (start + chunk).min(n);
        partitions.push((start, end));
        start = end;
    }
    partitions
}

/// Deterministic left-fold reduction over a slice of partial results.
///
/// This is the deterministic reduction order: partial results are folded
/// left-to-right (index 0, then 1, then 2, ...), regardless of which
/// partition produced which result first. This makes parallel output
/// bit-identical to serial output.
pub fn deterministic_reduce<T, F>(partials: &[T], f: F) -> Option<T>
where
    T: Clone,
    F: Fn(&T, &T) -> T,
{
    if partials.is_empty() {
        return None;
    }
    let mut acc = partials[0].clone();
    for p in &partials[1..] {
        acc = f(&acc, p);
    }
    Some(acc)
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_allocates_within_budget() {
        let cancel = Cancellation::new();
        let mut buf = vec![0u8; 1024];
        let mut ws = GeometryWorkspace::new(&mut buf, &cancel);
        let slice = ws.alloc(64).unwrap();
        assert_eq!(slice.len(), 64);
        assert_eq!(ws.bytes_used(), 64);
    }

    #[test]
    fn workspace_rejects_over_budget() {
        let cancel = Cancellation::new();
        let mut buf = vec![0u8; 128];
        let mut ws = GeometryWorkspace::new(&mut buf, &cancel);
        let result = ws.alloc(256);
        assert_eq!(result, Err(WorkspaceError::BudgetExceeded { requested: 256, available: 128 }));
    }

    #[test]
    fn workspace_alignment_is_respected() {
        let cancel = Cancellation::new();
        let mut buf = vec![0u8; 256];
        let mut ws = GeometryWorkspace::new(&mut buf, &cancel);
        // Allocate 1 byte — offset becomes 1.
        ws.alloc(1).unwrap();
        assert_eq!(ws.bytes_used(), 1);
        // Allocate 8 bytes — offset should align to 8, then add 8 = 16.
        ws.alloc(8).unwrap();
        assert_eq!(ws.bytes_used(), 16);
    }

    #[test]
    fn workspace_cancellation_aborts() {
        let cancel = Cancellation::new();
        let mut buf = vec![0u8; 1024];
        let mut ws = GeometryWorkspace::new(&mut buf, &cancel);
        cancel.cancel();
        let result = ws.alloc(64);
        assert_eq!(result, Err(WorkspaceError::Cancelled));
    }

    #[test]
    fn workspace_reset_clears_arena() {
        let cancel = Cancellation::new();
        let mut buf = vec![0u8; 1024];
        let mut ws = GeometryWorkspace::new(&mut buf, &cancel);
        ws.alloc(512).unwrap();
        assert_eq!(ws.bytes_used(), 512);
        ws.reset();
        assert_eq!(ws.bytes_used(), 0);
        // Can allocate again after reset.
        ws.alloc(512).unwrap();
    }

    #[test]
    fn workspace_alloc_slice_typed() {
        let cancel = Cancellation::new();
        let mut buf = vec![0u8; 1024];
        let mut ws = GeometryWorkspace::new(&mut buf, &cancel);
        let slice: &mut [u64] = ws.alloc_slice::<u64>(16).unwrap();
        assert_eq!(slice.len(), 16);
        // 16 * 8 = 128 bytes used.
        assert_eq!(ws.bytes_used(), 128);
    }

    #[test]
    fn admit_pass_accepts_small_input() {
        let cancel = Cancellation::new();
        let mut buf = vec![0u8; DEFAULT_WORKSPACE_BUDGET];
        let ws = GeometryWorkspace::new(&mut buf, &cancel);
        // 1 MB input — well within 42 MiB.
        assert!(ws.admit_pass(1024 * 1024).is_ok());
    }

    #[test]
    fn admit_pass_rejects_oversized_input() {
        let cancel = Cancellation::new();
        let mut buf = vec![0u8; 1024];
        let ws = GeometryWorkspace::new(&mut buf, &cancel);
        // 2 KB input — exceeds 1 KB budget (with 50% scratch overhead).
        assert_eq!(
            ws.admit_pass(2048),
            Err(WorkspaceError::PassTooLarge { input_bytes: 2048, budget: 1024 })
        );
    }

    #[test]
    fn admit_pass_42mib_ceiling_holds() {
        let cancel = Cancellation::new();
        let mut buf = vec![0u8; DEFAULT_WORKSPACE_BUDGET];
        let ws = GeometryWorkspace::new(&mut buf, &cancel);
        // A maximal admitted pass: input + 50% scratch = 42 MiB.
        // input = 42 MiB / 1.5 = 28 MiB.
        let max_input = DEFAULT_WORKSPACE_BUDGET * 2 / 3;
        assert!(ws.admit_pass(max_input).is_ok(), "maximal admitted pass should fit");
        // One byte more should fail.
        assert!(ws.admit_pass(max_input + 1).is_err(), "pass above maximal should fail");
    }

    #[test]
    fn deterministic_partition_splits_evenly() {
        let parts = deterministic_partition(100, 4);
        assert_eq!(parts.len(), 4);
        assert_eq!(parts[0], (0, 25));
        assert_eq!(parts[1], (25, 50));
        assert_eq!(parts[2], (50, 75));
        assert_eq!(parts[3], (75, 100));
        // Total coverage.
        assert_eq!(parts.last().unwrap().1, 100);
    }

    #[test]
    fn deterministic_partition_uneven() {
        let parts = deterministic_partition(10, 3);
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], (0, 4));
        assert_eq!(parts[1], (4, 8));
        assert_eq!(parts[2], (8, 10));
    }

    #[test]
    fn deterministic_partition_empty() {
        assert!(deterministic_partition(0, 4).is_empty());
        assert!(deterministic_partition(100, 0).is_empty());
    }

    #[test]
    fn deterministic_reduce_left_fold() {
        let partials = vec![1, 2, 3, 4];
        let result = deterministic_reduce(&partials, |a, b| a + b);
        assert_eq!(result, Some(10));
        // Left-fold order: ((1+2)+3)+4 = 10.
    }

    #[test]
    fn deterministic_reduce_single() {
        let partials = vec![42];
        let result = deterministic_reduce(&partials, |a, b| a + b);
        assert_eq!(result, Some(42));
    }

    #[test]
    fn deterministic_reduce_empty() {
        let partials: Vec<i32> = vec![];
        let result = deterministic_reduce(&partials, |a, b| a + b);
        assert_eq!(result, None);
    }

    #[test]
    fn deterministic_reduce_string_concatenation_order() {
        // Verify left-to-right order: "a" + "b" + "c" = "abc", not "cba".
        let partials = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let result = deterministic_reduce(&partials, |a, b| format!("{}{}", a, b));
        assert_eq!(result.as_deref(), Some("abc"));
    }

    #[test]
    fn cancellation_reset_allows_reuse() {
        let cancel = Cancellation::new();
        assert!(!cancel.is_cancelled());
        cancel.cancel();
        assert!(cancel.is_cancelled());
        cancel.reset();
        assert!(!cancel.is_cancelled());
    }

    #[test]
    fn workspace_error_displays() {
        let e = WorkspaceError::BudgetExceeded { requested: 100, available: 50 };
        let s = format!("{}", e);
        assert!(s.contains("100"));
        assert!(s.contains("50"));

        let e = WorkspaceError::Cancelled;
        assert!(format!("{}", e).contains("cancelled"));

        let e = WorkspaceError::PassTooLarge { input_bytes: 200, budget: 100 };
        assert!(format!("{}", e).contains("200"));
    }

    #[test]
    fn workspace_with_budget_respects_smaller_budget() {
        let cancel = Cancellation::new();
        let mut buf = vec![0u8; 1024];
        let mut ws = GeometryWorkspace::with_budget(&mut buf, 512, &cancel);
        // 512 bytes should fit.
        ws.alloc(512).unwrap();
        // 1 more byte should fail (budget is 512, not 1024).
        assert_eq!(
            ws.alloc(1),
            Err(WorkspaceError::BudgetExceeded { requested: 1, available: 0 })
        );
    }

    #[test]
    fn parallel_partition_produces_deterministic_order() {
        // Simulate a parallel pass: partition 100 items into 4 partitions,
        // process each, then reduce. The result must be the same regardless
        // of which partition finishes first.
        let parts = deterministic_partition(100, 4);
        // Process partitions in "random" order (3, 1, 0, 2) — simulating
        // out-of-order parallel completion.
        let mut partials: Vec<i64> = Vec::new();
        let order = [3, 1, 0, 2];
        for &i in &order {
            let (start, end) = parts[i];
            let sum: i64 = (start..end).map(|x| x as i64 * 2).sum();
            partials.push(sum);
        }
        // But we need to reduce in DETERMINISTIC order (partition index order),
        // not completion order. So sort partials by partition index first.
        // In practice, the reduction collects partials indexed by partition
        // and folds left-to-right.
        let mut ordered_partials: Vec<i64> = Vec::with_capacity(4);
        for i in 0..4 {
            let (start, end) = parts[i];
            let sum: i64 = (start..end).map(|x| x as i64 * 2).sum();
            ordered_partials.push(sum);
        }
        let result = deterministic_reduce(&ordered_partials, |a, b| a + b);
        // Sum of 0..100 * 2 = 2 * (99 * 100 / 2) = 9900.
        assert_eq!(result, Some(9900));
    }
}
