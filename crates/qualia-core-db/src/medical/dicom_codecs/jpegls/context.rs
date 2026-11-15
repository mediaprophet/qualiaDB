//! ITU-T T.87 LOCO-I context modeling (365 regular + 2 run-interruption contexts),
//! MED predictor, run mode, ILV=0/1/2 scans and near-lossless error reconstruction.
use super::bitstream::LsBitReader;
use super::headers::{DerivedCoding, JpegLsError};

const J_TABLE: [usize; 32] = [
    0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3,
    4, 4, 5, 5, 6, 6, 7, 7, 8, 9, 10, 11, 12, 13, 14, 15,
];

pub(super) struct ContextModel {
    pub coding: DerivedCoding,
    pub a: [i32; 367],
    pub b: [i32; 365],
    pub c: [i32; 365],
    pub n: [i32; 367],
    pub nn: [i32; 2],
}

impl ContextModel {
    pub fn new(coding: DerivedCoding) -> Self {
        let a_init = ((coding.range + 32) / 64).max(2);
        Self {
            coding,
            a: [a_init; 367],
            b: [0; 365],
            c: [0; 365],
            n: [1; 367],
            nn: [0; 2],
        }
    }

    pub fn reset_state(&mut self) {
        let a_init = ((self.coding.range + 32) / 64).max(2);
        self.a.fill(a_init);
        self.b.fill(0);
        self.c.fill(0);
        self.n.fill(1);
        self.nn.fill(0);
    }

    #[inline]
    fn quantize_grad(&self, d: i32) -> i32 {
        let c = &self.coding;
        if d <= -c.t3 { -4 }
        else if d <= -c.t2 { -3 }
        else if d <= -c.t1 { -2 }
        else if d < -c.near { -1 }
        else if d <= c.near { 0 }
        else if d < c.t1 { 1 }
        else if d < c.t2 { 2 }
        else if d < c.t3 { 3 }
        else { 4 }
    }

    #[inline]
    fn med(ra: i32, rb: i32, rc: i32) -> i32 {
        if rc >= ra.max(rb) {
            ra.min(rb)
        } else if rc <= ra.min(rb) {
            ra.max(rb)
        } else {
            ra + rb - rc
        }
    }

    #[inline]
    fn context_index(&self, d1: i32, d2: i32, d3: i32) -> (usize, i32) {
        let mut q1 = self.quantize_grad(d1);
        let mut q2 = self.quantize_grad(d2);
        let mut q3 = self.quantize_grad(d3);
        let sign = if q1 < 0 || (q1 == 0 && q2 < 0) || (q1 == 0 && q2 == 0 && q3 < 0) {
            q1 = -q1;
            q2 = -q2;
            q3 = -q3;
            -1
        } else {
            1
        };
        let q = ((q1 * 9 + q2) * 9 + q3) as usize;
        (q, sign)
    }

    #[inline]
    fn decode_regular(
        &mut self,
        reader: &mut LsBitReader<'_>,
        ra: i32,
        rb: i32,
        rc: i32,
        d1: i32,
        d2: i32,
        d3: i32,
    ) -> Result<i32, JpegLsError> {
        let (q, sign) = self.context_index(d1, d2, d3);
        let mut px = Self::med(ra, rb, rc) + sign * self.c[q];
        px = px.clamp(0, self.coding.maxval);

        let mut k = 0u8;
        while (self.n[q] << k) < self.a[q] {
            k += 1;
            if k >= 31 {
                return Err(JpegLsError::CorruptStream("Golomb parameter k overflow"));
            }
        }
        let glimit = self.coding.limit.saturating_sub(self.coding.qbpp + 1);
        let merr = reader.read_limited_golomb(k, glimit, self.coding.qbpp)?;
        let special = self.coding.near == 0 && k == 0 && (2 * self.b[q] <= -self.n[q]);
        let errval = if (merr & 1) == 0 {
            if special { -(merr >> 1) - 1 } else { merr >> 1 }
        } else if special {
            (merr - 1) >> 1
        } else {
            -((merr + 1) >> 1)
        };

        let near = self.coding.near;
        let mut unq = if near > 0 { errval * (2 * near + 1) } else { errval };
        if sign < 0 {
            unq = -unq;
        }
        let mut rx = px + unq;
        let modulus = self.coding.range * (2 * near + 1);
        if rx < -near {
            rx += modulus;
        } else if rx > self.coding.maxval + near {
            rx -= modulus;
        }
        let rx = rx.clamp(0, self.coding.maxval);

        self.b[q] += errval * (2 * near + 1);
        self.a[q] += errval.abs();
        if self.n[q] == self.coding.reset {
            self.a[q] >>= 1;
            self.b[q] >>= 1;
            self.n[q] >>= 1;
        }
        self.n[q] += 1;
        if self.b[q] <= -self.n[q] {
            self.b[q] += self.n[q];
            if self.c[q] > -128 {
                self.c[q] -= 1;
            }
            if self.b[q] <= -self.n[q] {
                self.b[q] = -self.n[q] + 1;
            }
        } else if self.b[q] > 0 {
            self.b[q] -= self.n[q];
            if self.c[q] < 127 {
                self.c[q] += 1;
            }
            if self.b[q] > 0 {
                self.b[q] = 0;
            }
        }
        Ok(rx)
    }

    #[inline]
    fn decode_run_interruption_sample(
        &mut self,
        reader: &mut LsBitReader<'_>,
        ra: i32,
        rb: i32,
        prev_run_index: usize,
    ) -> Result<i32, JpegLsError> {
        let near = self.coding.near;
        let ri_type: usize = if (ra - rb).abs() <= near { 1 } else { 0 };
        let px = if ri_type == 1 { ra } else { rb };
        let ctx = 365 + ri_type;
        let temp = self.a[ctx] + if ri_type == 1 { self.n[ctx] >> 1 } else { 0 };
        let mut k = 0u8;
        while (self.n[ctx] << k) < temp {
            k += 1;
            if k >= 31 {
                return Err(JpegLsError::CorruptStream("run Golomb k overflow"));
            }
        }
        let glimit = self
            .coding
            .limit
            .saturating_sub(J_TABLE[prev_run_index] as u8 + 1);
        let em_errval = reader.read_limited_golomb(k, glimit, self.coding.qbpp)?;
        let t = em_errval + ri_type as i32;
        let map = (t & 1) != 0;
        let mag = (em_errval + 1 + ri_type as i32) >> 1;
        let cond = k == 0 && (2 * self.nn[ri_type] < self.n[ctx]);
        let errval = if map == cond { mag } else { -mag };

        if errval < 0 {
            self.nn[ri_type] += 1;
        }
        self.a[ctx] += (em_errval + 1 - ri_type as i32) >> 1;
        if self.n[ctx] == self.coding.reset {
            self.a[ctx] >>= 1;
            self.n[ctx] >>= 1;
            self.nn[ri_type] >>= 1;
        }
        self.n[ctx] += 1;

        let sign = if ri_type == 0 && ra > rb { -1 } else { 1 };
        let mut unq = if near > 0 { errval * (2 * near + 1) } else { errval };
        if sign < 0 {
            unq = -unq;
        }
        let mut rx = px + unq;
        let modulus = self.coding.range * (2 * near + 1);
        if rx < -near {
            rx += modulus;
        } else if rx > self.coding.maxval + near {
            rx -= modulus;
        }
        Ok(rx.clamp(0, self.coding.maxval))
    }

    pub fn decode_line(
        &mut self,
        reader: &mut LsBitReader<'_>,
        prev: &mut [i32],
        curr: &mut [i32],
        width: usize,
        run_index: &mut usize,
    ) -> Result<(), JpegLsError> {
        curr[0] = prev[1];
        prev[width + 1] = prev[width];
        let near = self.coding.near;
        let mut col = 0usize;
        while col < width {
            let x = col + 1;
            let ra = curr[x - 1];
            let rb = prev[x];
            let rc = prev[x - 1];
            let rd = prev[x + 1];
            let (d1, d2, d3) = (rd - rb, rb - rc, rc - ra);
            if d1.abs() <= near && d2.abs() <= near && d3.abs() <= near {
                let mut eol = false;
                while col < width {
                    let bit = reader.read_bit()?;
                    if bit == 1 {
                        let rg = 1usize << J_TABLE[*run_index];
                        let rem = width - col;
                        if rem > rg {
                            curr[col + 1..col + 1 + rg].fill(ra);
                            col += rg;
                            if *run_index < 31 {
                                *run_index += 1;
                            }
                        } else {
                            curr[col + 1..=width].fill(ra);
                            col = width;
                            if rem == rg && *run_index < 31 {
                                *run_index += 1;
                            }
                            eol = true;
                            break;
                        }
                    } else {
                        let bits_j = J_TABLE[*run_index] as u8;
                        let run_rem = reader.read_bits(bits_j)? as usize;
                        if run_rem >= width - col {
                            return Err(JpegLsError::CorruptStream("JPEG-LS run exceeds row width"));
                        }
                        if run_rem > 0 {
                            curr[col + 1..col + 1 + run_rem].fill(ra);
                            col += run_rem;
                        }
                        let prev_ri = *run_index;
                        if *run_index > 0 {
                            *run_index -= 1;
                        }
                        let x_int = col + 1;
                        let rx = self.decode_run_interruption_sample(
                            reader,
                            curr[x_int - 1],
                            prev[x_int],
                            prev_ri,
                        )?;
                        curr[x_int] = rx;
                        col += 1;
                        break;
                    }
                }
                if eol {
                    break;
                }
            } else {
                curr[x] = self.decode_regular(reader, ra, rb, rc, d1, d2, d3)?;
                col += 1;
            }
        }
        Ok(())
    }

    pub fn decode_sample_interleaved_line(
        &mut self,
        reader: &mut LsBitReader<'_>,
        prev: &mut [Vec<i32>],
        curr: &mut [Vec<i32>],
        width: usize,
        run_index: &mut usize,
    ) -> Result<(), JpegLsError> {
        let nc = curr.len();
        for c in 0..nc {
            curr[c][0] = prev[c][1];
            prev[c][width + 1] = prev[c][width];
        }
        let near = self.coding.near;
        let mut col = 0usize;
        while col < width {
            let x = col + 1;
            let mut all_flat = true;
            for c in 0..nc {
                let (ra, rb, rc, rd) = (curr[c][x - 1], prev[c][x], prev[c][x - 1], prev[c][x + 1]);
                if (rd - rb).abs() > near || (rb - rc).abs() > near || (rc - ra).abs() > near {
                    all_flat = false;
                    break;
                }
            }
            if all_flat {
                let mut eol = false;
                while col < width {
                    let bit = reader.read_bit()?;
                    if bit == 1 {
                        let rg = 1usize << J_TABLE[*run_index];
                        let rem = width - col;
                        if rem > rg {
                            for c in 0..nc {
                                let ra = curr[c][col];
                                curr[c][col + 1..col + 1 + rg].fill(ra);
                            }
                            col += rg;
                            if *run_index < 31 {
                                *run_index += 1;
                            }
                        } else {
                            for c in 0..nc {
                                let ra = curr[c][col];
                                curr[c][col + 1..=width].fill(ra);
                            }
                            col = width;
                            if rem == rg && *run_index < 31 {
                                *run_index += 1;
                            }
                            eol = true;
                            break;
                        }
                    } else {
                        let run_rem = reader.read_bits(J_TABLE[*run_index] as u8)? as usize;
                        if run_rem >= width - col {
                            return Err(JpegLsError::CorruptStream("JPEG-LS ILV=2 run exceeds row"));
                        }
                        if run_rem > 0 {
                            for c in 0..nc {
                                let ra = curr[c][col];
                                curr[c][col + 1..col + 1 + run_rem].fill(ra);
                            }
                            col += run_rem;
                        }
                        let prev_ri = *run_index;
                        if *run_index > 0 {
                            *run_index -= 1;
                        }
                        let x_int = col + 1;
                        for c in 0..nc {
                            let ra = curr[c][x_int - 1];
                            let rb = prev[c][x_int];
                            curr[c][x_int] =
                                self.decode_run_interruption_sample(reader, ra, rb, prev_ri)?;
                        }
                        col += 1;
                        break;
                    }
                }
                if eol {
                    break;
                }
            } else {
                for c in 0..nc {
                    let (ra, rb, rc, rd) =
                        (curr[c][x - 1], prev[c][x], prev[c][x - 1], prev[c][x + 1]);
                    curr[c][x] =
                        self.decode_regular(reader, ra, rb, rc, rd - rb, rb - rc, rc - ra)?;
                }
                col += 1;
            }
        }
        Ok(())
    }
}
