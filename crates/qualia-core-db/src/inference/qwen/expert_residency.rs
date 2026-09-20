//! Fixed-capacity, layer-qualified residency metadata for Qwen4Exp experts.
//!
//! An MoE expert identifier is only unique within a transformer layer.  This
//! controller therefore keys cache entries by `(layer, expert)` and can drive
//! a future GPU payload uploader without cross-layer weight aliasing.

use super::QWEN4EXP_TOP_EXPERTS;

pub const MAX_QWEN_EXPERT_SLOTS: usize = 512;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QwenExpertKey {
    pub layer: u16,
    pub expert: u16,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Slot {
    key: Option<QwenExpertKey>,
    last_access: u64,
    pinned: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QwenExpertAccess {
    Hit {
        slot: usize,
    },
    Load {
        slot: usize,
        evicted: Option<QwenExpertKey>,
    },
    NoCapacity,
}

/// Cold-created cache metadata.  The runtime's GPU uploader owns the actual
/// payload buffers; this type only provides deterministic, zero-heap routing
/// decisions during decode.
pub struct QwenExpertResidency {
    slots: [Slot; MAX_QWEN_EXPERT_SLOTS],
    capacity: usize,
    tick: u64,
}

impl QwenExpertResidency {
    pub fn new(capacity: usize) -> Self {
        Self {
            slots: [Slot::default(); MAX_QWEN_EXPERT_SLOTS],
            capacity: capacity.clamp(1, MAX_QWEN_EXPERT_SLOTS),
            tick: 0,
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Resolve a layer-specific expert. Equal LRU ages prefer the lower slot,
    /// which makes eviction receipts deterministic.
    pub fn resolve(&mut self, key: QwenExpertKey) -> QwenExpertAccess {
        self.tick = self.tick.wrapping_add(1);
        for index in 0..self.capacity {
            let slot = &mut self.slots[index];
            if slot.key == Some(key) {
                slot.last_access = self.tick;
                return QwenExpertAccess::Hit { slot: index };
            }
        }
        let mut selected = None;
        let mut oldest = u64::MAX;
        for index in 0..self.capacity {
            let slot = self.slots[index];
            if slot.pinned {
                continue;
            }
            if slot.key.is_none() || slot.last_access < oldest {
                selected = Some(index);
                oldest = slot.last_access;
                if slot.key.is_none() {
                    break;
                }
            }
        }
        let Some(selected) = selected else {
            return QwenExpertAccess::NoCapacity;
        };
        let evicted = self.slots[selected].key;
        self.slots[selected] = Slot {
            key: Some(key),
            last_access: self.tick,
            pinned: false,
        };
        QwenExpertAccess::Load {
            slot: selected,
            evicted,
        }
    }

    /// Resolve the ten selected experts for one Qwen4Exp layer into an output
    /// supplied by the caller. Repeated ids are still routed independently by
    /// the caller but hit the same residency slot.
    pub fn resolve_topk(
        &mut self,
        layer: u16,
        experts: &[usize; QWEN4EXP_TOP_EXPERTS],
        out: &mut [QwenExpertAccess; QWEN4EXP_TOP_EXPERTS],
    ) {
        for index in 0..QWEN4EXP_TOP_EXPERTS {
            out[index] = self.resolve(QwenExpertKey {
                layer,
                expert: experts[index].min(u16::MAX as usize) as u16,
            });
        }
    }

    pub fn pin(&mut self, slot: usize) -> bool {
        if slot >= self.capacity || self.slots[slot].key.is_none() {
            return false;
        }
        self.slots[slot].pinned = true;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_qualifier_prevents_cross_layer_aliases() {
        let mut cache = QwenExpertResidency::new(2);
        assert_eq!(
            cache.resolve(QwenExpertKey {
                layer: 0,
                expert: 37
            }),
            QwenExpertAccess::Load {
                slot: 0,
                evicted: None
            }
        );
        assert_eq!(
            cache.resolve(QwenExpertKey {
                layer: 1,
                expert: 37
            }),
            QwenExpertAccess::Load {
                slot: 1,
                evicted: None
            }
        );
        assert_eq!(
            cache.resolve(QwenExpertKey {
                layer: 0,
                expert: 37
            }),
            QwenExpertAccess::Hit { slot: 0 }
        );
    }
}
