//! ISO/IEC 15444-1 Annex C MQ arithmetic decoder with Table C.2 state machine.

#[derive(Clone, Copy)]
struct MqEntry {
    qe: u32,
    nmps: u8,
    nlps: u8,
    switch: bool,
}

const QE_TABLE: [MqEntry; 47] = [
    MqEntry { qe: 0x5601, nmps: 1, nlps: 1, switch: true },
    MqEntry { qe: 0x3401, nmps: 2, nlps: 6, switch: false },
    MqEntry { qe: 0x1801, nmps: 3, nlps: 9, switch: false },
    MqEntry { qe: 0x0AC1, nmps: 4, nlps: 12, switch: false },
    MqEntry { qe: 0x0521, nmps: 5, nlps: 29, switch: false },
    MqEntry { qe: 0x0221, nmps: 38, nlps: 33, switch: false },
    MqEntry { qe: 0x5601, nmps: 7, nlps: 6, switch: true },
    MqEntry { qe: 0x5401, nmps: 8, nlps: 14, switch: false },
    MqEntry { qe: 0x4801, nmps: 9, nlps: 14, switch: false },
    MqEntry { qe: 0x3801, nmps: 10, nlps: 14, switch: false },
    MqEntry { qe: 0x3001, nmps: 11, nlps: 17, switch: false },
    MqEntry { qe: 0x2401, nmps: 12, nlps: 18, switch: false },
    MqEntry { qe: 0x1C01, nmps: 13, nlps: 20, switch: false },
    MqEntry { qe: 0x1601, nmps: 29, nlps: 21, switch: false },
    MqEntry { qe: 0x5601, nmps: 15, nlps: 14, switch: true },
    MqEntry { qe: 0x5401, nmps: 16, nlps: 14, switch: false },
    MqEntry { qe: 0x5101, nmps: 17, nlps: 15, switch: false },
    MqEntry { qe: 0x4801, nmps: 18, nlps: 16, switch: false },
    MqEntry { qe: 0x3801, nmps: 19, nlps: 17, switch: false },
    MqEntry { qe: 0x3401, nmps: 20, nlps: 18, switch: false },
    MqEntry { qe: 0x3001, nmps: 21, nlps: 19, switch: false },
    MqEntry { qe: 0x2801, nmps: 22, nlps: 19, switch: false },
    MqEntry { qe: 0x2401, nmps: 23, nlps: 20, switch: false },
    MqEntry { qe: 0x2201, nmps: 24, nlps: 21, switch: false },
    MqEntry { qe: 0x1C01, nmps: 25, nlps: 22, switch: false },
    MqEntry { qe: 0x1801, nmps: 26, nlps: 23, switch: false },
    MqEntry { qe: 0x1601, nmps: 27, nlps: 24, switch: false },
    MqEntry { qe: 0x1401, nmps: 28, nlps: 25, switch: false },
    MqEntry { qe: 0x1201, nmps: 29, nlps: 26, switch: false },
    MqEntry { qe: 0x1101, nmps: 30, nlps: 27, switch: false },
    MqEntry { qe: 0x0AC1, nmps: 31, nlps: 28, switch: false },
    MqEntry { qe: 0x09C1, nmps: 32, nlps: 29, switch: false },
    MqEntry { qe: 0x08A1, nmps: 33, nlps: 30, switch: false },
    MqEntry { qe: 0x0521, nmps: 34, nlps: 31, switch: false },
    MqEntry { qe: 0x0441, nmps: 35, nlps: 32, switch: false },
    MqEntry { qe: 0x02A1, nmps: 36, nlps: 33, switch: false },
    MqEntry { qe: 0x0221, nmps: 37, nlps: 34, switch: false },
    MqEntry { qe: 0x0141, nmps: 38, nlps: 35, switch: false },
    MqEntry { qe: 0x0111, nmps: 39, nlps: 36, switch: false },
    MqEntry { qe: 0x0085, nmps: 40, nlps: 37, switch: false },
    MqEntry { qe: 0x0049, nmps: 41, nlps: 38, switch: false },
    MqEntry { qe: 0x0025, nmps: 42, nlps: 39, switch: false },
    MqEntry { qe: 0x0015, nmps: 43, nlps: 40, switch: false },
    MqEntry { qe: 0x0009, nmps: 44, nlps: 41, switch: false },
    MqEntry { qe: 0x0005, nmps: 45, nlps: 42, switch: false },
    MqEntry { qe: 0x0001, nmps: 45, nlps: 43, switch: false },
    MqEntry { qe: 0x5601, nmps: 46, nlps: 46, switch: false },
];

pub struct MqDecoder<'a> {
    data: &'a [u8],
    pos: usize,
    b: u8,
    a: u32,
    c: u32,
    ct: u8,
    pub state: [u8; 19],
    pub mps: [u8; 19],
}

impl<'a> MqDecoder<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        let mut dec = Self {
            data,
            pos: 0,
            b: 0,
            a: 0x8000,
            c: 0,
            ct: 0,
            state: [0; 19],
            mps: [0; 19],
        };
        dec.reset_contexts();
        dec.init_dec();
        dec
    }

    pub fn reset_contexts(&mut self) {
        self.state.fill(0);
        self.mps.fill(0);
        self.state[0] = 4;
        self.state[17] = 3;
        self.state[18] = 46;
    }

    pub fn reinit_stream(&mut self, data: &'a [u8]) {
        self.data = data;
        self.pos = 0;
        self.init_dec();
    }

    fn init_dec(&mut self) {
        self.b = self.data.first().copied().unwrap_or(0xFF);
        self.pos = usize::from(!self.data.is_empty());
        self.c = (self.b as u32) << 16;
        self.byte_in();
        self.c <<= 7;
        self.ct = self.ct.saturating_sub(7);
        self.a = 0x8000;
    }

    #[inline]
    fn byte_in(&mut self) {
        if self.b == 0xFF {
            let b1 = self.data.get(self.pos).copied().unwrap_or(0xFF);
            if b1 > 0x8F {
                self.c = self.c.wrapping_add(0xFF00);
                self.ct = 8;
            } else {
                self.pos += usize::from(self.pos < self.data.len());
                self.b = b1;
                self.c = self.c.wrapping_add((b1 as u32) << 9);
                self.ct = 7;
            }
        } else {
            self.b = self.data.get(self.pos).copied().unwrap_or(0xFF);
            self.pos += usize::from(self.pos < self.data.len());
            self.c = self.c.wrapping_add((self.b as u32) << 8);
            self.ct = 8;
        }
    }

    #[inline]
    fn renormd(&mut self) {
        loop {
            if self.ct == 0 {
                self.byte_in();
            }
            self.a <<= 1;
            self.c <<= 1;
            self.ct -= 1;
            if (self.a & 0x8000) != 0 {
                break;
            }
        }
    }

    #[inline]
    pub fn decode_bit(&mut self, cx: usize) -> u8 {
        let entry = QE_TABLE[self.state[cx] as usize];
        self.a -= entry.qe;
        let d;
        if (self.c >> 16) < entry.qe {
            if self.a < entry.qe {
                d = self.mps[cx];
                self.state[cx] = entry.nmps;
            } else {
                d = 1 - self.mps[cx];
                if entry.switch {
                    self.mps[cx] ^= 1;
                }
                self.state[cx] = entry.nlps;
            }
            self.a = entry.qe;
            self.renormd();
        } else {
            self.c -= entry.qe << 16;
            if (self.a & 0x8000) == 0 {
                if self.a < entry.qe {
                    d = 1 - self.mps[cx];
                    if entry.switch {
                        self.mps[cx] ^= 1;
                    }
                    self.state[cx] = entry.nlps;
                } else {
                    d = self.mps[cx];
                    self.state[cx] = entry.nmps;
                }
                self.renormd();
            } else {
                d = self.mps[cx];
            }
        }
        d
    }
}
