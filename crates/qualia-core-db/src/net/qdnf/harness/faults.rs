//! Loss, reorder, duplicate and corruption injection.

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    None = 0,
    Drop = 1,
    Duplicate = 2,
    Reorder = 3,
    Corrupt = 4,
    Truncate = 5,
}

#[derive(Clone, Copy, Debug)]
pub struct FaultSchedule {
    pub pattern: [Fault; 8],
    pub index: usize,
}

impl FaultSchedule {
    pub const fn never() -> Self {
        Self {
            pattern: [Fault::None; 8],
            index: 0,
        }
    }

    pub fn next(&mut self) -> Fault {
        let f = self.pattern[self.index % self.pattern.len()];
        self.index = self.index.wrapping_add(1);
        f
    }

    pub fn apply<'a>(&self, fault: Fault, frame: &'a mut [u8]) -> Option<&'a [u8]> {
        match fault {
            Fault::None => Some(frame),
            Fault::Drop => None,
            Fault::Duplicate => Some(frame),
            Fault::Reorder => Some(frame),
            Fault::Corrupt => {
                if !frame.is_empty() {
                    frame[0] ^= 0xff;
                }
                Some(frame)
            }
            Fault::Truncate => {
                if frame.len() > 4 {
                    Some(&frame[..frame.len() / 2])
                } else {
                    None
                }
            }
        }
    }
}
