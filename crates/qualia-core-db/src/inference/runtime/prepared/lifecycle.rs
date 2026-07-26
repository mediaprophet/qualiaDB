#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparedPlanState {
    Unbuilt = 0,
    Ready = 1,
    Ineligible = 2,
    Failed = 3,
}
