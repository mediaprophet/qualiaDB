use super::*;
/// Explicit replay record: signed tick i64, channel u32, event count u64 (20 bytes).
/// This is a local view contract, not an allocated EMF/QAC wire record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventOrdering {
    AcquisitionOrder,
    NondecreasingTick,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    pub tick: i64,
    pub channel: u32,
    pub count: u64,
}
#[derive(Clone, Copy, Debug)]
pub struct EventView<'a> {
    region: Region<'a>,
    records: usize,
    channels: u32,
    order: ByteOrder,
    ordering: EventOrdering,
    clock: Clock,
    context: Context,
}
impl<'a> EventView<'a> {
    pub fn admit(
        region: Region<'a>,
        channels: u32,
        order: ByteOrder,
        ordering: EventOrdering,
        clock: Clock,
        context: Context,
        max_records: usize,
    ) -> Result<Self, Error> {
        if channels == 0 || region.bytes().len() % 20 != 0 {
            return Err(Error::Shape);
        }
        let records = region.bytes().len() / 20;
        if records > max_records {
            return Err(Error::Budget);
        }
        context.validate()?;
        let view = Self {
            region,
            records,
            channels,
            order,
            ordering,
            clock,
            context,
        };
        let mut previous = None;
        for i in 0..records {
            let e = view.event(i)?;
            if e.channel >= channels {
                return Err(Error::Bounds);
            }
            clock.tick_time(e.tick)?;
            if ordering == EventOrdering::NondecreasingTick && previous.is_some_and(|x| e.tick < x)
            {
                return Err(Error::Ordering);
            }
            previous = Some(e.tick);
        }
        Ok(view)
    }
    pub fn event(self, index: usize) -> Result<Event, Error> {
        if index >= self.records {
            return Err(Error::Bounds);
        }
        let start = index.checked_mul(20).ok_or(Error::Overflow)?;
        let b = &self.region.bytes()[start..start + 20];
        Ok(if self.order == ByteOrder::Little {
            Event {
                tick: i64::from_le_bytes(b[..8].try_into().unwrap()),
                channel: u32::from_le_bytes(b[8..12].try_into().unwrap()),
                count: u64::from_le_bytes(b[12..20].try_into().unwrap()),
            }
        } else {
            Event {
                tick: i64::from_be_bytes(b[..8].try_into().unwrap()),
                channel: u32::from_be_bytes(b[8..12].try_into().unwrap()),
                count: u64::from_be_bytes(b[12..20].try_into().unwrap()),
            }
        })
    }
    pub fn time_at(self, index: usize) -> Result<ExactTime, Error> {
        self.clock.tick_time(self.event(index)?.tick)
    }
    pub fn region(self) -> Region<'a> {
        self.region
    }
    pub fn records(self) -> usize {
        self.records
    }
    pub fn channels(self) -> u32 {
        self.channels
    }
    pub fn byte_order(self) -> ByteOrder {
        self.order
    }
    pub fn ordering(self) -> EventOrdering {
        self.ordering
    }
    pub fn context(self) -> Context {
        self.context
    }
    pub fn clock(self) -> Clock {
        self.clock
    }
}
