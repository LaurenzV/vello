// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Types for paints.

use std::sync::Arc;
use peniko::color::{AlphaColor, Srgb};
use peniko::Extend;
use crate::color::PremulColor;

/// A color stop.
#[derive(Debug, Clone, Copy)]
pub struct Stop {
    /// The normalized offset of the stop.
    pub offset: f32,
    /// The color of the stop.
    pub color: AlphaColor<Srgb>,
}

/// A color stop.
#[derive(Debug, Clone)]
pub struct InnerStop {
    /// The normalized offset of the stop.
    pub offset: f32,
    /// The color of the stop.
    pub color: [u8; 4],
}

impl From<Stop> for InnerStop {
    fn from(value: Stop) -> Self {
        Self {
            offset: value.offset,
            color: value.color.premultiply().to_rgba8_fast(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LinearGradient {
    /// The x coordinate of the first point.
    pub x0: f32,
    /// The x coordinate of the second point.
    pub x1: f32,
    /// The color stops of the linear gradient.
    pub stops: Vec<Stop>,
    pub extend: Extend
}

#[derive(Debug, Clone)]
pub struct InnerLinearGradient {
    pub end: f32,
    pub offset: f32,
    pub stops: Vec<InnerStop>,
    pub pad: bool,
}

impl From<LinearGradient> for InnerLinearGradient {
    fn from(value: LinearGradient) -> Self {
        let mut x0 = value.x0;
        let mut x1 = value.x1;
        
        let mut stops = if value.x0 <= value.x1 {
            value.stops.iter().map(|s| {
                let s: InnerStop = (*s).into();
                s
            }).collect()
        }   else {
            std::mem::swap(&mut x0, &mut x1);
            
            value.stops.iter().rev().map(|s| {
                InnerStop {
                    offset: 1.0 - s.offset,
                    color: s.color.premultiply().to_rgba8_fast() 
                }
            }).collect::<Vec<_>>()
        };
        
        if value.extend == Extend::Reflect {
            x1 += x1 - x0;
            
            let first_half = stops.iter().map(|s| InnerStop {
                offset: s.offset / 2.0,
                color: s.color,
            });
            
            let second_half = stops.iter().rev().map(|s| InnerStop {
                offset: 0.5 + (1.0 - s.offset) / 2.0,
                color: s.color,
            });
            
            let combined = first_half.chain(second_half).collect::<Vec<_>>();
            stops = combined;
        }
        
        let offset = -x0;
        
        InnerLinearGradient {
            offset,
            end: x1 + offset,
            stops,
            pad: value.extend == Extend::Pad
        }
    }
}

// TODO: This will probably turn into a generic type where
// vello-hybrid and vello-cpu provide their own instantiations for
// a `Pattern` type.
/// A paint used for filling or stroking paths.
#[derive(Debug, Clone)]
pub enum Paint {
    /// A solid color.
    Solid(AlphaColor<Srgb>),
    /// A gradient.
    Gradient(Arc<InnerLinearGradient>),
    /// A pattern.
    Pattern(()),
}

impl From<AlphaColor<Srgb>> for Paint {
    fn from(value: AlphaColor<Srgb>) -> Self {
        Self::Solid(value)
    }
}

impl From<LinearGradient> for Paint {
    fn from(value: LinearGradient) -> Self {
        Self::Gradient(Arc::new(InnerLinearGradient::from(value)))
    }
}


pub(crate) trait ColorExt {
    /// Using the already-existing `to_rgba8` is slow on x86 because it involves rounding, so
    /// we use a fast method with just + 0.5.
    fn to_rgba8_fast(&self) -> [u8; 4];
}

impl ColorExt for PremulColor<Srgb> {
    #[inline(always)]
    fn to_rgba8_fast(&self) -> [u8; 4] {
        [
            (self.components[0] * 255.0 + 0.5) as u8,
            (self.components[1] * 255.0 + 0.5) as u8,
            (self.components[2] * 255.0 + 0.5) as u8,
            (self.components[3] * 255.0 + 0.5) as u8,
        ]
    }
}

pub(crate) mod scalar {
    #[inline(always)]
    pub(crate) const fn div_255(val: u16) -> u16 {
        (val + 1 + (val >> 8)) >> 8
    }
}
