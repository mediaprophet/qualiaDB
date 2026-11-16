use super::*;

#[test]
fn live_capacities_and_alignment_are_admitted_together() {
    #[repr(align(64))]
    struct Wide([u8; 64]);
    let mut plan = CapacityPlan::default();
    plan.add_slice::<f64>(BufferClass::Input, 3).unwrap();
    plan.add_slice::<u32>(BufferClass::Output, 10).unwrap();
    plan.add_arena_slice::<Wide>(1).unwrap();
    assert_eq!(plan.total_bytes(), Ok(24 + 40 + 64 + 63));
    let cancel = Cancellation::new();
    let bytes = plan.total_bytes().unwrap();
    assert!(ProductionBudget::admit(plan, bytes, 0, &cancel).is_ok());
    assert!(matches!(ProductionBudget::admit(plan, bytes - 1, 0, &cancel),
        Err(BudgetError::ByteLimitExceeded { required, limit }) if required == bytes && limit == bytes - 1));
}

#[test]
fn zero_one_maximum_and_overflow_are_fail_closed() {
    let cancel = Cancellation::new();
    let mut zero = ProductionBudget::admit(CapacityPlan::default(), 0, 0, &cancel).unwrap();
    assert_eq!(zero.checkpoint(0), Ok(()));
    assert_eq!(zero.checkpoint(1), Err(BudgetError::WorkLimitExceeded { requested: 1, remaining: 0 }));
    let mut plan = CapacityPlan::default();
    plan.add_bytes(BufferClass::Input, usize::MAX).unwrap();
    let original = plan;
    assert_eq!(plan.add_bytes(BufferClass::Scratch, 1), Err(BudgetError::ArithmeticOverflow));
    assert_eq!(plan, original);
    assert_eq!(plan.add_slice::<u64>(BufferClass::Output, usize::MAX), Err(BudgetError::ArithmeticOverflow));
    assert_eq!(plan, original);
    let mut maximum = ProductionBudget::admit(plan, usize::MAX, u64::MAX, &cancel).unwrap();
    maximum.checkpoint(u64::MAX).unwrap();
    assert_eq!(maximum.work_used(), u64::MAX);
    assert_eq!(maximum.checkpoint(1), Err(BudgetError::WorkLimitExceeded { requested: 1, remaining: 0 }));
    assert_eq!(maximum.work_used(), u64::MAX);
}

#[test]
fn shared_cancellation_interrupts_without_consuming_work() {
    let cancel = Cancellation::new();
    let mut budget = ProductionBudget::admit_default(CapacityPlan::default(), 2, &cancel).unwrap();
    budget.checkpoint(1).unwrap();
    cancel.cancel();
    assert_eq!(budget.checkpoint(1), Err(BudgetError::Cancelled));
    assert_eq!(budget.work_used(), 1);
    assert!(matches!(ProductionBudget::admit_default(CapacityPlan::default(), 1, &cancel), Err(BudgetError::Cancelled)));
}
