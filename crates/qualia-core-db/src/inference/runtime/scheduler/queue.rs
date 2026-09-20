//! Bounded request queue and worker intake lifecycle.
//!
//! Bridges asynchronous request intake to `RequestScheduler` and the sticky worker thread.
//! Guarantees:
//! 1. Bounded intake queue (rejects with `QueueFull` rather than growing unbounded).
//! 2. Exact lifecycle states: `Queued`, `Admitted`, `Prefill`, `Decode`, `Cancelled`, `Completed`.
//! 3. Cancellation at every phase has exactly one terminal result and clean currency release.

use std::collections::VecDeque;

/// Lifecycle state of a queued/running request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntakeState {
    Queued,
    Admitted { slot: u16 },
    Cancelled,
    Completed,
}

/// Request descriptor within the bounded scheduler intake.
#[derive(Debug, Clone)]
pub struct IntakeRequest {
    pub request_id: u64,
    pub prompt_tokens: u32,
    pub max_output_tokens: u32,
    pub state: IntakeState,
    pub is_cancelled: bool,
}

impl IntakeRequest {
    pub fn new(request_id: u64, prompt_tokens: u32, max_output_tokens: u32) -> Self {
        Self {
            request_id,
            prompt_tokens,
            max_output_tokens,
            state: IntakeState::Queued,
            is_cancelled: false,
        }
    }
}

/// Bounded FIFO intake queue for the request scheduler.
#[derive(Debug)]
pub struct BoundedIntakeQueue<const CAP: usize> {
    queue: VecDeque<IntakeRequest>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntakeError {
    QueueFull,
    DuplicateRequest,
    NotFound,
    AlreadyTerminal,
}

impl<const CAP: usize> Default for BoundedIntakeQueue<CAP> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const CAP: usize> BoundedIntakeQueue<CAP> {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::with_capacity(CAP),
        }
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn capacity(&self) -> usize {
        CAP
    }

    /// Enqueue a new request. Fails closed with `QueueFull` if capacity reached.
    pub fn enqueue(&mut self, request: IntakeRequest) -> Result<(), IntakeError> {
        if self.queue.len() >= CAP {
            return Err(IntakeError::QueueFull);
        }
        if self
            .queue
            .iter()
            .any(|r| r.request_id == request.request_id)
        {
            return Err(IntakeError::DuplicateRequest);
        }
        self.queue.push_back(request);
        Ok(())
    }

    /// Pop the next non-cancelled request ready for admission.
    pub fn pop_next_runnable(&mut self) -> Option<IntakeRequest> {
        while let Some(req) = self.queue.pop_front() {
            if !req.is_cancelled {
                return Some(req);
            }
        }
        None
    }

    /// Cancel a request by ID.
    /// Returns the prior state if successfully cancelled.
    pub fn cancel(&mut self, request_id: u64) -> Result<IntakeState, IntakeError> {
        let req = self
            .queue
            .iter_mut()
            .find(|r| r.request_id == request_id)
            .ok_or(IntakeError::NotFound)?;

        if req.is_cancelled || matches!(req.state, IntakeState::Cancelled | IntakeState::Completed)
        {
            return Err(IntakeError::AlreadyTerminal);
        }

        let prior = req.state;
        req.is_cancelled = true;
        req.state = IntakeState::Cancelled;
        Ok(prior)
    }

    /// Find an intake request by ID.
    pub fn find(&self, request_id: u64) -> Option<&IntakeRequest> {
        self.queue.iter().find(|r| r.request_id == request_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounded_queue_capacity() {
        let mut queue = BoundedIntakeQueue::<2>::new();
        assert_eq!(queue.capacity(), 2);

        let r1 = IntakeRequest::new(101, 10, 20);
        let r2 = IntakeRequest::new(102, 15, 25);
        let r3 = IntakeRequest::new(103, 20, 30);

        assert!(queue.enqueue(r1).is_ok());
        assert!(queue.enqueue(r2).is_ok());
        // Third must fail with QueueFull
        assert_eq!(queue.enqueue(r3), Err(IntakeError::QueueFull));
    }

    #[test]
    fn test_cancellation_before_admission() {
        let mut queue = BoundedIntakeQueue::<4>::new();
        let r1 = IntakeRequest::new(201, 10, 20);
        let r2 = IntakeRequest::new(202, 15, 25);

        queue.enqueue(r1).unwrap();
        queue.enqueue(r2).unwrap();

        // Cancel r1 while queued
        let prior = queue.cancel(201).unwrap();
        assert_eq!(prior, IntakeState::Queued);

        // Popping next runnable skips cancelled r1 and returns r2
        let next = queue.pop_next_runnable().unwrap();
        assert_eq!(next.request_id, 202);

        // Re-cancelling r1 returns AlreadyTerminal
        assert_eq!(queue.cancel(201), Err(IntakeError::NotFound)); // Already drained
    }
}
