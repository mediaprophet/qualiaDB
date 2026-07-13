//! Caller-owned calculus workspace accounting.

pub const SENTINEL_PASS_BYTES: usize = 42 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceError {
    SizeOverflow,
    SentinelExceeded { required: usize },
    BufferTooSmall { required: usize, available: usize },
}

pub const fn checked_workspace_bytes<T>(elements: usize) -> Result<usize, WorkspaceError> {
    let Some(required) = elements.checked_mul(core::mem::size_of::<T>()) else {
        return Err(WorkspaceError::SizeOverflow);
    };
    if required > SENTINEL_PASS_BYTES {
        return Err(WorkspaceError::SentinelExceeded { required });
    }
    Ok(required)
}

pub struct Workspace<'a, T> {
    storage: &'a mut [T],
    used: usize,
}

impl<'a, T> Workspace<'a, T> {
    pub fn new(storage: &'a mut [T]) -> Result<Self, WorkspaceError> {
        checked_workspace_bytes::<T>(storage.len())?;
        Ok(Self { storage, used: 0 })
    }

    pub const fn capacity(&self) -> usize {
        self.storage.len()
    }

    pub const fn used(&self) -> usize {
        self.used
    }

    pub const fn remaining(&self) -> usize {
        self.storage.len() - self.used
    }

    pub fn reset(&mut self) {
        self.used = 0;
    }

    pub fn take(&mut self, elements: usize) -> Result<&mut [T], WorkspaceError> {
        let end = self
            .used
            .checked_add(elements)
            .ok_or(WorkspaceError::SizeOverflow)?;
        if end > self.storage.len() {
            return Err(WorkspaceError::BufferTooSmall {
                required: end,
                available: self.storage.len(),
            });
        }
        let start = self.used;
        self.used = end;
        Ok(&mut self.storage[start..end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_is_caller_owned_and_resettable() {
        let mut storage = [0.0_f64; 16];
        let mut workspace = Workspace::new(&mut storage).unwrap();
        assert_eq!(workspace.take(5).unwrap().len(), 5);
        assert_eq!(workspace.used(), 5);
        assert_eq!(workspace.remaining(), 11);
        workspace.reset();
        assert_eq!(workspace.used(), 0);
    }

    #[test]
    fn sentinel_and_capacity_fail_closed() {
        assert!(matches!(
            checked_workspace_bytes::<u64>(SENTINEL_PASS_BYTES / 8 + 1),
            Err(WorkspaceError::SentinelExceeded { .. })
        ));
        let mut storage = [0_u32; 4];
        let mut workspace = Workspace::new(&mut storage).unwrap();
        assert_eq!(
            workspace.take(5),
            Err(WorkspaceError::BufferTooSmall {
                required: 5,
                available: 4
            })
        );
    }
}
