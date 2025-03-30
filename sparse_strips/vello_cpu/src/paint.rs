use crate::fine::COLOR_COMPONENTS;
use crate::util::ColorExt;
use std::f32::consts::PI;
use std::iter;
use std::sync::Arc;
use vello_common::color::{AlphaColor, Srgb};
use vello_common::paint::{LinearGradient, Paint, Stop, SweepGradient};
use vello_common::peniko::Extend;

#[derive(Clone, Debug)]
pub enum EncodedPaint {
    Solid([u8; 4]),
    LinearGradient(Arc<EncodedLinearGradient>),
    SweepGradient(Arc<EncodedSweepGradient>),
}

#[derive(Debug)]
pub struct EncodedSweepGradient {
    pub rotation: f32,
    pub end_angle: f32,
    pub offsets: (f32, f32),
    pub stops: Vec<EncodedSweepStop>,
    pub pad: bool,
    pub has_opacities: bool,
}

impl From<SweepGradient> for EncodedSweepGradient {
    fn from(value: SweepGradient) -> Self {
        let mut start_angle = value.start_angle * (PI / 180.0);
        let mut end_angle = value.end_angle * (PI / 180.0);

        let has_opacities = value.stops.iter().any(|s| s.color.components[3] != 1.0);

        let mut stops = if start_angle <= end_angle {
            value
                .stops
                .iter()
                .map(|s| {
                    let s: EncodedSweepStop = (*s).into();
                    s
                })
                .collect()
        } else {
            std::mem::swap(&mut start_angle, &mut end_angle);

            value
                .stops
                .iter()
                .rev()
                .map(|s| EncodedSweepStop {
                    offset: 1.0 - s.offset,
                    color: s.color.premultiply().to_rgba8_fast(),
                })
                .collect::<Vec<_>>()
        };

        let offsets = (-value.center.x as f32, -value.center.y as f32);

        Self {
            rotation: -start_angle,
            end_angle: end_angle - start_angle,
            offsets,
            stops,
            pad: true,
            has_opacities,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EncodedLinearGradient {
    pub end: f32,
    pub offsets: (f32, f32),
    pub advances: (f32, f32),
    // Below are the factors that will be used to later on calculate
    // the distance of a strip to the line making up the gradient. Basis of the formula
    // is https://en.wikipedia.org/wiki/Distance_from_a_point_to_a_line#Line_defined_by_two_points
    // sqrt((y2 - y1)ˆ2 + (x2 - x1)ˆ2)
    pub denom: f32,
    // (y2 - y1)
    pub fact1: f32,
    // (x2 - x1)
    pub fact2: f32,
    pub ranges: Vec<GradientRange>,
    pub pad: bool,
    pub has_opacities: bool,
    pub sign: i8,
}

impl From<LinearGradient> for EncodedLinearGradient {
    fn from(value: LinearGradient) -> Self {
        let mut p0 = value.p0;
        let mut p1 = value.p1;

        let has_opacities = value.stops.iter().any(|s| s.color.components[3] != 1.0);

        let mut stops = if value.p0.x <= value.p1.x {
            value.stops
        } else {
            std::mem::swap(&mut p0, &mut p1);

            value
                .stops
                .iter()
                .rev()
                .map(|s| Stop {
                    offset: 1.0 - s.offset,
                    color: s.color,
                })
                .collect::<Vec<_>>()
        };

        let sign = if p0.y < p1.y { 1 } else { -1 };

        // Double the length of the iterator, and append stops in reverse order.
        // Then we can treat it the same as repeated gradients.
        if value.extend == Extend::Reflect {
            p1.x += p1.x - p0.x;
            p1.y += p1.y - p0.y;

            let first_half = stops.iter().map(|s| Stop {
                offset: s.offset / 2.0,
                color: s.color,
            });

            let second_half = stops.iter().rev().map(|s| Stop {
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
        let norm = (-dy, dx);

        let denom = (norm.1 * norm.1 + norm.0 * norm.0).sqrt();
        let fact1 = norm.1;
        let fact2 = norm.0;

        // How much do we advance in the direction of the gradient, when taking one step to the right
        // (i.e. when processing a new column in the strip)?
        let x_advance = if dx == 0.0 {
            0.0
        } else {
            let dy_dx = dy / dx;
            1.0 / (1.0 + dy_dx * dy_dx).sqrt()
        };
        // How much do we advance in the direction of the gradient, when taking one step to the bottom
        // (i.e. when processing a new pixel in the current column)?
        let y_advance = if dy == 0.0 {
            0.0
        } else {
            let dx_dy = dx / dy;
            1.0 / (1.0 + dx_dy * dx_dy).sqrt()
        };

        let end = (dx * dx + dy * dy).sqrt();

        let create_range = |left_stop: &Stop, right_stop: &Stop| {
            let x0 = end * left_stop.offset;
            let x1 = end * right_stop.offset;
            let c0 = left_stop.color.premultiply().to_rgba8_fast();
            let c1 = right_stop.color.premultiply().to_rgba8_fast();

            let mut im1 = [0.0; 4];
            let im2 = x1 - x0;
            let mut im3 = [0.0; 4];

            for i in 0..COLOR_COMPONENTS {
                im1[i] = c1[i] as f32 - c0[i] as f32;
                im3[i] = im1[i] / im2;
            }

            GradientRange {
                x0,
                x1,
                c0,
                c1,
                im1,
                im2,
                im3,
            }
        };

        let stop_ranges = stops.windows(2).map(|s| {
            let left_stop = &s[0];
            let right_stop = &s[1];

            create_range(left_stop, right_stop)
        });

        let pad = value.extend == Extend::Pad;
        let ranges = if pad {
            let left_range = iter::once({
                let first_stop = &stops[0];
                let mut encoded_range = create_range(first_stop, first_stop);
                encoded_range.x0 = f32::MIN;

                encoded_range
            });

            let right_range = iter::once({
                let last_stop = stops.last().unwrap();

                let mut encoded_range = create_range(&last_stop, &last_stop);
                encoded_range.x1 = f32::MAX;

                encoded_range
            });

            left_range.chain(stop_ranges.chain(right_range)).collect()
        } else {
            stop_ranges.collect()
        };

        EncodedLinearGradient {
            offsets: (x_offset, y_offset),
            advances: (x_advance, y_advance),
            denom,
            fact1,
            fact2,
            end: (dx * dx + dy * dy).sqrt(),
            ranges,
            pad: value.extend == Extend::Pad,
            has_opacities,
            sign,
        }
    }
}

/// A color stop.
#[derive(Debug, Clone)]
pub struct GradientRange {
    pub(crate) x0: f32,
    pub(crate) x1: f32,
    pub(crate) c0: [u8; 4],
    pub(crate) c1: [u8; 4],
    pub(crate) im1: [f32; 4],
    pub(crate) im2: f32,
    pub(crate) im3: [f32; 4],
}

/// A color stop.
#[derive(Debug, Clone)]
pub struct EncodedSweepStop {
    /// The normalized offset of the stop.
    pub offset: f32,
    /// The color of the stop.
    pub color: [u8; 4],
}

impl From<Stop> for EncodedSweepStop {
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
            Paint::SweepGradient(s) => s.into(),
            Paint::Pattern(_) => unimplemented!(),
        }
    }
}

impl From<SweepGradient> for EncodedPaint {
    fn from(value: SweepGradient) -> Self {
        EncodedPaint::SweepGradient(Arc::new(value.into()))
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
