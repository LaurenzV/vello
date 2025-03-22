use crate::util::ColorExt;
use std::sync::Arc;
use vello_common::color::{AlphaColor, Srgb};
use vello_common::paint::{LinearGradient, Paint, Stop};
use vello_common::peniko::Extend;

#[derive(Clone, Debug)]
pub enum EncodedPaint {
    Solid([u8; 4]),
    LinearGradient(Arc<EncodedLinearGradient>),
}

#[derive(Debug, Clone)]
pub struct EncodedLinearGradient {
    pub end: f32,
    pub offsets: (f32, f32),
    pub advances: (f32, f32),
    pub stops: Vec<EncodedStop>,
    pub pad: bool,
    pub has_opacities: bool,
}

impl From<LinearGradient> for EncodedLinearGradient {
    fn from(value: LinearGradient) -> Self {
        let mut p0 = value.p0;
        let mut p1 = value.p1;

        let has_opacities = value.stops.iter().any(|s| s.color.components[3] != 1.0);

        let mut stops = if value.p0.x <= value.p1.x {
            value
                .stops
                .iter()
                .map(|s| {
                    let s: EncodedStop = (*s).into();
                    s
                })
                .collect()
        } else {
            std::mem::swap(&mut p0, &mut p1);

            value
                .stops
                .iter()
                .rev()
                .map(|s| EncodedStop {
                    offset: 1.0 - s.offset,
                    color: s.color.premultiply().to_rgba8_fast(),
                })
                .collect::<Vec<_>>()
        };

        // Double the length of the iterator, and append stops in reverse order.
        // Then we can treat it the same as repeated gradients.
        if value.extend == Extend::Reflect {
            p1.x += p1.x - p0.x;

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

        let x_offset = -p0.x as f32;
        let y_offset = -p0.y as f32;

        let dx = p1.x as f32 + x_offset;
        let dy = p1.y as f32 + y_offset;

        let dy_dx = dy / dx;
        let dx_dy = dx / dy;

        // How much do we advance in the direction of the gradient, when taking one step to the right
        // (i.e. when processing a new column in the strip)?
        let x_advance = (1.0 + dy_dx * dy_dx).sqrt();
        // How much do we advance in the direction of the gradient, when taking one step to the bottom
        // (i.e. when processing a new pixel in the current column)?
        let y_advance = (1.0 + dx_dy * dx_dy).sqrt();

        EncodedLinearGradient {
            offsets: (x_offset, y_offset),
            advances: (x_advance, y_advance),
            end: p1.x as f32 + x_offset,
            stops,
            pad: value.extend == Extend::Pad,
            has_opacities,
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
            Paint::Pattern(_) => unimplemented!(),
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
