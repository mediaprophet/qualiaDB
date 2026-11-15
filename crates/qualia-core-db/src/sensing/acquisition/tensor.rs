use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ByteOrder {
    Little,
    Big,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    /// Existing scalar semantics, extended without allocating new wire IDs.
    Shared(Scalar),
    U16,
    U32,
    U64,
    ComplexI16,
    ComplexF64,
    /// Unknown registered encoding remains byte-readable; numeric access is unsupported.
    Opaque {
        profile: crate::sensing::SemanticId,
        element_bytes: u32,
    },
}
impl Encoding {
    pub fn bytes(self) -> usize {
        match self {
            Self::Shared(s) => s.bytes() as usize,
            Self::U16 => 2,
            Self::U32 => 4,
            Self::U64 => 8,
            Self::Opaque { element_bytes, .. } => element_bytes as usize,
            Self::ComplexI16 => 4,
            Self::ComplexF64 => 16,
        }
    }
    pub fn complex(self) -> bool {
        matches!(
            self,
            Self::Shared(Scalar::ComplexF32) | Self::ComplexI16 | Self::ComplexF64
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComponentOrder {
    Scalar,
    RealImag,
    ImagReal,
}
/// Axis meanings are explicit. No implicit image row/column interpretation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    Sample,
    Channel,
    Sensor,
    Lag,
    LeftChannel,
    RightChannel,
    Registered(crate::sensing::SemanticId),
}
#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub rank: u8,
    pub extents: [u32; 4],
    pub strides: [usize; 4],
    pub axes: [Axis; 4],
    pub encoding: Encoding,
    pub byte_order: ByteOrder,
    pub components: ComponentOrder,
}
impl Layout {
    pub fn span(self) -> Result<usize, Error> {
        let rank = usize::from(self.rank);
        if let Encoding::Opaque {
            profile,
            element_bytes,
        } = self.encoding
        {
            profile.value().ok_or(Error::Identity)?;
            if element_bytes == 0 {
                return Err(Error::Encoding);
            }
        }
        if !(1..=4).contains(&rank)
            || self.extents[..rank].contains(&0)
            || self.extents[rank..].iter().any(|&x| x != 0)
            || self.strides[rank..].iter().any(|&x| x != 0)
            || (self.encoding.complex() == (self.components == ComponentOrder::Scalar))
        {
            return Err(Error::Shape);
        }
        for i in 0..rank {
            if let Axis::Registered(id) = self.axes[i] {
                id.value().ok_or(Error::Identity)?;
            }
            if self.axes[..i].contains(&self.axes[i]) {
                return Err(Error::Shape);
            }
        }
        let mut span = self.encoding.bytes();
        let mut axes = [(0usize, 0u32); 4];
        let mut n = 0;
        for i in 0..rank {
            if self.extents[i] > 1 {
                axes[n] = (self.strides[i], self.extents[i]);
                n += 1;
            }
        }
        axes[..n].sort_unstable_by_key(|x| x.0);
        for &(stride, extent) in &axes[..n] {
            if stride < span {
                return Err(Error::Shape);
            }
            span = span
                .checked_add(
                    stride
                        .checked_mul((extent - 1) as usize)
                        .ok_or(Error::Overflow)?,
                )
                .ok_or(Error::Overflow)?;
        }
        Ok(span)
    }
}
/// All bytes including NaN payloads remain available; scientific access rejects nonfinite lanes.
#[derive(Clone, Copy, Debug)]
pub struct TensorView<'a> {
    region: Region<'a>,
    layout: Layout,
    context: Context,
    clock: Option<Clock>,
    axes: SignalAxes,
}
impl<'a> TensorView<'a> {
    pub fn admit(
        region: Region<'a>,
        layout: Layout,
        context: Context,
        clock: Option<Clock>,
        axes: SignalAxes,
    ) -> Result<Self, Error> {
        if layout.span()? != region.bytes().len() {
            return Err(Error::Bounds);
        }
        context.validate()?;
        axes.validate()?;
        if !layout.encoding.complex()
            && (axes.complex_convention.is_some() || axes.phase_origin.is_some())
        {
            return Err(Error::Semantics);
        }
        if let Some(clock) = clock {
            let i = layout.axes[..usize::from(layout.rank)]
                .iter()
                .position(|x| *x == Axis::Sample)
                .ok_or(Error::Clock)?;
            clock.time_at(u64::from(layout.extents[i] - 1))?;
        }
        Ok(Self {
            region,
            layout,
            context,
            clock,
            axes,
        })
    }
    pub fn region(self) -> Region<'a> {
        self.region
    }
    pub fn layout(self) -> Layout {
        self.layout
    }
    pub fn context(self) -> Context {
        self.context
    }
    pub fn clock(self) -> Option<Clock> {
        self.clock
    }
    pub fn signal_axes(self) -> SignalAxes {
        self.axes
    }
    pub fn element(self, index: [u32; 4]) -> Result<&'a [u8], Error> {
        let mut offset = 0usize;
        for (i, &value) in index.iter().enumerate() {
            if i >= usize::from(self.layout.rank) {
                if value != 0 {
                    return Err(Error::Bounds);
                }
                continue;
            }
            if value >= self.layout.extents[i] {
                return Err(Error::Bounds);
            }
            offset = offset
                .checked_add(
                    (value as usize)
                        .checked_mul(self.layout.strides[i])
                        .ok_or(Error::Overflow)?,
                )
                .ok_or(Error::Overflow)?;
        }
        self.region
            .bytes()
            .get(
                offset
                    ..offset
                        .checked_add(self.layout.encoding.bytes())
                        .ok_or(Error::Overflow)?,
            )
            .ok_or(Error::Bounds)
    }
    /// No calibration is applied. Known quantity/unit and finite lanes are required.
    pub fn numeric(self, index: [u32; 4]) -> Result<[f64; 2], Error> {
        self.context.quantity.require_known()?;
        let b = self.element(index)?;
        let big = self.layout.byte_order == ByteOrder::Big;
        let u16v = |b: &[u8]| {
            if big {
                u16::from_be_bytes(b.try_into().unwrap())
            } else {
                u16::from_le_bytes(b.try_into().unwrap())
            }
        };
        let u32v = |b: &[u8]| {
            if big {
                u32::from_be_bytes(b.try_into().unwrap())
            } else {
                u32::from_le_bytes(b.try_into().unwrap())
            }
        };
        let u64v = |b: &[u8]| {
            if big {
                u64::from_be_bytes(b.try_into().unwrap())
            } else {
                u64::from_le_bytes(b.try_into().unwrap())
            }
        };
        let mut value = match self.layout.encoding {
            Encoding::U16 => [u16v(b) as f64, 0.0],
            Encoding::Opaque { .. } => return Err(Error::Unsupported),
            Encoding::U32 => [u32v(b) as f64, 0.0],
            Encoding::U64 => {
                let n = u64v(b);
                if n > 9_007_199_254_740_992 {
                    return Err(Error::Unsupported);
                }
                [n as f64, 0.0]
            }
            Encoding::ComplexI16 => [u16v(&b[..2]) as i16 as f64, u16v(&b[2..]) as i16 as f64],
            Encoding::ComplexF64 => [f64::from_bits(u64v(&b[..8])), f64::from_bits(u64v(&b[8..]))],
            Encoding::Shared(s) => match s {
                Scalar::U8 => [b[0] as f64, 0.0],
                Scalar::I16 => [u16v(b) as i16 as f64, 0.0],
                Scalar::I32 => [u32v(b) as i32 as f64, 0.0],
                Scalar::I24 => {
                    let n = if big {
                        ((b[0] as i32) << 16) | ((b[1] as i32) << 8) | b[2] as i32
                    } else {
                        ((b[2] as i32) << 16) | ((b[1] as i32) << 8) | b[0] as i32
                    };
                    [(n << 8 >> 8) as f64, 0.0]
                }
                Scalar::F32 => [f32::from_bits(u32v(b)) as f64, 0.0],
                Scalar::F64 => [f64::from_bits(u64v(b)), 0.0],
                Scalar::ComplexF32 => [
                    f32::from_bits(u32v(&b[..4])) as f64,
                    f32::from_bits(u32v(&b[4..])) as f64,
                ],
            },
        };
        if self.layout.components == ComponentOrder::ImagReal {
            value.swap(0, 1);
        }
        if value.iter().any(|x| !x.is_finite()) {
            return Err(Error::Nonfinite);
        }
        Ok(value)
    }
}
