// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Types for paints.

use std::sync::Arc;
use peniko::color::{AlphaColor, Srgb};

/// A color stop.
#[derive(Debug, Clone)]
pub struct Stop {
    /// The normalized offset of the stop.
    pub offset: f32,
    /// The color of the stop.
    pub color: AlphaColor<Srgb>,
}

#[derive(Debug, Clone)]
pub struct LinearGradient {
    /// The x coordinate of the first point.
    pub x0: f32,
    /// The x coordinate of the second point.
    pub x1: f32,
    /// The color stops of the linear gradient.
    pub stops: Vec<Stop>,
    pub extend: peniko::Extend
}

#[derive(Debug, Clone)]
pub struct InnerLinearGradient {
    pub x0: f32,
    /// The x coordinate of the second point.
    pub x1: f32,
    pub stops: Vec<Stop>,
}

impl From<LinearGradient> for InnerLinearGradient {
    fn from(value: LinearGradient) -> Self {
        let mut x0 = value.x0;
        let mut x1 = value.x1;
        
        let stops = if value.x0 <= value.x1 {
            value.stops.clone()
        }   else {
            std::mem::swap(&mut x0, &mut x1);
            value.stops.iter().rev().map(|s| {
                Stop {
                    offset: 1.0 - s.offset,
                    color: s.color 
                }
            }).collect::<Vec<_>>()
        };
        
        InnerLinearGradient {
            x0,
            x1,
            stops,
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
