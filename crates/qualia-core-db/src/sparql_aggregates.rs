//! SPARQL Aggregate Functions
//!
//! Implements COUNT, SUM, AVG, MIN, MAX aggregate functions using zero-allocation patterns.

use crate::sparql_ast::*;

/// Aggregate function types
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregateFunction {
    Count,
    Sum,
    Avg,
    Min,
    Max,
}

/// Aggregate accumulator
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AggregateAccumulator {
    pub func: AggregateFunction,
    pub count: u64,
    pub sum: u64,
    pub min: u64,
    pub max: u64,
    pub initialized: bool,
}

impl AggregateAccumulator {
    pub fn new(func: AggregateFunction) -> Self {
        Self {
            func,
            count: 0,
            sum: 0,
            min: u64::MAX,
            max: 0,
            initialized: false,
        }
    }

    pub fn add_value(&mut self, value: u64) {
        self.count += 1;
        
        match self.func {
            AggregateFunction::Count => {
                // Count just increments count
            }
            AggregateFunction::Sum => {
                self.sum = self.sum.wrapping_add(value);
            }
            AggregateFunction::Avg => {
                self.sum = self.sum.wrapping_add(value);
            }
            AggregateFunction::Min => {
                if !self.initialized || value < self.min {
                    self.min = value;
                    self.initialized = true;
                }
            }
            AggregateFunction::Max => {
                if !self.initialized || value > self.max {
                    self.max = value;
                    self.initialized = true;
                }
            }
        }
    }

    pub fn get_result(&self) -> Option<u64> {
        if self.count == 0 {
            return None;
        }

        match self.func {
            AggregateFunction::Count => Some(self.count),
            AggregateFunction::Sum => Some(self.sum),
            AggregateFunction::Avg => {
                if self.count > 0 {
                    Some(self.sum / self.count)
                } else {
                    None
                }
            }
            AggregateFunction::Min => {
                if self.initialized {
                    Some(self.min)
                } else {
                    None
                }
            }
            AggregateFunction::Max => {
                if self.initialized {
                    Some(self.max)
                } else {
                    None
                }
            }
        }
    }
}

/// Aggregate group key (for GROUP BY)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GroupKey {
    pub values: [u64; MAX_VARIABLES],
    pub var_count: u8,
}

impl GroupKey {
    pub fn new() -> Self {
        Self {
            values: [0; MAX_VARIABLES],
            var_count: 0,
        }
    }

    pub fn set(&mut self, var_id: VariableId, value: u64) {
        if (var_id as usize) < MAX_VARIABLES {
            self.values[var_id as usize] = value;
            self.var_count = self.var_count.max(var_id + 1);
        }
    }
}

impl Default for GroupKey {
    fn default() -> Self {
        Self::new()
    }
}

/// Aggregation context
#[repr(C)]
pub struct AggregationContext {
    pub groups: [(GroupKey, AggregateAccumulator); 64], // Max 64 groups
    pub group_count: u8,
}

impl AggregationContext {
    pub fn new() -> Self {
        Self {
            groups: [(
                GroupKey::new(),
                AggregateAccumulator::new(AggregateFunction::Count),
            ); 64],
            group_count: 0,
        }
    }

    pub fn add_group(&mut self, key: GroupKey, func: AggregateFunction) -> Result<usize, String> {
        if self.group_count >= 64 {
            return Err("Group overflow".to_string());
        }
        
        // Check if group already exists
        for i in 0..self.group_count as usize {
            if self.groups[i].0 == key {
                return Ok(i);
            }
        }
        
        // Add new group
        let idx = self.group_count as usize;
        self.groups[idx] = (key, AggregateAccumulator::new(func));
        self.group_count += 1;
        Ok(idx)
    }

    pub fn find_or_create_group(&mut self, key: GroupKey, func: AggregateFunction) -> Result<usize, String> {
        // Try to find existing group
        for i in 0..self.group_count as usize {
            if self.groups[i].0 == key {
                return Ok(i);
            }
        }
        
        // Create new group
        self.add_group(key, func)
    }

    pub fn add_value_to_group(&mut self, group_idx: usize, value: u64) {
        self.groups[group_idx].1.add_value(value);
    }

    pub fn get_group_result(&self, group_idx: usize) -> Option<u64> {
        self.groups[group_idx].1.get_result()
    }
}

impl Default for AggregationContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_aggregate() {
        let mut agg = AggregateAccumulator::new(AggregateFunction::Count);
        agg.add_value(1);
        agg.add_value(2);
        agg.add_value(3);
        
        assert_eq!(agg.get_result(), Some(3));
    }

    #[test]
    fn test_sum_aggregate() {
        let mut agg = AggregateAccumulator::new(AggregateFunction::Sum);
        agg.add_value(1);
        agg.add_value(2);
        agg.add_value(3);
        
        assert_eq!(agg.get_result(), Some(6));
    }

    #[test]
    fn test_avg_aggregate() {
        let mut agg = AggregateAccumulator::new(AggregateFunction::Avg);
        agg.add_value(1);
        agg.add_value(2);
        agg.add_value(3);
        
        assert_eq!(agg.get_result(), Some(2));
    }

    #[test]
    fn test_min_aggregate() {
        let mut agg = AggregateAccumulator::new(AggregateFunction::Min);
        agg.add_value(5);
        agg.add_value(2);
        agg.add_value(8);
        
        assert_eq!(agg.get_result(), Some(2));
    }

    #[test]
    fn test_max_aggregate() {
        let mut agg = AggregateAccumulator::new(AggregateFunction::Max);
        agg.add_value(5);
        agg.add_value(2);
        agg.add_value(8);
        
        assert_eq!(agg.get_result(), Some(8));
    }

    #[test]
    fn test_group_key() {
        let mut key = GroupKey::new();
        key.set(0, 42);
        key.set(1, 43);
        
        assert_eq!(key.values[0], 42);
        assert_eq!(key.values[1], 43);
        assert_eq!(key.var_count, 2);
    }

    #[test]
    fn test_aggregation_context() {
        let mut ctx = AggregationContext::new();
        let mut key = GroupKey::new();
        key.set(0, 42);
        
        let idx = ctx.find_or_create_group(key, AggregateFunction::Count).unwrap();
        ctx.add_value_to_group(idx, 1);
        ctx.add_value_to_group(idx, 2);
        
        assert_eq!(ctx.get_group_result(idx), Some(2));
    }
}