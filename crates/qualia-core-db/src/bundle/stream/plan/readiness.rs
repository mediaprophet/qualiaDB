//! Readiness of a selected entry against declared mandatory closure.
//! Missing mandatory is not an empty field, silence, or a complete query answer.
use super::inventory::{EntryFamily, Inventory, Obligation, MAX_PLAN_ENTRIES};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Readiness {
    Ready,
    Incomplete { missing_mandatory: u32 },
    Opaque,
    Refused,
}
pub fn evaluate(
    inventory: &Inventory<'_>,
    entry_id: u32,
    expected_family: Option<EntryFamily>,
    present: &[u32],
) -> Readiness {
    let Some(root) = inventory.entry(entry_id) else {
        return Readiness::Refused;
    };
    if let Some(family) = expected_family {
        if family != root.family() {
            return Readiness::Refused;
        }
    }
    if root.family() == EntryFamily::Opaque {
        return match root.obligation() {
            Obligation::Optional => Readiness::Opaque,
            Obligation::Mandatory => Readiness::Refused,
        };
    }
    let mut stack = [0u32; MAX_PLAN_ENTRIES];
    let mut sp = 0usize;
    let mut seen = [false; MAX_PLAN_ENTRIES];
    let mut missing: Option<u32> = None;
    match push(inventory, &mut stack, &mut sp, &mut seen, entry_id) {
        Push::Ok => {}
        Push::Unknown(id) => {
            return Readiness::Incomplete {
                missing_mandatory: id,
            }
        }
        Push::Full => return Readiness::Refused,
    }
    while sp > 0 {
        sp -= 1;
        let id = stack[sp];
        let Some(entry) = inventory.entry(id) else {
            return Readiness::Refused;
        };
        if entry.family() == EntryFamily::Opaque {
            return Readiness::Refused;
        }
        for edge in inventory.edges() {
            if edge.from_id() != id || edge.obligation() != Obligation::Mandatory {
                continue;
            }
            let dep = edge.to_id();
            match inventory.entry(dep) {
                None => {
                    return Readiness::Incomplete {
                        missing_mandatory: dep,
                    }
                }
                Some(dep_entry) if dep_entry.family() == EntryFamily::Opaque => {
                    return Readiness::Refused;
                }
                Some(_) if !contains(present, dep) => {
                    missing = Some(min_id(missing, dep));
                }
                Some(_) => match push(inventory, &mut stack, &mut sp, &mut seen, dep) {
                    Push::Ok => {}
                    Push::Unknown(id) => {
                        return Readiness::Incomplete {
                            missing_mandatory: id,
                        };
                    }
                    Push::Full => return Readiness::Refused,
                },
            }
        }
    }
    if let Some(missing_mandatory) = missing {
        return Readiness::Incomplete { missing_mandatory };
    }
    if !contains(present, entry_id) {
        return match root.obligation() {
            Obligation::Mandatory => Readiness::Incomplete {
                missing_mandatory: entry_id,
            },
            Obligation::Optional => Readiness::Opaque,
        };
    }
    Readiness::Ready
}
enum Push {
    Ok,
    Unknown(u32),
    Full,
}
fn push(
    inventory: &Inventory<'_>,
    stack: &mut [u32; MAX_PLAN_ENTRIES],
    sp: &mut usize,
    seen: &mut [bool; MAX_PLAN_ENTRIES],
    id: u32,
) -> Push {
    let Some(idx) = inventory.index_of(id) else {
        return Push::Unknown(id);
    };
    if seen[idx] {
        return Push::Ok;
    }
    if *sp >= MAX_PLAN_ENTRIES {
        return Push::Full;
    }
    seen[idx] = true;
    stack[*sp] = id;
    *sp += 1;
    Push::Ok
}
fn contains(ids: &[u32], id: u32) -> bool {
    ids.iter().any(|x| *x == id)
}
fn min_id(current: Option<u32>, id: u32) -> u32 {
    match current {
        Some(c) if c < id => c,
        _ => id,
    }
}
