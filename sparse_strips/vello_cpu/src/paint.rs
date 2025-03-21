use std::sync::Arc;
use vello_common::color::{AlphaColor, Srgb};
use vello_common::paint::{LinearGradient, Paint, Stop};
use vello_common::peniko::Extend;
use crate::util::ColorExt;

#[derive(Clone, Debug)]
pub enum EncodedPaint {
    Solid([u8; 4]),
    LinearGradient(Arc<EncodedLinearGradient>)
}


#[derive(Debug, Clone)]
pub struct EncodedLinearGradient {
    pub end: f32,
    pub offset: f32,
    pub stops: Vec<EncodedStop>,
    pub pad: bool,
}

impl From<LinearGradient> for EncodedLinearGradient {
    fn from(value: LinearGradient) -> Self {
        let mut x0 = value.x0;
        let mut x1 = value.x1;

        let mut stops = if value.x0 <= value.x1 {
            value.stops.iter().map(|s| {
                let s: EncodedStop = (*s).into();
                s
            }).collect()
        }   else {
            std::mem::swap(&mut x0, &mut x1);

            value.stops.iter().rev().map(|s| {
                EncodedStop {
                    offset: 1.0 - s.offset,
                    color: s.color.premultiply().to_rgba8_fast()
                }
            }).collect::<Vec<_>>()
        };

        if value.extend == Extend::Reflect {
            x1 += x1 - x0;

            let first_half = stops.iter().map(|s| EncodedStop {
                offset: s.offset / 2.0,
                color: s.color,
            });

            let second_half = stops.iter().rev().map(|s| EncodedStop {
                offset: 0.5 + (1.0 - s.offset) / 2.0,
                color: s.color,
            });

            let combined = first_half.chain(second_half).collect::<Vec<_>>();
            stops = combined;
        }

        let offset = -x0;

        EncodedLinearGradient {
            offset,
            end: x1 + offset,
            stops,
            pad: value.extend == Extend::Pad
        }
    }
}

/// A color stop.
#[derive(Debug, Clone)]
pub struct EncodedStop {
    /// The normalized offset of the stop.
    pub offset: f32,
    /// The color of the stop.
    pub color: [u8; 4],
}

impl From<Stop> for EncodedStop {
    fn from(value: Stop) -> Self {
        Self {
            offset: value.offset,
            color: value.color.premultiply().to_rgba8_fast(),
        }
    }
}

impl From<Paint> for EncodedPaint {
    fn from(value: Paint) -> Self {
        match value {
            Paint::Solid(c) => c.into(),
            Paint::LinearGradient(l) => l.into(),
            Paint::Pattern(_) => unimplemented!()
        }
    }
}

impl From<LinearGradient> for EncodedPaint {
    fn from(value: LinearGradient) -> Self {
        EncodedPaint::LinearGradient(Arc::new(value.into()))
    }
}

impl From<AlphaColor<Srgb>> for EncodedPaint {
    fn from(value: AlphaColor<Srgb>) -> Self {
        EncodedPaint::Solid(value.premultiply().to_rgba8_fast())
    }
}
